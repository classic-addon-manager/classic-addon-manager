//! `Badge`: a small rounded label in one of five colour variants.
use super::style_ext::StyleMergeExt as _;
use gpui_kit::component::{Icon, IconNamed, StyledExt as _};
use gpui_kit::{
    AnyElement, App, FontWeight, InteractiveElement, IntoElement, ParentElement, RenderOnce, SharedString,
    StatefulInteractiveElement, StyleRefinement, Styled, Window, div, prelude::FluentBuilder as _, px,
};

use crate::ui::theme::{Tokens, TokensExt, Tw, TwText, alpha, radius, shadow_sm, tw};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BadgeVariant {
    #[default]
    Default,
    Secondary,
    Destructive,
    /// Black text on an orange background (darker on hover) with a shadow.
    Warning,
    Outline,
}

/// A small pill-shaped label: 12 px text, 8 px horizontal padding, 4 px gap between children, 12 px icons.
///
/// ```ignore
/// Badge::new("Installed")
/// Badge::new("Beta").variant(BadgeVariant::Secondary)
/// Badge::new("Update").variant(BadgeVariant::Warning).icon(icons::ARROW_UP)
/// Badge::empty().variant(BadgeVariant::Outline).child("custom").child(div().size_3())   // arbitrary children
/// ```
/// `Styled` is implemented, so callers can chain `.ml_2()`, `.text_color(..)`, `.bg(..)` to override the variant colours.
#[derive(IntoElement)]
pub struct Badge {
    variant: BadgeVariant,
    label: Option<SharedString>,
    icon: Option<Icon>,
    children: Vec<AnyElement>,
    style: StyleRefinement,
}

impl Badge {
    pub fn new(label: impl Into<SharedString>) -> Self {
        Self { label: Some(label.into()), ..Self::empty() }
    }

    /// Badge without a text label, fill it with `.child(..)`.
    pub fn empty() -> Self {
        Self {
            variant: BadgeVariant::Default,
            label: None,
            icon: None,
            children: Vec::new(),
            style: StyleRefinement::default(),
        }
    }

    pub fn variant(mut self, variant: BadgeVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Leading 12px icon.
    pub fn icon(mut self, icon: impl IconNamed) -> Self {
        self.icon = Some(Icon::new(icon).size(px(12.0)));
        self
    }
}

impl ParentElement for Badge {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Styled for Badge {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for Badge {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = cx.tokens();
        let transparent = gpui_kit::transparent_black();
        // (bg, fg, border, hover bg)
        let (bg, fg, border, hover_bg) = match self.variant {
            BadgeVariant::Default => (Some(t.primary), t.primary_foreground, transparent, None),
            BadgeVariant::Secondary => (Some(t.secondary), t.secondary_foreground, transparent, None),
            BadgeVariant::Destructive => (Some(alpha(t.destructive, 0.6)), Tokens::white(), transparent, None),
            BadgeVariant::Warning => {
                (Some(tw(Tw::Orange, 300)), gpui_kit::black(), transparent, Some(tw(Tw::Orange, 400)))
            }
            BadgeVariant::Outline => (None, t.foreground, t.border, None),
        };
        div()
            .id("badge")
            .flex()
            .flex_none()
            .items_center()
            .justify_center()
            .gap(px(4.0))
            .overflow_hidden()
            .whitespace_nowrap()
            .rounded(radius::MD)
            .border_1()
            .border_color(border)
            .px(px(8.0))
            .py(px(2.0))
            .text_xs_tw()
            .font_weight(FontWeight::MEDIUM)
            .text_color(fg)
            .when_some(bg, |d, c| d.bg(c))
            .when(self.variant == BadgeVariant::Warning, |d| d.shadow(shadow_sm()))
            .when_some(hover_bg, |d, c| d.hover(move |s| s.bg(c)))
            .merge_style(&self.style)
            .children(self.icon)
            .children(self.label)
            .children(self.children)
    }
}
