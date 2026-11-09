//! Positioning helper shared by `Popover`, `DropdownMenu`, `ContextMenu` and `Tooltip`.
//!
//! * [`Side`] and [`Align`] choose which side of the trigger the content opens on and how it lines up along that edge.
//! * [`Floating`] renders content in a `deferred` + `anchored` layer next to an anchor rectangle,
//!   flipping to the opposite side when the preferred side does not fit (needs the content size, which is measured
//!   during the first frame: that frame is drawn invisible).
//! * [`FloatState`] is the keyed state every overlay trigger keeps (`open`, trigger bounds, measured size, focus).
use std::{cell::Cell, rc::Rc};

use gpui_kit::{
    Anchor, AnyElement, App, Bounds, Context, ElementId, FocusHandle, InteractiveElement as _, IntoElement, KeyBinding,
    ParentElement, Pixels, Point, RenderOnce, Size, Styled, Subscription, Window, actions, anchored, canvas, deferred,
    div, point, prelude::FluentBuilder as _, px, size,
};

use super::menu::MenuView;

actions!(shadcn_overlay, [OverlayUp, OverlayDown, OverlayLeft, OverlayRight, OverlayConfirm, OverlayCancel]);

pub(crate) const MENU_CONTEXT: &str = "ShadcnMenu";
pub(crate) const POPOVER_CONTEXT: &str = "ShadcnPopover";

/// Deferred paint priorities of our overlay layers, chosen against gpui-base 0.9.0 (`gpui_base::POPUP_PRIORITY` = 100,
/// `TOOLTIP_PRIORITY` = 200, both not re-exported by gpui-kit) and its dialogs (`10 + layer`): popups must draw above
/// every dialog layer (including stacked ones) and above our toast layer (50, `crate::toast`), tooltips above popups.
pub(crate) const POPUP_PRIORITY: usize = 100;
pub(crate) const TOOLTIP_PRIORITY: usize = 200;

struct KeysBound;
impl gpui_kit::Global for KeysBound {}

/// Binds the keyboard shortcuts of menus/popovers once per app (lazily, so no init hook is required).
pub(crate) fn ensure_keys(cx: &mut App) {
    if cx.has_global::<KeysBound>() {
        return;
    }
    cx.set_global(KeysBound);
    cx.bind_keys([
        KeyBinding::new("up", OverlayUp, Some(MENU_CONTEXT)),
        KeyBinding::new("down", OverlayDown, Some(MENU_CONTEXT)),
        KeyBinding::new("left", OverlayLeft, Some(MENU_CONTEXT)),
        KeyBinding::new("right", OverlayRight, Some(MENU_CONTEXT)),
        KeyBinding::new("enter", OverlayConfirm, Some(MENU_CONTEXT)),
        KeyBinding::new("space", OverlayConfirm, Some(MENU_CONTEXT)),
        KeyBinding::new("escape", OverlayCancel, Some(MENU_CONTEXT)),
        KeyBinding::new("escape", OverlayCancel, Some(POPOVER_CONTEXT)),
    ]);
}

/// Which side of the trigger the content opens on.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Side {
    Top,
    Right,
    #[default]
    Bottom,
    Left,
}

impl Side {
    pub fn opposite(self) -> Self {
        match self {
            Side::Top => Side::Bottom,
            Side::Bottom => Side::Top,
            Side::Left => Side::Right,
            Side::Right => Side::Left,
        }
    }
}

/// How the content lines up along the trigger's edge: with its start (left or top), centered, or with its end
/// (right or bottom).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Align {
    Start,
    #[default]
    Center,
    End,
}

#[derive(Clone, Copy, Debug)]
pub struct Placement {
    pub side: Side,
    pub align: Align,
    pub side_offset: f32,
    pub align_offset: f32,
}

impl Placement {
    pub fn new(side: Side, align: Align, side_offset: f32, align_offset: f32) -> Self {
        Self { side, align, side_offset, align_offset }
    }

