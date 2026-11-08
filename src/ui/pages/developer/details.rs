//! Detail pane of a published addon: header, tab chips and the Overview, Statistics and History panes.
use gpui_kit::{
    AnyElement, AppContext as _, Context, Entity, FontWeight, IntoElement, ParentElement, Render, ScrollHandle,
    SharedString, Styled, Window, div, prelude::FluentBuilder as _, px,
};

use super::{
    Selection,
    bits::{DetailField, StatusChip, detail_list, h3, loading_block},
    edit_panel::{AddonEditPanel, EditPanelEvent, EditPanelProps},
    fmt,
    form::{bool_of, list_of, text_of, values_to_form},
    host::Host,
    owned::{AddonReview, OwnedAddon, ReviewOutcome, latest_review},
    queries::DevQueries,
    review_status::ReviewStatus,
    statistics::AddonStatistics,
    types::EditorSource,
};
use crate::ui::{
    components::{
        AddonIcon, Button, ButtonSize, ButtonVariant, ScrollArea, TabsList, TabsTrigger, TabsVariant, addon_icon_url,
    },
    icons,
    theme::{TokensExt, TwText, alpha, radius},
    utils::time_ago,
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    View,
    Edit,
}

pub struct AddonDetails {
    addon: OwnedAddon,
    webhook_install_url: String,
    host: Host,
    queries: Entity<DevQueries>,
    tab: SharedString,
    mode: Mode,
    edit_panel: Option<Entity<AddonEditPanel>>,
    statistics: Option<Entity<AddonStatistics>>,
    review_view: Option<(AddonReview, Entity<ReviewStatus>)>,
    /// Submission whose values were requested for editing.
    pending_requested: Option<i64>,
    scroll: ScrollHandle,
}

impl AddonDetails {
    pub fn new(addon: OwnedAddon, webhook_install_url: String, host: Host, cx: &mut Context<Self>) -> Self {
        let queries = host.queries.clone();
        let source = EditorSource::Addon { uuid: addon.uuid.clone() };
        queries.update(cx, |q, cx| q.ensure_values(&source, cx));
        cx.observe(&queries, |this, _, cx| {
            this.sync_pending(cx);
            cx.notify();
        })
        .detach();
        Self {
            addon,
            webhook_install_url,
            host,
            queries,
            tab: "overview".into(),
            mode: Mode::View,
            edit_panel: None,
            statistics: None,
            review_view: None,
            pending_requested: None,
            scroll: ScrollHandle::new(),
        }
    }

    /// Hands the page's latest data for this addon to the view. Called on every render of the page.
    pub fn set_props(&mut self, addon: OwnedAddon, webhook_install_url: String) {
        self.addon = addon;
        self.webhook_install_url = webhook_install_url;
    }

    fn source(&self) -> EditorSource {
        EditorSource::Addon { uuid: self.addon.uuid.clone() }
    }

    /// Submission being revised when editing: the review is in review or rejected.
    fn pending_id(review: Option<AddonReview>) -> Option<i64> {
        match review {
            Some(r) if matches!(r.status, ReviewOutcome::InReview | ReviewOutcome::Rejected) => Some(r.submission_id),
            _ => None,
        }
    }

    fn sync_pending(&mut self, cx: &mut Context<Self>) {
        if self.mode != Mode::Edit {
            return;
        }
        if let Some(id) = Self::pending_id(latest_review(&self.addon.review_history))
            && self.pending_requested != Some(id)
        {
            self.pending_requested = Some(id);
            self.queries.update(cx, |q, cx| q.ensure_values(&EditorSource::Submission { id }, cx));
        }
    }

    fn select_tab(&mut self, value: SharedString, window: &mut Window, cx: &mut Context<Self>) {
        self.mode = Mode::View;
        self.edit_panel = None;
        self.pending_requested = None;
        self.statistics = (value == "statistics").then(|| {
            let (uuid, alias) = (self.addon.uuid.clone(), self.addon.alias.clone());
            cx.new(|cx| AddonStatistics::new(uuid, alias, window, cx))
        });
        self.tab = value;
        cx.notify();
    }

