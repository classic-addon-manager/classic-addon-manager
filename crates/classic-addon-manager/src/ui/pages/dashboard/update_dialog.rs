//! Dialog offering the available update of an installed addon: release notes, a one-click update with progress that
//! closes the dialog with a toast on success, and an error state with a retry button.
use cam_core::{
    addon::Addon,
    api::{Release, encode_uri_component},
    version::release_bump,
};
use cam_ui::{
    components::{
        Button, ButtonSize, ButtonVariant, DialogDescription, DialogSpec, DialogTitle, Markdown, MarkdownStyle,
        ScrollbarAxis, dialog, scrollbar_overlay,
    },
    icons,
    theme::{TokensExt, TwText, alpha, blend, radius, shadow_lg_tinted},
    toast,
};
use chrono::{DateTime, Utc};
use gpui_kit::base::actions::Confirm;
use gpui_kit::component::Icon;
use gpui_kit::{
    App, AppContext as _, Context, Entity, FocusHandle, FontWeight, InteractiveElement as _, IntoElement,
    ParentElement, Render, ScrollHandle, SharedString, StatefulInteractiveElement as _, Styled, Window, div,
    linear_color_stop, linear_gradient, prelude::FluentBuilder as _, px,
};

use super::{
    install_flow::{Verb, outcome_failure},
    phase_ui::{Phase, error_alert, eyebrow, kbd, running_steps, running_view},
};
use crate::{
    state::{self, AddonState},
    ui::{
        addon_icon::{AddonIcon, AddonIconSize},
        format::{format_to_local_date, time_ago},
    },
};

// ------------------------------------------------------------------------------------------------ pure helpers

/// Time between the installed copy and the release, like `+3 days` or `same day`.
pub fn release_gap_label(installed_at: Option<DateTime<Utc>>, published_at: Option<DateTime<Utc>>) -> Option<String> {
    let (from, to) = (installed_at?, published_at?);
    if to < from {
        return None;
    }
    let days = (to - from).num_days();
    Some(match days {
        0 => "same day".to_string(),
        1 => "+1 day".to_string(),
        n => format!("+{n} days"),
    })
}

/// Whether the changelog has a Markdown heading line (one to six `#` followed by whitespace). If so, the release notes
/// are drawn with a timeline rail.
pub fn has_sections(changelog: &str) -> bool {
    changelog.lines().any(|line| {
        let hashes = line.chars().take_while(|c| *c == '#').count();
        (1..=6).contains(&hashes) && line[hashes..].chars().next().is_some_and(char::is_whitespace)
    })
}

// ------------------------------------------------------------------------------------------------ view

pub struct UpdateDialog {
    addon: Addon,
    release: Release,
    addons: Entity<AddonState>,
    /// While running, `started` is the installed addon and the release the update started with.
    phase: Phase<(Addon, Release)>,
    focus: FocusHandle,
    notes_scroll: ScrollHandle,
}

/// Opens the update dialog for `addon` and its newer `release`. It can't be dismissed while the update runs.
pub fn open(addon: Addon, release: Release, window: &mut Window, cx: &mut App) {
    let view = cx.new(|cx| UpdateDialog::new(addon, release, cx));
    let focus = view.read(cx).focus.clone();
    let running = view.clone();
    dialog::open(
        window,
        cx,
        DialogSpec::new()
            .padded(false)
            .max_height_frac(0.85)
            .locked(move |cx| running.read(cx).phase.is_running())
            .content_view(view),
    );
    window.defer(cx, move |window, cx| focus.focus(window, cx));
}

impl UpdateDialog {
    fn new(addon: Addon, release: Release, cx: &mut Context<Self>) -> Self {
        let addons = state::addons(cx);
        cx.observe(&addons, |this, store, cx| {
            this.phase.follow(store.read(cx), &this.addon.name);
            cx.notify();
        })
        .detach();
        cx.observe(&state::catalog(cx), |_, _, cx| cx.notify()).detach();
        Self { addon, release, addons, phase: Phase::Idle, focus: cx.focus_handle(), notes_scroll: ScrollHandle::new() }
    }

    /// While idle the dialog follows the store, so it shows the latest installed addon and release. Once the update has
    /// started it keeps showing the values it started with.
    fn live(&self, cx: &App) -> (Addon, Release) {
        match &self.phase {
            Phase::Running { started, .. } => started.clone(),
            _ => {
                let store = self.addons.read(cx);
                let addon =
                    store.installed_addons.iter().find(|a| a.name == self.addon.name).unwrap_or(&self.addon).clone();
                let release = store.latest_releases.get(&self.addon.name).unwrap_or(&self.release).clone();
                (addon, release)
            }
        }
    }

