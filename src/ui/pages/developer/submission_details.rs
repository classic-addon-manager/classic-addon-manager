//! Review view of a standalone submission (open, rejected or approved).
use std::collections::BTreeMap;

use gpui_kit::{
    AnyElement, AppContext as _, Context, Entity, FontWeight, IntoElement, ParentElement, Render, ScrollHandle, Styled,
    Window, div, prelude::FluentBuilder as _, px,
};

use super::{
    Selection,
    bits::{DetailField, StatusChip, detail_list, h3, loading_block},
    dialogs::{WITHDRAW_SUBMISSION, Withdrawal},
    edit_panel::{AddonEditPanel, EditPanelEvent, EditPanelProps},
    fmt,
    form::{bool_of, list_of, text_of, values_to_form},
    host::Host,
    owned::{OwnedSubmission, OwnedSubmissionStatus, ReviewHistoryTone, status_tone},
    queries::DevQueries,
    types::{DeclarationKind, EditorSource, SubmissionMessage, SubmissionStatus},
};
use crate::ui::{
    components::{Button, ButtonVariant, ScrollArea},
    icons,
    theme::{TokensExt, TwText, alpha, radius},
};

pub struct SubmissionDetails {
    submission: OwnedSubmission,
    /// A submission reached from an addon's review history returns to that addon: the addon and its label.
    back_to: Option<(Selection, String)>,
    host: Host,
    queries: Entity<DevQueries>,
    editing: bool,
    /// Also tells when the backend closed the review, so the submission can't be revised or withdrawn any more.
    withdrawal: Entity<Withdrawal>,
    edit_panel: Option<Entity<AddonEditPanel>>,
    scroll: ScrollHandle,
}

fn status_label(status: SubmissionStatus) -> &'static str {
    match status {
        SubmissionStatus::Open => "In review",
        SubmissionStatus::Approved => "Approved",
        SubmissionStatus::Rejected => "Changes requested",
    }
}

fn status_key(status: SubmissionStatus) -> &'static str {
    match status {
        SubmissionStatus::Open => "open",
        SubmissionStatus::Approved => "approved",
        SubmissionStatus::Rejected => "rejected",
    }
}

impl SubmissionDetails {
    pub fn new(
        submission: OwnedSubmission,
        back_to: Option<(Selection, String)>,
        host: Host,
        cx: &mut Context<Self>,
    ) -> Self {
        let queries = host.queries.clone();
        let id = submission.id;
        queries.update(cx, |q, cx| {
            q.ensure_values(&EditorSource::Submission { id }, cx);
            q.ensure_submission(id, cx);
        });
        cx.observe(&queries, |_, _, cx| cx.notify()).detach();
        let withdrawal = cx.new(|_| Withdrawal::new(&WITHDRAW_SUBMISSION, host.clone()));
        cx.observe(&withdrawal, |_, _, cx| cx.notify()).detach();
        Self {
            submission,
            back_to,
            host,
            queries,
            editing: false,
            withdrawal,
            edit_panel: None,
            scroll: ScrollHandle::new(),
        }
    }

    pub fn set_props(&mut self, submission: OwnedSubmission, back_to: Option<(Selection, String)>) {
        self.submission = submission;
        self.back_to = back_to;
    }

    fn message_block(m: &SubmissionMessage, text_sm: bool, cx: &Context<Self>) -> impl IntoElement {
        let t = cx.tokens();
        div()
            .child(div().when(text_sm, |d| d.text_sm_tw()).child(m.body.clone()))
            .when_some(m.created_at.as_ref(), |d, at| {
                d.child(div().mt(px(4.0)).text_xs_tw().text_color(t.muted_foreground).child(fmt::local_time(at)))
            })
    }
}

