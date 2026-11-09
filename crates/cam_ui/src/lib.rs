//! The widget kit of Classic Addon Manager on top of gpui-kit: the dark theme and its accent colour, Lucide icons,
//! the Geist fonts, the components (buttons, inputs, dialogs, menus, Markdown, ...) and toasts. It knows nothing about
//! addons; the app builds its pages from these parts.
// False positive on windows-gnu: `thread_local!` expands differently there and clippy flags initializers that already
// are `const { … }` (components/dialog.rs, components/floating.rs).
// https://github.com/rust-lang/rust-clippy/issues/13422
#![cfg_attr(all(windows, target_env = "gnu"), expect(clippy::missing_const_for_thread_local))]
pub mod anim;
pub mod assets;
pub mod components;
pub mod http_client;
pub mod icons;
mod icons_generated;
pub mod theme;
pub mod toast;

use gpui_kit::App;

/// Sets the kit up: fonts, the theme with `accent`, the HTTP client behind `gpui_kit::img("https://..")` (avatars,
/// addon icons, README images) and the toast layer. Call after `gpui_kit::init`, before the first window is opened.
pub fn init(cx: &mut App, accent: theme::Accent) {
    if let Err(e) = assets::register_fonts(cx) {
        log::error!("Failed to register fonts: {e:#}");
    }
    theme::init(cx, accent);
    http_client::install(cx);
    toast::init(cx);
}
