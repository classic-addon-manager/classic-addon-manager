//! Pick any published release of an addon (upgrade, rollback, reinstall), with release notes, keyboard
//! navigation and install progress. Closes with a toast on success.
use std::collections::HashSet;

use cam_core::{addon::Addon, api::Release};
use cam_ui::{
    anim::fade_slide_in,
    components::{
        Button, ButtonSize, ButtonVariant, DialogDescription, DialogSpec, DialogTitle, EmptyState, EmptyTone, Input,
        InputState, Markdown, MarkdownStyle, ScrollArea, ScrollbarAxis, Skeleton, dialog, scrollbar_overlay,
        text_input,
    },
    icons::{self, Lucide},
    theme::{TokensExt, Tw, TwText, alpha, radius, shadow_lg_tinted, tw},
    toast,
};
use chrono::{DateTime, Utc};
use gpui_kit::component::Icon;
use gpui_kit::{
    App, AppContext as _, Context, Entity, FocusHandle, FontWeight, InteractiveElement as _, IntoElement, KeyDownEvent,
    ParentElement, Render, ScrollHandle, SharedString, StatefulInteractiveElement as _, Styled, Window, div,
    linear_color_stop, linear_gradient, prelude::FluentBuilder as _, px,
};

use super::{
    install_flow::{Verb, outcome_failure},
    phase_ui::{Phase, error_alert, eyebrow, kbd, running_steps, running_view},
};
use crate::{
    state::{self, Load},
    ui::format::format_to_local_date,
};

const DAY_MS: i64 = 86_400_000;

// ------------------------------------------------------------------------------------------------ pure logic

/// Where the selected release sits relative to the installed one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Upgrade,
    Rollback,
    Reinstall,
    Install,
}

impl Direction {
    fn label(self) -> &'static str {
        match self {
            Direction::Upgrade => "Upgrade",
            Direction::Rollback => "Rollback",
            Direction::Reinstall => "Reinstall",
            Direction::Install => "Install",
        }
    }
    fn action(self) -> &'static str {
        match self {
            Direction::Upgrade => "Upgrade to",
            Direction::Rollback => "Roll back to",
            Direction::Reinstall => "Reinstall",
            Direction::Install => "Install",
        }
    }
    fn running(self) -> &'static str {
        match self {
            Direction::Upgrade => "Upgrading…",
            Direction::Rollback => "Rolling back…",
            Direction::Reinstall => "Reinstalling…",
            Direction::Install => "Installing…",
        }
    }
    fn done(self) -> &'static str {
        match self {
            Direction::Upgrade => "upgraded",
            Direction::Rollback => "rolled back",
            Direction::Reinstall => "reinstalled",
            Direction::Install => "installed",
        }
    }
    fn icon(self) -> Lucide {
        match self {
            Direction::Upgrade => icons::ARROW_UP,
            Direction::Rollback => icons::ROTATE_CCW,
            Direction::Reinstall => icons::REFRESH_CW,
            Direction::Install => icons::DOWNLOAD,
        }
    }
    fn tone(self, cx: &App) -> gpui_kit::Hsla {
        match self {
            Direction::Upgrade => tw(Tw::Emerald, 500),
            Direction::Rollback => cx.tokens().primary,
            _ => cx.tokens().muted_foreground,
        }
    }
}

/// Direction of the change the dialog would make. `installed_index` is the position of the installed tag in the
/// newest-first list.
pub fn direction(installed_index: Option<usize>, selected_index: usize) -> Direction {
    match installed_index {
        None => Direction::Install,
        Some(i) if selected_index < i => Direction::Upgrade,
        Some(i) if selected_index > i => Direction::Rollback,
        Some(_) => Direction::Reinstall,
    }
}

fn plural(count: i64, unit: &str) -> String {
    format!("{count} {unit}{}", if count == 1 { "" } else { "s" })
}

