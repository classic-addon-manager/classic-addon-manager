//! Troubleshooting page: addon log diagnostics, addon recovery (reset settings, uninstall all) and a guide for
//! reporting a problem with one addon.
use std::time::{Duration, Instant};

use gpui_kit::component::{Icon, StyledExt as _};
use gpui_kit::{
    AnyElement, AnyWindowHandle, App, AsyncApp, Context, Entity, FontWeight, IntoElement, ParentElement, Render,
    SharedString, Styled, Task, Transformation, WeakEntity, Window, div, percentage, prelude::FluentBuilder as _, px,
};

use crate::backend::{addon, util};
use crate::state::{self, AddonState, run_blocking, tasks::error_message, toast};
use crate::ui::{
    components::{
        alert_dialog::{self, AlertSpec},
        *,
    },
    icons,
    pages::settings::page_header,
    theme::{TokensExt, Tw, TwText, alpha, radius, tw},
};

struct Issue {
    error: String,
    file: String,
    count: i64,
}

/// Scan result with the issues grouped by addon, in the order the addons first appear.
#[derive(Default)]
struct Diagnostics {
    issue_count: usize,
    grouped: Vec<(String, Vec<Issue>)>,
}

pub struct TroubleshootingPage {
    addons: Entity<AddonState>,
    data: Diagnostics,
    is_loading: bool,
    error: String,
    run: u32,
    has_result: bool,
    resetting: bool,
    uninstalling: bool,
    _scan: Option<Task<()>>,
}

impl TroubleshootingPage {
    pub fn new(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        let addons = state::addons(cx);
        cx.observe(&addons, |_, _, cx| cx.notify()).detach();
        let mut this = Self {
            addons,
            data: Diagnostics::default(),
            is_loading: true,
            error: String::new(),
            run: 0,
            has_result: false,
            resetting: false,
            uninstalling: false,
            _scan: None,
        };
        this.scan(cx);
        this
    }

    /// Scans the addon log for issues in the background and stores the result when it finishes. A rescan takes at
    /// least 500 ms.
    fn scan(&mut self, cx: &mut Context<Self>) {
        let rescan = self.run > 0;
        let backend = state::backend(cx);
        let scan = run_blocking(cx, move || {
            let start = Instant::now();
            let res = backend.config.aac_dir().and_then(|aac| util::diagnose_issues(&aac));
            if rescan && let Some(rem) = Duration::from_millis(500).checked_sub(start.elapsed()) {
                std::thread::sleep(rem);
            }
            res
        });
        self._scan = Some(cx.spawn(async move |this, cx| {
            let res = scan.await;
            let _ = this.update(cx, |this, cx| {
                match res {
                    Err(e) => {
                        this.error = error_message(&e);
                        this.data = Diagnostics::default();
                    }
                    Ok(issues) => {
                        let mut grouped: Vec<(String, Vec<Issue>)> = Vec::new();
                        let count = issues.len();
                        for i in issues {
                            let issue = Issue { error: i.error, file: i.file, count: i.count };
                            match grouped.iter_mut().find(|(n, _)| *n == i.addon) {
                                Some((_, v)) => v.push(issue),
                                None => grouped.push((i.addon, vec![issue])),
                            }
                        }
                        this.data = Diagnostics { issue_count: count, grouped };
                        this.error.clear();
                    }
                }
                this.has_result = true;
                this.is_loading = false;
                cx.notify();
            });
        }));
    }

    fn run_diagnostics(&mut self, cx: &mut Context<Self>) {
        self.is_loading = true;
        self.run += 1;
        self.scan(cx);
        cx.notify();
    }

