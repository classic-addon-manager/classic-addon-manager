//! `EmptyState`: what a view shows in place of content it doesn't have, such as nothing installed, no search match
//! or a failed load. A round icon tile (or a picture), a title, an optional description and an optional row of
//! actions, centred.
//!
//! ```ignore
//! EmptyState::new(icons::PACKAGE_SEARCH, "No addons found")
//!     .description("Try a different query or clear the search.")
//!     .compact()
//!     .min_h(px(200.0))
//! ```
//! `Styled` calls style the outer box: its size and padding, and the text colour the title inherits.
use gpui_kit::component::Icon;
use gpui_kit::{
    AnyElement, App, FontWeight, IntoElement, ParentElement, RenderOnce, SharedString, StyleRefinement, Styled, Window,
    div, prelude::FluentBuilder as _, px,
};

use crate::ui::{
    components::StyleMergeExt as _,
    icons::Lucide,
    theme::{TokensExt, TwText, alpha},
};

/// Colour of the icon tile.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EmptyTone {
    Muted,
    Primary,
    Destructive,
}

enum Media {
    Icon(Lucide),
    Custom(AnyElement),
}

#[derive(IntoElement)]
pub struct EmptyState {
    media: Media,
    title: SharedString,
    description: Option<SharedString>,
    tone: EmptyTone,
    compact: bool,
    actions: Vec<AnyElement>,
    style: StyleRefinement,
}

impl EmptyState {
    pub fn new(icon: Lucide, title: impl Into<SharedString>) -> Self {
        Self::with_media(Media::Icon(icon), title.into())
    }

    /// Shows `picture` instead of the icon tile.
    pub fn picture(picture: impl IntoElement, title: impl Into<SharedString>) -> Self {
        Self::with_media(Media::Custom(picture.into_any_element()), title.into())
    }

    fn with_media(media: Media, title: SharedString) -> Self {
        Self {
            media,
            title,
            description: None,
            tone: EmptyTone::Muted,
            compact: false,
            actions: Vec::new(),
            style: StyleRefinement::default(),
        }
    }

    pub fn description(mut self, description: impl Into<SharedString>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn tone(mut self, tone: EmptyTone) -> Self {
        self.tone = tone;
        self
    }

    /// The smaller variant for panes and dialogs: a 40 px tile and a 14 px title. The default fills a page with a
    /// 64 px tile and an 18 px title.
    pub fn compact(mut self) -> Self {
        self.compact = true;
        self
    }
}

/// Children are the actions, laid out in a row below the description.
impl ParentElement for EmptyState {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.actions.extend(elements);
    }
}

impl Styled for EmptyState {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for EmptyState {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = cx.tokens();
        let compact = self.compact;
        let media = match self.media {
            Media::Icon(icon) => {
                let (border, bg, fg) = match self.tone {
                    EmptyTone::Muted => (t.border, t.muted, t.muted_foreground),
                    EmptyTone::Primary => (alpha(t.primary, 0.2), alpha(t.primary, 0.1), t.primary),
                    EmptyTone::Destructive => (alpha(t.destructive, 0.3), alpha(t.destructive, 0.1), t.destructive),
                };
                div()
                    .flex()
                    .items_center()
                    .justify_center()
                    .size(px(if compact { 40.0 } else { 64.0 }))
                    .rounded_full()
                    .border_1()
                    .border_color(border)
                    .bg(bg)
                    .text_color(fg)
                    .child(Icon::new(icon).size(px(if compact { 18.0 } else { 32.0 })))
                    .into_any_element()
            }
            Media::Custom(picture) => picture,
        };
        let title =
            div().font_weight(if compact { FontWeight::MEDIUM } else { FontWeight::SEMIBOLD }).child(self.title);
        div()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .px(px(16.0))
            .text_center()
            .child(div().flex_none().mb(px(if compact { 8.0 } else { 16.0 })).child(media))
            .child(if compact { title.mb(px(4.0)).text_sm_tw() } else { title.mb(px(8.0)).text_lg_tw() })
            .when_some(self.description, |d, description| {
                d.child(
                    div()
                        .max_w(px(if compact { 320.0 } else { 384.0 }))
                        .text_sm_tw()
                        .text_color(t.muted_foreground)
                        .child(description),
                )
            })
            .when(!self.actions.is_empty(), |d| {
                d.child(div().flex().gap_3().mt(px(if compact { 12.0 } else { 24.0 })).children(self.actions))
            })
            .merge_style(&self.style)
    }
}
