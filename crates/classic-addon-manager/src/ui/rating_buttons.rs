//! `RatingButtons`: thumbs up / down with tooltips, a ring around the selected one and the emoji burst that plays
//! after a successful save. Pair it with `state::ratings::RatingModel`.
//!
//! ```ignore
//! let m = self.rating.read(cx);
//! RatingButtons::new("rating-btns").rating(m.rating(cx).into()).disabled(m.busy(cx)).size(RatingSize::Sm)
//!     .on_rate({ let model = self.rating.clone(); move |value, _w, cx| model.update(cx, |m, cx| m.rate(value, cx)) })
//! ```
//! `on_rate(value, window, cx)` returns a task that resolves to true once the vote was saved, the only case where the
//! burst plays. Signed-out users get the greyed-out icons with the "Sign in using Discord to rate addons" tooltip.
use std::{rc::Rc, time::Duration};

use cam_ui::components::tooltip::Tooltip;
use cam_ui::{
    icons::{self, Lucide},
    theme::{TokensExt, alpha, radius},
};
use gpui_kit::component::Icon;
use gpui_kit::{
    Animation, AnimationExt as _, App, ElementId, FontWeight, InteractiveElement as _, IntoElement, ParentElement,
    RenderOnce, StatefulInteractiveElement as _, Styled, Task, Window, div, prelude::FluentBuilder as _, px,
};

use crate::state;

type RateHandler = Rc<dyn Fn(i8, &mut Window, &mut App) -> Task<bool>>;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RatingSize {
    /// 28x28 px buttons, 16px icons, 12px gap when signed out
    Sm,
    /// 36x32 px buttons, 20px icons
    #[default]
    Md,
}

const HAPPY: [&str; 5] = ["🎉", "✨", "🥳", "😊", "💖"];
const SAD: [&str; 5] = ["😢", "💔", "😞", "🥺", "😭"];
const BURST_MS: u64 = 1600;

#[derive(Clone)]
struct Particle {
    emoji: &'static str,
    x: f32,
    y: f32,
}

#[derive(Default)]
struct Burst {
    key: u64,
    /// value of the button that was rated (1 or -1)
    value: i8,
    particles: Vec<Particle>,
}

/// Cheap non-cryptographic randomness in `0.0..1.0`, no extra dependency.
fn next_random(seed: &mut u64) -> f32 {
    *seed ^= *seed << 13;
    *seed ^= *seed >> 7;
    *seed ^= *seed << 17;
    (*seed >> 40) as f32 / (1u64 << 24) as f32
}

fn create_particles(value: i8) -> Vec<Particle> {
    let emojis = if value == 1 { HAPPY } else { SAD };
    let mut seed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0x9E37_79B9_7F4A_7C15)
        | 1;
    emojis
        .iter()
        .enumerate()
        .map(|(i, emoji)| {
            let angle = (i as f32 / (emojis.len() - 1) as f32 - 0.5) * std::f32::consts::PI * 0.8;
            let dist = 50.0 + next_random(&mut seed) * 40.0;
            Particle { emoji, x: angle.sin() * dist, y: -angle.cos() * dist }
        })
        .collect()
}

#[derive(IntoElement)]
pub struct RatingButtons {
    id: ElementId,
    rating: i32,
    disabled: bool,
    size: RatingSize,
    on_rate: Option<RateHandler>,
}

