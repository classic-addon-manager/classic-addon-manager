//! Error dialog: red alert icon, a headline and message per problem, raw error text in a monospace box, one OK.
//! One item puts its headline and message in the dialog header, several items are listed under a count headline.
//!
//! ```ignore
//! error_dialog::open(window, cx, vec![
//!     ErrorItem::new("Settings were reset", "config.toml was moved aside.").details("TOML parse error at line 1 …"),
//! ]);
//! ```
use std::rc::Rc;

use gpui_kit::component::Icon;
use gpui_kit::{
    AnyElement, App, FontWeight, InteractiveElement as _, IntoElement, ParentElement, ScrollHandle, SharedString,
    StatefulInteractiveElement as _, Styled, Window, div, prelude::FluentBuilder as _, px,
};

use super::{
    button::Button,
    dialog::{self, DialogFooter, DialogSpec},
    scroll_area::{ScrollbarAxis, scrollbar_overlay},
};
use crate::ui::{
    icons,
    theme::{Tokens, TokensExt, TwText, alpha, radius},
};

/// One problem shown by [`open`].
#[derive(Clone)]
pub struct ErrorItem {
    title: SharedString,
    message: SharedString,
    details: Option<SharedString>,
}

impl ErrorItem {
    pub fn new(title: impl Into<SharedString>, message: impl Into<SharedString>) -> Self {
        Self { title: title.into(), message: message.into(), details: None }
    }

    /// Raw error text, shown verbatim in a monospace box (keeps line breaks and alignment, trailing whitespace dropped).
    pub fn details(mut self, details: impl Into<SharedString>) -> Self {
        let details: SharedString = details.into();
        let trimmed = details.trim_end();
        self.details = Some(if trimmed.len() == details.len() { details } else { trimmed.to_string().into() });
        self
    }
}

/// Opens the error dialog. Does nothing for an empty list.
pub fn open(window: &mut Window, cx: &mut App, items: Vec<ErrorItem>) {
    if items.is_empty() {
        return;
    }
    let items: Rc<[ErrorItem]> = items.into();
    // Owned by this dialog (not keyed window state), so stacked error dialogs never share a scroll position.
    let scroll = ScrollHandle::default();
    let spec =
        DialogSpec::new()
            .max_width(px(560.0))
            .show_close(false)
            .overlay_closes(false)
            .destructive(true)
            // Unpadded card: the scroll region spans the full card width so the scrollbar sits on the card edge
            // (see `scrollbar_overlay`). The card's usual 24px padding and 16px gap move into the scrolled body and
            // the footer.
            .padded(false)
            .content(move |_, cx| {
                // Not `ScrollArea`: its `size_full` viewport has no content height in this auto-height card. This box is
                // content-sized, shrinks (with the card's window-bounded max height) and scrolls, the footer stays put.
                div()
                    .relative()
                    .flex()
                    .flex_col()
                    .flex_shrink(1.0)
                    .min_h_0()
                    .child(
                        div()
                            .id("error-dialog-scroll")
                            .flex_shrink(1.0)
                            .min_h_0()
                            .overflow_y_scroll()
                            .track_scroll(&scroll)
                            .child(div().px_6().pt_6().pb_4().child(body(&items, cx.tokens()))),
                    )
                    .child(scrollbar_overlay("error-dialog-scrollbar", &scroll, ScrollbarAxis::Vertical))
            })
            .footer(|_, _| {
                DialogFooter::new().flex_none().px_6().pb_6().child(
                    Button::new("error-dialog-ok").label("OK").on_click(|_, window, cx| dialog::close(window, cx)),
                )
            });
    dialog::open(window, cx, spec);
}

fn body(items: &[ErrorItem], t: Tokens) -> AnyElement {
    let (title, message): (SharedString, Option<SharedString>) = match items {
        [only] => (only.title.clone(), Some(only.message.clone())),
        _ => (format!("{} problems need your attention", items.len()).into(), None),
    };
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
                .border_color(alpha(t.destructive, 0.25))
                .bg(alpha(t.destructive, 0.12))
                .text_color(t.destructive)
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
                .child(div().text_lg_tw().line_height(px(28.0)).font_weight(FontWeight::SEMIBOLD).child(title))
                .when_some(message, |d, m| d.child(div().text_sm_tw().text_color(t.muted_foreground).child(m))),
        );

    let rest: Vec<AnyElement> = match items {
        [only] => only.details.clone().map(|d| details_box(d, t).into_any_element()).into_iter().collect(),
        _ => items.iter().map(|item| item_block(item, t).into_any_element()).collect(),
    };
    div().flex().flex_col().gap_4().child(header).children(rest).into_any_element()
}

fn item_block(item: &ErrorItem, t: Tokens) -> impl IntoElement {
    // message and details line up with the headline text: 14px icon + 8px gap
    let indent = px(22.0);
    div()
        .flex()
        .flex_col()
        .gap_1()
        .pt_4()
        .border_t_1()
        .border_color(t.border)
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(Icon::new(icons::CIRCLE_ALERT).size(px(14.0)).text_color(t.destructive))
                .child(div().text_sm_tw().font_weight(FontWeight::MEDIUM).child(item.title.clone())),
        )
        .child(div().pl(indent).text_sm_tw().text_color(t.muted_foreground).child(item.message.clone()))
        .when_some(item.details.clone(), |d, details| d.child(div().pl(indent).pt_1().child(details_box(details, t))))
}

fn details_box(details: SharedString, t: Tokens) -> impl IntoElement {
    div()
        .rounded(radius::MD)
        .border_1()
        .border_color(alpha(t.destructive, 0.2))
        .bg(alpha(t.destructive, 0.06))
        .px_3()
        .py_2()
        .font_mono_tw()
        .text_xs_tw()
        .text_color(alpha(t.foreground, 0.85))
        .child(details)
}