/// How long ago a release was published, like `today`, `3 days ago` or `2 months ago`. `None` when it has no date.
pub fn release_age(published_at: Option<DateTime<Utc>>, now: &DateTime<Utc>) -> Option<String> {
    let days = (*now - published_at?).num_milliseconds().div_euclid(DAY_MS);
    Some(if days < 1 {
        "today".to_string()
    } else if days < 14 {
        format!("{} ago", plural(days, "day"))
    } else if days < 60 {
        format!("{} ago", plural(days / 7, "week"))
    } else if days < 730 {
        format!("{} ago", plural(days / 30, "month"))
    } else {
        format!("{} ago", plural(days / 365, "year"))
    })
}

/// Sorts releases newest first, by whole milliseconds. The sort is stable: releases published in the same millisecond
/// and releases without a date keep the order the API gave them, and those without a date go to the end.
pub fn newest_first(mut releases: Vec<Release>) -> Vec<Release> {
    releases.sort_by_key(|r| std::cmp::Reverse(r.published_at.map(|t| t.timestamp_millis())));
    releases
}

/// Keeps the first release of each tag.
pub fn unique_by_tag(releases: Vec<Release>) -> Vec<Release> {
    let mut seen = HashSet::new();
    releases.into_iter().filter(|r| seen.insert(r.tag_name.clone())).collect()
}

/// Case-insensitive substring filter of the version list (visible rows keep their absolute index).
pub fn visible_indices(releases: &[Release], filter: &str) -> Vec<usize> {
    let query = filter.trim().to_lowercase();
    releases
        .iter()
        .enumerate()
        .filter(|(_, r)| query.is_empty() || r.tag_name.to_lowercase().contains(&query))
        .map(|(i, _)| i)
        .collect()
}

/// Arrow key navigation: next selected absolute index (`None` when nothing is visible).
pub fn step_selection(visible: &[usize], selected_index: usize, down: bool) -> Option<usize> {
    if visible.is_empty() {
        return None;
    }
    let at = visible.iter().position(|i| *i == selected_index);
    let next = match at {
        None => 0,
        Some(at) => (at as isize + if down { 1 } else { -1 }).clamp(0, visible.len() as isize - 1) as usize,
    };
    Some(visible[next])
}

// ------------------------------------------------------------------------------------------------ view

pub struct VersionDialog {
    addon: Addon,
    versions: Entity<state::queries::VersionsQuery>,
    /// The versions to pick from: unique tags, newest first. Derived from the query's value.
    list: Vec<Release>,
    selected_tag: Option<String>,
    filter: Entity<InputState>,
    /// While running, `started` is the tag being installed and the direction of the change.
    phase: Phase<(String, Direction)>,
    focus: FocusHandle,
    list_scroll: ScrollHandle,
}

/// Opens the version picker dialog for `addon`. It can't be dismissed while an install runs.
pub fn open(addon: Addon, window: &mut Window, cx: &mut App) {
    let view = cx.new(|cx| VersionDialog::new(addon, window, cx));
    let focus = view.read(cx).focus.clone();
    let running = view.clone();
    dialog::open(
        window,
        cx,
        DialogSpec::new()
            .padded(false)
            .max_width(px(720.0))
            .max_height_frac(0.9)
            .locked(move |cx| running.read(cx).is_running())
            .content_view(view),
    );
    window.defer(cx, move |window, cx| focus.focus(window, cx));
}

impl VersionDialog {
    fn new(addon: Addon, window: &mut Window, cx: &mut Context<Self>) -> Self {
        cx.observe(&state::addons(cx), |this, store, cx| {
            this.phase.follow(store.read(cx), &this.addon.name);
            cx.notify();
        })
        .detach();
        let versions = state::queries::versions(cx);
        cx.observe(&versions, |this, _, cx| {
            this.sync_list(cx);
            cx.notify();
        })
        .detach();
        let filter = text_input(window, cx, "Filter versions");
        cx.observe(&filter, |_, _, cx| cx.notify()).detach();
        let mut this = Self {
            addon,
            versions,
            list: Vec::new(),
            selected_tag: None,
            filter,
            phase: Phase::Idle,
            focus: cx.focus_handle(),
            list_scroll: ScrollHandle::new(),
        };
        this.sync_list(cx);
        this.load(false, cx);
        this
    }