    fn diagnostics_panel(&self, cx: &mut Context<Self>) -> AnyElement {
        let t = cx.tokens();
        let border60 = alpha(t.border, 0.6);
        let issue_count = self.data.issue_count;
        let n_addons = self.data.grouped.len();
        let initial_loading = self.is_loading && !self.has_result;
        let stale = self.is_loading && self.has_result;
        let plural = |n: usize, one: &str, many: &str| if n == 1 { one.to_string() } else { many.to_string() };

        let dot = |c| div().flex_none().size(px(8.0)).rounded_full().bg(c);
        let status: AnyElement = if self.is_loading {
            div()
                .flex()
                .items_center()
                .gap(px(10.0))
                .child(Icon::new(icons::LOADER_CIRCLE).size(px(14.0)).text_color(t.muted_foreground))
                .child(div().text_sm_tw().text_color(t.muted_foreground).child("Scanning addon log..."))
                .into_any_element()
        } else if !self.error.is_empty() {
            div()
                .flex()
                .items_center()
                .gap(px(10.0))
                .child(dot(t.destructive))
                .child(div().text_sm_tw().font_weight(FontWeight::MEDIUM).child("Scan failed"))
                .into_any_element()
        } else if issue_count == 0 {
            div()
                .flex()
                .items_center()
                .gap(px(10.0))
                .child(dot(tw(Tw::Emerald, 500)))
                .child(div().text_sm_tw().font_weight(FontWeight::MEDIUM).child("No issues detected"))
                .into_any_element()
        } else {
            div()
                .flex()
                .items_center()
                .gap(px(10.0))
                .child(dot(t.destructive))
                .child(div().text_sm_tw().font_weight(FontWeight::MEDIUM).child(format!(
                    "{issue_count} {} across {n_addons} {}",
                    plural(issue_count, "issue", "issues"),
                    plural(n_addons, "addon", "addons")
                )))
                .into_any_element()
        };

        let header = div()
            .flex()
            .items_center()
            .justify_between()
            .gap(px(24.0))
            .px_4()
            .py(px(14.0))
            .child(div().min_w_0().child(status))
            .child(
                Button::new("rescan")
                    .variant(ButtonVariant::Ghost)
                    .size(ButtonSize::Sm)
                    .icon(icons::REFRESH_CW)
                    .label("Rescan")
                    .loading(self.is_loading)
                    .disabled(self.is_loading)
                    .on_click(cx.listener(|this, _, _, cx| this.run_diagnostics(cx))),
            );

        let error_block = (!self.error.is_empty()).then(|| {
            div()
                .border_t_1()
                .border_color(border60)
                .bg(alpha(t.muted, 0.15))
                .px_4()
                .py_4()
                .when(stale, |d| d.opacity(0.5))
                .child(error_box(&t, "Diagnostics failed", self.error.clone()))
        });

        let body: Option<AnyElement> = if initial_loading {
            Some(
                div()
                    .border_t_1()
                    .border_color(border60)
                    .children((0..2).map(|i| {
                        div()
                            .when(i > 0, |d| d.border_t_1().border_color(border60))
                            .flex()
                            .items_center()
                            .gap_3()
                            .px_4()
                            .py(px(14.0))
                            .child(Skeleton::new().size(px(36.0)).rounded(radius::LG))
                            .child(
                                div()
                                    .min_w_0()
                                    .flex_1()
                                    .flex()
                                    .flex_col()
                                    .gap_2()
                                    .child(Skeleton::new().h(px(14.0)).w(px(128.0)))
                                    .child(Skeleton::new().h(px(12.0)).w(px(80.0))),
                            )
                    }))
                    .into_any_element(),
            )
        } else if issue_count > 0 {
            let rows = self.data.grouped.iter().enumerate().map(|(gi, (name, issues))| {
                let title = if name == "x2ui" { "Addon API".to_string() } else { name.clone() };
                let n = issues.len();
                let sub = format!("{n} {}", plural(n, "issue", "issues"));
                let trigger = move |open: bool| {
                    div()
                        .flex()
                        .w_full()
                        .items_center()
                        .gap_3()
                        .px_4()
                        .py(px(14.0))
                        .child(
                            div()
                                .flex()
                                .flex_none()
                                .items_center()
                                .justify_center()
                                .size(px(36.0))
                                .rounded(radius::LG)
                                .bg(alpha(t.destructive, 0.1))
                                .text_color(t.destructive)
                                .child(Icon::new(icons::TRIANGLE_ALERT).size(px(16.0))),
                        )
                        .child(
                            div()
                                .min_w_0()
                                .flex_1()
                                .child(
                                    div()
                                        .text_sm_tw()
                                        .line_height(px(14.0))
                                        .font_weight(FontWeight::MEDIUM)
                                        .child(title),
                                )
                                .child(div().mt_1().text_xs_tw().text_color(t.muted_foreground).child(sub)),
                        )
                        .child(
                            Icon::new(icons::CHEVRON_RIGHT)
                                .size(px(14.0))
                                .text_color(t.muted_foreground)
                                .when(open, |i| i.transform(Transformation::rotate(percentage(0.25)))),
                        )
                        .into_any_element()
                };
                let cards: Vec<AnyElement> = issues
                    .iter()
                    .map(|issue| {
                        div()
                            .rounded(radius::LG)
                            .border_1()
                            .border_color(border60)
                            .bg(alpha(t.background, 0.4))
                            .px_3()
                            .py(px(10.0))
                            .child(
                                div()
                                    .flex()
                                    .items_start()
                                    .gap_3()
                                    .child(
                                        div()
                                            .min_w_0()
                                            .flex_1()
                                            .text_xs_tw()
                                            .font_mono_tw()
                                            .text_color(t.destructive)
                                            .child(issue.error.clone()),
                                    )
                                    .when(issue.count > 1, |d| {
                                        d.child(
                                            div()
                                                .flex_none()
                                                .text_xs_tw()
                                                .text_color(t.muted_foreground)
                                                .child(format!("×{}", issue.count)),
                                        )
                                    }),
                            )
                            .child(
                                div()
                                    .mt(px(6.0))
                                    .text_xs_tw()
                                    .font_mono_tw()
                                    .text_color(alpha(t.foreground, 0.7))
                                    .child(issue.file.clone()),
                            )
                            .into_any_element()
                    })
                    .collect();
                div().when(gi > 0, |d| d.border_t_1().border_color(border60)).child(
                    Collapsible::new(("diag", gi))
                        .trigger_with(trigger)
                        .trigger_hover(move |s| s.bg(alpha(t.muted, 0.4)))
                        .content_with(move || {
                            div()
                                .flex()
                                .flex_col()
                                .gap_2()
                                .border_t_1()
                                .border_color(border60)
                                .bg(alpha(t.muted, 0.15))
                                .px_4()
                                .py_3()
                                .children(cards)
                                .into_any_element()
                        }),
                )
            });
            Some(
                div()
                    .border_t_1()
                    .border_color(border60)
                    .when(stale, |d| d.opacity(0.5))
                    .children(rows)
                    .into_any_element(),
            )
        } else {
            None
        };

        div().child(header).children(error_block).children(body).into_any_element()
    }

