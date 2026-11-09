//! Classic Addon Manager. Bootstrap: CLI flag, single instance, assets/fonts/theme, window.
// Release builds link as a GUI app on Windows so launching doesn't spawn a console, debug keeps the console for stderr.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod assets;
mod state;
mod ui;

use std::sync::Arc;

use assets::Assets;
use cam_core::{Backend, headless, ipc, logger, paths::Paths, startup};
use cam_ui::components::{ErrorItem, error_dialog};
use cam_ui::toast;
use gpui_kit::{
    AnyWindowHandle, App, AppContext, Bounds, WindowBackgroundAppearance, WindowBounds, WindowDecorations, WindowKind,
    WindowOptions, point, px, size,
};
use ui::Shell;

/// Window logical size in pixels, 985x640, also the minimum. gpui-kit's `WindowBorder` (hosted by `Root`) draws the
/// client-side shadow margin (Linux only, `SHADOW_SIZE` = 20px in gpui-component 0.9.0) *inside* the window, so the
/// real window is larger by that margin on each side.
const WINDOW_W: f32 = 985.0;
const WINDOW_H: f32 = 640.0;
#[cfg(target_os = "linux")]
const SHADOW_MARGIN: f32 = 20.0;
#[cfg(not(target_os = "linux"))]
const SHADOW_MARGIN: f32 = 0.0;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    // `--check-updates`: headless addon update check, no window.
    let check_updates = startup::is_check_updates_mode(&args);
    let paths = Paths::from_env();
    logger::init(paths.as_ref().ok().map(|p| p.config.clone()), check_updates);
    install_panic_hook();
    let paths = match paths {
        Ok(paths) => paths,
        Err(e) => {
            log::error!("Cannot start: {e:#}");
            eprintln!("{}: {e:#}", startup::FATAL_ERROR_TITLE);
            std::process::exit(1);
        }
    };
    let backend = Arc::new(Backend::new(paths));

    if check_updates {
        std::process::exit(headless::run(&backend));
    }

    // GUI mode: config bootstrap + single-instance check (an auth deeplink is forwarded to the running instance).
    let outcome = startup::run_startup(&backend, args.first().map(String::as_str));
    let Some(mut instance) = outcome.instance else {
        for e in &outcome.errors {
            eprintln!("{}: {}: {}", startup::FATAL_ERROR_TITLE, e.title, e.message);
            if let Some(details) = &e.details {
                eprintln!("{details}");
            }
        }
        std::process::exit(0);
    };
    let startup_errors = outcome.errors;

    // Arm the single-instance IPC server right away (before the slow GPU/window init) so a second launch always
    // finds us. Auth deeplink tokens are queued until the UI is up.
    let (token_tx, token_rx) = async_channel::unbounded::<String>();
    // Registering the scheme handler writes a .desktop file in $HOME; skip while sandboxed.
    if !backend.paths.sandboxed {
        ipc::register_deeplink();
    }
    instance.start_server(move |token| {
        let _ = token_tx.send_blocking(token);
    });

    gpui_kit::application().with_assets(Assets).run(move |cx: &mut App| {
        gpui_kit::init(cx);
        #[cfg(debug_assertions)]
        ui::inspector::init(cx);
        // After `gpui_kit::init`: the toast layer is a deferred (priority 50) `Root` plugin.
        cam_ui::init(cx, ui::accent(Default::default()));
        state::init(cx, backend);
        state::bootstrap(cx);

        let margin = SHADOW_MARGIN * 2.0;
        let min = size(px(WINDOW_W + margin), px(WINDOW_H + margin));
        let bounds = Bounds::centered(None, min, cx);
        let (window, _shell) = gpui_kit::open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(gpui_kit::TitlebarOptions {
                    title: Some("Classic Addon Manager".into()),
                    appears_transparent: true,
                    traffic_light_position: Some(point(px(9.0), px(9.0))),
                }),
                window_min_size: Some(min),
                window_decorations: Some(WindowDecorations::Client),
                window_background: WindowBackgroundAppearance::Transparent,
                kind: WindowKind::Normal,
                app_id: Some("classic-addon-manager".into()),
                ..Default::default()
            },
            cx,
            |window, cx| cx.new(|cx| Shell::new(window, cx)),
        )
        .expect("failed to open window");

        forward_auth_tokens(window, token_rx, cx);
        // Startup problems are shown in one in-app error dialog listing every one of them.
        let items: Vec<_> = startup_errors
            .into_iter()
            .map(|e| {
                let item = ErrorItem::new(e.title, e.message);
                match e.details {
                    Some(details) => item.details(details),
                    None => item,
                }
            })
            .collect();
        if !items.is_empty() {
            let _ = cx.update_window(window, |_, window, cx| error_dialog::open(window, cx, items));
        }

        cx.on_window_closed(move |cx, _window_id| {
            if cx.windows().is_empty() {
                instance.cleanup();
                cx.quit();
            }
        })
        .detach();
        cx.activate(true);
    });
}

/// Writes every panic to the log. Windows release builds have no console, so stderr alone leaves no trace.
/// Stack overflows abort without running the hook.
fn install_panic_hook() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let payload = info
            .payload()
            .downcast_ref::<&str>()
            .copied()
            .or_else(|| info.payload().downcast_ref::<String>().map(String::as_str))
            .unwrap_or("<non-string panic payload>");
        let location = info.location().map(|l| l.to_string()).unwrap_or_default();
        let thread = std::thread::current();
        let backtrace = std::backtrace::Backtrace::force_capture();
        log::error!("panic in thread '{}' at {location}: {payload}\n{backtrace}", thread.name().unwrap_or("<unnamed>"),);
        logger::sync();
        default_hook(info);
    }));
}

/// Auth deeplink tokens (received by the IPC server thread) are passed to the UI thread, which saves each one, raises
/// the window and toasts the result.
fn forward_auth_tokens(window: AnyWindowHandle, rx: async_channel::Receiver<String>, cx: &mut App) {
    cx.spawn(async move |cx| {
        while let Ok(token) = rx.recv().await {
            let save = cx.update(|cx| {
                let _ = window.update(cx, |_, window, _| window.activate_window());
                state::user(cx).update(cx, |u, cx| u.save_token(token, cx))
            });
            let res = save.await;
            cx.update(|cx| match res {
                Ok(()) => {
                    toast::success_titled(cx, "Success", "You have successfully signed in.");
                }
                Err(e) => {
                    log::error!("Failed to save authentication session: {e:#}");
                    toast::error(cx, "Failed to save authentication session.");
                }
            });
        }
    })
    .detach();
}
