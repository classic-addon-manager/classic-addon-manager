//! Startup update check, the "Update Available" dialog and the self update it starts.
use gpui_kit::{
    App, AppContext as _, Context, IntoElement, ParentElement, Render, Styled, Task, Window, div,
    prelude::FluentBuilder as _, px,
};

use super::widgets::{TintedButton, icon_tile};
use crate::{
    backend::{
        api::{self, ApplicationRelease},
        updater,
    },
    state::{self, run_blocking, toast},
    ui::{
        components::{Button, ButtonVariant, DialogDescription, DialogSpec, DialogTitle, dialog, spinner},
        icons,
        theme::{Tw, TwText, alpha, radius, tw},
        utils::semver_gt,
    },
};
use gpui_kit::component::Icon;

/// Startup check: toasts a failed previous self update, learns whether this installation can update itself and looks
/// for a newer release. Resolves to true when it found one, which `AppInfoState` then holds.
pub fn check(cx: &mut App) -> Task<bool> {
    let backend = state::backend(cx);
    cx.spawn(async move |cx| match run_blocking(cx, move || updater::consume_update_failure(&backend.paths)).await {
        Ok(report) if !report.is_empty() => {
            cx.update(|cx| toast::show(cx, toast::Toast::new("Update failed", report).icon(icons::TRIANGLE_ALERT)));
        }
        Ok(_) => {}
        Err(e) => log::error!("Reading the last update failure report failed: {e:#}"),
    })
    .detach();
    let current = state::app_info(cx).read(cx).version.clone();
    cx.spawn(async move |cx| {
        let supported = run_blocking(cx, updater::self_update_supported);
        let latest = run_blocking(cx, api::latest_application_release);
        let (supported, latest) = (supported.await, latest.await);
        cx.update(|cx| {
            let info = state::app_info(cx);
            info.update(cx, |s, cx| s.set_self_update_supported(supported, cx));
            match latest {
                Ok(release) if semver_gt(&release.version, &current) => {
                    info.update(cx, |s, cx| s.set_update_available(release, cx));
                    true
                }
                Ok(_) => false,
                Err(e) => {
                    log::error!("Checking for an app update failed: {e:#}");
                    false
                }
            }
        })
    })
}

/// Opens the "Update Available" dialog for the release the startup check found. Does nothing without one.
pub fn open(window: &mut Window, cx: &mut App) {
    if !state::app_info(cx).read(cx).update_available() {
        return;
    }
    let content = cx.new(|cx| {
        cx.observe(&state::app_info(cx), |_, _, cx| cx.notify()).detach();
        UpdateContent
    });
    dialog::open(window, cx, DialogSpec::new().max_width(px(448.0)).content_view(content));
}

/// "Update Now" button: starts the self update unless one is running or this installation cannot update itself. On
/// success the backend restarts or exits the process.
fn update_now(cx: &mut App) {
    let info = state::app_info(cx);
    let s = info.read(cx);
    if s.self_updating || !s.self_update_supported {
        return;
    }
    let Some(ApplicationRelease { url, checksum, .. }) = s.update_information.clone() else { return };
    info.update(cx, |s, cx| s.set_self_updating(true, cx));
    let backend = state::backend(cx);
    cx.spawn(async move |cx| {
        if let Err(e) = run_blocking(cx, move || updater::self_update(&backend.paths, &url, &checksum)).await {
            log::error!("Update failed: {e:#}");
            cx.update(|cx| {
                toast::show(cx, toast::Toast::new("Update failed", format!("{e:#}")).icon(icons::TRIANGLE_ALERT));
                info.update(cx, |s, cx| s.set_self_updating(false, cx));
            });
        }
    })
    .detach();
}

/// The dialog body: header, current and new version, a notice when self update is unsupported, and the buttons.
struct UpdateContent;

impl Render for UpdateContent {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let (current, new, is_updating, supported) = {
            let info = state::app_info(cx).read(cx);
            let clean = |v: &str| v.strip_prefix('v').unwrap_or(v).to_string();
            (
                clean(&info.version),
                info.update_information.as_ref().map(|i| clean(&i.version)).unwrap_or_default(),
                info.self_updating,
                info.self_update_supported,
            )
        };
        let emerald = |shade| tw(Tw::Emerald, shade);
        let white = |a: f32| gpui_kit::hsla(0.0, 0.0, 1.0, a);

        let header = div()
            .flex()
            .items_center()
            .gap(px(12.0))
            .child(icon_tile(icons::CIRCLE_ARROW_UP, alpha(emerald(500), 0.2), emerald(400)))
            .child(
                div()
                    .child(DialogTitle::new("Update Available"))
                    .child(DialogDescription::new("A new version of Classic Addon Manager is available.")),
            );

        let versions = div()
            .flex()
            .items_center()
            .justify_center()
            .gap(px(8.0))
            .rounded(radius::LG)
            .border_1()
            .border_color(white(0.05))
            .bg(alpha(tw(Tw::Zinc, 900), 0.5))
            .px(px(16.0))
            .py(px(12.0))
            .child(
                div()
                    .rounded(radius::MD)
                    .bg(white(0.05))
                    .px(px(10.0))
                    .py(px(4.0))
                    .text_sm_tw()
                    .font_mono_tw()
                    .text_color(white(0.5))
                    .child(current),
            )
            .child(Icon::new(icons::ARROW_RIGHT).size(px(16.0)))
            .child(
                // A 1 px border, with the padding reduced by 1 px so the box keeps the size of the one on the left.
                div()
                    .rounded(radius::MD)
                    .bg(alpha(emerald(500), 0.1))
                    .border_1()
                    .border_color(alpha(emerald(500), 0.2))
                    .px(px(9.0))
                    .py(px(3.0))
                    .text_sm_tw()
                    .font_mono_tw()
                    .text_color(emerald(300))
                    .child(new),
            );

        let later = Button::new("update-later")
            .variant(ButtonVariant::Ghost)
            .label("Later")
            .disabled(is_updating)
            .on_click(|_, window, cx| dialog::close(window, cx));
        let update = TintedButton::new("update-now", emerald(900), emerald(800), alpha(emerald(900), 0.2))
            .disabled(is_updating || !supported)
            .has_icon(true)
            .on_click(|_, _, cx| update_now(cx));
        let update = if is_updating {
            update.child(div().mr_2().child(spinner())).child("Updating…")
        } else {
            update.child(Icon::new(icons::DOWNLOAD).size(px(16.0))).child("Update Now")
        };

        div()
            .flex()
            .flex_col()
            .gap(px(16.0))
            .child(header)
            .child(versions)
            .when(!supported, |d| {
                d.child(div().text_sm_tw().text_color(white(0.6)).child(
                    "Automatic updates require Windows or a Linux AppImage. Please update other installations manually using their package manager or a fresh download.",
                ))
            })
            .child(div().flex().justify_end().gap(px(8.0)).child(later).child(update))
    }
}
