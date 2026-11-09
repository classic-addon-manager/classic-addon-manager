//! Modal dialogs, opened imperatively through gpui-kit's `Root` dialog layer (hosted by `Root` itself). Focus is moved
//! into the dialog, and Escape, a click on the overlay or the X button close it.
//!
//! ```ignore
//! dialog::open(window, cx, DialogSpec::new()
//!     .title("Edit profile").description("Make changes to your profile here.")
//!     .max_width(px(425.))                                   // default 512
//!     .content(move |window, cx| div().child(my_input.clone()))   // called every frame: keep entities captured
//!     .footer(|window, cx| DialogFooter::new()
//!         .child(Button::new("cancel").variant(ButtonVariant::Outline).label("Cancel")
//!             .on_click(|_, window, cx| dialog::close(window, cx)))
//!         .child(Button::new("save").label("Save").on_click(|_, window, cx| dialog::close(window, cx)))));
//! ```
//! Custom layouts (remote addon / chat / update dialogs): `DialogSpec::new().padded(false).content_view(entity)`
//! gives an empty card with no padding or gap, holding your own view. Size it with
//! `.max_width / .min_width / .max_width_frac / .height / .max_height / .height_frac / .max_height_frac`.
//! Close from anywhere with `dialog::close(window, cx)` (the topmost dialog). `ModalFrame` is the bare card chrome.
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

use gpui_kit::component::{
    IconNamed, WindowExt as _,
    animation::cubic_bezier,
    dialog::{Dialog, DialogEntrance},
};
use gpui_kit::{
    Animation, AnimationExt as _, AnyElement, AnyView, App, ClickEvent, ElementId, FocusHandle, Hsla,
    InteractiveElement as _, IntoElement, MouseButton, ParentElement, Pixels, RenderOnce, SharedString,
    StatefulInteractiveElement as _, StyleRefinement, Styled, Window, div, hsla, prelude::FluentBuilder as _, px, svg,
    transparent_black,
};

use super::style_ext::StyleMergeExt as _;
use super::{floating::window_area, scroll_area::fill_overlay};
use crate::{
    icons,
    theme::{TokensExt, Tw, TwText, alpha, radius, shadow_2xl, tw},
};

pub type DialogHandler = Rc<dyn Fn(&mut Window, &mut App)>;
type ContentFn = Rc<dyn Fn(&mut Window, &mut App) -> AnyElement>;
type LockFn = Rc<dyn Fn(&App) -> bool>;

/// Closes the topmost dialog (`window.close_dialog(cx)`).
pub fn close(window: &mut Window, cx: &mut App) {
    window.close_dialog(cx);
}

/// Closes every open dialog.
pub fn close_all(window: &mut Window, cx: &mut App) {
    window.close_all_dialogs(cx);
}

/// `true` while a dialog is open.
pub fn is_open(window: &mut Window, cx: &mut App) -> bool {
    window.has_active_dialog(cx)
}

/// Dim colour behind dialogs (black, 50%).
fn backdrop_color() -> Hsla {
    hsla(0.0, 0.0, 0.0, 0.5)
}

/// The dim behind dialogs. The app view draws it, not gpui-kit.
///
/// gpui-kit's dim fills the whole OS window. On Linux that includes the clear 20px shadow border around the app, so
/// the dim showed as a dark box around the window. The app view sits inside that border, so this dim does not.
/// gpui-kit's dim is still there to handle clicks, but it is transparent (see `theme.overlay`).
///
/// Dialogs under the top one dim their own card (see [`open_layer`]).
pub fn backdrop(window: &mut Window, cx: &mut App) -> Option<impl IntoElement> {
    is_open(window, cx).then(|| fill_overlay(div().size_full().bg(backdrop_color())))
}

thread_local! {
    /// Ids of the open dialogs, bottom to top, in the same order as gpui-kit's list.
    static STACK: RefCell<Vec<u64>> = const { RefCell::new(Vec::new()) };
    static NEXT_ID: Cell<u64> = const { Cell::new(0) };
}

/// One entry in [`STACK`], owned by the dialog's build closure. gpui-kit drops that closure when the dialog closes,
/// which removes the entry.
struct StackEntry(u64);

impl StackEntry {
    fn push() -> Self {
        let id = NEXT_ID.with(|n| {
            let id = n.get();
            n.set(id + 1);
            id
        });
        STACK.with(|s| s.borrow_mut().push(id));
        Self(id)
    }

