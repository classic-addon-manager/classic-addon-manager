//! `error_box`: the red-bordered box with an alert icon, a title and a message that shows a failed action inside a
//! page or dialog.
use gpui_kit::component::Icon;
use gpui_kit::{FontWeight, ParentElement, SharedString, Styled, div, px};

use crate::ui::{
    icons,
    theme::{Tokens, TwText, alpha, radius},
};

/// Size and margins are the caller's: the box takes the width it is given.
pub fn error_box(t: &Tokens, title: impl Into<SharedString>, message: impl Into<SharedString>) -> gpui_kit::Div {
    div()
        .flex()
        .gap_2()
        .rounded(radius::LG)
        .border_1()
        .border_color(alpha(t.destructive, 0.2))
        .bg(alpha(t.destructive, 0.1))
        .p_3()
        .child(
            div()
                .mt(px(2.0))
                .flex_none()
                .child(Icon::new(icons::CIRCLE_ALERT).size(px(16.0)).text_color(t.destructive)),
        )
        .child(
            div()
                .min_w_0()
                .flex()
                .flex_col()
                .gap(px(2.0))
                .child(div().text_sm_tw().font_weight(FontWeight::MEDIUM).text_color(t.destructive).child(title.into()))
                .child(div().text_xs_tw().text_color(alpha(t.destructive, 0.8)).child(message.into())),
        )
}
