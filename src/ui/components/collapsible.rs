//! `Collapsible`: a trigger plus content that shows and hides when the trigger is clicked.
use std::rc::Rc;

use super::style_ext::StyleMergeExt as _;
use gpui_kit::component::StyledExt as _;
use gpui_kit::{
    AnyElement, App, ElementId, InteractiveElement, IntoElement, ParentElement, RenderOnce, StatefulInteractiveElement,
    StyleRefinement, Styled, Window, div, prelude::FluentBuilder as _,
};

type OpenChange = Rc<dyn Fn(bool, &mut Window, &mut App)>;

/// `Collapsible::new(id)` + `.trigger(..)`/`.trigger_with(|open| ..)` + `.content(..)`.
///
/// State: kept by the widget by default (stored in the window keyed by `id`, start open with `.default_open(true)`),
/// or by the caller with `.open(bool)` + `.on_open_change(|open, window, cx| ..)` (the callback also fires when the
/// widget keeps the state).
///
/// ```ignore
/// Collapsible::new("changelog")
///     .trigger_with(|open| h_flex().justify_between().child("What's new").child(chevron(open)))
///     .trigger_hover(|s| s.text_color(t.primary))
///     .content(div().mt_3().child("…"))
/// ```
/// The trigger is wrapped in a clickable element with a pointer cursor. The content is only built and rendered while
/// open when passed through `.content_with(|| ..)`.
#[derive(IntoElement)]
pub struct Collapsible {
    id: ElementId,
    open: Option<bool>,
    default_open: bool,
    disabled: bool,
    on_open_change: Option<OpenChange>,
    trigger: Option<Box<dyn FnOnce(bool) -> AnyElement>>,
    trigger_hover: Option<Box<dyn Fn(StyleRefinement) -> StyleRefinement>>,
    content: Option<Box<dyn FnOnce() -> AnyElement>>,
    style: StyleRefinement,
}

impl Collapsible {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            open: None,
            default_open: false,
            disabled: false,
            on_open_change: None,
            trigger: None,
            trigger_hover: None,
            content: None,
            style: StyleRefinement::default(),
        }
    }

    /// Open state kept by the caller.
    pub fn open(mut self, open: bool) -> Self {
        self.open = Some(open);
        self
    }

    pub fn default_open(mut self, open: bool) -> Self {
        self.default_open = open;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn on_open_change(mut self, f: impl Fn(bool, &mut Window, &mut App) + 'static) -> Self {
        self.on_open_change = Some(Rc::new(f));
        self
    }

    /// Static trigger content.
    pub fn trigger(mut self, trigger: impl IntoElement) -> Self {
        let el = trigger.into_any_element();
        self.trigger = Some(Box::new(move |_| el));
        self
    }

    /// Trigger content built from the current open state (to rotate a chevron etc.).
    pub fn trigger_with(mut self, f: impl FnOnce(bool) -> AnyElement + 'static) -> Self {
        self.trigger = Some(Box::new(f));
        self
    }

    /// Hover style of the trigger wrapper, for example `|s| s.text_color(t.primary)`.
    pub fn trigger_hover(mut self, f: impl Fn(StyleRefinement) -> StyleRefinement + 'static) -> Self {
        self.trigger_hover = Some(Box::new(f));
        self
    }

    pub fn content(mut self, content: impl IntoElement) -> Self {
        let el = content.into_any_element();
        self.content = Some(Box::new(move || el));
        self
    }

    /// Lazily built content (only evaluated while open).
    pub fn content_with(mut self, f: impl FnOnce() -> AnyElement + 'static) -> Self {
        self.content = Some(Box::new(f));
        self
    }
}

impl Styled for Collapsible {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for Collapsible {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let default_open = self.default_open;
        let state = window.use_keyed_state(self.id.clone(), cx, move |_, _| default_open);
        let controlled = self.open.is_some();
        let open = self.open.unwrap_or_else(|| *state.read(cx));
        let disabled = self.disabled;
        let on_open_change = self.on_open_change;
        let hover = self.trigger_hover;

        let trigger = self.trigger.map(|f| {
            div()
                .id((self.id.clone(), "trigger"))
                .when(!disabled, |d| d.cursor_pointer())
                .when(disabled, |d| d.opacity(0.5))
                .when_some(hover.filter(|_| !disabled), |d, f| d.hover(f))
                .when(!disabled, |d| {
                    d.on_click(move |_, window, cx| {
                        let next = !open;
                        if !controlled {
                            state.update(cx, |s, cx| {
                                *s = next;
                                cx.notify();
                            });
                        }
                        if let Some(cb) = &on_open_change {
                            cb(next, window, cx);
                        }
                    })
                })
                .child(f(open))
        });

        div()
            .id(self.id)
            .merge_style(&self.style)
            .children(trigger)
            .when(open, |d| d.children(self.content.map(|f| f())))
    }
}
