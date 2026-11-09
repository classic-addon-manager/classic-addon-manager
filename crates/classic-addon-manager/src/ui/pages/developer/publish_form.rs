//! Full-page "Publish addon" form opened from the toolbar or the empty state.
use cam_core::api::developer::{
    declaration::{PublishFormState, is_publish_form_dirty},
    types::DeclarationKind,
};
use cam_ui::{
    components::{
        Button, ButtonVariant, ScrollArea,
        alert_dialog::{self, AlertSpec},
    },
    icons,
    theme::{TokensExt, Tw, TwText, alpha, radius, tw},
};
use gpui_kit::component::Icon;
use gpui_kit::{
    AppContext as _, Context, Entity, EventEmitter, FontWeight, IntoElement, ParentElement, Pixels, Render,
    ScrollHandle, Styled, Window, div, prelude::FluentBuilder as _, px,
};

use super::{
    cta::{CtaState, cta_button},
    dialogs,
    fields::AddonDeclarationFields,
    form::{DeclarationForm, FormEvent},
};

pub enum PublishFormEvent {
    /// The form was submitted, or the user went back from it.
    Close,
}

pub struct PublishAddonForm {
    form: Entity<DeclarationForm>,
    fields: Entity<AddonDeclarationFields>,
    scroll: ScrollHandle,
    already_open_shown: bool,
}

impl EventEmitter<PublishFormEvent> for PublishAddonForm {}

/// Widest the header content may get: the largest of 1536, 1280, 1024, 768 or 640 px that fits the usable window
/// width, or `None` when the window is narrower than 640 px.
fn container_width(window: &Window) -> Option<Pixels> {
    let pad = gpui_kit::component::window_paddings(window);
    let width = f32::from(window.viewport_size().width - pad.left - pad.right);
    let max = [(1536.0, 1536.0), (1280.0, 1280.0), (1024.0, 1024.0), (768.0, 768.0), (640.0, 640.0)]
        .iter()
        .find(|(bp, _)| width >= *bp)
        .map(|(_, max)| *max);
    max.map(px)
}

impl PublishAddonForm {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let form = cx.new(|cx| DeclarationForm::new(PublishFormState::default(), None, cx));
        let fields = cx.new(|cx| AddonDeclarationFields::new(form.clone(), Vec::new(), window, cx));
        let scroll = ScrollHandle::new();
        fields.update(cx, |f, _| f.set_scroll_handle(scroll.clone()));
        cx.subscribe_in(&form, window, |this, _, event: &FormEvent, window, cx| match event {
            FormEvent::Submitted => cx.emit(PublishFormEvent::Close),
            FormEvent::Resumed(kind) => {
                let locked = if *kind == DeclarationKind::Update { vec!["name".to_string()] } else { Vec::new() };
                this.fields.update(cx, |f, cx| f.set_locked_fields(locked, cx));
            }
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
        Self { form, fields, scroll, already_open_shown: false }
    }

    fn handle_back(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if is_publish_form_dirty(&self.form.read(cx).form, &PublishFormState::default()) {
            let this = cx.entity();
            alert_dialog::confirm(
                window,
                cx,
                AlertSpec::new("Discard this addon?", "Your declaration will be lost.")
                    .cancel_label("Stay")
                    .confirm_label("Discard")
                    .destructive(true)
                    .on_confirm(move |_, cx| this.update(cx, |_, cx| cx.emit(PublishFormEvent::Close))),
            );
            return;
        }
        cx.emit(PublishFormEvent::Close);
    }
}

impl Render for PublishAddonForm {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = cx.tokens();
        let (busy, validated, validating, submitting, submission_id, fail_copy, publish_error_none) = {
            let f = self.form.read(cx);
            (
                f.busy(),
                f.validated,
                f.validating,
                f.submitting,
                f.submission_id,
                f.fail_copy(),
                f.publish_error.is_none(),
            )
        };
        let fail_header = fail_copy.is_some();
        let success_header = validated && publish_error_none;
        let emerald = tw(Tw::Emerald, 500);
        let container = container_width(window);
        let (header_border, header_tint) = if success_header {
            (alpha(emerald, 0.2), Some(alpha(emerald, 0.1)))
        } else if fail_header {
            (alpha(t.destructive, 0.2), Some(alpha(t.destructive, 0.1)))
        } else {
            (t.border, None)
        };
        let (icon_bg, icon, icon_color) = if success_header {
            (alpha(emerald, 0.2), icons::CHECK, tw(Tw::Emerald, 400))
        } else if fail_header {
            (alpha(t.destructive, 0.2), icons::CIRCLE_ALERT, t.destructive)
        } else {
            (alpha(t.primary, 0.1), icons::CODE_XML, t.primary)
        };
        let subtitle_color = if success_header {
            tw(Tw::Emerald, 400)
        } else if fail_header {
            t.destructive
        } else {
            t.muted_foreground
        };
        let subtitle = if success_header {
            "Declaration valid - ready to publish.".to_string()
        } else if let Some(copy) = fail_copy {
            copy
        } else {
            "Fill in the addon declaration.".to_string()
        };
        let form = self.form.clone();