    fn is_topmost(&self) -> bool {
        STACK.with(|s| s.borrow().last() == Some(&self.0))
    }
}

impl Drop for StackEntry {
    fn drop(&mut self) {
        STACK.with(|s| s.borrow_mut().retain(|&id| id != self.0));
    }
}

/// Opens a dialog and tracks it in [`STACK`]. Every dialog must open through this. `build` gets `topmost = false`
/// while another dialog is open above this one.
pub fn open_layer(
    window: &mut Window,
    cx: &mut App,
    build: impl Fn(Dialog, bool, &mut Window, &mut App) -> Dialog + 'static,
) {
    let entry = StackEntry::push();
    window.open_dialog(cx, move |dialog, window, cx| build(dialog, entry.is_topmost(), window, cx));
}

/// Dialog description for [`open`]. All closures are called on every frame while the dialog is open.
pub struct DialogSpec {
    pub(crate) title: Option<SharedString>,
    pub(crate) description: Option<SharedString>,
    pub(crate) content: Option<ContentFn>,
    pub(crate) footer: Option<ContentFn>,
    pub(crate) padded: bool,
    pub(crate) show_close: bool,
    pub(crate) escape_closes: bool,
    pub(crate) overlay_closes: bool,
    /// While it answers `true` the dialog ignores X / Escape / overlay clicks (busy states; see [`DialogSpec::locked`]).
    pub(crate) locked: Option<LockFn>,
    pub(crate) on_close: Option<DialogHandler>,
    pub(crate) max_width: Pixels,
    pub(crate) max_width_frac: Option<f32>,
    pub(crate) min_width: Option<Pixels>,
    pub(crate) height: Option<Pixels>,
    pub(crate) height_frac: Option<f32>,
    pub(crate) max_height: Option<Pixels>,
    pub(crate) max_height_frac: Option<f32>,
    pub(crate) min_height: Option<Pixels>,
    /// Alert dialog titles keep the normal large-text line height (28 px) instead of the tight 18 px one.
    pub(crate) alert_title: bool,
    /// Red-tinted card border (error dialogs).
    pub(crate) destructive: bool,
    /// Orange-tinted card border (warning dialogs).
    pub(crate) warning: bool,
}

impl Default for DialogSpec {
    fn default() -> Self {
        Self::new()
    }
}

impl DialogSpec {
    pub fn new() -> Self {
        Self {
            title: None,
            description: None,
            content: None,
            footer: None,
            padded: true,
            show_close: true,
            escape_closes: true,
            overlay_closes: true,
            locked: None,
            on_close: None,
            max_width: px(512.0),
            max_width_frac: None,
            min_width: None,
            height: None,
            height_frac: None,
            max_height: None,
            max_height_frac: None,
            min_height: None,
            alert_title: false,
            destructive: false,
            warning: false,
        }
    }