    fn set_phase(&mut self, phase: Phase<(Addon, Release)>, cx: &mut Context<Self>) {
        self.phase = phase;
        cx.notify();
    }

    /// Updates through the addon store and shows the outcome.
    fn handle_update(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.phase.is_running() {
            return;
        }
        let (from, to) = self.live(cx);
        let update = self.addons.update(cx, |s, cx| s.update(from.name.clone(), to.tag_name.clone(), cx));
        self.set_phase(Phase::running((from.clone(), to.clone())), cx);
        cx.spawn_in(window, async move |this, cx| {
            let outcome = update.await;
            let _ = this.update_in(cx, |this, window, cx| {
                match outcome_failure(outcome, &from.name, &from.alias, &to.tag_name, Verb::Update, cx) {
                    Some(failure) => this.set_phase(Phase::Failed(failure), cx),
                    None => {
                        this.set_phase(Phase::Idle, cx);
                        dialog::close(window, cx);
                        toast::show(
                            cx,
                            toast::Toast::new(
                                format!("{} is up to date", from.alias),
                                format!("{} → {}", from.version, to.tag_name),
                            ),
                        );
                    }
                }
            });
        })
        .detach();
    }
}

fn chip(
    icon: cam_ui::icons::Lucide,
    text: String,
    mono: bool,
    title: Option<&'static str>,
    cx: &App,
) -> impl IntoElement {
    let t = cx.tokens();
    let _ = title;
    div()
        .flex()
        .items_center()
        .gap(px(6.0))
        .rounded(radius::LG)
        .border_1()
        .border_color(alpha(t.border, 0.7))
        .bg(alpha(t.background, 0.4))
        .px(px(10.0))
        .py_1()
        .text_xs_tw()
        .text_color(t.muted_foreground)
        .when(mono, |d| d.font_mono_tw())
        .child(Icon::new(icon).size(px(14.0)))
        .child(SharedString::from(text))
}