    fn origin(&self, side: Side, anchor: Bounds<Pixels>, content: Size<Pixels>) -> Point<Pixels> {
        let off = px(self.side_offset);
        let ao = px(self.align_offset);
        let horizontal = |align: Align| match align {
            Align::Start => anchor.left() + ao,
            Align::Center => anchor.center().x - content.width / 2.0 + ao,
            Align::End => anchor.right() - content.width - ao,
        };
        let vertical = |align: Align| match align {
            Align::Start => anchor.top() + ao,
            Align::Center => anchor.center().y - content.height / 2.0 + ao,
            Align::End => anchor.bottom() - content.height - ao,
        };
        match side {
            Side::Bottom => point(horizontal(self.align), anchor.bottom() + off),
            Side::Top => point(horizontal(self.align), anchor.top() - off - content.height),
            Side::Right => point(anchor.right() + off, vertical(self.align)),
            Side::Left => point(anchor.left() - off - content.width, vertical(self.align)),
        }
    }

    /// Top-left origin of the content, flipping the side when the preferred one overflows `area` and the opposite fits.
    pub fn resolve(&self, anchor: Bounds<Pixels>, content: Size<Pixels>, area: Bounds<Pixels>) -> Point<Pixels> {
        let fits = |side: Side| {
            let o = self.origin(side, anchor, content);
            match side {
                Side::Bottom => o.y + content.height <= area.bottom(),
                Side::Top => o.y >= area.top(),
                Side::Right => o.x + content.width <= area.right(),
                Side::Left => o.x >= area.left(),
            }
        };
        let side = if fits(self.side) || !fits(self.side.opposite()) { self.side } else { self.side.opposite() };
        self.origin(side, anchor, content)
    }
}

/// Window area available to overlays (inside the client-side shadow margin).
pub fn window_area(window: &Window) -> Bounds<Pixels> {
    let pad = gpui_kit::component::window_paddings(window);
    let vp = window.viewport_size();
    Bounds {
        origin: point(pad.left, pad.top),
        size: size(vp.width - pad.left - pad.right, vp.height - pad.top - pad.bottom),
    }
}

/// Content layer attached to `anchor`. See the module docs.
#[derive(IntoElement)]
pub struct Floating {
    anchor: Bounds<Pixels>,
    measured: Rc<Cell<Option<Size<Pixels>>>>,
    placement: Placement,
    child: AnyElement,
    priority: usize,
    occlude: bool,
}

impl Floating {
    pub fn new(
        anchor: Bounds<Pixels>,
        measured: Rc<Cell<Option<Size<Pixels>>>>,
        placement: Placement,
        child: impl IntoElement,
    ) -> Self {
        Self { anchor, measured, placement, child: child.into_any_element(), priority: POPUP_PRIORITY, occlude: true }
    }

    pub fn priority(mut self, priority: usize) -> Self {
        self.priority = priority;
        self
    }

    /// Tooltips must not capture the pointer (otherwise hovering them would end the trigger hover).
    pub fn occlude(mut self, occlude: bool) -> Self {
        self.occlude = occlude;
        self
    }
}

thread_local! {
    /// > 0 while the children of a `Floating` (a `deferred` layer) are being laid out: nested overlays must not
    /// `defer` again (gpui panics: "cannot call defer_draw during deferred drawing").
    static DEFERRED_DEPTH: Cell<usize> = const { Cell::new(0) };
}

fn in_deferred_layer() -> bool {
    DEFERRED_DEPTH.with(|d| d.get() > 0)
}

/// Transparent element that marks its subtree as living inside a deferred layer (see `DEFERRED_DEPTH`). Components
/// are rendered during `request_layout`, so nested `RenderOnce::render` calls observe the counter.
struct DeferredScope {
    child: AnyElement,
}

impl IntoElement for DeferredScope {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}

