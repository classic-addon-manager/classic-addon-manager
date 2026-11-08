//! `Section`: a titled block whose children sit in a bordered, rounded panel.
use super::style_ext::StyleMergeExt as _;
use gpui_kit::component::StyledExt as _;
use gpui_kit::{
    AnyElement, App, FontWeight, IntoElement, ParentElement, RenderOnce, SharedString, StyleRefinement, Styled, Window,
    div, px,
};

use crate::ui::theme::{TokensExt, TwText, alpha, radius};

/// A heading (title plus optional smaller description in the muted colour) above a rounded, bordered, translucent
/// panel that holds the children, with a 12 px gap between them.
///
/// ```ignore
/// Section::new("Appearance").description("Pick an accent colour").child(div().p_4().child("..."))
/// ```
#[derive(IntoElement)]
pub struct Section {
    title: SharedString,
    description: Option<SharedString>,
    children: Vec<AnyElement>,
    style: StyleRefinement,
}

impl Section {
    pub fn new(title: impl Into<SharedString>) -> Self {
        Self { title: title.into(), description: None, children: Vec::new(), style: StyleRefinement::default() }
    }

    pub fn description(mut self, description: impl Into<SharedString>) -> Self {
        self.description = Some(description.into());
        self
    }
}

impl ParentElement for Section {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Styled for Section {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for Section {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = cx.tokens();
        div()
            .flex()
            .flex_col()
            .gap(px(12.0))
            .merge_style(&self.style)
            .child(
                div().px(px(4.0)).child(div().text_sm_tw().font_weight(FontWeight::MEDIUM).child(self.title)).children(
                    self.description.map(|d| div().mt(px(2.0)).text_xs_tw().text_color(t.muted_foreground).child(d)),
                ),
            )
            .child(
                div()
                    .overflow_hidden()
                    .rounded(radius::XL)
                    .border_1()
                    .border_color(alpha(t.border, 0.6))
                    .bg(alpha(t.card, 0.4))
                    .children(self.children),
            )
    }
}