impl RatingButtons {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self { id: id.into(), rating: 0, disabled: false, size: RatingSize::Md, on_rate: None }
    }

    /// Current rating: -1, 0 or 1.
    pub fn rating(mut self, rating: i32) -> Self {
        self.rating = rating;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn size(mut self, size: RatingSize) -> Self {
        self.size = size;
        self
    }

    pub fn on_rate(mut self, handler: impl Fn(i8, &mut Window, &mut App) -> Task<bool> + 'static) -> Self {
        self.on_rate = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for RatingButtons {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = cx.tokens();
        let (button_w, button_h, icon_px, gap) = match self.size {
            RatingSize::Sm => (28.0, 28.0, 16.0, 12.0),
            RatingSize::Md => (36.0, 32.0, 20.0, 4.0),
        };

        if !state::user(cx).read(cx).is_authenticated() {
            return Tooltip::new("Sign in using Discord to rate addons")
                .id((self.id.clone(), "signed-out"))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(gap))
                        .p_2()
                        .opacity(0.5)
                        .cursor_not_allowed()
                        .child(Icon::new(icons::THUMBS_UP).size(px(icon_px)).text_color(t.muted_foreground))
                        .child(Icon::new(icons::THUMBS_DOWN).size(px(icon_px)).text_color(t.muted_foreground)),
                )
                .into_any_element();
        }

        let burst = window.use_keyed_state((self.id.clone(), "burst"), cx, |_, _| Burst::default());
        let (burst_key, burst_value, particles) = {
            let b = burst.read(cx);
            (b.key, b.value, b.particles.clone())
        };

        let options: [(i8, Lucide, &str, &str, gpui_kit::Hsla); 2] = [
            (1, icons::THUMBS_UP, "Like", "Liked", t.primary),
            (-1, icons::THUMBS_DOWN, "Dislike", "Disliked", t.destructive),
        ];
        let row = div().flex().items_center().gap_1().relative().children(options.into_iter().map(
            |(value, icon, tip, tip_on, tone)| {
                let selected = self.rating == value as i32;
                let on_rate = self.on_rate.clone();
                let burst = burst.clone();
                let disabled = self.disabled;
                let button = div()
                    .id((self.id.clone(), format!("btn{value}")))
                    .flex()
                    .items_center()
                    .justify_center()
                    .w(px(button_w))
                    .h(px(button_h))
                    .rounded(radius::MD)
                    .border_1()
                    .border_color(if selected { alpha(tone, 0.4) } else { gpui_kit::transparent_black() })
                    .when(selected, |d| d.bg(alpha(tone, 0.1)))
                    .when(!disabled, |d| d.cursor_pointer().hover(move |s| s.bg(alpha(tone, 0.1))))
                    .when(disabled, |d| d.opacity(0.5))
                    .child(Icon::new(icon).size(px(icon_px)).text_color(if selected {
                        tone
                    } else {
                        t.muted_foreground
                    }))
                    .on_click(move |_, window, cx| {
                        if disabled {
                            return;
                        }
                        let Some(on_rate) = &on_rate else { return };
                        let saved = on_rate(value, window, cx);
                        let burst = burst.clone();
                        cx.spawn(async move |cx| {
                            if saved.await {
                                burst.update(cx, |b, cx| {
                                    b.key += 1;
                                    b.value = value;
                                    b.particles = create_particles(value);
                                    cx.notify();
                                });
                            }
                        })
                        .detach();
                    });
                let label = if selected { tip_on } else { tip };
                let wrapper = div()
                    .relative()
                    .child(Tooltip::new(label).id((self.id.clone(), format!("tip{value}"))).child(button));
                // the emoji burst is drawn on the clicked button, above everything
                wrapper.when(burst_key > 0 && burst_value == value, |d| {
                    d.children(particles.iter().enumerate().map(|(i, p)| {
                        let (px_x, px_y) = (p.x, p.y);
                        div()
                            .absolute()
                            .left(px(button_w / 2.0 - 9.0))
                            .top(px(button_h / 2.0 - 14.0))
                            .text_size(px(18.0))
                            .line_height(px(28.0))
                            .font_weight(FontWeight::NORMAL)
                            .child(p.emoji)
                            .with_animation(
                                (self.id.clone(), format!("particle{burst_key}-{i}")),
                                Animation::new(Duration::from_millis(BURST_MS))
                                    .with_easing(|t| 1.0 - (1.0 - t) * (1.0 - t)),
                                move |el, t| {
                                    // Fully opaque for the first two thirds of the duration, then fades out.
                                    let opacity = if t < 2.0 / 3.0 { 1.0 } else { 1.0 - (t - 2.0 / 3.0) * 3.0 };
                                    el.left(px(button_w / 2.0 - 9.0 + px_x * t))
                                        .top(px(button_h / 2.0 - 14.0 + px_y * t))
                                        .opacity(opacity.clamp(0.0, 1.0))
                                },
                            )
                    }))
                })
            },
        ));
        row.into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn particles_fan_upwards() {
        let p = create_particles(1);
        assert_eq!(p.len(), 5);
        assert!(p.iter().all(|p| p.y < 0.0));
        assert!(p[0].x < 0.0 && p[4].x > 0.0);
        assert_eq!(create_particles(-1)[0].emoji, "😢");
    }
}