impl Render for SubmissionDetails {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = cx.tokens();
        let id = self.submission.id;
        let (loaded, detail) = {
            let q = self.queries.read(cx);
            (q.values(Some(&EditorSource::Submission { id }), cx), q.submission(id, cx))
        };
        if loaded.loading || detail.loading {
            return div()
                .size_full()
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .child(loading_block("Loading declaration...", cx))
                .into_any_element();
        }
        if let Some(error) = loaded.error.clone().or(detail.error.clone()) {
            return div()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .p(px(20.0))
                .text_sm_tw()
                .text_color(t.destructive)
                .child(error)
                .into_any_element();
        }
        let form = loaded.values.as_ref().map(values_to_form).unwrap_or_default();
        let detail = detail.submission;
        let status = detail.as_ref().map(|d| d.status).unwrap_or(match self.submission.status {
            OwnedSubmissionStatus::Open => SubmissionStatus::Open,
            OwnedSubmissionStatus::Rejected => SubmissionStatus::Rejected,
        });
        let kind = detail.as_ref().map(|d| d.kind).unwrap_or(self.submission.kind);
        let created_at = detail.as_ref().and_then(|d| d.created_at).or(self.submission.created_at);
        // Without the detail there are no review messages to show.
        let messages: &[SubmissionMessage] = detail.as_ref().map_or(&[], |d| d.messages.as_slice());
        let round = detail.as_ref().map(|d| d.revision).unwrap_or(1).max(1);
        // Comments stay with the revision the reviewer saw, even after resubmission.
        let mut by_round: BTreeMap<i64, Vec<&SubmissionMessage>> = BTreeMap::new();
        for m in messages {
            by_round.entry(m.revision.max(1)).or_default().push(m);
        }
        let relevant: Vec<&SubmissionMessage> = by_round.get(&round).cloned().unwrap_or_default();
        let has_messages = !relevant.is_empty();
        let open = status == SubmissionStatus::Open;
        let rejected = status == SubmissionStatus::Rejected;
        let review_threads: Vec<(i64, Vec<&SubmissionMessage>)> = by_round
            .iter()
            .rev()
            .filter(|(rev, _)| !rejected || **rev != round)
            .map(|(r, th)| (*r, th.clone()))
            .collect();
        let (busy, closed) = {
            let w = self.withdrawal.read(cx);
            (w.busy(), w.closed())
        };
        let editable = open && !closed;
        let from_history = self.back_to.is_some();
        let repo = text_of(&form.values, "repo");

        let title = {
            let alias = text_of(&form.values, "alias");
            if alias.is_empty() { text_of(&form.values, "name") } else { alias }
        };

        // ---- header ----
        let chip = if closed {
            // The backend closed this review, its outcome is unknown here.
            StatusChip::new(ReviewHistoryTone::Unknown, "No longer open")
        } else {
            StatusChip::new(status_tone(Some(status_key(status))), status_label(status))
        };
        let header = div()
            .flex()
            .items_start()
            .justify_between()
            .gap(px(12.0))
            .child(
                div()
                    .min_w_0()
                    .child(div().text_xl_tw().font_weight(FontWeight::SEMIBOLD).child(title))
                    .child(div().mt(px(4.0)).child(chip)),
            )
            .when_some(self.back_to.clone(), |d, (selection, label)| {
                let host = self.host.clone();
                d.child(
                    Button::new("submission-back")
                        .variant(ButtonVariant::Ghost)
                        .size(crate::ui::components::ButtonSize::Sm)
                        .flex_none()
                        .icon(icons::ARROW_LEFT)
                        .label(format!("Back to {label}"))
                        .on_click(move |_, _, cx| host.select(selection.clone(), cx)),
                )
            });

