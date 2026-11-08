//! `Label`: 14 px medium-weight text with a tight line height, laid out in a row with an 8 px gap between children.
use super::style_ext::StyleMergeExt as _;
use gpui_kit::component::StyledExt as _;
use gpui_kit::{
    AnyElement, App, FontWeight, IntoElement, ParentElement, RenderOnce, SharedString, StyleRefinement, Styled, Window,
    div, prelude::FluentBuilder as _, px, relative,
};

/// ```ignore
/// Label::new("Addon path")
/// Label::new("Disabled").disabled(true)          // drawn at half opacity
/// Label::empty().child(icon).child("Mixed")      // arbitrary children
/// ```
#[derive(IntoElement)]
pub struct Label {
    text: Option<SharedString>,
    disabled: bool,
    children: Vec<AnyElement>,
    style: StyleRefinement,
}

impl Label {
    pub fn new(text: impl Into<SharedString>) -> Self {
        Self { text: Some(text.into()), ..Self::empty() }
    }

    pub fn empty() -> Self {
        Self { text: None, disabled: false, children: Vec::new(), style: StyleRefinement::default() }
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl ParentElement for Label {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Styled for Label {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for Label {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        div()
            .flex()
            .items_center()
            .gap(px(8.0))
            .text_size(px(14.0))
            .line_height(relative(1.0))
            .font_weight(FontWeight::MEDIUM)
            .when(self.disabled, |d| d.opacity(0.5))
            .merge_style(&self.style)
            .children(self.text)
            .children(self.children)
    }
}
