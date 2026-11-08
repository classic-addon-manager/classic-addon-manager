//! `Button` with variants default, destructive, outline, secondary, ghost and link, and sizes default, sm, lg and icon.
use super::style_ext::StyleMergeExt as _;
use gpui_kit::component::{Icon, IconNamed, StyledExt as _};
use gpui_kit::{
    AnyElement, App, ClickEvent, ElementId, FontWeight, InteractiveElement, IntoElement, ParentElement, RenderOnce,
    SharedString, StatefulInteractiveElement, StyleRefinement, Styled, Window, div, prelude::FluentBuilder as _, px,
};

use crate::ui::theme::{Tokens, TokensExt, TwText, alpha, radius, shadow_xs, sp};

use super::spinner::spinner;

/// Click callback shared by the clickable widgets: `|event, window, cx|`.
pub type ClickHandler = Box<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ButtonVariant {
    #[default]
    Default,
    Destructive,
    Outline,
    Secondary,
    Ghost,
    Link,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ButtonSize {
    /// 36 px tall, 16 px horizontal padding (12 px with an icon).
    #[default]
    Default,
    /// 32 px tall, 6 px gap, 12 px horizontal padding (10 px with an icon).
    Sm,
    /// 40 px tall, 24 px horizontal padding (16 px with an icon).
    Lg,
    /// 36 px square, no padding.
    Icon,
}

/// A clickable button with a variant (colour scheme) and a size.
///
/// ```ignore
/// Button::new("save").label("Save").on_click(cx.listener(|this, _, _, cx| this.save(cx)))
/// Button::new("dl").variant(ButtonVariant::Outline).size(ButtonSize::Sm).icon(icons::DOWNLOAD).label("Install")
/// Button::new("x").size(ButtonSize::Icon).variant(ButtonVariant::Ghost).icon(icons::X)
/// Button::new("busy").label("Refresh").loading(is_refreshing)   // spinner replaces the icon, button disabled
/// Button::new("custom").child(div().child("anything"))           // arbitrary children (add `.has_icon(true)` if they hold an svg)
/// ```
/// `Styled` is implemented, so callers can chain `.w_full()`, `.mt_2()`, `.h(px(32.))`, `.text_color(..)` to
/// override the defaults. Buttons are not focusable (the app is mouse driven). A disabled button is drawn at half
/// opacity and ignores clicks.
#[derive(IntoElement)]
pub struct Button {
    id: ElementId,
    variant: ButtonVariant,
    size: ButtonSize,
    disabled: bool,
    loading: bool,
    has_icon: bool,
    icon: Option<Icon>,
    icon_end: Option<Icon>,
    label: Option<SharedString>,
    children: Vec<AnyElement>,
    on_click: Option<ClickHandler>,
    style: StyleRefinement,
}

impl Button {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            variant: ButtonVariant::Default,
            size: ButtonSize::Default,
            disabled: false,
            loading: false,
            has_icon: false,
            icon: None,
            icon_end: None,
            label: None,
            children: Vec::new(),
            on_click: None,
            style: StyleRefinement::default(),
        }
    }

    pub fn variant(mut self, variant: ButtonVariant) -> Self {
        self.variant = variant;
        self
    }

    pub fn size(mut self, size: ButtonSize) -> Self {
        self.size = size;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Replaces the leading icon by a rotating spinner and disables the button.
    pub fn loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Leading 16px icon, shown before the text.
    pub fn icon(mut self, icon: impl IconNamed) -> Self {
        self.icon = Some(Icon::new(icon).size(px(16.0)));
        self
    }

    /// Trailing 16px icon, shown after the text (for example an external-link marker).
    pub fn icon_end(mut self, icon: impl IconNamed) -> Self {
        self.icon_end = Some(Icon::new(icon).size(px(16.0)));
        self
    }

    /// Text label (after the leading icon).
    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Use the smaller horizontal padding because arbitrary `.child(..)` children contain an icon.
    /// Automatically true when `.icon`/`.icon_end`/`.loading` are used.
    pub fn has_icon(mut self, has_icon: bool) -> Self {
        self.has_icon = has_icon;
        self
    }

    pub fn on_click(mut self, handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static) -> Self {
        self.on_click = Some(Box::new(handler));
        self
    }
}

impl ParentElement for Button {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Styled for Button {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for Button {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = cx.tokens();
        let inactive = self.disabled || self.loading;
        let has_icon = self.has_icon || self.icon.is_some() || self.icon_end.is_some() || self.loading;

        let (height, pad_x, gap) = match self.size {
            ButtonSize::Default => (36.0, if has_icon { 12.0 } else { 16.0 }, 8.0),
            ButtonSize::Sm => (32.0, if has_icon { 10.0 } else { 12.0 }, 6.0),
            ButtonSize::Lg => (40.0, if has_icon { 16.0 } else { 24.0 }, 8.0),
            ButtonSize::Icon => (36.0, 0.0, 8.0),
        };

        // (background, foreground, hover background, border colour, shadow)
        let white = Tokens::white();
        let (bg, fg, hover_bg, border) = match self.variant {
            ButtonVariant::Default => (Some(t.primary), t.primary_foreground, Some(alpha(t.primary, 0.9)), None),
            ButtonVariant::Destructive => {
                (Some(alpha(t.destructive, 0.6)), white, Some(alpha(t.destructive, 0.9)), None)
            }
            ButtonVariant::Outline => {
                (Some(alpha(t.input, 0.3)), t.foreground, Some(alpha(t.input, 0.5)), Some(t.input))
            }
            ButtonVariant::Secondary => {
                (Some(t.secondary), t.secondary_foreground, Some(alpha(t.secondary, 0.8)), None)
            }
            ButtonVariant::Ghost => (None, t.foreground, Some(alpha(t.accent, 0.5)), None),
            ButtonVariant::Link => (None, t.primary, None, None),
        };
        let shadow = matches!(
            self.variant,
            ButtonVariant::Default | ButtonVariant::Destructive | ButtonVariant::Outline | ButtonVariant::Secondary
        );
        let link = self.variant == ButtonVariant::Link;

        let leading: Option<AnyElement> =
            if self.loading { Some(spinner().into_any_element()) } else { self.icon.map(|i| i.into_any_element()) };
        let trailing: Option<AnyElement> = self.icon_end.map(|i| i.into_any_element());

        let on_click = self.on_click;
        div()
            .id(self.id)
            .flex()
            .flex_shrink_0()
            .items_center()
            .justify_center()
            .gap(px(gap))
            .whitespace_nowrap()
            .h(px(height))
            .when(self.size == ButtonSize::Icon, |d| d.w(px(height)))
            .px(px(pad_x))
            .rounded(radius::MD)
            .text_sm_tw()
            .font_weight(FontWeight::MEDIUM)
            .text_color(fg)
            .when_some(bg, |d, c| d.bg(c))
            .when_some(border, |d, c| d.border_1().border_color(c))
            .when(shadow, |d| d.shadow(shadow_xs()))
            .when(inactive, |d| d.opacity(0.5))
            .when(!inactive, |d| {
                d.cursor_pointer()
                    .when_some(hover_bg, |d, c| d.hover(move |s| s.bg(c)))
                    .when(link, move |d| {
                        d.hover(move |s| s.underline().text_sm_tw().font_weight(FontWeight::MEDIUM).text_color(fg))
                    })
                    .when_some(on_click, |d, f| d.on_click(move |e, w, cx| f(e, w, cx)))
            })
            .merge_style(&self.style)
            .children(leading)
            .children(self.label)
            .children(self.children)
            .children(trailing)
    }
}
