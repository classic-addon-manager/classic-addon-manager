//! Fade animations for view changes, list items and empty states.
use std::time::Duration;

use gpui_kit::{Animation, AnimationExt, ElementId, IntoElement, Styled, ease_out_quint, px};

pub const VIEW_FADE_DURATION: Duration = Duration::from_millis(250);
pub const VIEW_SLIDE_DURATION: Duration = Duration::from_millis(200);

/// Fades from transparent to opaque over `duration` (views use [`VIEW_FADE_DURATION`]). The animation restarts
/// whenever `key` changes.
pub fn fade_in<E: Styled + IntoElement + 'static>(
    el: E,
    key: impl Into<ElementId>,
    duration: Duration,
) -> impl IntoElement {
    el.with_animation(key, Animation::new(duration).with_easing(ease_out_quint()), |el, t| el.opacity(t))
}

/// Fades from transparent to opaque while sliding 4 px up over 200 ms (list items, empty states).
pub fn fade_slide_in<E: Styled + IntoElement + 'static>(el: E, key: impl Into<ElementId>) -> impl IntoElement {
    el.with_animation(key, Animation::new(VIEW_SLIDE_DURATION).with_easing(ease_out_quint()), |el, t| {
        el.opacity(t).mt(px(4.0 * (1.0 - t)))
    })
}