        let header = div()
            .relative()
            .flex_none()
            .border_b_1()
            .border_color(header_border)
            .bg(alpha(t.background, 0.95))
            .when_some(header_tint, |d, tint| d.child(div().absolute().top_0().left_0().right_0().bottom_0().bg(tint)))
            .child(
                div()
                    .relative()
                    .flex()
                    .h(px(64.0))
                    .items_center()
                    .justify_between()
                    .gap(px(16.0))
                    .px(px(16.0))
                    .when_some(container, |d, w| d.max_w(w))
                    .child(
                        div()
                            .flex()
                            .min_w_0()
                            .items_center()
                            .gap(px(12.0))
                            .child(
                                div()
                                    .rounded(radius::LG)
                                    .p(px(8.0))
                                    .bg(icon_bg)
                                    .child(Icon::new(icon).size(px(24.0)).text_color(icon_color)),
                            )
                            .child(
                                div()
                                    .min_w_0()
                                    .child(div().text_xl_tw().font_weight(FontWeight::SEMIBOLD).child(
                                        if submission_id.is_none() { "Publish addon" } else { "Update submission" },
                                    ))
                                    .child(div().text_sm_tw().text_color(subtitle_color).child(subtitle)),
                            ),
                    )
                    .child(
                        Button::new("publish-back")
                            .variant(ButtonVariant::Outline)
                            .w(px(128.0))
                            .icon(icons::ARROW_LEFT)
                            .label("Back")
                            .disabled(busy)
                            .on_click(cx.listener(|this, _, window, cx| this.handle_back(window, cx))),
                    ),
            );

        let main = ScrollArea::new("publish-form-scroll")
            .flex_1()
            .min_h_0()
            .track_scroll(&self.scroll)
            .child(div().w_full().max_w(px(672.0)).mx_auto().px(px(16.0)).py(px(32.0)).child(self.fields.clone()));

        let footer = div()
            .flex()
            .flex_none()
            .justify_end()
            .border_t_1()
            .border_color(t.border)
            .bg(alpha(t.background, 0.95))
            .px(px(16.0))
            .py(px(12.0))
            .child(cta_button(
                "publish-cta",
                CtaState {
                    validated,
                    validating,
                    submitting,
                    disabled: busy,
                    submit_label: if submission_id.is_none() {
                        if submitting { "Publish" } else { "Submit" }.into()
                    } else {
                        "Update".into()
                    },
                    width: Some(px(128.0)),
                },
                move |_, cx| form.update(cx, |f, cx| f.primary_action(cx)),
            ));

        div().size_full().min_h_0().flex().flex_col().child(header).child(main).child(footer)
    }
}
