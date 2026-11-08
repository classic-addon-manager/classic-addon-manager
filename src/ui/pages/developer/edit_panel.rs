//! Inline editor for an addon declaration, used by addon details and by rejected or open submissions.
use gpui_kit::{
    AppContext as _, Context, Entity, EventEmitter, IntoElement, ParentElement, Render, ScrollHandle, SharedString,
    Styled, Window, div, prelude::FluentBuilder as _, px,
};

use super::{
    cta::{CtaState, cta_button},
    dialogs,
    fields::AddonDeclarationFields,
    form::{DeclarationForm, FormEvent, PublishFormState},
};
use crate::ui::{
    components::{Button, ButtonVariant},
    theme::{TokensExt, TwText},
};

pub enum EditPanelEvent {
    /// The user pressed Cancel.
    Cancel,
    /// The submission was sent. The panel does not close itself, the owner leaves edit mode.
    Submitted,
}

pub struct EditPanelProps {
    pub initial: PublishFormState,
    /// Pinned submission being revised, `None` creates a new submission.
    pub initial_submission_id: Option<i64>,
    pub name_locked: bool,
    /// Require unsaved changes before validating, off for resubmitting an unchanged declaration.
    pub require_changes: bool,
    pub intro_title: &'static str,
    pub intro_note: &'static str,
    pub submit_label: &'static str,
    /// Scroll area holding the panel (used to bring the first error into view).
    pub scroll: Option<ScrollHandle>,
}

impl EditPanelProps {
    pub const DEFAULT_TITLE: &'static str = "Edit listing details";
    pub const DEFAULT_NOTE: &'static str =
        "Leaving this form discards unsent edits. Your published listing stays unchanged until approval.";
}

pub struct AddonEditPanel {
    form: Entity<DeclarationForm>,
    fields: Entity<AddonDeclarationFields>,
    intro_title: &'static str,
    intro_note: &'static str,
    submit_label: SharedString,
    require_changes: bool,
    already_open_shown: bool,
}

impl EventEmitter<EditPanelEvent> for AddonEditPanel {}

impl AddonEditPanel {
    pub fn new(props: EditPanelProps, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let form = cx.new(|cx| DeclarationForm::new(props.initial, props.initial_submission_id, cx));
        let locked = if props.name_locked { vec!["name".to_string()] } else { Vec::new() };
        let fields = cx.new(|cx| AddonDeclarationFields::new(form.clone(), locked, window, cx));
        if let Some(scroll) = props.scroll {
            fields.update(cx, |f, _| f.set_scroll_handle(scroll));
        }
        cx.subscribe_in(&form, window, |this, _, event: &FormEvent, window, cx| match event {
            FormEvent::Submitted => cx.emit(EditPanelEvent::Submitted),
            FormEvent::Resumed(_) => {}
            FormEvent::ScrollToError(key) => {
                let key = *key;
                this.fields.update(cx, |f, cx| f.scroll_to_field(key, window, cx));
            }
        })
        .detach();
        cx.observe_in(&form, window, |this, form, window, cx| {
            let open = form.read(cx).already_open_id.is_some();
            if open && !this.already_open_shown {
                this.already_open_shown = true;
                let (dismiss, resume) = (this.form.clone(), this.form.clone());
                dialogs::open_already_open(
                    window,
                    cx,
                    move |_, cx| dismiss.update(cx, |f, cx| f.dismiss_already_open(cx)),
                    move |_, cx| resume.update(cx, |f, cx| f.resume_already_open(cx)),
                );
            } else if !open {
                this.already_open_shown = false;
            }
            cx.notify();
        })
        .detach();
        Self {
            form,
            fields,
            intro_title: props.intro_title,
            intro_note: props.intro_note,
            submit_label: props.submit_label.into(),
            require_changes: props.require_changes,
            already_open_shown: false,
        }
    }
}

impl Render for AddonEditPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = cx.tokens();
        let (busy, validated, validating, submitting, dirty, fail_copy) = {
            let f = self.form.read(cx);
            (f.busy(), f.validated, f.validating, f.submitting, f.dirty(), f.fail_copy())
        };
        let cta_disabled = busy || (!validated && self.require_changes && !dirty);
        let form = self.form.clone();
        div()
            .flex()
            .flex_col()
            .gap(px(24.0))
            .child(
                div()
                    .child(div().font_weight(gpui_kit::FontWeight::MEDIUM).child(self.intro_title))
                    .child(div().mt(px(4.0)).text_sm_tw().text_color(t.muted_foreground).child(self.intro_note)),
            )
            .child(self.fields.clone())
            .when_some(fail_copy, |d, copy| d.child(div().text_sm_tw().text_color(t.destructive).child(copy)))
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap(px(8.0))
                    .border_t_1()
                    .border_color(t.border)
                    .pt(px(16.0))
                    .child(cta_button(
                        "edit-panel-cta",
                        CtaState {
                            validated,
                            validating,
                            submitting,
                            disabled: cta_disabled,
                            submit_label: self.submit_label.clone(),
                            width: None,
                        },
                        move |_, cx| form.update(cx, |f, cx| f.primary_action(cx)),
                    ))
                    .child(
                        Button::new("edit-panel-cancel")
                            .variant(ButtonVariant::Outline)
                            .label("Cancel")
                            .disabled(busy)
                            .on_click(cx.listener(|_, _, _, cx| cx.emit(EditPanelEvent::Cancel))),
                    ),
            )
    }
}
