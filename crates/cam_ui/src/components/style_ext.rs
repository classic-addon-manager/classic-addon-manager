//! `merge_style`: apply a caller-supplied `StyleRefinement` (the `Styled` calls made on a widget) on top of the
//! widget's own base style.
//!
//! `StyleRefinement::refine` merges the `text` sub-style (font size, line height, weight, colour, family, underline)
//! field by field, so `Button::new(..).text_color(red)` keeps the button's small, medium-weight text.
//!
//! `.hover(|s| ..)` / `.focus(..)` closures build a fresh `StyleRefinement` that is refined onto the element's style by
//! gpui itself, which merges the text sub-style the same way.
use gpui_kit::{Refineable as _, StyleRefinement, Styled};

pub trait StyleMergeExt: Styled + Sized {
    fn merge_style(mut self, user: &StyleRefinement) -> Self {
        self.style().refine(user);
        self
    }
}

impl<T: Styled + Sized> StyleMergeExt for T {}
