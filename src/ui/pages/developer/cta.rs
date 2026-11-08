//! The validate, then submit button shared by `AddonEditPanel` and `PublishAddonForm`. Once validated it turns emerald
//! and plays a short ring pulse (`publish-cta-ring`).
use std::time::Duration;

use gpui_kit::{
    Animation, AnimationExt, App, InteractiveElement, IntoElement, ParentElement, Pixels, SharedString, Styled, Window,
    div, ease_out_quint, prelude::FluentBuilder as _,
};

use crate::ui::{
    components::{Button, ButtonVariant},
    icons,
    theme::{Tw, alpha, box_shadow, radius, tw},
};

pub struct CtaState {
    pub validated: bool,
    pub validating: bool,
    pub submitting: bool,
    pub disabled: bool,
    /// Label while submitting / once validated.
    pub submit_label: SharedString,
    pub width: Option<Pixels>,
}

/// Renders the button. `on_click` runs the primary action (validate, then submit once validated).
pub fn cta_button(
    id: &'static str,
    state: CtaState,
    on_click: impl Fn(&mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let CtaState { validated, validating, submitting, disabled, submit_label, width } = state;
    let mut button = Button::new(id).disabled(disabled).on_click(move |_, window, cx| on_click(window, cx));
    button = if submitting {
        button.loading(true).label(submit_label)
    } else if validating {
        button.loading(true).label("Validate")
    } else if validated {
        button.icon(icons::CHECK).label(submit_label)
    } else {
        button.label("Validate")
    };
    if validated {
        button = button.variant(ButtonVariant::Default).bg(tw(Tw::Emerald, 600)).text_color(tw(Tw::Emerald, 50));
    }
    if let Some(w) = width {
        button = button.w(w);
    }
    div()
        .relative()
        .when(validated, |d| {
            d.child(div().absolute().top_0().left_0().right_0().bottom_0().rounded(radius::MD).with_animation(
                "publish-cta-ring",
                Animation::new(Duration::from_millis(600)).with_easing(ease_out_quint()),
                |el, t| {
                    let a = 0.55 * (1.0 - t);
                    el.opacity(a).shadow(vec![box_shadow(alpha(tw(Tw::Emerald, 500), a), 0.0, 0.0, 12.0 * t)])
                },
            ))
        })
        .child(button)
}
