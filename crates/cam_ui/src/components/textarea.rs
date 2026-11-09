//! Multi-line text `Textarea`: at least 64 px tall, grows with its content, and has the same focus, invalid and
//! disabled states as [`Input`](super::Input).
//!
//! ```ignore
//! self.desc = text_area(window, cx, "Describe your addon...");      // once
//! Textarea::new(&self.desc).invalid(self.errors.desc)                  // every render
//! ```
pub use gpui_kit::component::input::TextareaState;
use gpui_kit::component::{StyledExt as _, input::Textarea as RawTextarea};
use gpui_kit::{
    App, AppContext as _, Entity, Focusable, IntoElement, Refineable as _, RenderOnce, SharedString, StyleRefinement,
    Styled, Window, px,
};

use super::input::field_style;
use crate::theme::TokensExt;

/// Creates a multi-line state that starts at 2 rows (64 px) and grows with its content up to 12 rows, then scrolls.
pub fn text_area(window: &mut Window, cx: &mut App, placeholder: impl Into<SharedString>) -> Entity<TextareaState> {
    text_area_rows(window, cx, placeholder, 2, 12)
}

/// [`text_area`] with explicit minimum / maximum visible rows.
pub fn text_area_rows(
    window: &mut Window,
    cx: &mut App,
    placeholder: impl Into<SharedString>,
    min_rows: usize,
    max_rows: usize,
) -> Entity<TextareaState> {
    let placeholder = placeholder.into();
    cx.new(|cx| TextareaState::new(window, cx).auto_grow(min_rows, max_rows).placeholder(placeholder))
}

/// Draws a multi-line [`TextareaState`] created with [`text_area`]. Read the text with `state.read(cx).value()`;
/// subscribe with [`on_input_change`](super::on_input_change). Implements [`Styled`] (`.h(px(160.))` for a fixed
/// height makes it scroll instead of grow; `.w(..)`, `.mt_2()`, `.font_mono_tw()`...).
#[derive(IntoElement)]
pub struct Textarea {
    state: Entity<TextareaState>,
    style: StyleRefinement,
    disabled: bool,
    invalid: bool,
}

impl Textarea {
    pub fn new(state: &Entity<TextareaState>) -> Self {
        Self { state: state.clone(), style: StyleRefinement::default(), disabled: false, invalid: false }
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Marks the field as invalid: the border is destructive-coloured, also while focused.
    pub fn invalid(mut self, invalid: bool) -> Self {
        self.invalid = invalid;
        self
    }
}

impl Styled for Textarea {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for Textarea {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = cx.tokens();
        let focused = self.state.read(cx).focus_handle(cx).is_focused(window);
        // gpui-component pads the multi-line editor itself (10px x / 8px y, set by `Input` from its size) and does
        // not pad the frame, so the frame only adds the 2px missing to the 12px horizontal padding of the field.
        // Padding the frame by the full 12px x / 8px y would double it. `appearance(false)`: `field_style`
        // draws the whole chrome (Textarea has no `focus_bordered`).
        let mut base = field_style(&t, focused, self.invalid, self.disabled).min_h(px(64.0)).px(px(2.0));
        base.refine(&self.style);
        RawTextarea::new(&self.state).appearance(false).disabled(self.disabled).refine_style(&base)
    }
}
