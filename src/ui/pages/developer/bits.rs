//! Small shared pieces: the review status chip, label/value detail rows and the loading block repeated across the page.
use gpui_kit::component::Sizable as _;
use gpui_kit::{
    AnyElement, App, FontWeight, InteractiveElement, IntoElement, ParentElement, RenderOnce, SharedString,
    StatefulInteractiveElement, Styled, Window, div, prelude::FluentBuilder as _, px,
};

use super::owned::ReviewHistoryTone;
use crate::ui::{
    components::{Badge, BadgeVariant, spinner},
    icons,
    theme::{TokensExt, Tw, TwText, alpha, tw},
};

/// One chip per review outcome: colour, icon and word always agree.
#[derive(IntoElement)]
pub struct StatusChip {
    tone: ReviewHistoryTone,
    label: SharedString,
}

impl StatusChip {
    pub fn new(tone: ReviewHistoryTone, label: impl Into<SharedString>) -> Self {
        Self { tone, label: label.into() }
    }
}

impl RenderOnce for StatusChip {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = cx.tokens();
        let (icon, fg, bg, border, dashed) = match self.tone {
            ReviewHistoryTone::Review => {
                (icons::CLOCK, t.primary, alpha(t.primary, 0.15), alpha(t.primary, 0.6), false)
            }
            ReviewHistoryTone::Approved => (
                icons::CIRCLE_CHECK,
                tw(Tw::Emerald, 300),
                alpha(tw(Tw::Emerald, 500), 0.12),
                alpha(tw(Tw::Emerald, 500), 0.4),
                false,
            ),
            ReviewHistoryTone::Rejected => {
                (icons::CIRCLE_X, t.destructive, alpha(t.destructive, 0.1), alpha(t.destructive, 0.4), false)
            }
            // The outline *is* the signal here, so it is stronger than a filled chip's border.
            ReviewHistoryTone::Unknown => (
                icons::CIRCLE_DASHED,
                t.muted_foreground,
                gpui_kit::transparent_black(),
                alpha(t.muted_foreground, 0.4),
                true,
            ),
        };
        Badge::new(self.label)
            .variant(BadgeVariant::Outline)
            .icon(icon)
            .rounded_full()
            .py(px(2.0))
            .text_color(fg)
            .bg(bg)
            .border_color(border)
            .when(dashed, |b| b.border_dashed())
    }
}

/// A label on the left and a value on the right (a link-styled button when `href` is set and the value is not empty).
#[derive(IntoElement)]
pub struct DetailField {
    label: SharedString,
    value: SharedString,
    href: Option<String>,
}

impl DetailField {
    pub fn new(label: impl Into<SharedString>, value: impl Into<SharedString>) -> Self {
        Self { label: label.into(), value: value.into(), href: None }
    }

    pub fn href(mut self, href: Option<String>) -> Self {
        self.href = href;
        self
    }
}

impl RenderOnce for DetailField {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = cx.tokens();
        let value: AnyElement = match (self.value.is_empty(), self.href) {
            (false, Some(href)) => {
                let primary = t.primary;
                div()
                    .id(SharedString::from(format!("detail-link-{}", self.label)))
                    .cursor_pointer()
                    .text_color(primary)
                    .child(self.value)
                    .hover(|s| s.text_sm_tw().underline())
                    .on_click(move |_, _, cx| cx.open_url(&href))
                    .into_any_element()
            }
            (true, _) => div().child("None").into_any_element(),
            (false, None) => div().child(self.value).into_any_element(),
        };
        div()
            .flex()
            .flex_wrap()
            .justify_between()
            .gap_x(px(16.0))
            .gap_y(px(4.0))
            .py(px(12.0))
            .child(div().text_color(t.muted_foreground).child(self.label))
            .child(div().min_w_0().child(value))
    }
}

/// A bordered, rounded list of detail rows with thin dividers between them.
pub fn detail_list(rows: Vec<DetailField>, cx: &App) -> impl IntoElement {
    let t = cx.tokens();
    div()
        .flex()
        .flex_col()
        .rounded(crate::ui::theme::radius::XL)
        .border_1()
        .border_color(t.border)
        .bg(alpha(t.card, 0.4))
        .px(px(16.0))
        .text_sm_tw()
        .children(
            rows.into_iter()
                .enumerate()
                .map(|(i, row)| div().when(i > 0, |d| d.border_t_1().border_color(t.border)).child(row)),
        )
}

/// Centered 32 px spinner at half opacity with a caption below it.
pub fn loading_block(caption: &str, cx: &App) -> gpui_kit::Div {
    let t = cx.tokens();
    div()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .text_color(t.muted_foreground)
        .child(div().opacity(0.5).child(spinner().with_size(px(32.0))))
        .child(div().mt(px(12.0)).text_sm_tw().child(caption.to_string()))
}

/// Section heading used all over the detail panes (small text, medium weight).
pub fn h3(text: impl Into<SharedString>) -> gpui_kit::Div {
    div().text_sm_tw().font_weight(FontWeight::MEDIUM).child(text.into())
}
