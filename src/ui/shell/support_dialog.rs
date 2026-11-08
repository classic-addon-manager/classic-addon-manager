//! The Ko-fi support modal. The shell shows it at startup when no app update is available and the app state says it
//! is due.
use gpui_kit::component::Icon;
use gpui_kit::{
    App, AppContext as _, Context, IntoElement, ParentElement, Render, Styled, Task, Window, div,
    prelude::FluentBuilder as _, px,
};

use super::widgets::{TintedButton, icon_tile};
use crate::{
    backend::appstate,
    state::{self, run_blocking},
    ui::{
        components::{Button, ButtonVariant, DialogDescription, DialogSpec, DialogTitle, dialog},
        icons,
        theme::{TokensExt, Tw, TwText, alpha, radius, tw},
    },
};

const KOFI_URL: &str = "https://ko-fi.com/gaijindev";

/// Whether the Ko-fi dialog is due, as the app state file says. When it can't be read, logs and answers false.
pub fn is_due(cx: &App) -> Task<bool> {
    let backend = state::backend(cx);
    run_blocking(cx, move || {
        appstate::kofi_modal_due(&backend.paths).unwrap_or_else(|e| {
            log::error!("Failed to check kofi modal eligibility: {e:#}");
            false
        })
    })
}

/// Opens the Ko-fi dialog. However it is closed, the app state records it as shown.
pub fn open(window: &mut Window, cx: &mut App) {
    let content = cx.new(|_| SupportContent { recorded: false });
    let on_close = content.clone();
    dialog::open(
        window,
        cx,
        DialogSpec::new().max_width(px(448.0)).content_view(content).on_close(move |_, cx| {
            // Closed with the X, Escape or a click on the overlay.
            on_close.update(cx, |this, cx| this.record_shown(cx));
        }),
    );
}

struct SupportContent {
    recorded: bool,
}

impl SupportContent {
    /// Tells the backend the modal was shown. Only the first call reaches it.
    fn record_shown(&mut self, cx: &mut Context<Self>) {
        if self.recorded {
            return;
        }
        self.recorded = true;
        let backend = state::backend(cx);
        run_blocking(cx, move || {
            if let Err(e) = appstate::record_kofi_modal_shown(&backend.paths) {
                log::error!("Failed to record kofi modal shown: {e:#}");
            }
        })
        .detach();
    }

    fn maybe_later(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        dialog::close(window, cx);
        self.record_shown(cx);
    }

    fn support(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        cx.open_url(KOFI_URL);
        dialog::close(window, cx);
        self.record_shown(cx);
    }
}

impl Render for SupportContent {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = cx.tokens();
        let rose = |shade| tw(Tw::Rose, shade);
        let white = |a: f32| gpui_kit::hsla(0.0, 0.0, 1.0, a);

        let header = div()
            .flex()
            .items_start()
            .gap(px(12.0))
            .child(icon_tile(icons::HEART, alpha(rose(500), 0.2), rose(400)))
            .child(
                div()
                    .child(DialogTitle::new("Enjoying Classic Addon Manager?"))
                    .child(DialogDescription::new("Free to use, supported by players like you")),
            );

        let paragraph =
            |text: &'static str| div().w_full().text_center().text_sm_tw().text_color(t.muted_foreground).child(text);
        let body = div()
            .flex()
            .flex_col()
            .items_center()
            .gap(px(16.0))
            .rounded(radius::LG)
            .border_1()
            .border_color(white(0.05))
            .bg(alpha(tw(Tw::Zinc, 900), 0.5))
            .px(px(16.0))
            .py(px(20.0))
            .child(paragraph(
                "Classic Addon Manager is an independent project developed and maintained by Sami for ArcheAge Classic.",
            ))
            .child(paragraph("Tips on Ko-fi help fund infrastructure and continued development. "))
            .child(paragraph("Thanks for supporting the project!"));

        let later = Button::new("support-later")
            .variant(ButtonVariant::Ghost)
            .label("Maybe later")
            .on_click(cx.listener(|this, _, window, cx| this.maybe_later(window, cx)));
        let support = TintedButton::new("support-kofi", rose(900), rose(800), alpha(rose(900), 0.2))
            .has_icon(true)
            .on_click(cx.listener(|this, _, window, cx| this.support(window, cx)))
            .child(Icon::new(icons::HEART).size(px(16.0)))
            .child("Support on Ko-fi");

        div()
            .flex()
            .flex_col()
            .gap(px(16.0))
            .child(header)
            .child(body)
            .child(div().flex().justify_end().gap(px(8.0)).child(later).child(support))
    }
}