    /// `DialogTitle` (inside a `DialogHeader`).
    pub fn title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// `DialogDescription` (inside the `DialogHeader`).
    pub fn description(mut self, description: impl Into<SharedString>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Body, between header and footer.
    pub fn content<E: IntoElement>(mut self, content: impl Fn(&mut Window, &mut App) -> E + 'static) -> Self {
        self.content = Some(Rc::new(move |w, cx| content(w, cx).into_any_element()));
        self
    }

    /// Body = an entity (any `Entity<impl Render>` / `AnyView`) that lives as long as you keep it (captured).
    pub fn content_view(mut self, view: impl Into<AnyView>) -> Self {
        let view: AnyView = view.into();
        self.content = Some(Rc::new(move |_, _| view.clone().into_any_element()));
        self
    }

    /// Footer below the body (usually a `DialogFooter` with buttons).
    pub fn footer<E: IntoElement>(mut self, footer: impl Fn(&mut Window, &mut App) -> E + 'static) -> Self {
        self.footer = Some(Rc::new(move |w, cx| footer(w, cx).into_any_element()));
        self
    }

    /// Card padding of 24 px with 16 px between children (default). `false` removes both: the content draws everything,
    /// including its own padding, but the title, description and footer are still added if set.
    pub fn padded(mut self, padded: bool) -> Self {
        self.padded = padded;
        self
    }

    /// Whether to show the X button at the top right. Default true.
    pub fn show_close(mut self, show: bool) -> Self {
        self.show_close = show;
        self
    }

    /// Escape key closes (default true). Set false for non-dismissable busy states.
    pub fn escape_closes(mut self, closes: bool) -> Self {
        self.escape_closes = closes;
        self
    }

    /// Clicking the dimmed overlay closes (default true).
    pub fn overlay_closes(mut self, closes: bool) -> Self {
        self.overlay_closes = closes;
        self
    }

    /// Dynamic version of `modal()`: while `is_locked` answers `true` the X disappears and Escape and overlay clicks
    /// are ignored. Use it for busy states that must not be dismissed. It is asked on every frame, so it should read
    /// the busy state from an entity: a notify on that entity redraws the dialog.
    pub fn locked(mut self, is_locked: impl Fn(&App) -> bool + 'static) -> Self {
        self.locked = Some(Rc::new(is_locked));
        self
    }

    /// Shorthand: no X, no Escape, no overlay click (the dialog can only be closed from its content).
    pub fn modal(self) -> Self {
        self.show_close(false).escape_closes(false).overlay_closes(false)
    }

    /// Called for user-initiated closes (X, Escape, overlay click). Not called for `dialog::close`.
    pub fn on_close(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_close = Some(Rc::new(handler));
        self
    }

    /// Widest the dialog may get (default `px(512.)`). It is as wide as that, or as the window minus a 32 px margin if
    /// the window is narrower.
    pub fn max_width(mut self, width: Pixels) -> Self {
        self.max_width = width;
        self
    }

    /// Red-tinted card border, for error dialogs.
    pub fn destructive(mut self, destructive: bool) -> Self {
        self.destructive = destructive;
        self
    }

    /// Keeps the title on the normal large-text line height (28 px) instead of the tight 18 px one, as alert dialogs
    /// do.
    pub fn alert_title(mut self, alert_title: bool) -> Self {
        self.alert_title = alert_title;
        self
    }

    /// Orange-tinted card border, for warning dialogs.
    pub fn warning(mut self, warning: bool) -> Self {
        self.warning = warning;
        self
    }

    /// Widest the dialog may get, as a fraction of the window width. Combined with `max_width`, the smaller one wins.
    pub fn max_width_frac(mut self, frac: f32) -> Self {
        self.max_width_frac = Some(frac);
        self
    }

    /// Narrowest the dialog may get. It wins over the maximums.
    pub fn min_width(mut self, width: Pixels) -> Self {
        self.min_width = Some(width);
        self
    }

    /// Fixed height in pixels (capped by the maximum height).
    pub fn height(mut self, height: Pixels) -> Self {
        self.height = Some(height);
        self
    }

    /// Fixed height as a fraction of the window height.
    pub fn height_frac(mut self, frac: f32) -> Self {
        self.height_frac = Some(frac);
        self
    }

    /// Tallest the dialog may get, in pixels (default: the window height minus 32 px).
    pub fn max_height(mut self, height: Pixels) -> Self {
        self.max_height = Some(height);
        self
    }

    /// Tallest the dialog may get, as a fraction of the window height.
    pub fn max_height_frac(mut self, frac: f32) -> Self {
        self.max_height_frac = Some(frac);
        self
    }

    /// Shortest the dialog may get, in pixels.
    pub fn min_height(mut self, height: Pixels) -> Self {
        self.min_height = Some(height);
        self
    }

    fn is_locked(&self, cx: &App) -> bool {
        self.locked.as_ref().is_some_and(|is_locked| is_locked(cx))
    }

    fn frame(&self, window: &Window, cx: &App) -> ModalFrame {
        let area = window_area(window);
        let mut frame = ModalFrame::new()
            .max_width(self.max_width)
            .padded(self.padded)
            .show_close(self.show_close && !self.is_locked(cx));
        frame.max_width_frac = self.max_width_frac;
        frame.min_width = self.min_width;
        frame.height = self.height.or(self.height_frac.map(|f| area.size.height * f));
        frame.max_height = self.max_height.or(self.max_height_frac.map(|f| area.size.height * f));
        frame.min_height = self.min_height;
        frame
    }
}

/// Opens a dialog. See the module docs.
pub fn open(window: &mut Window, cx: &mut App, spec: DialogSpec) {
    let focus = cx.focus_handle();
    let spec = Rc::new(spec);
    open_layer(window, cx, move |dialog, topmost, window, cx| build(dialog, &spec, &focus, topmost, window, cx));
}

/// Fills gpui-kit's `Dialog` shell. The shell only provides the layer (backdrop, focus, Escape, stacking). The visible
/// card is ours, centered in a transparent shell that is sized to fill the window minus the shell's own 16px margin
/// (the same 32px that `ModalFrame` already leaves free).
///
/// gpui-kit 0.7 specifics handled here:
/// - The shell is positioned by `Positioner::corner(.., (x, margin_top))` and clamped into the window with a 16px
///   margin, so `margin_top(0)` gives a shell spanning `[16, height - 16]` and the card is centered.
/// - The shell clamps its height to `height - 32 - 16 * layer`, so the content is laid out to fill the shell
///   (`flex_1`) instead of being window-sized (which would show a scrollbar).
/// - `DialogEntrance::SlideDown` (default) moves the shell from y = 0 to `margin_top`, which the clamp turns into no
///   movement at all, and the shell's `Fade` is invisible: it sets `opacity` on the shell's outer `div`, but gpui-base
///   draws the dialog host through `deferred(..)` and `Window::defer_draw` does not capture `element_opacity`
///   (`gpui-pre window.rs` `DeferredDraw`), so deferred content is painted at opacity 1. The entrance is therefore
///   `None` and both the 250ms fade and the 30px slide are done on our own layer, which lives inside the deferred
///   host and so does inherit its own opacity.
/// - gpui-kit's dim is transparent and [`backdrop`] draws it instead. It shows at once and does not fade in.
/// - Enter (`Confirm`) would close the dialog in 0.7 (deferred close). This dialog has no `on_ok` action, so `on_ok`
///   returns `false` to ignore it.
fn build(
    dialog: Dialog,
    spec: &Rc<DialogSpec>,
    focus: &FocusHandle,
    topmost: bool,
    window: &mut Window,
    cx: &mut App,
) -> Dialog {
    let area = window_area(window);
    let on_close = spec.on_close.clone();
    let close_fn: DialogHandler = {
        let on_close = on_close.clone();
        Rc::new(move |window, cx| {
            if let Some(f) = &on_close {
                f(window, cx);
            }
            window.close_dialog(cx);
        })
    };
    let overlay_closes = spec.overlay_closes && !spec.is_locked(cx);

    let (layer_spec, layer_focus, layer_close) = (spec.clone(), focus.clone(), close_fn);
    dialog
        .width(area.size.width)
        .margin_top(px(0.0))
        .overlay(true)
        // the backdrop (the 16px ring around the shell) closes through the shell; clicks inside the shell hit our layer
        .overlay_closable(overlay_closes)
        .close_button(false)
        .keyboard(spec.escape_closes && !spec.is_locked(cx))
        .entrance(DialogEntrance::None)
        .on_ok(|_, _, _| false)
        .on_close(move |_, window, cx| {
            if let Some(f) = &on_close {
                f(window, cx);
            }
        })
        .p_0()
        .bg(transparent_black())
        .border_0()
        .min_h(px(0.0))
        .h(area.size.height)
        .content(move |content, window, cx| {
            let mut frame = layer_spec.frame(window, cx).focus(layer_focus.clone());
            frame.on_close = Some(layer_close.clone());
            // dim this card while another dialog is open above it
            let mut frame = frame
                .dimmed(!topmost)
                .when(layer_spec.destructive, |f| f.border_color(alpha(cx.tokens().destructive, 0.35)))
                .when(layer_spec.warning, |f| f.border_color(alpha(tw(Tw::Orange, 400), 0.35)))
                .children(header(&layer_spec, cx))
                .children(layer_spec.content.as_ref().map(|c| c(window, cx)));
            if let Some(footer) = &layer_spec.footer {
                frame = frame.child(footer(window, cx));
            }

            let close_fn = layer_close.clone();
            let overlay_closes = layer_spec.overlay_closes && !layer_spec.is_locked(cx);
            content.child(
                div()
                    .id("shadcn-dialog-layer")
                    .relative()
                    .w_full()
                    .flex_1()
                    .min_h(px(0.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                        if overlay_closes {
                            close_fn(window, cx);
                        }
                    })
                    .child(frame)
                    // the card slides down 30px while fading in over 250ms with an ease-out curve. The bezier uses
                    // x = 1/3, 2/3 because that keeps the time axis linear in the gpui-base 0.9 solver
                    .with_animation(
                        "shadcn-dialog-slide",
                        Animation::new(Duration::from_secs_f64(0.25)).with_easing(cubic_bezier(
                            1. / 3.,
                            0.72,
                            2. / 3.,
                            1.0,
                        )),
                        |layer, delta| layer.top(px(-30.0 * (1.0 - delta))).opacity(delta),
                    ),
            )
        })
}

fn header(spec: &DialogSpec, _cx: &mut App) -> Option<DialogHeader> {
    if spec.title.is_none() && spec.description.is_none() {
        return None;
    }
    let mut header = DialogHeader::new();
    if let Some(title) = &spec.title {
        header = header.child(DialogTitle::new(title.clone()).leading_none(!spec.alert_title));
    }
    if let Some(description) = &spec.description {
        header = header.child(DialogDescription::new(description.clone()));
    }
    // custom-layout dialogs (`padded(false)`) keep their own padding; the header still needs the card padding
    Some(if spec.padded { header } else { header.px(px(24.0)).pt(px(24.0)) })
}

/// The dialog card: a 1 px translucent white border, rounded corners, a slightly translucent background, a large drop
/// shadow, an optional X close button and, when padded, 24 px padding with 16 px between children. Children are laid
/// out as a column. Used by `dialog::open` and `alert_dialog::confirm`. Use it directly only to build a fully custom
/// overlay.
#[derive(IntoElement)]
pub struct ModalFrame {
    id: ElementId,
    max_width: Pixels,
    max_width_frac: Option<f32>,
    min_width: Option<Pixels>,
    height: Option<Pixels>,
    max_height: Option<Pixels>,
    min_height: Option<Pixels>,
    padded: bool,
    show_close: bool,
    on_close: Option<DialogHandler>,
    focus: Option<FocusHandle>,
    dimmed: bool,
    style: StyleRefinement,
    children: Vec<AnyElement>,
}

impl Default for ModalFrame {
    fn default() -> Self {
        Self::new()
    }
}

impl ModalFrame {
    pub fn new() -> Self {
        Self {
            id: "shadcn-dialog".into(),
            max_width: px(512.0),
            max_width_frac: None,
            min_width: None,
            height: None,
            max_height: None,
            min_height: None,
            padded: true,
            show_close: false,
            on_close: None,
            focus: None,
            dimmed: false,
            style: StyleRefinement::default(),
            children: Vec::new(),
        }
    }

