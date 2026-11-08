//! Checkbox: a 16 px rounded square that shows a check mark when checked, with focus, invalid and disabled states.
//!
//! Value kept by the caller: `Checkbox::new("id").checked(self.flag).on_change(cx.listener(|this, v: &bool, _, cx| { this.flag = *v; cx.notify(); }))`.
//! Value kept by the widget: omit `.checked(..)` (optionally `.default_checked(true)`), the widget keeps its own state.
//! With a label: `.label("Accept")` renders the checkbox next to a small text label (clicking the label toggles).
use std::rc::Rc;

use gpui_kit::component::{IconNamed, StyledExt as _};
use gpui_kit::{
    AnyElement, App, ClickEvent, ElementId, InteractiveElement, IntoElement, KeyDownEvent, ParentElement, RenderOnce,
    SharedString, StatefulInteractiveElement, StyleRefinement, Styled, Window, div, prelude::FluentBuilder as _, px,
    svg,
};

use super::input::{focus_vis, track_focus_vis};
use crate::ui::{
    icons,
    theme::{TokensExt, TwText, alpha, radius, shadow_xs},
};

type ChangeFn = Rc<dyn Fn(&bool, &mut Window, &mut App) + 'static>;

#[derive(IntoElement)]
pub struct Checkbox {
    id: ElementId,
    checked: Option<bool>,
    default_checked: bool,
    disabled: bool,
    invalid: bool,
    label: Option<SharedString>,
    children: Vec<AnyElement>,
    on_change: Option<ChangeFn>,
    style: StyleRefinement,
}

impl Checkbox {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            checked: None,
            default_checked: false,
            disabled: false,
            invalid: false,
            label: None,
            children: Vec::new(),
            on_change: None,
            style: StyleRefinement::default(),
        }
    }

    /// Value kept by the caller. The widget does not flip by itself then: update your field in `on_change`.
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

    /// Marks the checkbox as invalid: the border is destructive-coloured.
    pub fn invalid(mut self, invalid: bool) -> Self {
        self.invalid = invalid;
        self
    }

    /// Text label to the right of the box. The whole row toggles on click.
    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Called with the NEW value. Works with `cx.listener(|this, v: &bool, window, cx| ..)`.
    pub fn on_change(mut self, f: impl Fn(&bool, &mut Window, &mut App) + 'static) -> Self {
        self.on_change = Some(Rc::new(f));
        self
    }
}

impl ParentElement for Checkbox {
    /// Children are rendered next to the box as the label (instead of `.label(..)`).
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Styled for Checkbox {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for Checkbox {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = cx.tokens();
        let state = window.use_keyed_state((self.id.clone(), "state"), cx, |_, _| self.default_checked);
        let checked = self.checked.unwrap_or_else(|| *state.read(cx));
        let controlled = self.checked.is_some();
        let vis = focus_vis((self.id.clone(), "focus"), window, cx);
        let ring = !self.disabled && vis.read(cx).visible(window);

        let (border, bg) = if checked { (t.primary, t.primary) } else { (t.input, alpha(t.input, 0.3)) };
        let (border, shadows) = if self.invalid {
            (t.destructive, shadow_xs())
        } else if ring {
            (t.ring, shadow_xs())
        } else {
            (border, shadow_xs())
        };

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

        let mut bx = div()
            .id((self.id.clone(), "box"))
            .flex()
            .flex_none()
            .items_center()
            .justify_center()
            .size(px(16.0))
            .rounded(radius::DEFAULT)
            .border_1()
            .border_color(border)
            .bg(bg)
            .text_color(t.primary_foreground)
            .shadow(shadows);
        if self.disabled {
            bx = bx.opacity(0.5).cursor_not_allowed();
        } else {
            bx = track_focus_vis(bx, &vis, window, cx).on_key_down({
                let toggle = toggle.clone();
                move |ev: &KeyDownEvent, window, cx| {
                    if ev.keystroke.key == "space" {
                        cx.stop_propagation();
                        toggle(window, cx);
                    }
                }
            });
        }
        let bx = bx.when(checked, |d| {
            d.child(svg().path(icons::CHECK.path()).size(px(14.0)).flex_none().text_color(t.primary_foreground))
        });

        let mut root = div().id(self.id.clone()).flex().items_center().gap(px(8.0)).refine_style(&self.style);
        if self.disabled {
            root = root.cursor_not_allowed();
        } else {
            root = root.on_click({
                let toggle = toggle.clone();
                move |_: &ClickEvent, window, cx| toggle(window, cx)
            });
        }
        root.child(bx)
            .when_some(self.label, |d, label| {
                d.child(
                    div()
                        .text_sm_tw()
                        .line_height(px(14.0))
                        .font_weight(gpui_kit::FontWeight::MEDIUM)
                        .opacity(if self.disabled { 0.5 } else { 1.0 })
                        .child(label),
                )
            })
            .children(self.children)
    }
}