    // ---- Addon recovery --------------------------------------------------------------------------------------

    fn confirm_reset(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let this = cx.weak_entity();
        alert_dialog::confirm(
            window,
            cx,
            AlertSpec::new(
                "Reset addon settings?",
                "This clears the addon_settings file. Your addons stay installed, but their saved settings are lost.",
            )
            .confirm_label("Reset settings")
            .auto_close(false)
            .on_confirm(move |window, cx| {
                let handle = window.window_handle();
                on_reset(this.clone(), handle, true, cx);
            }),
        );
    }

    fn confirm_uninstall(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let this = cx.weak_entity();
        let count = self.addons.read(cx).installed_addons.len();
        let noun = if count == 1 { "addon" } else { "addons" };
        alert_dialog::confirm(
            window,
            cx,
            AlertSpec::new(
                "Uninstall all addons?",
                format!(
                    "This removes all {count} installed {noun} and resets addon settings. You can reinstall them later."
                ),
            )
            .cancel_label("Keep addons")
            .confirm_label("Uninstall all")
            .destructive(true)
            .auto_close(false)
            .on_confirm(move |window, cx| {
                let handle = window.window_handle();
                on_uninstall_all(this.clone(), handle, cx);
            }),
        );
    }

    fn recovery(&self, cx: &mut Context<Self>) -> AnyElement {
        let t = cx.tokens();
        let n = self.addons.read(cx).installed_addons.len();
        let row = |title: &'static str, desc: String, button: Button| {
            div()
                .flex()
                .items_center()
                .justify_between()
                .gap(px(24.0))
                .px_4()
                .py(px(14.0))
                .child(
                    div()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .gap_1()
                        .child(div().text_sm_tw().line_height(px(14.0)).font_weight(FontWeight::MEDIUM).child(title))
                        .child(div().text_xs_tw().text_color(t.muted_foreground).child(desc)),
                )
                .child(button)
        };
        div()
            .child(row(
                "Reset addon settings",
                "Clears addon_settings. Try this first.".into(),
                Button::new("reset-settings")
                    .variant(ButtonVariant::Outline)
                    .size(ButtonSize::Sm)
                    .label("Reset")
                    .disabled(self.resetting)
                    .on_click(cx.listener(|this, _, window, cx| this.confirm_reset(window, cx))),
            ))
            .child(
                div().border_t_1().border_color(alpha(t.border, 0.6)).child(row(
                    "Uninstall all addons",
                    if n == 0 {
                        "No addons installed".into()
                    } else {
                        "Removes every installed addon, then resets settings. Last resort.".into()
                    },
                    Button::new("uninstall-all")
                        .variant(ButtonVariant::Destructive)
                        .size(ButtonSize::Sm)
                        .label("Uninstall all")
                        .disabled(n == 0)
                        .on_click(cx.listener(|this, _, window, cx| this.confirm_uninstall(window, cx))),
                )),
            )
            .into_any_element()
    }

    fn report_guide(&self, cx: &mut Context<Self>) -> AnyElement {
        let t = cx.tokens();
        let steps: Vec<AnyElement> = vec![
            div().child("Open the Dashboard").into_any_element(),
            div().child("Select the addon from the list").into_any_element(),
            div().child("Open the menu in the details pane").into_any_element(),
            div()
                .flex()
                .child("Choose\u{00a0}")
                .child(div().font_weight(FontWeight::MEDIUM).text_color(t.foreground).child("Report issue"))
                .into_any_element(),
        ];
        div()
            .child(div().flex().flex_col().gap(px(10.0)).px_4().py(px(14.0)).children(
                steps.into_iter().enumerate().map(|(i, step)| {
                    div()
                        .flex()
                        .items_center()
                        .gap(px(10.0))
                        .child(
                            div()
                                .flex()
                                .flex_none()
                                .items_center()
                                .justify_center()
                                .size(px(20.0))
                                .rounded(radius::MD)
                                .bg(alpha(t.muted, 0.6))
                                .text_px_tw(11.0)
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(t.muted_foreground)
                                .child(format!("{}", i + 1)),
                        )
                        .child(div().text_sm_tw().text_color(alpha(t.foreground, 0.9)).child(step))
                }),
            ))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .border_t_1()
                    .border_color(alpha(t.border, 0.6))
                    .bg(alpha(t.muted, 0.15))
                    .px_4()
                    .py_3()
                    .child(Icon::new(icons::INFO).size(px(14.0)).text_color(t.muted_foreground))
                    .child(
                        div()
                            .text_xs_tw()
                            .text_color(t.muted_foreground)
                            .child("Available for managed addons only. Requires a GitHub account."),
                    ),
            )
            .into_any_element()
    }
}

