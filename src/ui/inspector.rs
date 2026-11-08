//! Opening the inspector makes the window wider instead of squeezing the page.
//!
//! GPUI shows the inspector inside the window and makes the page narrower to fit it.
//! This handler replaces gpui-component's toggle and resizes the window, so the page keeps its width.
//! The inspector only exists in debug builds.

use std::collections::HashMap;

use gpui_kit::component::ToggleInspector;
use gpui_kit::{App, Global, WindowId, rems, size};

const INSPECTOR_REMS: f32 = 30.0;

/// Windows with the inspector open. The value is true if opening it made the window wider.
#[derive(Default)]
struct OpenInspectors(HashMap<WindowId, bool>);

impl Global for OpenInspectors {}

/// Call after `gpui_kit::component::init`, otherwise gpui-component's handler runs instead of this one.
pub fn init(cx: &mut App) {
    cx.set_global(OpenInspectors::default());
    cx.on_action(|_: &ToggleInspector, cx| {
        let Some(handle) = cx.active_window() else {
            return;
        };
        // The window is busy handling the action, so wait until it's done.
        cx.defer(move |cx| {
            _ = handle.update(cx, |_, window, cx| {
                let id = handle.window_id();
                let width = rems(INSPECTOR_REMS).to_pixels(window.rem_size());
                let viewport = window.viewport_size();
                match cx.global_mut::<OpenInspectors>().0.remove(&id) {
                    None => {
                        // A maximized or fullscreen window can't grow, so the page gets narrower instead.
                        let grow = !window.is_maximized() && !window.is_fullscreen();
                        if grow {
                            window.resize(size(viewport.width + width, viewport.height));
                        }
                        cx.global_mut::<OpenInspectors>().0.insert(id, grow);
                    }
                    Some(grew) => {
                        if grew && !window.is_maximized() && !window.is_fullscreen() {
                            window.resize(size(viewport.width - width, viewport.height));
                        }
                    }
                }
                window.toggle_inspector(cx);
            });
        });
    });
}