    /// Refreshes the derived version list from the query's value, keeping the last list while there is none.
    fn sync_list(&mut self, cx: &App) {
        if let Some(list) = self.versions.read(cx).value(&self.addon.name) {
            self.list = newest_first(unique_by_tag(list.clone()));
        }
    }

    /// Asks the server for the published releases of the addon (the Retry button forces it). A failure is shown as a
    /// toast while the dialog is open, the query logs it.
    fn load(&mut self, force: bool, cx: &mut Context<Self>) {
        let name = self.addon.name.clone();
        let task = self.versions.update(cx, |q, cx| if force { q.refetch(name, cx) } else { q.fetch(name, cx) });
        cx.spawn(async move |this, cx| {
            if let Err(e) = task.await {
                let _ = this.update(cx, |_, cx| {
                    toast::show(cx, toast::Toast::new("Error", format!("Failed to load versions: {}", e.message())));
                });
            }
        })
        .detach();
    }

    fn list(&self) -> &[Release] {
        &self.list
    }

    fn installed_index(&self) -> Option<usize> {
        self.list().iter().position(|r| r.tag_name == self.addon.version)
    }

    fn selected_index(&self) -> usize {
        self.selected_tag.as_ref().and_then(|tag| self.list().iter().position(|r| &r.tag_name == tag)).unwrap_or(0)
    }

    fn is_running(&self) -> bool {
        self.phase.is_running()
    }

    fn set_phase(&mut self, phase: Phase<(String, Direction)>, cx: &mut Context<Self>) {
        self.phase = phase;
        cx.notify();
    }

    fn select(&mut self, tag: String, cx: &mut Context<Self>) {
        if self.is_running() {
            return;
        }
        self.selected_tag = Some(tag);
        self.phase = Phase::Idle;
        cx.notify();
    }

    fn close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.is_running() {
            return;
        }
        dialog::close(window, cx);
    }

    /// Installs the selected release through the addon store and shows the outcome.
    fn handle_install(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.is_running() || self.list().is_empty() {
            return;
        }
        let index = self.selected_index();
        let to = self.list()[index].tag_name.clone();
        let dir = direction(self.installed_index(), index);
        let addon = self.addon.clone();
        let update = state::addons(cx).update(cx, |s, cx| s.update(addon.name.clone(), to.clone(), cx));
        self.set_phase(Phase::running((to.clone(), dir)), cx);
        cx.spawn_in(window, async move |this, cx| {
            let outcome = update.await;
            let _ = this.update_in(cx, |this, window, cx| {
                match outcome_failure(outcome, &addon.name, &addon.alias, &to, Verb::Install, cx) {
                    Some(failure) => this.set_phase(Phase::Failed(failure), cx),
                    None => {
                        this.set_phase(Phase::Idle, cx);
                        this.close(window, cx);
                        let description = if dir == Direction::Reinstall || addon.version.is_empty() {
                            to.clone()
                        } else {
                            format!("{} → {}", addon.version, to)
                        };
                        toast::show(cx, toast::Toast::new(format!("{} {}", addon.alias, dir.done()), description));
                    }
                }
            });
        })
        .detach();
    }

    fn on_key(&mut self, ev: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        if self.is_running() || ev.keystroke.modifiers.number_of_modifiers() != 0 {
            return;
        }
        match ev.keystroke.key.as_str() {
            key @ ("down" | "up") => {
                let query = self.filter.read(cx).value().to_string();
                let visible = visible_indices(self.list(), &query);
                if let Some(next) = step_selection(&visible, self.selected_index(), key == "down") {
                    let tag = self.list()[next].tag_name.clone();
                    self.select(tag, cx);
                    if let Some(pos) = visible.iter().position(|i| *i == next) {
                        self.list_scroll.scroll_to_item(pos);
                    }
                    cx.stop_propagation();
                }
            }
            "enter" => {
                self.handle_install(window, cx);
                cx.stop_propagation();
            }
            _ => {}
        }
    }
}

// ------------------------------------------------------------------------------------------------ rendering