fn close_dialog(handle: AnyWindowHandle, cx: &mut App) {
    let _ = handle.update(cx, |_, window, cx| alert_dialog_close(window, cx));
}

fn alert_dialog_close(window: &mut Window, cx: &mut App) {
    crate::ui::components::dialog::close(window, cx);
}

/// Resets the addon settings file and toasts the outcome. Closes the dialog after a success when `close_on_success`
/// is set. Does nothing while a reset is already running.
fn on_reset(this: WeakEntity<TroubleshootingPage>, handle: AnyWindowHandle, close_on_success: bool, cx: &mut App) {
    cx.spawn(async move |cx| {
        if reset_settings(&this, cx).await == Some(true) && close_on_success {
            cx.update(|cx| close_dialog(handle, cx));
        }
    })
    .detach();
}

/// Resets the addon settings file and toasts the outcome. Returns whether the reset succeeded, or `None` without
/// doing anything while a reset is already running.
async fn reset_settings(this: &WeakEntity<TroubleshootingPage>, cx: &mut AsyncApp) -> Option<bool> {
    let busy = this
        .update(cx, |t, cx| {
            cx.notify();
            std::mem::replace(&mut t.resetting, true)
        })
        .unwrap_or(true);
    if busy {
        return None;
    }
    let backend = cx.update(|cx| state::backend(cx));
    let res = run_blocking(cx, move || addon::reset_addon_settings(&backend.config.aac_dir()?)).await;
    let ok = cx.update(|cx| reset_toast(res, cx));
    this.update(cx, |t, cx| {
        t.resetting = false;
        cx.notify();
    })
    .ok();
    Some(ok)
}