    fn start_edit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.tab = "overview".into();
        self.statistics = None;
        self.mode = Mode::Edit;
        self.edit_panel = None;
        self.pending_requested = None;
        self.sync_pending(cx);
        let _ = window;
        cx.notify();
    }

    fn leave_edit(&mut self, cx: &mut Context<Self>) {
        self.mode = Mode::View;
        self.edit_panel = None;
        cx.notify();
    }

    fn display(&self, form: Option<&super::form::PublishFormState>) -> OwnedAddon {
        let Some(form) = form else { return self.addon.clone() };
        let alias = text_of(&form.values, "alias");
        let branch = text_of(&form.values, "branch");
        OwnedAddon {
            alias: if alias.is_empty() { self.addon.alias.clone() } else { alias },
            repo: text_of(&form.values, "repo"),
            branch: (!branch.is_empty()).then_some(branch),
            author: text_of(&form.values, "author"),
            description: text_of(&form.values, "description"),
            tags: list_of(&form.values, "tags"),
            library: bool_of(&form.values, "library"),
            ..self.addon.clone()
        }
    }
}

impl Render for AddonDetails {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = cx.tokens();
        let loaded = self.queries.read(cx).values(Some(&self.source()), cx);
        let form = loaded.values.as_ref().map(values_to_form);
        let display = self.display(form.as_ref());
        // The addon's current status is its latest submission, never a previewed one.
        let review = latest_review(&display.review_history);
        let pending_id = Self::pending_id(review);
        let editing = self.mode == Mode::Edit;
        let pending = match (editing, pending_id) {
            (true, Some(id)) => self.queries.read(cx).values(Some(&EditorSource::Submission { id }), cx),
            _ => Default::default(),
        };
        let pending_form = pending.values.as_ref().map(values_to_form);
        let submission_count = display.review_history.len();
        let submit_label = match review.map(|r| r.status) {
            Some(ReviewOutcome::InReview) => "Update submission",
            Some(ReviewOutcome::Rejected) => "Resubmit for review",
            _ => "Submit for review",
        };
        let edit_label = match review.map(|r| r.status) {
            Some(ReviewOutcome::Rejected) => "Revise changes",
            Some(ReviewOutcome::InReview) => "Edit pending changes",
            _ => "Update details",
        };