fn row_badge(text: &'static str, fg: gpui_kit::Hsla) -> gpui_kit::Div {
    div()
        .flex_none()
        .rounded_full()
        .bg(alpha(fg, 0.15))
        .px(px(6.0))
        .py(px(1.0))
        .text_size(px(9.0))
        .line_height(px(13.5))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(fg)
        .child(text.to_uppercase())
}

struct RowProps<'a> {
    release: &'a Release,
    selected: bool,
    installed: bool,
    newer: bool,
    latest: bool,
    first: bool,
    last: bool,
}

/// `on_select` gets the row's tag.
fn version_row(
    props: RowProps,
    on_select: impl Fn(&str, &mut Window, &mut App) + 'static,
    cx: &App,
) -> impl IntoElement {
    let t = cx.tokens();
    let RowProps { release, selected, installed, newer, latest, first, last } = props;
    let id = SharedString::from(format!("version-{}", release.tag_name));
    let tag = SharedString::from(release.tag_name.clone());
    let emerald = tw(Tw::Emerald, 500);

    let rail_color = if newer { alpha(t.primary, 0.4) } else { t.border };
    let mut rail = div().absolute().left(px(15.0)).w(px(1.0));
    rail = if first { rail.top(gpui_kit::relative(0.5)) } else { rail.top_0() };
    rail = if last { rail.bottom(gpui_kit::relative(0.5)) } else { rail.bottom_0() };
    rail = if installed {
        rail.bg(linear_gradient(180.0, linear_color_stop(alpha(t.primary, 0.4), 0.5), linear_color_stop(t.border, 0.5)))
    } else {
        rail.bg(rail_color)
    };

    let dot = if installed {
        div().size(px(9.0)).rounded_full().bg(t.primary)
    } else {
        let size = if selected { 7.0 * 1.25 } else { 7.0 };
        div().size(px(size)).rounded_full().border(px(1.5)).bg(t.background).border_color(if selected {
            t.foreground
        } else {
            alpha(t.muted_foreground, 0.6)
        })
    };

    let foreground = t.foreground;
    let tag_color = if selected || installed { t.foreground } else { t.muted_foreground };
    let highlight = {
        let base = div().absolute().left_0().right_0().top(px(2.0)).bottom(px(2.0)).rounded(radius::LG);
        if selected {
            base.bg(t.card).border_1().border_color(t.border)
        } else {
            base.group_hover("version-row", move |s| s.bg(alpha(foreground, 0.04)))
        }
    };

    div()
        .id(id)
        .group("version-row")
        .relative()
        .flex()
        .items_center()
        .py(px(10.0))
        .pr_2()
        .pl_1()
        .cursor_pointer()
        .on_click({
            let tag = tag.clone();
            move |_, window, cx| on_select(&tag, window, cx)
        })
        .child(highlight)
        .child(rail)
        .child(div().relative().flex_none().w(px(22.0)).flex().justify_center().child(dot))
        .child(
            div()
                .relative()
                .min_w_0()
                .flex_1()
                .pl(px(6.0))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(6.0))
                        .child(
                            div()
                                .min_w_0()
                                .truncate()
                                .font_mono_tw()
                                .text_size(px(13.0))
                                .line_height(px(19.5))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(tag_color)
                                .child(tag),
                        )
                        .children(installed.then(|| row_badge("Installed", t.primary)))
                        .children(latest.then(|| row_badge("Latest", emerald))),
                )
                .child(
                    div()
                        .mt(px(1.0))
                        .text_size(px(11.0))
                        .line_height(px(16.5))
                        .text_color(alpha(t.muted_foreground, 0.7))
                        .child(SharedString::from(
                            release_age(release.published_at, &Utc::now()).unwrap_or_else(|| " ".into()),
                        )),
                ),
        )
}

