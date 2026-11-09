//! Confirmation dialog without an X button. Clicking the overlay does not dismiss it and Escape acts as Cancel. The
//! footer holds an outline Cancel button and a primary (or destructive) Action button.
//!
//! ```ignore
//! alert_dialog::confirm(window, cx, AlertSpec::new("Delete addon?", "This cannot be undone.")
//!     .confirm_label("Delete").destructive(true)
//!     .on_confirm(|window, cx| { /* runs, then the dialog closes */ }));
//! alert_dialog::alert(window, cx, AlertSpec::new("Already open", "Close the other window first.").confirm_label("OK"));
//! ```
//! `.auto_close(false)`: the Action does not close the dialog (close it yourself with `dialog::close(window, cx)`,
//! e.g. after an async job). `.content(|window, cx| element)` adds a body between description and footer.
use std::rc::Rc;

use gpui_kit::{AnyElement, App, IntoElement, ParentElement, Pixels, SharedString, Window, px};

use super::{
    button::{Button, ButtonVariant},
    dialog::{self, DialogFooter, DialogHandler, DialogSpec},
};

type ContentFn = Rc<dyn Fn(&mut Window, &mut App) -> AnyElement>;

/// Description of an alert dialog for [`confirm`] / [`alert`].
pub struct AlertSpec {
    title: SharedString,
    description: SharedString,
    confirm_label: SharedString,
    cancel_label: SharedString,
    destructive: bool,
    auto_close: bool,
    on_confirm: Option<DialogHandler>,
    on_cancel: Option<DialogHandler>,
    content: Option<ContentFn>,
    max_width: Pixels,
}

impl AlertSpec {
    pub fn new(title: impl Into<SharedString>, description: impl Into<SharedString>) -> Self {
        Self {
            title: title.into(),
            description: description.into(),
            confirm_label: "Continue".into(),
            cancel_label: "Cancel".into(),
            destructive: false,
            auto_close: true,
            on_confirm: None,
            on_cancel: None,
            content: None,
            max_width: px(512.0),
        }
    }

    /// Text of the Action button (default "Continue").
    pub fn confirm_label(mut self, label: impl Into<SharedString>) -> Self {
        self.confirm_label = label.into();
        self
    }

    /// Text of the Cancel button (default "Cancel").
    pub fn cancel_label(mut self, label: impl Into<SharedString>) -> Self {
        self.cancel_label = label.into();
        self
    }

    /// Destructive (red) Action button.
    pub fn destructive(mut self, destructive: bool) -> Self {
        self.destructive = destructive;
        self
    }

    /// Close the dialog after `on_confirm` (default true).
    pub fn auto_close(mut self, auto_close: bool) -> Self {
        self.auto_close = auto_close;
        self
    }

    pub fn on_confirm(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_confirm = Some(Rc::new(handler));
        self
    }

    /// Runs for Cancel and Escape.
    pub fn on_cancel(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_cancel = Some(Rc::new(handler));
        self
    }

    /// Extra body between the description and the footer (called every frame).
    pub fn content<E: IntoElement>(mut self, content: impl Fn(&mut Window, &mut App) -> E + 'static) -> Self {
        self.content = Some(Rc::new(move |w, cx| content(w, cx).into_any_element()));
        self
    }

    pub fn max_width(mut self, width: Pixels) -> Self {
        self.max_width = width;
        self
    }
}

fn open(window: &mut Window, cx: &mut App, spec: AlertSpec, show_cancel: bool) {
    let AlertSpec {
        title,
        description,
        confirm_label,
        cancel_label,
        destructive,
        auto_close,
        on_confirm,
        on_cancel,
        content,
        max_width,
    } = spec;

    let mut dialog = DialogSpec::new()
        .title(title)
        .description(description)
        .max_width(max_width)
        .show_close(false)
        .overlay_closes(false);
    dialog.alert_title = true;
    if let Some(on_cancel) = on_cancel.clone() {
        dialog = dialog.on_close(move |window, cx| on_cancel(window, cx));
    }
    if let Some(content) = content {
        dialog = dialog.content(move |window, cx| content(window, cx));
    }
    dialog = dialog.footer(move |_, _| {
        let cancel = on_cancel.clone();
        let confirm = on_confirm.clone();
        let cancel_label = cancel_label.clone();
        let confirm_label = confirm_label.clone();
        DialogFooter::new()
            .when_cancel(show_cancel, move |footer| {
                footer.child(
                    Button::new("alert-dialog-cancel").variant(ButtonVariant::Outline).label(cancel_label).on_click(
                        move |_, window, cx| {
                            if let Some(f) = &cancel {
                                f(window, cx);
                            }
                            dialog::close(window, cx);
                        },
                    ),
                )
            })
            .child(
                Button::new("alert-dialog-action")
                    .variant(if destructive { ButtonVariant::Destructive } else { ButtonVariant::Default })
                    .label(confirm_label)
                    .on_click(move |_, window, cx| {
                        if auto_close {
                            dialog::close(window, cx);
                        }
                        if let Some(f) = &confirm {
                            f(window, cx);
                        }
                    }),
            )
    });
    dialog::open(window, cx, dialog);
}

/// Confirmation dialog: Cancel + Action.
pub fn confirm(window: &mut Window, cx: &mut App, spec: AlertSpec) {
    open(window, cx, spec, true);
}

/// Information dialog: only the Action button (`AlertSpec::confirm_label`, e.g. "OK").
pub fn alert(window: &mut Window, cx: &mut App, spec: AlertSpec) {
    open(window, cx, spec, false);
}

/// Small helper so the footer chain stays readable.
trait WhenCancel: Sized {
    fn when_cancel(self, cond: bool, f: impl FnOnce(Self) -> Self) -> Self {
        if cond { f(self) } else { self }
    }
}
impl WhenCancel for DialogFooter {}