        let icon_url = addon_icon_url(display.icon.as_deref(), Some(&display.repo), display.branch.as_deref(), true);
        let header = div()
            .flex()
            .flex_col()
            .gap(px(16.0))
            .flex_none()
            .border_b_1()
            .border_color(t.border)
            .px(px(20.0))
            .pt(px(20.0))
            .pb(px(12.0))
            .child(
                div().flex().items_start().gap(px(12.0)).child(AddonIcon::new(display.alias.clone(), icon_url)).child(
                    div()
                        .min_w_0()
                        .child(div().text_xl_tw().font_weight(FontWeight::SEMIBOLD).child(display.alias.clone()))
                        .child(div().mt(px(4.0)).text_xs_tw().text_color(t.muted_foreground).child(format!(
                            "Published{}",
                            if display.author.is_empty() { String::new() } else { format!(" · {}", display.author) }
                        ))),
                ),
            )
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap(px(8.0))
                    .child(
                        Button::new("addon-edit")
                            .label(edit_label)
                            .disabled(loaded.loading || loaded.error.is_some())
                            .on_click(cx.listener(|this, _, window, cx| this.start_edit(window, cx))),
                    )
                    .when(!display.repo.is_empty(), |d| {
                        let url = format!("https://github.com/{}", display.repo);
                        d.child(
                            Button::new("addon-view-code")
                                .variant(ButtonVariant::Outline)
                                .icon(icons::GITHUB)
                                .label("View code")
                                .on_click(move |_, _, cx| cx.open_url(&url)),
                        )
                    }),
            )
            .child(
                TabsList::new("addon-details-tabs")
                    .variant(TabsVariant::Chip)
                    .value(self.tab.clone())
                    .on_value_change(
                        cx.listener(|this, v: &SharedString, window, cx| this.select_tab(v.clone(), window, cx)),
                    )
                    .trigger(TabsTrigger::new("overview").icon(icons::LAYOUT_LIST).label("Overview"))
                    .trigger(TabsTrigger::new("statistics").icon(icons::CHART_COLUMN).label("Statistics"))
                    .trigger(TabsTrigger::new("history").icon(icons::HISTORY).label("History").when(
                        submission_count > 0,
                        |tr| {
                            tr.child(
                                div()
                                    .rounded_full()
                                    .bg(t.muted)
                                    .px(px(6.0))
                                    .text_xs_tw()
                                    .child(submission_count.to_string()),
                            )
                        },
                    )),
            );

        let body: AnyElement = match self.tab.as_ref() {
            "statistics" => match &self.statistics {
                Some(s) => s.clone().into_any_element(),
                None => div().into_any_element(),
            },
            "history" => self.history_pane(&display, cx),
            _ => {
                if loaded.loading || (editing && pending.loading) {
                    loading_block("Loading declaration...", cx).py(px(64.0)).into_any_element()
                } else if let Some(error) = loaded.error.clone() {
                    div().text_sm_tw().text_color(t.destructive).child(error).into_any_element()
                } else if editing
                    && let Some(id) = pending_id
                    && pending_form.is_none()
                {
                    div()
                        .flex()
                        .flex_col()
                        .items_center()
                        .justify_center()
                        .gap(px(12.0))
                        .py(px(64.0))
                        .child(div().text_center().text_sm_tw().text_color(t.destructive).child(
                            pending.error.clone().unwrap_or_else(|| "This submission could not be loaded.".into()),
                        ))
                        .child(Button::new("pending-retry").variant(ButtonVariant::Outline).label("Retry").on_click({
                            let queries = self.queries.clone();
                            move |_, _, cx| {
                                queries.update(cx, |q, cx| q.refetch_values(&EditorSource::Submission { id }, cx))
                            }
                        }))
                        .into_any_element()
                } else if editing {
                    if self.edit_panel.is_none() {
                        let initial = pending_form.or(form).unwrap_or_default();
                        let props = EditPanelProps {
                            initial,
                            initial_submission_id: pending_id,
                            name_locked: true,
                            require_changes: review.map(|r| r.status) != Some(ReviewOutcome::Rejected),
                            intro_title: EditPanelProps::DEFAULT_TITLE,
                            intro_note: EditPanelProps::DEFAULT_NOTE,
                            submit_label,
                            scroll: Some(self.scroll.clone()),
                        };
                        let panel = cx.new(|cx| AddonEditPanel::new(props, window, cx));
                        cx.subscribe(&panel, |this, _, event: &EditPanelEvent, cx| match event {
                            EditPanelEvent::Cancel => this.leave_edit(cx),
                            EditPanelEvent::Submitted => {
                                this.leave_edit(cx);
                                this.host.refresh(cx).detach();
                            }
                        })
                        .detach();
                        self.edit_panel = Some(panel);
                    }
                    self.edit_panel.clone().unwrap().into_any_element()
                } else {
                    self.overview_pane(&display, review, cx)
                }
            }
        };

        div().size_full().min_h_0().flex().flex_col().child(header).child(
            ScrollArea::new("addon-details-scroll")
                .flex_1()
                .min_h_0()
                .track_scroll(&self.scroll)
                .content_style(|d| d.px(px(20.0)).pt(px(16.0)).pb(px(20.0)).flex().flex_col().gap(px(20.0)))
                .child(body),
        )
    }
}

