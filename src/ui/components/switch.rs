//! Switch: a 32 px wide pill with a 16 px thumb that slides over in 150 ms. The pill uses the primary colour when on,
//! a dimmed input colour when off, and shows a focus ring for keyboard focus.
//!
//! Value kept by the caller: `Switch::new("id").checked(self.on).on_change(cx.listener(|this, v: &bool, _, cx| { this.on = *v; cx.notify(); }))`.
//! Value kept by the widget: omit `.checked(..)` (optionally `.default_checked(true)`).
use std::{rc::Rc, time::Duration};

use gpui_kit::component::StyledExt as _;
use gpui_kit::{
    Animation, AnimationExt as _, App, ClickEvent, ElementId, InteractiveElement, IntoElement, KeyDownEvent,
    ParentElement, RenderOnce, StatefulInteractiveElement, StyleRefinement, Styled, Window, div, ease_in_out, px,
};

use super::input::{focus_vis, track_focus_vis};
use crate::ui::theme::{TokensExt, alpha, shadow_xs};

type ChangeFn = Rc<dyn Fn(&bool, &mut Window, &mut App) + 'static>;

/// Last value that was rendered + number of toggles (the thumb animation restarts on every toggle).
struct Anim {
    last: bool,
    toggles: usize,
}

#[derive(IntoElement)]
pub struct Switch {
    id: ElementId,
    checked: Option<bool>,
    default_checked: bool,
    disabled: bool,
    on_change: Option<ChangeFn>,
    style: StyleRefinement,
}

impl Switch {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            checked: None,
            default_checked: false,
            disabled: false,
            on_change: None,
            style: StyleRefinement::default(),
        }
    }

    /// Value kept by the caller (update your field in `on_change`).
    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = Some(checked);
        self
    }

    /// Initial value when the widget keeps its own state (no `.checked(..)`).
    pub fn default_checked(mut self, checked: bool) -> Self {
        self.default_checked = checked;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Called with the NEW value. Works with `cx.listener(|this, v: &bool, window, cx| ..)`.
    pub fn on_change(mut self, f: impl Fn(&bool, &mut Window, &mut App) + 'static) -> Self {
        self.on_change = Some(Rc::new(f));
        self
    }
}

impl Styled for Switch {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for Switch {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = cx.tokens();
        let state = window.use_keyed_state((self.id.clone(), "state"), cx, |_, _| self.default_checked);
        let checked = self.checked.unwrap_or_else(|| *state.read(cx));
        let controlled = self.checked.is_some();
        let vis = focus_vis((self.id.clone(), "focus"), window, cx);
        let ring = !self.disabled && vis.read(cx).visible(window);

        let anim = window.use_keyed_state((self.id.clone(), "anim"), cx, |_, _| Anim { last: checked, toggles: 0 });
        if anim.read(cx).last != checked {
            anim.update(cx, |a, _| {
                a.last = checked;
                a.toggles += 1;
            });
        }
        let toggles = anim.read(cx).toggles;

        let toggle = {
            let (state, on_change, disabled) = (state.clone(), self.on_change.clone(), self.disabled);
            Rc::new(move |window: &mut Window, cx: &mut App| {
                if disabled {
                    return;
                }
                let next = !checked;
                if !controlled {
                    state.update(cx, |s, cx| {
                        *s = next;
                        cx.notify();
                    });
                }
                if let Some(f) = &on_change {
                    f(&next, window, cx);
                }
            })
        };

        let track = if checked { t.primary } else { alpha(t.input, 0.8) };
        let thumb_color = if checked { t.primary_foreground } else { t.foreground };
        // The thumb moves 14px: the 16px thumb minus 2px.
        const TRAVEL: f32 = 14.0;

        let thumb = div().flex_none().size(px(16.0)).rounded_full().bg(thumb_color);
        let thumb = if toggles == 0 {
            thumb.ml(px(if checked { TRAVEL } else { 0.0 })).into_any_element()
        } else {
            thumb
                .with_animation(
                    (self.id.clone(), gpui_kit::SharedString::from(format!("thumb-{toggles}"))),
                    Animation::new(Duration::from_millis(150)).with_easing(ease_in_out),
                    move |el, delta| el.ml(px(if checked { TRAVEL * delta } else { TRAVEL * (1.0 - delta) })),
                )
                .into_any_element()
        };

        let mut root = div()
            .id(self.id.clone())
            .flex()
            .flex_none()
            .items_center()
            .w(px(32.0))
            .h(px(18.4))
            .rounded_full()
            .border_1()
            .border_color(if ring { t.ring } else { gpui_kit::transparent_black() })
            .bg(track)
            .shadow(shadow_xs());
        if self.disabled {
            root = root.opacity(0.5).cursor_not_allowed();
        } else {
            root = track_focus_vis(root, &vis, window, cx).on_key_down({
                let toggle = toggle.clone();
                move |ev: &KeyDownEvent, window, cx| {
                    if ev.keystroke.key == "space" || ev.keystroke.key == "enter" {
                        cx.stop_propagation();
                        toggle(window, cx);
                    }
                }
            });
            root = root.on_click({
                let toggle = toggle.clone();
                move |_: &ClickEvent, window, cx| toggle(window, cx)
            });
        }
        root.refine_style(&self.style).child(thumb)
    }
}