fn loading_state(cx: &App) -> impl IntoElement {
    let t = cx.tokens();
    let sk = |w: f32, h: f32| Skeleton::new().w(px(w)).h(px(h));
    let rows = [96.0, 80.0, 112.0, 80.0, 96.0].into_iter().map(|w| {
        div()
            .flex()
            .items_center()
            .gap_3()
            .pl_1()
            .child(Skeleton::new().size(px(8.0)).rounded_full())
            .child(div().flex().flex_col().gap(px(6.0)).child(sk(w, 12.0)).child(sk(56.0, 8.0)))
    });
    div()
        .flex()
        .size_full()
        .child(
            div()
                .flex()
                .flex_col()
                .gap_4()
                .flex_none()
                .w(px(256.0))
                .p_4()
                .border_r_1()
                .border_color(alpha(t.border, 0.6))
                .bg(alpha(t.muted, 0.2))
                .child(Skeleton::new().h(px(32.0)).w_full())
                .children(rows),
        )
        .child(
            div().flex_1().flex().flex_col().gap_3().px_6().pt_5().child(sk(112.0, 24.0)).child(sk(192.0, 12.0)).child(
                div()
                    .pt_5()
                    .flex()
                    .flex_col()
                    .gap(px(10.0))
                    .child(Skeleton::new().h(px(12.0)).w_full())
                    .child(Skeleton::new().h(px(12.0)).w(gpui_kit::relative(0.8333)))
                    .child(Skeleton::new().h(px(12.0)).w(gpui_kit::relative(0.6667))),
            ),
        )
}

impl VersionDialog {
    fn release_detail(
        &self,
        release: &Release,
        dir: Direction,
        distance: usize,
        latest: bool,
        cx: &App,
    ) -> impl IntoElement {
        let t = cx.tokens();
        let changelog = release.body.trim();
        let summary = match dir {
            Direction::Reinstall => "This is the version you have installed".to_string(),
            Direction::Install => "Your installed version is not in this list".to_string(),
            _ => format!(
                "{} {} than installed",
                plural(distance as i64, "release"),
                if dir == Direction::Upgrade { "newer" } else { "older" }
            ),
        };
        let dot = match dir {
            Direction::Upgrade => tw(Tw::Emerald, 500),
            Direction::Install => alpha(t.muted_foreground, 0.6),
            _ => t.primary,
        };
        let notes = if changelog.is_empty() {
            div()
                .text_sm_tw()
                .italic()
                .text_color(alpha(t.muted_foreground, 0.7))
                .child("No release notes were published for this version.")
                .into_any_element()
        } else {
            Markdown::new(
                SharedString::from(format!("version-notes-{}-{}", self.addon.name, release.tag_name)),
                SharedString::from(changelog.to_string()),
            )
            .style(MarkdownStyle::ReleaseNotes)
            .timeline(false)
            .into_any_element()
        };

        div()
            .flex()
            .flex_col()
            .flex_1()
            .min_w_0()
            .px_6()
            .pt_5()
            .child(
                div()
                    .flex_none()
                    .flex()
                    .items_baseline()
                    .justify_between()
                    .gap_3()
                    .child(
                        div()
                            .min_w_0()
                            .truncate()
                            .font_mono_tw()
                            .text_2xl_tw()
                            .line_height(px(24.0))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(SharedString::from(release.tag_name.clone())),
                    )
                    .children(release.published_at.map(|at| {
                        div()
                            .flex_none()
                            .text_xs_tw()
                            .text_color(t.muted_foreground)
                            .child(SharedString::from(format!("Released {}", format_to_local_date(&at, true))))
                    })),
            )
            .child(
                div()
                    .mt(px(10.0))
                    .flex_none()
                    .flex()
                    .items_center()
                    .gap(px(6.0))
                    .text_xs_tw()
                    .text_color(t.muted_foreground)
                    .child(div().size(px(6.0)).rounded_full().bg(dot))
                    .child(SharedString::from(format!(
                        "{summary}{}",
                        if latest && dir != Direction::Reinstall { " · latest" } else { "" }
                    ))),
            )
            .child(div().mt(px(14.0)).flex_1().min_h_0().border_t_1().border_dashed().border_color(t.border).child(
                ScrollArea::new("version-detail-scroll").size_full().child(fade_slide_in(
                    div().pt(px(14.0)).pr_3().pb_5().child(notes),
                    SharedString::from(format!("detail-{}", release.tag_name)),
                )),
            ))
    }

