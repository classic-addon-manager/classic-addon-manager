//! Small widgets shared by the dialogs of this module (not generally reusable enough for `cam_ui::components`).
use cam_ui::{
    components::{ClickHandler, StyleMergeExt as _},
    icons::Lucide,
    theme::{TwText, radius, shadow_md_tinted},
};
use gpui_kit::component::Icon;
use gpui_kit::{
    AnyElement, App, ClickEvent, ElementId, FontWeight, Hsla, InteractiveElement, IntoElement, ParentElement,
    RenderOnce, StatefulInteractiveElement, StyleRefinement, Styled, Window, div, hsla, prelude::FluentBuilder as _,
    px,
};

/// Solid-colour button with its own hover colour and a shadow tinted to match. The shared `Button` cannot restyle its
/// hover state, so the dialogs use this one for their emerald action buttons.
#[derive(IntoElement)]
pub struct TintedButton {
    id: ElementId,
    bg: Hsla,
    hover_bg: Hsla,
    shadow: Hsla,
    disabled: bool,
    has_icon: bool,
    on_click: Option<ClickHandler>,
    children: Vec<AnyElement>,
    style: StyleRefinement,
}

impl TintedButton {
    pub fn new(id: impl Into<ElementId>, bg: Hsla, hover_bg: Hsla, shadow: Hsla) -> Self {
        Self {
            id: id.into(),
            bg,
            hover_bg,
            shadow,
            disabled: false,
            has_icon: false,
            on_click: None,
            children: Vec::new(),
            style: StyleRefinement::default(),
        }
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Use the narrower 12 px horizontal padding (instead of 16 px) when the button holds an icon.
    pub fn has_icon(mut self, has_icon: bool) -> Self {
        self.has_icon = has_icon;
        self
    }

    pub fn on_click(mut self, handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static) -> Self {
        self.on_click = Some(Box::new(handler));
        self
    }
}

impl ParentElement for TintedButton {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Styled for TintedButton {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for TintedButton {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let (hover_bg, disabled) = (self.hover_bg, self.disabled);
        div()
            .id(self.id)
            .flex()
            .flex_shrink_0()
            .items_center()
            .justify_center()
            .gap(px(8.0))
            .whitespace_nowrap()
            .h(px(36.0))
            .px(px(if self.has_icon { 12.0 } else { 16.0 }))
            .rounded(radius::MD)
            .text_sm_tw()
            .font_weight(FontWeight::MEDIUM)
            .text_color(hsla(0.0, 0.0, 1.0, 1.0))
            .bg(self.bg)
            .shadow(shadow_md_tinted(self.shadow))
            .when(disabled, |d| d.opacity(0.5))
            .when(!disabled, |d| {
                d.cursor_pointer()
                    .hover(move |s| s.bg(hover_bg))
                    .when_some(self.on_click, |d, f| d.on_click(move |e, w, cx| f(e, w, cx)))
            })
            .merge_style(&self.style)
            .children(self.children)
    }
}

/// The tinted 40 px rounded tile with a centred 20 px icon in the headers of the update and support dialogs.
pub fn icon_tile(icon: Lucide, bg: Hsla, fg: Hsla) -> impl IntoElement {
    div()
        .flex()
        .flex_shrink_0()
        .items_center()
        .justify_center()
        .size(px(40.0))
        .rounded(radius::XL)
        .bg(bg)
        .child(Icon::new(icon).size(px(20.0)).text_color(fg))
}