impl UpdateDialog {
    fn hero(&self, addon: &Addon, release: &Release, icon_url: Option<String>, cx: &App) -> impl IntoElement {
        let t = cx.tokens();
        let bump = release_bump(&addon.version, &release.tag_name);
        let gap = release_gap_label(addon.updated_at, release.published_at);
        let installed_at = addon.updated_at.map(|at| format_to_local_date(&at, true));
        let published_at =
            release.published_at.map(|at| format!("{} · {}", format_to_local_date(&at, true), time_ago(&at)));
        let byline = [
            (!addon.author.is_empty()).then(|| format!("by {}", addon.author)),
            (!addon.repo.is_empty()).then(|| addon.repo.clone()),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(" · ");
        let dependency_count = addon.dependencies.len();
        let nbsp = "\u{a0}".to_string();

        let installed = div()
            .min_w_0()
            .flex_none()
            .child(eyebrow("Installed").mb_1().text_color(t.muted_foreground))
            .child(
                div()
                    .flex()
                    .h(px(36.0))
                    .max_w(px(144.0))
                    .items_center()
                    .truncate()
                    .font_mono_tw()
                    .text_2xl_tw()
                    .text_color(t.muted_foreground)
                    .child(SharedString::from(if addon.version.is_empty() {
                        "—".to_string()
                    } else {
                        addon.version.clone()
                    })),
            )
            .child(
                div()
                    .mt_1()
                    .text_xs_tw()
                    .text_color(t.muted_foreground)
                    .child(SharedString::from(installed_at.unwrap_or_else(|| nbsp.clone()))),
            );

        let rail = div()
            .relative()
            .flex()
            .flex_1()
            .min_w(px(48.0))
            .items_center()
            .mt(px(30.0))
            .child(div().flex_1().h(px(2.0)).border_t_1().border_dashed().border_color(alpha(t.primary, 0.5)))
            .child(
                div()
                    .ml(px(-6.0))
                    .flex_none()
                    .child(Icon::new(icons::CHEVRON_RIGHT).size(px(16.0)).text_color(t.primary)),
            )
            .children(gap.map(|gap| {
                div().absolute().left_0().right_0().flex().justify_center().child(
                    div()
                        .rounded_full()
                        .border_1()
                        .border_color(t.border)
                        .bg(t.background)
                        .px_2()
                        .py(px(2.0))
                        .text_size(px(11.0))
                        .line_height(px(16.5))
                        .whitespace_nowrap()
                        .text_color(t.muted_foreground)
                        .child(SharedString::from(gap)),
                )
            }));

        let available = div()
            .min_w_0()
            .flex_none()
            .child(eyebrow("Available").mb_1().text_color(t.primary))
            .child(
                div()
                    .flex()
                    .h(px(36.0))
                    .max_w(px(192.0))
                    .items_center()
                    .truncate()
                    .font_mono_tw()
                    .text_size(px(30.0))
                    .line_height(px(36.0))
                    .font_weight(FontWeight::BOLD)
                    .child(SharedString::from(release.tag_name.clone())),
            )
            .child(
                div()
                    .mt_1()
                    .text_xs_tw()
                    .text_color(t.muted_foreground)
                    .child(SharedString::from(published_at.unwrap_or(nbsp))),
            );

        div()
            .flex_none()
            .px_6()
            .pt_6()
            .child(
                div()
                    .flex()
                    .items_start()
                    .gap_4()
                    .pr_8()
                    .child(
                        AddonIcon::new(SharedString::from(addon.alias.clone()), icon_url)
                            .size(AddonIconSize::Lg)
                            .shadow(shadow_lg_tinted(alpha(t.primary, 0.1))),
                    )
                    .child(
                        div()
                            .min_w_0()
                            .flex_1()
                            .pt_1()
                            .child(
                                DialogTitle::new(SharedString::from(addon.alias.clone()))
                                    .truncate()
                                    .text_xl_tw()
                                    .line_height(px(25.0)),
                            )
                            .child(
                                DialogDescription::new(SharedString::from(if byline.is_empty() {
                                    "Update available".to_string()
                                } else {
                                    byline
                                }))
                                .mt_1()
                                .truncate(),
                            ),
                    )
                    .children(bump.map(|bump| {
                        div()
                            .mt(px(6.0))
                            .flex_none()
                            .rounded_full()
                            .bg(alpha(t.primary, 0.15))
                            .px(px(10.0))
                            .py_1()
                            .text_size(px(10.0))
                            .line_height(px(15.0))
                            .font_weight(FontWeight::BOLD)
                            .text_color(t.primary)
                            .child(bump.label().to_uppercase())
                    })),
            )
            .child(div().mt_6().flex().items_start().gap_4().child(installed).child(rail).child(available))
            .when(dependency_count > 0 || !addon.commit.is_empty() || !addon.branch.is_empty(), |d| {
                d.child(
                    div()
                        .mt(px(20.0))
                        .flex()
                        .flex_wrap()
                        .gap_2()
                        .children((dependency_count > 0).then(|| {
                            chip(
                                icons::BOXES,
                                format!(
                                    "{dependency_count} {}",
                                    if dependency_count == 1 { "dependency" } else { "dependencies" }
                                ),
                                false,
                                None,
                                cx,
                            )
                        }))
                        .children((!addon.commit.is_empty()).then(|| {
                            chip(
                                icons::GIT_COMMIT_HORIZONTAL,
                                addon.commit.chars().take(7).collect(),
                                true,
                                Some("Installed commit"),
                                cx,
                            )
                        }))
                        .children(
                            (!addon.branch.is_empty())
                                .then(|| chip(icons::GIT_BRANCH, addon.branch.clone(), false, None, cx)),
                        ),
                )
            })
    }

    fn release_notes(&self, repo: &str, release: &Release, cx: &App) -> impl IntoElement {
        let t = cx.tokens();
        let changelog = release.body.trim().to_string();
        let sections = has_sections(&changelog);
        let link = (!repo.is_empty()).then(|| {
            let url = format!("https://github.com/{repo}/releases/tag/{}", encode_uri_component(&release.tag_name));
            let primary = t.primary;
            div()
                .id("full-changelog")
                .flex()
                .items_center()
                .gap_1()
                .rounded(radius::SM)
                .text_xs_tw()
                .text_color(primary)
                .cursor_pointer()
                .hover(|s| s.underline().text_xs_tw().text_color(primary))
                .on_click(move |_, _, cx| cx.open_url(&url))
                .child("Full changelog")
                .child(Icon::new(icons::ARROW_UP_RIGHT).size(px(14.0)))
        });

        let body = if changelog.is_empty() {
            div()
                .rounded(radius::XL)
                .border_1()
                .border_dashed()
                .border_color(alpha(t.border, 0.6))
                .px_4()
                .py_6()
                .text_center()
                .text_sm_tw()
                .text_color(t.muted_foreground)
                .child("No release notes provided.")
                .into_any_element()
        } else {
            let card = blend(t.card, 0.4, t.background);
            div()
                .flex()
                .flex_col()
                .min_h_0()
                .rounded(radius::XL)
                .border_1()
                .border_color(alpha(t.border, 0.6))
                .bg(alpha(t.card, 0.4))
                .overflow_hidden()
                .child(
                    div()
                        .relative()
                        .max_h(px(256.0))
                        .min_h_0()
                        .child(
                            div()
                                .id("release-notes-scroll")
                                .max_h(px(256.0))
                                .overflow_y_scroll()
                                .track_scroll(&self.notes_scroll)
                                .child(
                                    Markdown::new(
                                        SharedString::from(format!(
                                            "update-notes-{}-{}",
                                            self.addon.name, release.tag_name
                                        )),
                                        SharedString::from(changelog),
                                    )
                                    .style(MarkdownStyle::ReleaseNotes)
                                    .timeline(sections)
                                    .p_4()
                                    .pb_8(),
                                ),
                        )
                        .child(scrollbar_overlay("notes-scrollbar", &self.notes_scroll, ScrollbarAxis::Vertical))
                        .child(div().absolute().left_0().right_0().bottom_0().h(px(32.0)).bg(linear_gradient(
                            180.0,
                            linear_color_stop(gpui_kit::transparent_black(), 0.0),
                            linear_color_stop(card, 1.0),
                        ))),
                )
                .into_any_element()
        };

        div()
            .mt(px(20.0))
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .border_t_1()
            .border_color(alpha(t.border, 0.6))
            .px_6()
            .pt_4()
            .child(
                div()
                    .mb_3()
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(div().text_sm_tw().font_weight(FontWeight::SEMIBOLD).child("What's new"))
                    .children(link),
            )
            .child(body)
    }

    fn footer(&self, to: &Release, cx: &mut Context<Self>) -> impl IntoElement {
        let t = cx.tokens();
        let this = cx.entity();
        let content = match &self.phase {
            Phase::Running { started: (from, to), op } => {
                running_steps("update-steps", running_view(&from.name, &to.tag_name, op, "Updating…"), cx)
                    .into_any_element()
            }
            phase => {
                let is_error = matches!(phase, Phase::Failed(_));
                let label = if is_error { "Retry update".to_string() } else { format!("Update to {}", to.tag_name) };
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        Button::new("update-later")
                            .variant(ButtonVariant::Ghost)
                            .child("Later")
                            .child(kbd("Esc", false, cx))
                            .on_click(|_, window, cx| dialog::close(window, cx)),
                    )
                    .child(
                        Button::new("update-now")
                            .size(ButtonSize::Lg)
                            .has_icon(true)
                            .ml_auto()
                            .mr_2()
                            .h(px(40.0))
                            .min_w(px(208.0))
                            .rounded(radius::XL)
                            .px_4()
                            .justify_start()
                            .shadow(shadow_lg_tinted(alpha(t.primary, 0.2)))
                            .child(Icon::new(if is_error { icons::ROTATE_CCW } else { icons::DOWNLOAD }).size(px(16.0)))
                            .child(div().min_w_0().truncate().child(SharedString::from(label)))
                            .child(kbd("↵", true, cx).ml_auto())
                            .on_click(move |_, window, cx| this.update(cx, |t, cx| t.handle_update(window, cx))),
                    )
                    .into_any_element()
            }
        };
        div()
            .mt_5()
            .flex_none()
            .border_t_1()
            .border_color(alpha(t.border, 0.6))
            .bg(alpha(t.muted, 0.2))
            .px_6()
            .py_4()
            .child(content)
    }
}