impl gpui_kit::Element for DeferredScope {
    type RequestLayoutState = ();
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&gpui_kit::GlobalElementId>,
        _inspector_id: Option<&gpui_kit::InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (gpui_kit::LayoutId, ()) {
        DEFERRED_DEPTH.with(|d| d.set(d.get() + 1));
        let layout = self.child.request_layout(window, cx);
        DEFERRED_DEPTH.with(|d| d.set(d.get() - 1));
        (layout, ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&gpui_kit::GlobalElementId>,
        _inspector_id: Option<&gpui_kit::InspectorElementId>,
        _bounds: Bounds<Pixels>,
        _state: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        self.child.prepaint(window, cx);
    }

    fn paint(
        &mut self,
        _id: Option<&gpui_kit::GlobalElementId>,
        _inspector_id: Option<&gpui_kit::InspectorElementId>,
        _bounds: Bounds<Pixels>,
        _state: &mut (),
        _prepaint: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        self.child.paint(window, cx);
    }
}

impl RenderOnce for Floating {
    fn render(self, window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let view_id = window.current_view();
        let area = window_area(window);
        let measured = self.measured;
        let (origin, hidden) = match measured.get() {
            Some(content) => (self.placement.resolve(self.anchor, content, area), false),
            // not measured yet: draw invisible in the window corner (away from the pointer) for one frame
            None => (area.origin, true),
        };
        let nested = in_deferred_layer();
        let layer = anchored().position(origin).anchor(Anchor::TopLeft).snap_to_window_with_margin(px(8.0)).child(
            div()
                .relative()
                .when(self.occlude, |d| d.occlude())
                .when(hidden, |d| d.opacity(0.0))
                .child(DeferredScope { child: self.child })
                .child(
                    canvas(
                        move |bounds, window, cx| {
                            if measured.get() != Some(bounds.size) {
                                measured.set(Some(bounds.size));
                                cx.notify(view_id);
                                window.request_animation_frame();
                            }
                        },
                        |_, _, _, _| {},
                    )
                    .absolute()
                    .inset_0(),
                ),
        );
        if nested {
            // already inside a deferred layer: paint inline (z-order = tree order)
            layer.into_any_element()
        } else {
            deferred(layer).with_priority(self.priority).into_any_element()
        }
    }
}

/// State kept (keyed by the overlay's element id) by popovers / menus / tooltips.
pub(crate) struct FloatState {
    pub open: bool,
    /// Bounds of the trigger (or the click point for context menus), updated every frame.
    pub anchor: Bounds<Pixels>,
    pub measured: Rc<Cell<Option<Size<Pixels>>>>,
    pub focus: FocusHandle,
    pub prev_focus: Option<FocusHandle>,
    pub menu: Option<gpui_kit::Entity<MenuView>>,
    pub _subscription: Option<Subscription>,
}

impl FloatState {
    pub fn new(open: bool, cx: &mut Context<Self>) -> Self {
        Self {
            open,
            anchor: Bounds::default(),
            measured: Rc::new(Cell::new(None)),
            focus: cx.focus_handle(),
            prev_focus: None,
            menu: None,
            _subscription: None,
        }
    }
}

/// Records the bounds of its parent into `state.anchor` (absolute, fills the parent; add as a child of the trigger).
pub(crate) fn track_bounds(state: gpui_kit::Entity<FloatState>) -> impl IntoElement {
    canvas(
        move |bounds, _, cx| {
            state.update(cx, |s, _| s.anchor = bounds);
        },
        |_, _, _, _| {},
    )
    .absolute()
    .inset_0()
}

/// Element id from an explicit `.id(..)` or the `#[track_caller]` call site of the widget constructor.
pub(crate) fn element_id(id: Option<ElementId>, loc: &'static std::panic::Location<'static>) -> ElementId {
    id.unwrap_or_else(|| ElementId::from(loc))
}
