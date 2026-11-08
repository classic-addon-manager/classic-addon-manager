//! Alert dialogs of the developer page: the "submission already exists" notice, and the withdraw action of the review
//! and submission views with its confirmation (the confirm action keeps the dialog open and shows a busy state).
use std::rc::Rc;

use gpui_kit::{App, Context, Entity, ParentElement as _, SharedString, Window};

use super::{
    api::{self, WithdrawResult},
    host::Host,
};
use crate::{
    state::run_blocking,
    ui::{
        components::{
            Button, ButtonVariant,
            alert_dialog::{self, AlertSpec},
            dialog::{self, DialogFooter, DialogSpec},
        },
        icons,
        toast::{self, Toast},
    },
};

/// "Keep editing" (also Escape) dismisses, "Resume existing" resumes the existing submission.
pub fn open_already_open(
    window: &mut Window,
    cx: &mut App,
    on_dismiss: impl Fn(&mut Window, &mut App) + 'static,
    on_resume: impl Fn(&mut Window, &mut App) + 'static,
) {
    alert_dialog::confirm(
        window,
        cx,
        AlertSpec::new("Submission already exists", "You already have an open or rejected submission for this name.")
            .cancel_label("Keep editing")
            .confirm_label("Resume existing")
            .on_cancel(on_dismiss)
            .on_confirm(on_resume),
    );
}

/// One kind of withdrawal: what its confirmation and toasts say, and what happens to the submission afterwards.
pub struct WithdrawKind {
    title: &'static str,
    description: &'static str,
    /// Action label while idle (the busy label is always "Withdrawing...").
    action_label: &'static str,
    /// Title of the toasts that say why nothing was withdrawn.
    warn_title: &'static str,
    done_title: &'static str,
    not_open: &'static str,
    /// The withdrawn or missing submission leaves the owned list at once. Without it the view stays and stops offering
    /// withdrawal.
    drops_submission: bool,
}

/// Withdrawing the changes under review of a published addon.
pub const WITHDRAW_CHANGES: WithdrawKind = WithdrawKind {
    title: "Withdraw these changes?",
    description: "This closes the review and removes the working proposal. The published addon and Submission history stay unchanged.",
    action_label: "Withdraw changes",
    warn_title: "Withdraw changes",
    done_title: "Changes withdrawn",
    not_open: "This review is no longer open.",
    drops_submission: false,
};

/// Withdrawing a submission of a new addon.
pub const WITHDRAW_SUBMISSION: WithdrawKind = WithdrawKind {
    title: "Withdraw this submission?",
    description: "This closes the review and removes the working proposal. Submission history is retained.",
    action_label: "Withdraw submission",
    warn_title: "Withdraw submission",
    done_title: "Submission withdrawn",
    not_open: "This submission is no longer open.",
    drops_submission: true,
};

/// The withdraw action of a review or submission view. The view keeps this entity, observes it, and reads [`busy`]
/// and [`closed`] for its buttons.
///
/// [`busy`]: Withdrawal::busy
/// [`closed`]: Withdrawal::closed
pub struct Withdrawal {
    kind: &'static WithdrawKind,
    host: Host,
    busy: bool,
    closed: bool,
}

impl Withdrawal {
    pub fn new(kind: &'static WithdrawKind, host: Host) -> Self {
        Self { kind, host, busy: false, closed: false }
    }

    /// A withdraw request is running.
    pub fn busy(&self) -> bool {
        self.busy
    }

    /// The submission is closed, so withdrawing it is no longer possible.
    pub fn closed(&self) -> bool {
        self.closed
    }

    /// Asks for confirmation, then withdraws submission `id`. While the request runs the dialog stays open, its buttons
    /// are disabled, and X, Escape and the overlay are ignored. The dialog closes when the server answered, except
    /// after an error the user may retry.
    pub fn confirm(this: &Entity<Self>, id: i64, window: &mut Window, cx: &mut App) {
        let kind = this.read(cx).kind;
        let busy = Rc::new({
            let this = this.clone();
            move |cx: &App| this.read(cx).busy
        });
        let busy_footer = busy.clone();
        let on_confirm = Rc::new({
            let this = this.clone();
            move |window: &mut Window, cx: &mut App| this.update(cx, |this, cx| this.run(id, window, cx))
        });
        let action_label: SharedString = kind.action_label.into();
        let mut dialog_spec = DialogSpec::new()
            .title(kind.title)
            .description(kind.description)
            .show_close(false)
            .overlay_closes(false)
            .locked(move |cx| busy(cx))
            .footer(move |_, cx| {
                let is_busy = busy_footer(cx);
                let on_confirm = on_confirm.clone();
                DialogFooter::new()
                    .child(
                        Button::new("withdraw-cancel")
                            .variant(ButtonVariant::Outline)
                            .label("Keep in review")
                            .disabled(is_busy)
                            .on_click(|_, window, cx| dialog::close(window, cx)),
                    )
                    .child(
                        Button::new("withdraw-action")
                            .variant(ButtonVariant::Destructive)
                            .label(if is_busy { SharedString::from("Withdrawing...") } else { action_label.clone() })
                            .disabled(is_busy)
                            .on_click(move |_, window, cx| on_confirm(window, cx)),
                    )
            });
        dialog_spec.alert_title = true;
        dialog::open(window, cx, dialog_spec);
    }

    fn run(&mut self, id: i64, window: &mut Window, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        self.busy = true;
        cx.notify();
        cx.spawn_in(window, async move |this, cx| {
            let result = run_blocking(cx, move || api::withdraw_submission(id)).await;
            let _ = this.update_in(cx, |this, window, cx| {
                this.busy = false;
                this.apply(id, result, window, cx);
                cx.notify();
            });
        })
        .detach();
    }

    fn apply(&mut self, id: i64, result: WithdrawResult, window: &mut Window, cx: &mut Context<Self>) {
        let kind = self.kind;
        let warn = |cx: &mut Context<Self>, message: String| {
            toast::show(cx, Toast::new(kind.warn_title, message).icon(icons::TRIANGLE_ALERT));
        };
        match result {
            WithdrawResult::Withdrawn => {
                dialog::close(window, cx);
                toast::show(cx, Toast::new(kind.done_title, "The review is closed.").icon(icons::CHECK));
                if kind.drops_submission {
                    self.host.drop_submission(id, cx);
                } else {
                    self.closed = true;
                }
            }
            WithdrawResult::NotOpen(_) => {
                dialog::close(window, cx);
                self.closed = true;
                warn(cx, kind.not_open.into());
            }
            WithdrawResult::NotFound(message) => {
                dialog::close(window, cx);
                warn(cx, message);
                if kind.drops_submission {
                    self.host.drop_submission(id, cx);
                }
            }
            // The dialog stays open, so the user can try again.
            WithdrawResult::Error(message) => return warn(cx, message),
        }
        self.host.refresh(cx).detach();
    }
}