    pub fn id(mut self, id: impl Into<ElementId>) -> Self {
        self.id = id.into();
        self
    }

    pub fn max_width(mut self, width: Pixels) -> Self {
        self.max_width = width;
        self
    }

    pub fn padded(mut self, padded: bool) -> Self {
        self.padded = padded;
        self
    }

    /// Shows the X at the top right; `on_close` runs when it is pressed.
    pub fn show_close(mut self, show: bool) -> Self {
        self.show_close = show;
        self
    }

    pub fn on_close(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_close = Some(Rc::new(handler));
        self
    }

    pub(crate) fn focus(mut self, focus: FocusHandle) -> Self {
        self.focus = Some(focus);
        self
    }

    /// Dims the card. Used while another dialog is open above this one.
    pub(crate) fn dimmed(mut self, dimmed: bool) -> Self {
        self.dimmed = dimmed;
        self
    }
}

impl Styled for ModalFrame {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl ParentElement for ModalFrame {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for ModalFrame {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = cx.tokens();
        let area = window_area(window);
        let avail_w = (area.size.width - px(32.0)).max(px(0.0));
        let mut w = self.max_width.min(avail_w);
        if let Some(f) = self.max_width_frac {
            w = w.min(area.size.width * f);
        }
        if let Some(min) = self.min_width {
            w = w.max(min);
        }
        let max_h = self.max_height.unwrap_or((area.size.height - px(32.0)).max(px(0.0)));