    fn version_list(
        &self,
        visible: &[usize],
        selected_index: usize,
        installed_index: Option<usize>,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let t = cx.tokens();
        let list = self.list();
        let this = cx.entity();
        let rows = visible.iter().enumerate().map(|(position, &index)| {
            let this = this.clone();
            version_row(
                RowProps {
                    release: &list[index],
                    selected: index == selected_index,
                    installed: Some(index) == installed_index,
                    newer: installed_index.is_some_and(|i| index < i),
                    latest: index == 0,
                    first: position == 0,
                    last: position + 1 == visible.len(),
                },
                move |tag, _, cx| this.update(cx, |d, cx| d.select(tag.to_string(), cx)),
                cx,
            )
        });

        div()
            .flex()
            .flex_col()
            .flex_none()
            .w(px(256.0))
            .border_r_1()
            .border_color(alpha(t.border, 0.6))
            .bg(alpha(t.muted, 0.2))
            .child(
                div()
                    .flex_none()
                    .p(px(10.0))
                    .pb(px(6.0))
                    .child(Input::new(&self.filter).h(px(32.0)).prefix_icon(icons::SEARCH).pl(px(32.0)).text_xs_tw()),
            )
            .child(
                div()
                    .relative()
                    .flex_1()
                    .min_h_0()
                    .child(
                        div()
                            .id("version-list")
                            .size_full()
                            .overflow_y_scroll()
                            .track_scroll(&self.list_scroll)
                            .px_2()
                            .pt(px(2.0))
                            .pb(px(10.0))
                            .children(visible.is_empty().then(|| {
                                div()
                                    .px_3()
                                    .py(px(28.0))
                                    .text_center()
                                    .text_xs_tw()
                                    .text_color(t.muted_foreground)
                                    .child("No version matches that filter.")
                            }))
                            .children(rows),
                    )
                    .child(scrollbar_overlay("version-list-scrollbar", &self.list_scroll, ScrollbarAxis::Vertical)),
            )
    }
}

impl Render for VersionDialog {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = cx.tokens();
        let this = cx.entity();

        let root = div()
            .id("version-dialog")
            .track_focus(&self.focus)
            .capture_key_down(cx.listener(|this, ev: &KeyDownEvent, window, cx| this.on_key(ev, window, cx)))
            .relative()
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0();

        let running = self.is_running();
        let state: Load<()> = self.versions.read(cx).get(&self.addon.name).as_ref().map(|_| ());
        let ready = matches!(state, Load::Ready(()));
        let selected_index = self.selected_index();
        let installed_index = self.installed_index();
        let selected: Option<&Release> = if ready { self.list().get(selected_index) } else { None };
        let dir = direction(installed_index, selected_index);
        let query = self.filter.read(cx).value().to_string();
        let visible = visible_indices(self.list(), &query);

