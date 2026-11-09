//! Status card of the addon's latest review with withdraw, inspect and dismiss actions.
use cam_ui::{
    components::{Button, ButtonSize, ButtonVariant},
    theme::{TokensExt, Tw, TwText, alpha, radius, tw},
};
use gpui_kit::{AppContext as _, Context, Entity, IntoElement, ParentElement, Render, Styled, Window, div, px};

use super::{
    Selection,
    bits::h3,
    dialogs::{WITHDRAW_CHANGES, Withdrawal},
    host::Host,
    owned::{AddonReview, ReviewOutcome},
};

pub struct ReviewStatus {
    review: AddonReview,
    host: Host,
    dismissed: bool,
    /// Also tells when the backend closed the review underneath us, so withdrawal is no longer offered.
    withdrawal: Entity<Withdrawal>,
}

impl ReviewStatus {
    pub fn new(review: AddonReview, host: Host, cx: &mut Context<Self>) -> Self {
        let withdrawal = cx.new(|_| Withdrawal::new(&WITHDRAW_CHANGES, host.clone()));
        cx.observe(&withdrawal, |_, _, cx| cx.notify()).detach();
        Self { review, host, dismissed: false, withdrawal }
    }

    /// An approved review that was dismissed renders nothing.
    pub fn hidden(&self) -> bool {
        self.dismissed && self.review.status == ReviewOutcome::Approved
    }

    fn labels(&self) -> (&'static str, &'static str) {
        match self.review.status {
            ReviewOutcome::InReview => ("Changes in review", "Your published listing remains live until approval."),
            ReviewOutcome::Approved => ("Changes approved and published", "Your updated listing is now live."),
            ReviewOutcome::Rejected => {
                ("Listing edits rejected", "Your addon is still published. None of these changes went live.")
            }
        }
    }
}

impl Render for ReviewStatus {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.hidden() {
            return div().into_any_element();
        }
        let t = cx.tokens();
        let (title, description) = self.labels();
        let status = self.review.status;
        let (bg, border) = match status {
            ReviewOutcome::InReview => (alpha(t.primary, 0.05), t.border),
            ReviewOutcome::Rejected => (alpha(t.destructive, 0.05), alpha(t.destructive, 0.3)),
            ReviewOutcome::Approved => (alpha(tw(Tw::Emerald, 500), 0.05), alpha(tw(Tw::Emerald, 500), 0.3)),
        };
        let submission_id = self.review.submission_id;
        let host = self.host.clone();
        let (busy, closed) = {
            let w = self.withdrawal.read(cx);
            (w.busy(), w.closed())
        };
        let withdrawal = self.withdrawal.clone();
        div()
            .flex()
            .flex_col()
            .gap(px(12.0))
            .rounded(radius::XL)
            .border_1()
            .border_color(border)
            .bg(bg)
            .p(px(16.0))
            .child(
                div()
                    .child(h3(title))
                    .child(div().mt(px(4.0)).text_sm_tw().text_color(t.muted_foreground).child(description)),
            )
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap(px(8.0))
                    .when_inspect(status, move |d| {
                        d.child(
                            Button::new("review-inspect")
                                .variant(ButtonVariant::Outline)
                                .size(ButtonSize::Sm)
                                .label(if status == ReviewOutcome::Rejected {
                                    "View rejected changes"
                                } else {
                                    "View submitted changes"
                                })
                                .on_click(move |_, _, cx| host.select(Selection::Submission(submission_id), cx)),
                        )
                    })
                    .when_withdraw(status, closed, |d| {
                        d.child(
                            Button::new("review-withdraw")
                                .variant(ButtonVariant::Ghost)
                                .size(ButtonSize::Sm)
                                .label("Withdraw changes")
                                .disabled(busy)
                                .on_click(move |_, window, cx| {
                                    Withdrawal::confirm(&withdrawal, submission_id, window, cx)
                                }),
                        )
                    })
                    .when_dismiss(status, |d| {
                        d.child(
                            Button::new("review-dismiss")
                                .variant(ButtonVariant::Ghost)
                                .size(ButtonSize::Sm)
                                .label("Dismiss")
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.dismissed = true;
                                    cx.notify();
                                })),
                        )
                    }),
            )
            .into_any_element()
    }
}

/// Readability helpers for the conditional action buttons.
trait ActionRow: Sized {
    fn when_inspect(self, status: ReviewOutcome, f: impl FnOnce(Self) -> Self) -> Self;
    fn when_withdraw(self, status: ReviewOutcome, closed: bool, f: impl FnOnce(Self) -> Self) -> Self;
    fn when_dismiss(self, status: ReviewOutcome, f: impl FnOnce(Self) -> Self) -> Self;
}

impl ActionRow for gpui_kit::Div {
    fn when_inspect(self, status: ReviewOutcome, f: impl FnOnce(Self) -> Self) -> Self {
        if matches!(status, ReviewOutcome::InReview | ReviewOutcome::Rejected) { f(self) } else { self }
    }
    fn when_withdraw(self, status: ReviewOutcome, closed: bool, f: impl FnOnce(Self) -> Self) -> Self {
        if status == ReviewOutcome::InReview && !closed { f(self) } else { self }
    }
    fn when_dismiss(self, status: ReviewOutcome, f: impl FnOnce(Self) -> Self) -> Self {
        if status == ReviewOutcome::Approved { f(self) } else { self }
    }
}
