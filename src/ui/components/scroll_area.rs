//! `ScrollArea`: the single entry point for scrolling regions.
//!
//! Built on gpui-component's `Scrollbar` (always visible, thumb `scrollbar_thumb` = white at 20% opacity, hover/drag
//! `scrollbar_thumb_hover` = white at 80% from the mapped theme). The thumb is 6px wide at rest and widens to 8px
//! while hovered or dragged (gpui-kit defaults). It is inset 4px from the right edge, so it never overlaps the
//! window's resize border.
use super::style_ext::StyleMergeExt as _;
use gpui_kit::component::{
    StyledExt as _,
    scroll::{Scrollbar, ScrollbarHandle, ScrollbarMode},
};
use gpui_kit::{
    AnyElement, App, Div, ElementId, InteractiveElement, IntoElement, ParentElement, RenderOnce, ScrollHandle,
    StatefulInteractiveElement, StyleRefinement, Styled, Window, div, relative,
};

pub use gpui_kit::component::scroll::ScrollbarAxis;

/// A box that fills its parent and holds `child`, like an absolute `div` with all insets 0.
///
/// It is not a `div` on purpose. The GPUI inspector gives every div a hitbox, so a full-size overlay div hides
/// everything under it from the inspector. Use this for full-size overlays instead.
pub fn fill_overlay(child: impl IntoElement) -> impl IntoElement {
    FillOverlay { child: child.into_any_element() }
}

struct FillOverlay {
    child: AnyElement,
}

impl IntoElement for FillOverlay {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}

impl gpui_kit::Element for FillOverlay {
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
        let child = self.child.request_layout(window, cx);
        let zero = gpui_kit::Length::Definite(gpui_kit::px(0.0).into());
        let style = gpui_kit::Style {
            position: gpui_kit::Position::Absolute,
            inset: gpui_kit::Edges { top: zero, right: zero, bottom: zero, left: zero },
            ..Default::default()
        };
        (window.request_layout(style, [child], cx), ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&gpui_kit::GlobalElementId>,
        _inspector_id: Option<&gpui_kit::InspectorElementId>,
        _bounds: gpui_kit::Bounds<gpui_kit::Pixels>,
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
        _bounds: gpui_kit::Bounds<gpui_kit::Pixels>,
        _state: &mut (),
        _prepaint: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        self.child.paint(window, cx);
    }
}

/// Always-visible scrollbar for `handle`. Put it as the last child of a `.relative()` container: the bar is laid out
/// along that container's edge, not the handle's viewport, so a scrolled element inset by padding or a centred
/// max-width column still gets its bar at the container edge. Used by [`ScrollArea`]; also usable for
/// `uniform_list`/`list` handles (`UniformListScrollHandle`, `ListState`).
pub fn scrollbar_overlay<H: ScrollbarHandle + Clone>(
    id: impl Into<ElementId>,
    handle: &H,
    axis: ScrollbarAxis,
) -> impl IntoElement {
    fill_overlay(Scrollbar::new(handle).id(id).axis(axis).mode(ScrollbarMode::Always).viewport_from_layout())
}

/// A scrolling region with a visible scrollbar. Vertical by default.
///
/// ```ignore
/// ScrollArea::new("addon-list").flex_1().min_h_0().child(div().p_6().child("long content"))
/// ScrollArea::new("code").axis(ScrollbarAxis::Both).child(..)
/// // programmatic scrolling (chat auto-scroll): keep a `ScrollHandle` in your view and pass it in
/// ScrollArea::new("chat").track_scroll(&self.scroll).child(..);  self.scroll.scroll_to_bottom();
/// ```
/// The root fills its parent and clips its content. Give it a bounded size from its parent (`.flex_1().min_h_0()`,
/// `.h(px(300.))`, ...) or it will grow with the content and never scroll. `Styled` applies to the root. Children
/// are laid out in a block content box that fills the viewport, style that box with
/// `.content_style(|d| d.p_6().flex().flex_col().gap_4())`.
#[derive(IntoElement)]
pub struct ScrollArea {
    id: ElementId,
    axis: ScrollbarAxis,
    handle: Option<ScrollHandle>,
    children: Vec<AnyElement>,
    content_style: Option<Box<dyn FnOnce(Div) -> Div>>,
    style: StyleRefinement,
}

impl ScrollArea {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            axis: ScrollbarAxis::Vertical,
            handle: None,
            children: Vec::new(),
            content_style: None,
            style: StyleRefinement::default(),
        }
    }

    pub fn axis(mut self, axis: ScrollbarAxis) -> Self {
        self.axis = axis;
        self
    }

    /// Use an externally owned handle (to read/set the offset, `scroll_to_bottom`, ...).
    pub fn track_scroll(mut self, handle: &ScrollHandle) -> Self {
        self.handle = Some(handle.clone());
        self
    }

    /// Style the content box wrapping the children (padding, flex layout, gap).
    pub fn content_style(mut self, f: impl FnOnce(Div) -> Div + 'static) -> Self {
        self.content_style = Some(Box::new(f));
        self
    }
}

impl ParentElement for ScrollArea {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Styled for ScrollArea {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for ScrollArea {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let handle = match self.handle {
            Some(h) => h,
            None => window
                .use_keyed_state((self.id.clone(), "scroll-handle"), cx, |_, _| ScrollHandle::default())
                .read(cx)
                .clone(),
        };

        let mut content = match self.axis {
            ScrollbarAxis::Vertical => div().flex_1().w_full(),
            ScrollbarAxis::Horizontal => div().flex_none().h_full().min_w(relative(1.0)),
            ScrollbarAxis::Both => {
                super::card::align_self_start(div().flex_none().min_w(relative(1.0)).min_h(relative(1.0)))
            }
        };
        if let Some(f) = self.content_style {
            content = f(content);
        }
        let content = content.children(self.children);

        let viewport = div().id("scroll-area-viewport").flex().size_full().track_scroll(&handle);
        let mut viewport = match self.axis {
            ScrollbarAxis::Vertical => viewport.flex_col().overflow_y_scroll(),
            ScrollbarAxis::Horizontal => viewport.flex_row().overflow_x_scroll(),
            ScrollbarAxis::Both => viewport.flex_col().overflow_scroll(),
        };
        // gpui turns a vertical wheel delta into horizontal scroll on x-only containers, without this, wheeling the
        // page past a horizontal strip (e.g. a readme code block) scrolls the strip sideways.
        if self.axis == ScrollbarAxis::Horizontal {
            viewport.style().restrict_scroll_to_axis = Some(true);
        }

        div()
            .id(self.id)
            .relative()
            .size_full()
            .overflow_hidden()
            .merge_style(&self.style)
            .child(viewport.child(content))
            .child(scrollbar_overlay("scrollbar", &handle, self.axis))
    }
}
