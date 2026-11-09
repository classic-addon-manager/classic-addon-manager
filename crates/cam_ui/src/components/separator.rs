//! `Separator`: a 1 px line in the border colour, either horizontal (full width) or vertical (full height).
use gpui_kit::{App, IntoElement, RenderOnce, StyleRefinement, Styled, Window, div, prelude::FluentBuilder as _, px};

use super::style_ext::StyleMergeExt as _;
use crate::theme::TokensExt;

/// `Separator::horizontal()` (default) / `Separator::vertical()`; vertical needs a parent with a definite height
/// (e.g. `h_flex` + `.h_4()` on the separator).
#[derive(IntoElement)]
pub struct Separator {
    vertical: bool,
    style: StyleRefinement,
}

impl Separator {
    pub fn horizontal() -> Self {
        Self { vertical: false, style: StyleRefinement::default() }
    }

    pub fn vertical() -> Self {
        Self { vertical: true, style: StyleRefinement::default() }
    }
}

impl Styled for Separator {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for Separator {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        div()
            .flex_shrink_0()
            .bg(cx.tokens().border)
            .when(!self.vertical, |d| d.h(px(1.0)).w_full())
            .when(self.vertical, |d| d.w(px(1.0)).h_full())
            .merge_style(&self.style)
    }
}