impl Render for UpdateDialog {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let (addon, release) = self.live(cx);
        let icon_url = state::catalog::current(cx).and_then(|c| c.icon_url(&addon.name).map(str::to_owned));

        let root = div()
            .id("update-dialog")
            .track_focus(&self.focus)
            // The dialog turns Enter into a Confirm action before key handlers run, so listen for that.
            .on_action(cx.listener(|this, _: &Confirm, window, cx| this.handle_update(window, cx)))
            .relative()
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0();

        let alert = match &self.phase {
            Phase::Failed(failure) => Some(error_alert(failure, cx)),
            _ => None,
        };
        root.child(self.hero(&addon, &release, icon_url, cx))
            .child(self.release_notes(&addon.repo, &release, cx))
            .children(alert)
            .child(self.footer(&release, cx))
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};

    #[test]
    fn gap_labels() {
        let d = |day| Some(Utc.with_ymd_and_hms(2025, 7, day, 12, 0, 0).unwrap());
        assert_eq!(release_gap_label(d(1), d(1)).as_deref(), Some("same day"));
        assert_eq!(release_gap_label(d(1), d(2)).as_deref(), Some("+1 day"));
        assert_eq!(release_gap_label(d(1), d(11)).as_deref(), Some("+10 days"));
        assert_eq!(release_gap_label(d(5), d(1)), None);
        assert_eq!(release_gap_label(None, d(1)), None);
    }

    #[test]
    fn detects_sections() {
        assert!(has_sections("# Title\ntext"));
        assert!(has_sections("intro\n\n### Fixes\n- a"));
        assert!(!has_sections("plain\n#nospace\n####### seven"));
        assert!(!has_sections(""));
    }
}