        // ---- actions ----
        let actions = div()
            .flex()
            .flex_wrap()
            .gap(px(8.0))
            .when(!repo.is_empty() && !from_history, |d| {
                let url = format!("https://github.com/{repo}");
                d.child(
                    Button::new("submission-code")
                        .variant(ButtonVariant::Outline)
                        .icon(icons::GITHUB)
                        .label("View code")
                        .on_click(move |_, _, cx| cx.open_url(&url)),
                )
            })
            .when(!self.editing && (editable || (rejected && !closed)), |d| {
                d.child(
                    Button::new("submission-revise")
                        .variant(ButtonVariant::Outline)
                        .label(if editable { "Revise submission" } else { "Resubmit for review" })
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.editing = true;
                            this.edit_panel = None;
                            cx.notify();
                        })),
                )
            })
            .when(editable, |d| {
                let withdrawal = self.withdrawal.clone();
                d.child(
                    Button::new("submission-withdraw")
                        .variant(ButtonVariant::Ghost)
                        .label("Withdraw submission")
                        .disabled(busy)
                        .on_click(move |_, window, cx| Withdrawal::confirm(&withdrawal, id, window, cx)),
                )
            });

        // ---- status card ----
        let (card_bg, card_border) = if rejected {
            (alpha(t.destructive, 0.05), alpha(t.destructive, 0.3))
        } else {
            (alpha(t.primary, 0.05), t.border)
        };
        let card = div()
            .flex()
            .flex_col()
            .gap(px(12.0))
            .rounded(radius::XL)
            .border_1()
            .border_color(card_border)
            .bg(card_bg)
            .p(px(16.0))
            .map(|d| {
                if open {
                    d.child(h3(if has_messages { "In review" } else { "Awaiting review" })).when(!has_messages, |d| {
                        d.child(
                            div()
                                .mt(px(4.0))
                                .text_sm_tw()
                                .text_color(t.muted_foreground)
                                .child("Reviewers will leave feedback here."),
                        )
                    })
                } else if rejected {
                    d.child(
                        div().child(h3("Changes requested")).child(
                            div()
                                .mt(px(4.0))
                                .text_sm_tw()
                                .text_color(t.muted_foreground)
                                .child("None of these changes are live."),
                        ),
                    )
                    .child(
                        div()
                            .border_l_2()
                            .border_color(t.destructive)
                            .pl(px(12.0))
                            .text_sm_tw()
                            .child(div().text_xs_tw().text_color(t.muted_foreground).child("Reviewer feedback"))
                            .child(if has_messages {
                                div()
                                    .mt(px(4.0))
                                    .flex()
                                    .flex_col()
                                    .gap(px(12.0))
                                    .children(relevant.iter().map(|m| Self::message_block(m, false, cx)))
                            } else {
                                div().mt(px(4.0)).child("Feedback is not available.")
                            }),
                    )
                } else {
                    d.child(h3("Approved")).child(
                        div()
                            .mt(px(4.0))
                            .text_sm_tw()
                            .text_color(t.muted_foreground)
                            .child("This submission was approved and is now live in the app."),
                    )
                }
            });

        // ---- previous review threads ----
        let threads = (!review_threads.is_empty()).then(|| {
            div().flex().flex_col().gap(px(24.0)).children(review_threads.iter().map(|(revision, thread)| {
                div()
                    .flex()
                    .flex_col()
                    .gap(px(12.0))
                    .child(h3(if *revision == round {
                        "Review"
                    } else if thread.len() > 1 {
                        "Previous review comments"
                    } else {
                        "Previous review comment"
                    }))
                    .children(thread.iter().map(|m| Self::message_block(m, true, cx)))
            }))
        });

        // ---- editor / declaration ----
        let main: AnyElement = if self.editing {
            if self.edit_panel.is_none() {
                let props = EditPanelProps {
                    initial: form.clone(),
                    initial_submission_id: Some(id),
                    name_locked: kind == DeclarationKind::Update,
                    require_changes: false,
                    intro_title: if rejected { "Resubmit for review" } else { "Update submission" },
                    intro_note: "Leaving this form discards unsent edits. Nothing goes live until this submission is approved.",
                    submit_label: if rejected { "Resubmit for review" } else { "Update submission" },
                    scroll: Some(self.scroll.clone()),
                };
                let panel = cx.new(|cx| AddonEditPanel::new(props, window, cx));
                cx.subscribe(&panel, |this, _, event: &EditPanelEvent, cx| match event {
                    EditPanelEvent::Cancel => {
                        this.editing = false;
                        this.edit_panel = None;
                        cx.notify();
                    }
                    EditPanelEvent::Submitted => {
                        this.editing = false;
                        this.edit_panel = None;
                        cx.notify();
                        // A save rewrites the rows this view renders, refresh fetches them again.
                        this.host.refresh(cx).detach();
                    }
                })
                .detach();
                self.edit_panel = Some(panel);
            }
            self.edit_panel.clone().unwrap().into_any_element()
        } else {
            let kofi = text_of(&form.values, "kofi");
            let description = text_of(&form.values, "description");
            div()
                .flex()
                .flex_col()
                .gap(px(12.0))
                .child(h3("Submitted declaration"))
                .child(div().text_sm_tw().text_color(t.muted_foreground).child(if description.is_empty() {
                    "No description provided.".to_string()
                } else {
                    description
                }))
                .child(detail_list(
                    vec![
                        DetailField::new("Name", text_of(&form.values, "name")),
                        DetailField::new("Alias", text_of(&form.values, "alias")),
                        DetailField::new("Author", text_of(&form.values, "author")),
                        DetailField::new("Repository", repo.clone())
                            .href((!repo.is_empty()).then(|| format!("https://github.com/{repo}"))),
                        DetailField::new("Branch", text_of(&form.values, "branch")),
                        DetailField::new("Tags", list_of(&form.values, "tags").join(", ")),
                        DetailField::new("Keywords", list_of(&form.values, "keywords").join(", ")),
                        DetailField::new("Dependencies", list_of(&form.values, "dependencies").join(", ")),
                        DetailField::new("Library", if bool_of(&form.values, "library") { "Yes" } else { "No" }),
                        DetailField::new("Ko-fi", kofi.clone())
                            .href((!kofi.is_empty()).then(|| format!("https://ko-fi.com/{kofi}"))),
                        DetailField::new(
                            "Submitted",
                            created_at.as_ref().map(fmt::local_date).unwrap_or_else(|| "Not available".into()),
                        ),
                    ],
                    cx,
                ))
                .into_any_element()
        };

        ScrollArea::new(("submission-scroll", id as usize))
            .size_full()
            .track_scroll(&self.scroll)
            .content_style(|d| d.p(px(20.0)).flex().flex_col().gap(px(20.0)))
            .child(header)
            .child(actions)
            .child(card)
            .children(threads)
            .child(main)
            .into_any_element()
    }
}
