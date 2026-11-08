//! `Progress`: an 8 px tall rounded bar that is filled in proportion to a value from 0 to 100.
use super::style_ext::StyleMergeExt as _;
use gpui_kit::component::StyledExt as _;
use gpui_kit::{App, IntoElement, ParentElement, RenderOnce, StyleRefinement, Styled, Window, div, px, relative};

use crate::ui::theme::{TokensExt, alpha};

/// `Progress::new(42.0)` (value 0..=100). `Styled` overrides the track (`.h(px(4.))`, `.w(px(200.))`).
/// The filled part is flat on its right edge below 100 and fully round at 100.
#[derive(IntoElement)]
pub struct Progress {
    value: f32,
    style: StyleRefinement,
}

impl Progress {
    pub fn new(value: f32) -> Self {
        Self { value, style: StyleRefinement::default() }
    }

    pub fn value(mut self, value: f32) -> Self {
        self.value = value;
        self
    }
}

impl Styled for Progress {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for Progress {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = cx.tokens();
        let frac = (self.value / 100.0).clamp(0.0, 1.0);
        let mut bar = div().h_full().w(relative(frac)).bg(t.primary);
        if frac >= 1.0 {
            bar = bar.rounded_full();
        } else {
            bar = bar.rounded_l_full();
        }
        div()
            .relative()
            .h(px(8.0))
            .w_full()
            .overflow_hidden()
            .rounded_full()
            .bg(alpha(t.primary, 0.2))
            .merge_style(&self.style)
            .child(bar)
    }
}