        let close = self.show_close.then(|| {
            let on_close = self.on_close.clone();
            div()
                .id("shadcn-dialog-close")
                .absolute()
                .top(px(16.0))
                .right(px(16.0))
                .size(px(16.0))
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(2.0))
                .cursor_pointer()
                .opacity(0.7)
                .hover(|s| s.opacity(1.0).bg(alpha(t.background, 0.6)))
                .on_click(move |_: &ClickEvent, window, cx| {
                    cx.stop_propagation();
                    match &on_close {
                        Some(f) => f(window, cx),
                        None => window.close_dialog(cx),
                    }
                })
                .child(svg().path(icons::X.path()).size(px(16.0)).flex_none().text_color(t.foreground))
        });

        div()
            .id(self.id)
            .relative()
            .flex()
            .flex_col()
            .flex_none()
            .w(w)
            .max_h(max_h)
            .when_some(self.height, |d, h| d.h(h.min(max_h)))
            .when_some(self.min_height, |d, h| d.min_h(h))
            .overflow_hidden()
            .when(self.padded, |d| d.p(px(24.0)).gap(px(16.0)))
            .rounded(radius::LG)
            .border_1()
            .border_color(hsla(0.0, 0.0, 1.0, 0.1))
            .bg(alpha(t.background, 0.95))
            .text_color(t.foreground)
            .shadow(shadow_2xl())
            .occlude()
            .when_some(self.focus, |d, f| d.track_focus(&f))
            // clicks on the card must not reach the dimmed backdrop
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .merge_style(&self.style)
            .children(self.children)
            .children(close)
            .when(self.dimmed, |d| d.child(fill_overlay(div().size_full().rounded(radius::LG).bg(backdrop_color()))))
    }
}

// ---------------------------------------------------------------------------------------------------------
// Header / title / description / footer
// ---------------------------------------------------------------------------------------------------------

/// Column holding the title and the description, 8 px apart.
#[derive(IntoElement)]
pub struct DialogHeader {
    style: StyleRefinement,
    children: Vec<AnyElement>,
}

impl Default for DialogHeader {
    fn default() -> Self {
        Self::new()
    }
}

impl DialogHeader {
    pub fn new() -> Self {
        Self { style: StyleRefinement::default(), children: Vec::new() }
    }
}

impl Styled for DialogHeader {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl ParentElement for DialogHeader {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for DialogHeader {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        div().flex().flex_col().gap(px(8.0)).merge_style(&self.style).children(self.children)
    }
}

/// Dialog title: 18 px semibold text.
#[derive(IntoElement)]
pub struct DialogTitle {
    leading_none: bool,
    style: StyleRefinement,
    children: Vec<AnyElement>,
}

impl DialogTitle {
    pub fn new(text: impl Into<SharedString>) -> Self {
        Self { leading_none: true, style: StyleRefinement::default(), children: vec![text.into().into_any_element()] }
    }

