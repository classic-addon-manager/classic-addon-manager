//! Title bar: 56 px tall, with the logo, a slot for the page toolbar and the window controls. Dragging it moves the
//! window.
//!
//! Pages put their toolbar into the bar with `titlebar::set_slot(cx, view)` and the shell clears it on every page
//! switch. Wrap interactive toolbar groups with `.no_drag()`, otherwise pressing the mouse there starts a window move.
//!
//! Moving is platform-split: Linux (X11/Wayland) starts the move from the mouse listeners via
//! `start_window_move`, which gpui does not implement on Windows. There the bar is a
//! `WindowControlArea::Drag` region that gpui reports as `HTCAPTION` from `WM_NCHITTEST`, so the OS
//! performs the move (plus Aero Snap, double-click maximise and the system menu).
use gpui_kit::{
    AnyView, App, Decorations, Div, Global, InteractiveElement, IntoElement, MouseButton, ParentElement, RenderOnce,
    StatefulInteractiveElement, Styled, Window, WindowControlArea, div, img, prelude::FluentBuilder as _, px, svg,
};

use super::{
    assets::images,
    icons,
    theme::{TokensExt, alpha, radius},
};
use gpui_kit::component::{IconNamed, InteractiveElementExt as _};

#[derive(Default)]
pub struct TitleBarSlot(pub Option<AnyView>);
impl Global for TitleBarSlot {}

/// Show `view` in the title bar centre slot until the page changes.
pub fn set_slot(cx: &mut App, view: impl Into<AnyView>) {
    cx.set_global(TitleBarSlot(Some(view.into())));
}

pub fn clear_slot(cx: &mut App) {
    if cx.try_global::<TitleBarSlot>().is_some_and(|s| s.0.is_some()) {
        cx.set_global(TitleBarSlot(None));
    }
}

/// Mouse presses on this element never start a window move.
///
/// `occlude` keeps the title bar's `Drag` hitbox out of the Windows hit test under this element, so it
/// stays a client area instead of `HTCAPTION`.
pub trait NoDrag: InteractiveElement + Sized {
    fn no_drag(self) -> Self {
        self.occlude().on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
    }
}
impl<T: InteractiveElement + Sized> NoDrag for T {}

#[derive(Default)]
struct DragState {
    should_move: bool,
}

// `window.use_state` requires a `Render` entity.
impl gpui_kit::Render for DragState {
    fn render(&mut self, _: &mut Window, _: &mut gpui_kit::Context<Self>) -> impl IntoElement {
        div()
    }
}

#[derive(IntoElement)]
pub struct TitleBar {
    slot: Option<AnyView>,
}

impl TitleBar {
    pub fn new(slot: Option<AnyView>) -> Self {
        Self { slot }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Control {
    Minimize,
    Maximize,
    Close,
}

fn control_button(kind: Control, maximized: bool, cx: &App) -> impl IntoElement {
    let t = cx.tokens();
    let (id, icon, title) = match kind {
        Control::Minimize => ("win-min", icons::WIN_MINIMIZE, "Minimise"),
        Control::Maximize if maximized => ("win-max", icons::WIN_RESTORE, "Restore"),
        Control::Maximize => ("win-max", icons::WIN_MAXIMIZE, "Maximise"),
        Control::Close => ("win-close", icons::WIN_CLOSE, "Close"),
    };
    let _ = title;
    let (hover_bg, hover_fg) = match kind {
        Control::Close => (t.destructive, t.destructive_foreground),
        _ => (t.accent, t.accent_foreground),
    };
    div()
        .id(id)
        .group(id)
        .flex()
        .items_center()
        .justify_center()
        .size(px(32.0))
        .rounded(radius::MD)
        .cursor_pointer()
        .text_color(t.muted_foreground)
        .hover(move |s| s.bg(hover_bg))
        .no_drag()
        .on_click(move |_, window, cx| {
            cx.stop_propagation();
            match kind {
                Control::Minimize => window.minimize_window(),
                Control::Maximize => toggle_maximize(window),
                Control::Close => window.remove_window(),
            }
        })
        .child(
            svg()
                .path(icon.path())
                .size(px(14.0))
                .text_color(t.muted_foreground)
                .group_hover(id, move |s| s.text_color(hover_fg)),
        )
}

/// Maximise or restore. gpui's Windows `zoom_window` only ever maximises (`SW_MAXIMIZE`), so restoring there
/// goes through `ShowWindowAsync(SW_RESTORE)` on the raw HWND, other platforms toggle in `zoom_window`.
fn toggle_maximize(window: &mut Window) {
    #[cfg(windows)]
    if window.is_maximized() {
        use raw_window_handle::{HasWindowHandle as _, RawWindowHandle};

        #[link(name = "user32")]
        unsafe extern "system" {
            fn ShowWindowAsync(hwnd: isize, cmd_show: i32) -> i32;
        }
        const SW_RESTORE: i32 = 9;

        if let Ok(handle) = window.window_handle()
            && let RawWindowHandle::Win32(h) = handle.as_raw()
        {
            // SAFETY: `h.hwnd` is the live HWND of this gpui window.
            unsafe { ShowWindowAsync(h.hwnd.get(), SW_RESTORE) };
        }
        return;
    }
    window.zoom_window();
}

impl RenderOnce for TitleBar {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = cx.tokens();
        let state = window.use_state(cx, |_, _| DragState::default());
        let maximized = window.is_maximized();
        let client_decorated = matches!(window.window_decorations(), Decorations::Client { .. });

        let drag_down = window.listener_for(&state, |s: &mut DragState, _, _, _| s.should_move = true);
        let drag_up = window.listener_for(&state, |s: &mut DragState, _, _, _| s.should_move = false);
        let drag_out = window.listener_for(&state, |s: &mut DragState, _, _, _| s.should_move = false);
        let drag_move = window.listener_for(&state, |s: &mut DragState, _, window, _| {
            if s.should_move {
                s.should_move = false;
                window.start_window_move();
            }
        });

        div()
            .id("title-bar")
            .flex()
            .flex_none()
            .items_center()
            .h(px(56.0))
            .w_full()
            .px(px(12.0))
            .border_b_1()
            .border_color(t.border)
            .bg(alpha(t.background, 0.95))
            .window_control_area(WindowControlArea::Drag)
            .on_double_click(|_, window, _| window.zoom_window())
            .on_mouse_down_out(drag_out)
            .on_mouse_down(MouseButton::Left, drag_down)
            .on_mouse_up(MouseButton::Left, drag_up)
            .on_mouse_move(drag_move)
            .when(client_decorated, |this| {
                this.on_mouse_down(MouseButton::Right, |ev, window, _| window.show_window_menu(ev.position))
            })
            // logo column: sized to the logo (not the 220px sidebar) so the page toolbar slot gets the space
            .child(
                div().flex().items_center().flex_none().child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(12.0))
                        .rounded(radius::MD)
                        .px(px(8.0))
                        .py(px(4.0))
                        .child(img(images::LOGO_WIDE_2X).h(px(32.0)).w(px(116.0))),
                ),
            )
            // page toolbar slot
            .child(
                div()
                    .flex()
                    .flex_1()
                    .min_w_0()
                    .items_center()
                    .px(px(16.0))
                    .when_some(self.slot, |this: Div, slot| this.child(slot)),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(4.0))
                    .flex_none()
                    .child(control_button(Control::Minimize, maximized, cx))
                    .child(control_button(Control::Maximize, maximized, cx))
                    .child(control_button(Control::Close, maximized, cx)),
            )
    }
}
