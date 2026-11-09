//! `Card` and its parts: `CardHeader`, `CardTitle`, `CardDescription`, `CardAction`, `CardContent`, `CardFooter`.
use gpui_kit::{
    AnyElement, App, FontWeight, IntoElement, ParentElement, RenderOnce, SharedString, StyleRefinement, Styled, Window,
    div, prelude::FluentBuilder as _, relative,
};

use super::style_ext::StyleMergeExt as _;
use crate::theme::{TokensExt, TwText, radius, shadow_sm, sp};

/// Aligns `el` to the top of its row inside a flex container (gpui has no helper): `align_self_start(el)`.
pub fn align_self_start<T: Styled>(mut el: T) -> T {
    el.style().align_self = Some(gpui_kit::AlignItems::FlexStart);
    el
}

macro_rules! div_like {
    ($name:ident) => {
        impl ParentElement for $name {
            fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
                self.children.extend(elements);
            }
        }
        impl Styled for $name {
            fn style(&mut self) -> &mut StyleRefinement {
                &mut self.style
            }
        }
    };
}

/// Rounded bordered panel with the card colours, a small shadow and 24 px vertical padding.
///
/// ```ignore
/// Card::new().w(px(380.)).child(
///     CardHeader::new()
///         .child(CardTitle::new("Title")).child(CardDescription::new("Description"))
///         .action(Button::new("a").size(ButtonSize::Sm).label("Act")),
/// ).child(CardContent::new().child("body")).child(CardFooter::new().child("footer"))
/// ```
/// Children are stacked vertically with a 24 px gap. Override with `.p_0()` / `.gap_0()` and other `Styled` methods.
#[derive(IntoElement)]
pub struct Card {
    children: Vec<AnyElement>,
    style: StyleRefinement,
}

impl Card {
    pub fn new() -> Self {
        Self { children: Vec::new(), style: StyleRefinement::default() }
    }
}

impl Default for Card {
    fn default() -> Self {
        Self::new()
    }
}
div_like!(Card);

impl RenderOnce for Card {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = cx.tokens();
        div()
            .flex()
            .flex_col()
            .gap(sp(6.0))
            .rounded(radius::XL)
            .border_1()
            .border_color(t.border)
            .bg(t.card)
            .text_color(t.card_foreground)
            .py(sp(6.0))
            .shadow(shadow_sm())
            .merge_style(&self.style)
            .children(self.children)
    }
}

/// Header with 24 px horizontal padding and a 6 px gap. Title and description are stacked, an optional `.action(..)`
/// sits on the right, and `.bordered(true)` adds a bottom border with 24 px of padding below it.
#[derive(IntoElement)]
pub struct CardHeader {
    children: Vec<AnyElement>,
    action: Option<AnyElement>,
    bordered: bool,
    style: StyleRefinement,
}

impl CardHeader {
    pub fn new() -> Self {
        Self { children: Vec::new(), action: None, bordered: false, style: StyleRefinement::default() }
    }

    /// Placed at the top-right of the header, next to the stacked title and description.
    pub fn action(mut self, action: impl IntoElement) -> Self {
        self.action = Some(action.into_any_element());
        self
    }

    /// Draws a bottom border under the header and adds 24 px of padding above it.
    pub fn bordered(mut self, bordered: bool) -> Self {
        self.bordered = bordered;
        self
    }
}

impl Default for CardHeader {
    fn default() -> Self {
        Self::new()
    }
}
div_like!(CardHeader);

impl RenderOnce for CardHeader {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = cx.tokens();
        div()
            .flex()
            .items_start()
            .gap(sp(1.5))
            .px(sp(6.0))
            .when(self.bordered, |d| d.border_b_1().border_color(t.border).pb(sp(6.0)))
            .merge_style(&self.style)
            .child(div().flex().flex_col().flex_1().min_w_0().gap(sp(1.5)).children(self.children))
            .children(self.action.map(|a| align_self_start(div().flex_none().child(a))))
    }
}

/// Tight line height and semibold weight.
#[derive(IntoElement)]
pub struct CardTitle {
    text: Option<SharedString>,
    children: Vec<AnyElement>,
    style: StyleRefinement,
}

impl CardTitle {
    pub fn new(text: impl Into<SharedString>) -> Self {
        Self { text: Some(text.into()), children: Vec::new(), style: StyleRefinement::default() }
    }
}
div_like!(CardTitle);

impl RenderOnce for CardTitle {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        div()
            .line_height(relative(1.0))
            .font_weight(FontWeight::SEMIBOLD)
            .merge_style(&self.style)
            .children(self.text)
            .children(self.children)
    }
}

/// Small text in the muted colour.
#[derive(IntoElement)]
pub struct CardDescription {
    text: Option<SharedString>,
    children: Vec<AnyElement>,
    style: StyleRefinement,
}

impl CardDescription {
    pub fn new(text: impl Into<SharedString>) -> Self {
        Self { text: Some(text.into()), children: Vec::new(), style: StyleRefinement::default() }
    }
}
div_like!(CardDescription);

impl RenderOnce for CardDescription {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        div()
            .text_sm_tw()
            .text_color(cx.tokens().muted_foreground)
            .merge_style(&self.style)
            .children(self.text)
            .children(self.children)
    }
}

/// Content aligned to the top of its row: pass to `CardHeader::action(..)`.
#[derive(IntoElement)]
pub struct CardAction {
    children: Vec<AnyElement>,
    style: StyleRefinement,
}

impl CardAction {
    pub fn new() -> Self {
        Self { children: Vec::new(), style: StyleRefinement::default() }
    }
}

impl Default for CardAction {
    fn default() -> Self {
        Self::new()
    }
}
div_like!(CardAction);

impl RenderOnce for CardAction {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        align_self_start(div()).merge_style(&self.style).children(self.children)
    }
}

/// Body with 24 px horizontal padding.
#[derive(IntoElement)]
pub struct CardContent {
    children: Vec<AnyElement>,
    style: StyleRefinement,
}

impl CardContent {
    pub fn new() -> Self {
        Self { children: Vec::new(), style: StyleRefinement::default() }
    }
}

impl Default for CardContent {
    fn default() -> Self {
        Self::new()
    }
}
div_like!(CardContent);

impl RenderOnce for CardContent {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        div().px(sp(6.0)).merge_style(&self.style).children(self.children)
    }
}

/// Horizontally padded (24 px) row with vertically centred children. `.bordered(true)` adds a top border and 24 px
/// of padding below it.
#[derive(IntoElement)]
pub struct CardFooter {
    bordered: bool,
    children: Vec<AnyElement>,
    style: StyleRefinement,
}

impl CardFooter {
    pub fn new() -> Self {
        Self { bordered: false, children: Vec::new(), style: StyleRefinement::default() }
    }

    pub fn bordered(mut self, bordered: bool) -> Self {
        self.bordered = bordered;
        self
    }
}

impl Default for CardFooter {
    fn default() -> Self {
        Self::new()
    }
}
div_like!(CardFooter);

impl RenderOnce for CardFooter {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = cx.tokens();
        div()
            .flex()
            .items_center()
            .px(sp(6.0))
            .when(self.bordered, |d| d.border_t_1().border_color(t.border).pt(sp(6.0)))
            .merge_style(&self.style)
            .children(self.children)
    }
}