        // ---------------------------------------------------------------- header
        let header = div()
            .flex_none()
            .flex()
            .items_end()
            .justify_between()
            .gap_4()
            .px_6()
            .pt_5()
            .pr(px(56.0))
            .pb_4()
            .child(
                div()
                    .min_w_0()
                    .child(
                        DialogDescription::new("Install version")
                            .mb_1()
                            .text_color(t.primary)
                            .text_size(px(10.0))
                            .line_height(px(16.0))
                            .font_weight(FontWeight::SEMIBOLD),
                    )
                    .child(
                        DialogTitle::new(SharedString::from(self.addon.alias.clone()))
                            .truncate()
                            .text_xl_tw()
                            .line_height(px(25.0)),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_none()
                    .items_end()
                    .gap(px(10.0))
                    .font_mono_tw()
                    .text_xs_tw()
                    .child(
                        div()
                            .text_color(t.muted_foreground)
                            .child(
                                eyebrow("Installed")
                                    .font_family(cam_ui::theme::FONT_SANS)
                                    .text_color(alpha(t.muted_foreground, 0.7)),
                            )
                            .child(div().max_w(px(112.0)).truncate().child(SharedString::from(
                                if self.addon.version.is_empty() {
                                    "—".to_string()
                                } else {
                                    self.addon.version.clone()
                                },
                            ))),
                    )
                    .children(selected.map(|sel| {
                        div()
                            .flex()
                            .items_end()
                            .gap(px(10.0))
                            .child(div().mb(px(2.0)).child(
                                Icon::new(icons::ARROW_RIGHT).size(px(16.0)).text_color(alpha(t.muted_foreground, 0.6)),
                            ))
                            .child(
                                div()
                                    .text_right()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child(
                                        eyebrow(dir.label())
                                            .font_family(cam_ui::theme::FONT_SANS)
                                            .text_color(dir.tone(cx)),
                                    )
                                    .child(
                                        div()
                                            .max_w(px(112.0))
                                            .truncate()
                                            .child(SharedString::from(sel.tag_name.clone())),
                                    ),
                            )
                    })),
            );

        // ---------------------------------------------------------------- body
        let body_inner: gpui_kit::AnyElement = match &state {
            Load::Idle | Load::Pending => loading_state(cx).into_any_element(),
            Load::Failed(e) => {
                let this = this.clone();
                EmptyState::new(icons::CIRCLE_ALERT, "Couldn't load versions")
                    .tone(EmptyTone::Destructive)
                    .description(e.message())
                    .compact()
                    .flex_1()
                    .child(
                        Button::new("retry-releases")
                            .variant(ButtonVariant::Outline)
                            .size(ButtonSize::Sm)
                            .icon(icons::REFRESH_CW)
                            .label("Retry")
                            .on_click(move |_, _, cx| this.update(cx, |d, cx| d.load(true, cx))),
                    )
                    .into_any_element()
            }
            Load::Ready(()) => match selected {
                None => EmptyState::new(icons::TAG, "Nothing else to install")
                    .description("This addon has no other published versions.")
                    .compact()
                    .flex_1()
                    .into_any_element(),
                Some(sel) => div()
                    .flex()
                    .size_full()
                    .child(self.version_list(&visible, selected_index, installed_index, cx))
                    .child(self.release_detail(
                        sel,
                        dir,
                        selected_index.abs_diff(installed_index.unwrap_or(0)),
                        selected_index == 0,
                        cx,
                    ))
                    .into_any_element(),
            },
        };
        let body = div()
            .flex()
            .h(px(372.0))
            .min_h_0()
            .flex_shrink(1.0)
            .border_y_1()
            .border_color(alpha(t.border, 0.6))
            .when(running, |d| d.opacity(0.5))
            .child(body_inner);

        let alert = match &self.phase {
            Phase::Failed(failure) => Some(error_alert(failure, cx)),
            _ => None,
        };

        // ---------------------------------------------------------------- footer
        let footer = match &self.phase {
            Phase::Running { started: (tag, d), op } => div().flex_none().px_6().py_4().child(running_steps(
                "version-steps",
                running_view(&self.addon.name, tag, op, d.running()),
                cx,
            )),
            _ => {
                let cancel = this.clone();
                let install = this.clone();
                let action = match selected {
                    Some(sel) => div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(Icon::new(dir.icon()).size(px(16.0)))
                        .child(
                            div()
                                .flex()
                                .gap(px(4.0))
                                .child(dir.action())
                                .child(div().font_mono_tw().child(SharedString::from(sel.tag_name.clone()))),
                        )
                        .into_any_element(),
                    None => div().child("Install").into_any_element(),
                };
                div()
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_3()
                    .px_6()
                    .py(px(14.0))
                    .pr_4()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(6.0))
                            .text_xs_tw()
                            .text_color(alpha(t.muted_foreground, 0.8))
                            .child(kbd("↑", false, cx))
                            .child(kbd("↓", false, cx))
                            .child(div().mr(px(6.0)).child("browse"))
                            .child(kbd("↵", false, cx))
                            .child("install"),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(
                                Button::new("version-cancel")
                                    .variant(ButtonVariant::Ghost)
                                    .label("Cancel")
                                    .on_click(move |_, window, cx| cancel.update(cx, |d, cx| d.close(window, cx))),
                            )
                            .child(
                                Button::new("version-install")
                                    .has_icon(selected.is_some())
                                    .disabled(selected.is_none())
                                    .rounded(radius::LG)
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .shadow(shadow_lg_tinted(alpha(t.primary, 0.2)))
                                    .child(action)
                                    .on_click(move |_, window, cx| {
                                        install.update(cx, |d, cx| d.handle_install(window, cx))
                                    }),
                            ),
                    )
            }
        };

        root.child(header).child(body).children(alert).child(footer).into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn rel(tag: &str, day: Option<u32>) -> Release {
        Release {
            tag_name: tag.into(),
            published_at: day.map(|d| Utc.with_ymd_and_hms(2025, 1, d, 0, 0, 0).unwrap()),
            ..Release::default()
        }
    }

    #[test]
    fn directions() {
        assert_eq!(direction(None, 0), Direction::Install);
        assert_eq!(direction(Some(2), 0), Direction::Upgrade);
        assert_eq!(direction(Some(2), 2), Direction::Reinstall);
        assert_eq!(direction(Some(2), 3), Direction::Rollback);
    }

    #[test]
    fn ages() {
        let now = Utc.with_ymd_and_hms(2025, 6, 1, 12, 0, 0).unwrap();
        let ago = |days: i64| release_age(Some(now - chrono::Duration::days(days)), &now).unwrap();
        assert_eq!(ago(0), "today");
        assert_eq!(ago(1), "1 day ago");
        assert_eq!(ago(13), "13 days ago");
        assert_eq!(ago(14), "2 weeks ago");
        assert_eq!(ago(59), "8 weeks ago");
        assert_eq!(ago(60), "2 months ago");
        assert_eq!(ago(729), "24 months ago");
        assert_eq!(ago(730), "2 years ago");
        assert_eq!(release_age(None, &now), None);
    }

    #[test]
    fn sorting_keeps_undated_last_in_api_order() {
        let out = newest_first(vec![rel("u1", None), rel("old", Some(1)), rel("u2", None), rel("new", Some(9))]);
        let tags: Vec<_> = out.iter().map(|r| r.tag_name.as_str()).collect();
        assert_eq!(tags, ["new", "old", "u1", "u2"]);
    }

    #[test]
    fn releases_in_the_same_millisecond_keep_api_order() {
        let at = |micros| Release {
            tag_name: format!("{micros}"),
            published_at: Some(
                Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap() + chrono::Duration::microseconds(micros),
            ),
            ..Release::default()
        };
        let out = newest_first(vec![at(100), at(900), at(1_000)]);
        let tags: Vec<_> = out.iter().map(|r| r.tag_name.as_str()).collect();
        assert_eq!(tags, ["1000", "100", "900"]);
    }

    #[test]
    fn duplicate_tags_keep_the_first_release() {
        let list = unique_by_tag(vec![rel("1.0", Some(2)), rel("1.0", Some(1)), rel("0.9", None)]);
        assert_eq!(list, [rel("1.0", Some(2)), rel("0.9", None)]);
    }

    #[test]
    fn filter_and_arrow_navigation() {
        let list = vec![rel("v1.2", Some(3)), rel("v1.1", Some(2)), rel("v0.9", Some(1))];
        assert_eq!(visible_indices(&list, ""), vec![0, 1, 2]);
        assert_eq!(visible_indices(&list, " V1 "), vec![0, 1]);
        assert_eq!(visible_indices(&list, "zzz"), Vec::<usize>::new());
        assert_eq!(step_selection(&[0, 1], 0, true), Some(1));
        assert_eq!(step_selection(&[0, 1], 1, true), Some(1));
        assert_eq!(step_selection(&[0, 1], 0, false), Some(0));
        assert_eq!(step_selection(&[1, 2], 0, true), Some(1)); // selection hidden by the filter: jumps to the first row
        assert_eq!(step_selection(&[], 0, true), None);
    }
}