    /// Tight 18 px line height (default). `false` keeps the normal large-text line height of 28 px (alert dialogs).
    pub fn leading_none(mut self, leading_none: bool) -> Self {
        self.leading_none = leading_none;
        self
    }
}

impl Styled for DialogTitle {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl ParentElement for DialogTitle {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for DialogTitle {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        div()
            .text_lg_tw()
            .when(self.leading_none, |d| d.line_height(px(18.0)))
            .font_weight(gpui_kit::FontWeight::SEMIBOLD)
            .merge_style(&self.style)
            .children(self.children)
    }
}

/// Dialog description: small muted text.
#[derive(IntoElement)]
pub struct DialogDescription {
    style: StyleRefinement,
    children: Vec<AnyElement>,
}

impl DialogDescription {
    pub fn new(text: impl Into<SharedString>) -> Self {
        Self { style: StyleRefinement::default(), children: vec![text.into().into_any_element()] }
    }
}

impl Styled for DialogDescription {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl ParentElement for DialogDescription {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for DialogDescription {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = cx.tokens();
        div().text_sm_tw().text_color(t.muted_foreground).merge_style(&self.style).children(self.children)
    }
}

/// Row of buttons aligned to the right, 8 px apart.
#[derive(IntoElement)]
pub struct DialogFooter {
    style: StyleRefinement,
    children: Vec<AnyElement>,
}

impl Default for DialogFooter {
    fn default() -> Self {
        Self::new()
    }
}

impl DialogFooter {
    pub fn new() -> Self {
        Self { style: StyleRefinement::default(), children: Vec::new() }
    }
}

impl Styled for DialogFooter {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl ParentElement for DialogFooter {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for DialogFooter {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        div().flex().flex_row().justify_end().gap(px(8.0)).merge_style(&self.style).children(self.children)
    }
}