impl AddonDetails {
    fn overview_pane(
        &mut self,
        display: &OwnedAddon,
        review: Option<AddonReview>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let t = cx.tokens();
        // Create a fresh `ReviewStatus` whenever the review status or submission changes, so it starts clean.
        let review_el = review.map(|review| {
            if self.review_view.as_ref().map(|(r, _)| *r) != Some(review) {
                let host = self.host.clone();
                self.review_view = Some((review, cx.new(|cx| ReviewStatus::new(review, host, cx))));
            }
            self.review_view.as_ref().unwrap().1.clone()
        });
        if review.is_none() {
            self.review_view = None;
        }
        let show_review = review_el.as_ref().is_some_and(|e| !e.read(cx).hidden());
        let webhook = (display.webhook_enabled || !self.webhook_install_url.is_empty()).then(|| {
            let enabled = display.webhook_enabled;
            let install = self.webhook_install_url.clone();
            div()
                .flex()
                .items_center()
                .justify_between()
                .gap(px(12.0))
                .rounded(radius::XL)
                .border_1()
                .border_color(t.border)
                .bg(alpha(t.card, 0.4))
                .px(px(16.0))
                .py(px(12.0))
                .text_sm_tw()
                .child(
                    div()
                        .flex()
                        .flex_1()
                        .min_w_0()
                        .items_start()
                        .gap(px(12.0))
                        .child(
                            div().mt(px(2.0)).flex_none().child(
                                gpui_kit::component::Icon::new(icons::ZAP).size(px(16.0)).text_color(if enabled {
                                    t.primary
                                } else {
                                    t.muted_foreground
                                }),
                            ),
                        )
                        .child(
                            div()
                                .min_w_0()
                                .child(
                                    div()
                                        .font_weight(FontWeight::MEDIUM)
                                        .child(format!("Instant updates: {}", if enabled { "on" } else { "off" })),
                                )
                                .child(div().mt(px(4.0)).text_xs_tw().text_color(t.muted_foreground).child(
                                    if !enabled {
                                        "Install the GitHub App on this addon's repository. Org repos need an org owner to install."
                                            .to_string()
                                    } else if let Some(at) = &display.webhook_last_event_at {
                                        format!("Last release event {}", time_ago(at))
                                    } else {
                                        "No release events yet".to_string()
                                    },
                                )),
                        ),
                )
                .when(!enabled && !install.is_empty(), |d| {
                    d.child(
                        Button::new("webhook-enable")
                            .size(ButtonSize::Sm)
                            .variant(ButtonVariant::Outline)
                            .flex_none()
                            .label("Enable")
                            .on_click(move |_, _, cx| cx.open_url(&install)),
                    )
                })
        });
        let repo_href = (!display.repo.is_empty()).then(|| format!("https://github.com/{}", display.repo));
        div()
            .flex()
            .flex_col()
            .gap(px(20.0))
            .when(show_review, |d| d.children(review_el))
            .when_some(display.warning.clone(), |d, warning| {
                d.child(
                    div()
                        .rounded(radius::LG)
                        .border_1()
                        .border_color(alpha(t.destructive, 0.3))
                        .bg(alpha(t.destructive, 0.05))
                        .p(px(12.0))
                        .text_sm_tw()
                        .child(warning),
                )
            })
            .children(webhook)
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(12.0))
                    .child(h3("Listing details"))
                    .child(div().text_sm_tw().text_color(t.muted_foreground).child(if display.description.is_empty() {
                        "No description provided.".to_string()
                    } else {
                        display.description.clone()
                    }))
                    .child(detail_list(
                        vec![
                            DetailField::new("Repository", display.repo.clone()).href(repo_href),
                            DetailField::new(
                                "Branch",
                                display.branch.clone().unwrap_or_else(|| "Not specified".into()),
                            ),
                            DetailField::new("Tags", display.tags.join(", ")),
                            DetailField::new(
                                "Published on",
                                display
                                    .added_at
                                    .as_ref()
                                    .map(fmt::local_date)
                                    .unwrap_or_else(|| "Not available".into()),
                            ),
                        ],
                        cx,
                    )),
            )
            .into_any_element()
    }

    fn history_pane(&mut self, display: &OwnedAddon, cx: &mut Context<Self>) -> AnyElement {
        let t = cx.tokens();
        if display.review_history.is_empty() {
            return div().text_sm_tw().text_color(t.muted_foreground).child("No submissions yet.").into_any_element();
        }
        div()
            .flex()
            .flex_col()
            .children(display.review_history.iter().enumerate().map(|(i, entry)| {
                let host = self.host.clone();
                let selection = Selection::Submission(entry.submission_id);
                div()
                    .when(i > 0, |d| d.border_t_1().border_color(t.border))
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(px(8.0))
                    .py(px(12.0))
                    .child(
                        div()
                            .min_w_0()
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(8.0))
                                    .child(
                                        div()
                                            .text_xs_tw()
                                            .text_color(t.muted_foreground)
                                            .child(format!("#{}", entry.submission_id)),
                                    )
                                    .child(StatusChip::new(entry.tone, entry.status.clone())),
                            )
                            .child(div().mt(px(4.0)).text_xs_tw().text_color(t.muted_foreground).child(
                                entry.date.as_ref().map(fmt::local_date).unwrap_or_else(|| "Not available".into()),
                            )),
                    )
                    .child(
                        Button::new(("history-view", entry.submission_id as usize))
                            .size(ButtonSize::Sm)
                            .variant(ButtonVariant::Ghost)
                            .label("View review")
                            .on_click(move |_, _, cx| host.select(selection.clone(), cx)),
                    )
            }))
            .into_any_element()
    }
}