fn reset_toast(res: anyhow::Result<()>, cx: &mut App) -> bool {
    match res {
        Err(e) => {
            log::error!("Resetting addon settings failed: {e:#}");
            toast::show(cx, toast::Toast::new("Error", "Failed to reset addon settings").icon(icons::TRIANGLE_ALERT));
            false
        }
        Ok(()) => {
            toast::show(
                cx,
                toast::Toast::new("Addon settings reset", "Addon settings reset. Restart the game.").icon(icons::CHECK),
            );
            true
        }
    }
}

/// Uninstalls every installed addon through the store, then resets the addon settings. Does nothing while an
/// uninstall is already running.
fn on_uninstall_all(this: WeakEntity<TroubleshootingPage>, handle: AnyWindowHandle, cx: &mut App) {
    let busy = this.update(cx, |t, _| std::mem::replace(&mut t.uninstalling, true)).unwrap_or(true);
    if busy {
        return;
    }
    this.update(cx, |_, cx| cx.notify()).ok();
    let uninstall = state::addons(cx).update(cx, |a, cx| a.uninstall_all(cx));
    cx.spawn(async move |cx| {
        match uninstall.await {
            Ok(removed) => {
                cx.update(|cx| {
                    toast::show(
                        cx,
                        toast::Toast::new(
                            "Success",
                            format!(
                                "{removed} {} uninstalled. Restart the game.",
                                if removed == 1 { "addon" } else { "addons" }
                            ),
                        )
                        .icon(icons::CHECK),
                    );
                    close_dialog(handle, cx);
                });
                // Reset the addon settings now that all addons are gone. The `uninstalling` flag stays set until the
                // reset has finished. When a reset from its own button is still running, it is not started a second
                // time.
                reset_settings(&this, cx).await;
            }
            Err(e) => {
                log::error!("Uninstalling all addons failed: {e:#}");
                cx.update(|cx| {
                    toast::show(cx, toast::Toast::new("Error", error_message(&e)).icon(icons::TRIANGLE_ALERT))
                });
            }
        }
        this.update(cx, |t, cx| {
            t.uninstalling = false;
            cx.notify();
        })
        .ok();
    })
    .detach();
}

impl Render for TroubleshootingPage {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = cx.tokens();
        let diagnostics = self.diagnostics_panel(cx);
        let recovery = self.recovery(cx);
        let guide = self.report_guide(cx);
        let _: Option<SharedString> = None;
        div()
            .flex()
            .flex_col()
            .size_full()
            .min_h_0()
            .child(page_header(&t, icons::WRENCH, "Troubleshooting", "Diagnose and fix addon problems"))
            .child(
                ScrollArea::new("troubleshooting-scroll").flex_1().min_h_0().child(
                    div()
                        .w_full()
                        .max_w(px(672.0))
                        .mx_auto()
                        .px_4()
                        .py_8()
                        .child(
                            Section::new("Diagnostics")
                                .description("Errors found in the game's addon log")
                                .child(diagnostics),
                        )
                        .child(
                            Section::new("Addon recovery")
                                .mt_8()
                                .description(
                                    "When every addon stops working, the addon_settings file is usually corrupted",
                                )
                                .child(recovery),
                        )
                        .child(
                            Section::new("Problem with one addon")
                                .mt_8()
                                .description("Report it to the addon author from the Dashboard")
                                .child(guide),
                        ),
                ),
            )
    }
}
