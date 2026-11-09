//! Warning shown before a link leaves the app for the web browser: orange alert icon, the full target url in a
//! monospace box, Cancel and Proceed. Escape acts as Cancel, clicking the overlay does not dismiss it and Enter does
//! nothing, so the browser only opens on Proceed.
//!
//! ```ignore
//! external_link_dialog::open(window, cx, "https://example.com/".into());
//! ```
use gpui_kit::component::Icon;
use gpui_kit::{
    App, FontWeight, InteractiveElement as _, IntoElement, ParentElement, SharedString,
    StatefulInteractiveElement as _, Styled, Window, div, px,
};

use super::{
    button::{Button, ButtonVariant},
    dialog::{self, DialogFooter, DialogSpec},
};
use crate::{
    icons,
    theme::{Tokens, TokensExt, Tw, TwText, alpha, radius, tw},
};

/// Opens the warning for `url`; Proceed closes it and opens `url` in the OS handler.
pub fn open(window: &mut Window, cx: &mut App, url: SharedString) {
    let target = url.clone();
    let spec = DialogSpec::new()
        .show_close(false)
        .overlay_closes(false)
        .warning(true)
        .content(move |_, cx| body(url.clone(), cx.tokens()))
        .footer(move |_, _| {
            let target = target.clone();
            DialogFooter::new()
                .child(
                    Button::new("external-link-cancel")
                        .variant(ButtonVariant::Outline)
                        .label("Cancel")
                        .on_click(|_, window, cx| dialog::close(window, cx)),
                )
                .child(Button::new("external-link-proceed").label("Proceed").icon_end(icons::EXTERNAL_LINK).on_click(
                    move |_, window, cx| {
                        dialog::close(window, cx);
                        cx.open_url(&target);
                    },
                ))
        });
    dialog::open(window, cx, spec);
}

fn body(url: SharedString, t: Tokens) -> impl IntoElement {
    let orange = tw(Tw::Orange, 400);
    let header = div()
        .flex()
        .items_start()
        .gap_4()
        .child(
            div()
                .flex()
                .flex_none()
                .items_center()
                .justify_center()
                .size(px(40.0))
                .rounded(radius::LG)
                .border_1()
                .border_color(alpha(orange, 0.25))
                .bg(alpha(orange, 0.12))
                .text_color(orange)
                .child(Icon::new(icons::TRIANGLE_ALERT).size(px(20.0))),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .gap_1()
                .flex_1()
                .min_w_0()
                // centre a one-line headline on the 40px icon
                .pt(px(6.0))
                .child(
                    div()
                        .text_lg_tw()
                        .line_height(px(28.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .child("You are leaving Classic Addon Manager"),
                )
                .child(
                    div()
                        .text_sm_tw()
                        .text_color(t.muted_foreground)
                        .child("This link opens in your web browser. Only proceed if you trust where it leads."),
                ),
        );
    let target = div()
        .id("external-link-url")
        .max_h(px(160.0))
        .overflow_y_scroll()
        .rounded(radius::MD)
        .border_1()
        .border_color(alpha(orange, 0.2))
        .bg(alpha(orange, 0.06))
        .px_3()
        .py_2()
        .font_mono_tw()
        .text_xs_tw()
        .text_color(alpha(t.foreground, 0.85))
        .child(url);
    div().flex().flex_col().gap_4().child(header).child(target)
}
