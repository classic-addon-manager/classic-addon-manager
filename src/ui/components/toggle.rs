//! `Toggle`: a two-state button that is highlighted while pressed.
use std::rc::Rc;

use super::style_ext::StyleMergeExt as _;
use gpui_kit::component::{Icon, IconNamed, StyledExt as _};
use gpui_kit::{
    AnyElement, App, Div, ElementId, FontWeight, InteractiveElement, IntoElement, ParentElement, RenderOnce,
    SharedString, Stateful, StatefulInteractiveElement, StyleRefinement, Styled, Window, div,
    prelude::FluentBuilder as _, px,
};

use crate::ui::theme::{Tokens, TokensExt, TwText, radius, shadow_xs};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ToggleVariant {
    /// No border or fill until hovered or pressed.
    #[default]
    Default,
    /// Bordered, with a small shadow, and filled with the accent colour on hover.
    Outline,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ToggleSize {
    /// 36 px tall, at least 36 px wide, 8 px horizontal padding.
    #[default]
    Default,
    /// 32 px tall, at least 32 px wide, 6 px horizontal padding.
    Sm,
    /// 40 px tall, at least 40 px wide, 10 px horizontal padding.
    Lg,
}

/// Shared look of `Toggle` and `ToggleGroupItem`: returns the styled root element (with its id) but no content.
pub(crate) fn toggle_root(
    id: ElementId,
    variant: ToggleVariant,
    size: ToggleSize,
    on: bool,
    disabled: bool,
    t: &Tokens,
) -> Stateful<Div> {
    let (h, px_x) = match size {
        ToggleSize::Default => (36.0, 8.0),
        ToggleSize::Sm => (32.0, 6.0),
        ToggleSize::Lg => (40.0, 10.0),
    };
    let (accent, accent_fg, muted, muted_fg) = (t.accent, t.accent_foreground, t.muted, t.muted_foreground);
    let outline = variant == ToggleVariant::Outline;
    div()
        .id(id)
        .flex()
        .flex_shrink_0()
        .items_center()
        .justify_center()
        .gap(px(8.0))
        .h(px(h))
        .min_w(px(h))
        .px(px(px_x))
        .rounded(radius::MD)
        .text_sm_tw()
        .font_weight(FontWeight::MEDIUM)
        .whitespace_nowrap()
        .when(outline, |d| d.border_1().border_color(t.input).shadow(shadow_xs()))
        .when(disabled, |d| d.opacity(0.5))
        .when(!disabled, |d| d.cursor_pointer())
        .when(on, |d| d.bg(accent).text_color(accent_fg))
        .when(!on && !disabled, |d| {
            if outline {
                d.hover(move |s| s.bg(accent).text_color(accent_fg).text_sm_tw().font_weight(FontWeight::MEDIUM))
            } else {
                d.hover(move |s| s.bg(muted).text_color(muted_fg).text_sm_tw().font_weight(FontWeight::MEDIUM))
            }
        })
}

type PressedChange = Rc<dyn Fn(bool, &mut Window, &mut App)>;

/// A two-state button.
///
/// State: either kept by the caller with `.pressed(bool)` + `.on_pressed_change(|pressed, window, cx| ..)`, or kept
/// by the widget (`.default_pressed(bool)`, stored in the window by `id`).
///
/// ```ignore
/// Toggle::new(("tag", i)).variant(ToggleVariant::Outline).size(ToggleSize::Sm).label("Tank")
///     .pressed(selected).on_pressed_change(cx.listener(move |this, on: &bool, _, cx| this.set_tag(i, *on, cx)))
/// ```
#[derive(IntoElement)]
pub struct Toggle {
    id: ElementId,
    variant: ToggleVariant,
    size: ToggleSize,
    pressed: Option<bool>,
    default_pressed: bool,
    disabled: bool,
    on_pressed_change: Option<PressedChange>,
    icon: Option<Icon>,
    label: Option<SharedString>,
    children: Vec<AnyElement>,
    style: StyleRefinement,
}

impl Toggle {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            variant: ToggleVariant::Default,
            size: ToggleSize::Default,
            pressed: None,
            default_pressed: false,
            disabled: false,
            on_pressed_change: None,
            icon: None,
            label: None,
            children: Vec::new(),
            style: StyleRefinement::default(),
        }
    }

    pub fn variant(mut self, variant: ToggleVariant) -> Self {
        self.variant = variant;
        self
    }

    pub fn size(mut self, size: ToggleSize) -> Self {
        self.size = size;
        self
    }

    pub fn pressed(mut self, pressed: bool) -> Self {
        self.pressed = Some(pressed);
        self
    }

    pub fn default_pressed(mut self, pressed: bool) -> Self {
        self.default_pressed = pressed;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn on_pressed_change(mut self, f: impl Fn(bool, &mut Window, &mut App) + 'static) -> Self {
        self.on_pressed_change = Some(Rc::new(f));
        self
    }

    /// 16px icon before the label.
    pub fn icon(mut self, icon: impl IconNamed) -> Self {
        self.icon = Some(Icon::new(icon).size(px(16.0)));
        self
    }

    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }
}

impl ParentElement for Toggle {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Styled for Toggle {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for Toggle {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = cx.tokens();
        let default_pressed = self.default_pressed;
        let state = window.use_keyed_state(self.id.clone(), cx, move |_, _| default_pressed);
        let controlled = self.pressed.is_some();
        let on = self.pressed.unwrap_or_else(|| *state.read(cx));
        let disabled = self.disabled;
        let on_change = self.on_pressed_change;
        toggle_root(self.id, self.variant, self.size, on, disabled, &t)
            .when(!disabled, |d| {
                d.on_click(move |_, window, cx| {
                    let next = !on;
                    if !controlled {
                        state.update(cx, |s, cx| {
                            *s = next;
                            cx.notify();
                        });
                    }
                    if let Some(f) = &on_change {
                        f(next, window, cx);
                    }
                })
            })
            .merge_style(&self.style)
            .children(self.icon)
            .children(self.label)
            .children(self.children)
    }
}
