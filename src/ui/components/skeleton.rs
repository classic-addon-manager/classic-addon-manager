//! `Skeleton`: a pulsing, rounded placeholder block shown while content is loading.
use std::time::Duration;

use super::style_ext::StyleMergeExt as _;
use gpui_kit::component::StyledExt as _;
use gpui_kit::{
    Animation, AnimationExt as _, App, IntoElement, RenderOnce, StyleRefinement, Styled, Window, div, ease_in_out,
};

use crate::ui::theme::{TokensExt, radius};

/// Placeholder block. Give it a size with the `Styled` methods: `Skeleton::new().h_4().w(relative(0.8))`,
/// `Skeleton::new().size_10().rounded(radius::LG)`. Its opacity pulses 1 -> 0.5 -> 1 every 2 seconds.
#[derive(IntoElement)]
pub struct Skeleton {
    style: StyleRefinement,
}

impl Skeleton {
    pub fn new() -> Self {
        Self { style: StyleRefinement::default() }
    }
}

impl Default for Skeleton {
    fn default() -> Self {
        Self::new()
    }
}

impl Styled for Skeleton {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for Skeleton {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        div().bg(cx.tokens().accent).rounded(radius::MD).merge_style(&self.style).with_animation(
            "skeleton-pulse",
            Animation::new(Duration::from_millis(2000)).repeat(),
            |d, delta| {
                // Opacity falls to 0.5 at the half-way point and back, with ease-in-out on each half.
                let x = if delta < 0.5 { delta * 2.0 } else { (1.0 - delta) * 2.0 };
                d.opacity(1.0 - 0.5 * ease_in_out(x))
            },
        )
    }
}
