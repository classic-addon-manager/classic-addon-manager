//! Popover: a floating panel that opens next to a trigger. By default it is 288 px wide with 16 px padding, a rounded
//! border, the popover background colour and a medium drop shadow.
//!
//! ```ignore
//! Popover::new()
//!     .trigger(Button::new("p").variant(ButtonVariant::Outline).child("Open popover"))
//!     .side(Side::Bottom).align(Align::Center)
//!     .content(|window, cx| div().child("Place content for the popover here."))
//! ```
//! * Uncontrolled by default: click the trigger to toggle, click outside or press Escape to close, and focus returns to
//!   the previously focused element. Controlled: `.open(bool)` + `.on_open_change(|open, window, cx| ..)`.
//!   `.default_open(true)` starts open.
//! * The content closure runs on every frame while open: keep entities (inputs, views) in captured variables.
//!   Use [`Popover::content_with`] when the content must close the popover itself (`ctx.close(window, cx)`).
//! * Appearance overrides: `.content_width(px(320.))`, `.content_style(|d| d.p_0())`, `.unstyled(true)` (no box at all).
//! * Give `.id(..)` when several popovers are created from the same code location (loops).
use std::{panic::Location, rc::Rc};

use super::style_ext::StyleMergeExt as _;
use gpui_kit::{
    AnyElement, App, Div, ElementId, Entity, InteractiveElement as _, IntoElement, MouseButton, ParentElement, Pixels,
    RenderOnce, StatefulInteractiveElement as _, StyleRefinement, Styled, Window, div, prelude::FluentBuilder as _, px,
};

use super::floating::{
    Align, FloatState, Floating, OverlayCancel, POPOVER_CONTEXT, Placement, Side, element_id, ensure_keys, track_bounds,
};
use crate::ui::theme::{TokensExt, radius};

type OpenChange = Rc<dyn Fn(bool, &mut Window, &mut App)>;
type ContentFn = Rc<dyn Fn(&PopoverCtx, &mut Window, &mut App) -> AnyElement>;

/// Handle passed to [`Popover::content_with`] to close the popover from inside its content.
#[derive(Clone)]
pub struct PopoverCtx {
    state: Entity<FloatState>,
    on_change: Option<OpenChange>,
    controlled: bool,
}

impl PopoverCtx {
    /// `PopoverClose`.
    pub fn close(&self, window: &mut Window, cx: &mut App) {
        set_open(&self.state, &self.on_change, self.controlled, false, window, cx);
    }
}

fn set_open(
    state: &Entity<FloatState>,
    on_change: &Option<OpenChange>,
    controlled: bool,
    open: bool,
    window: &mut Window,
    cx: &mut App,
) {
    if controlled {
        if let Some(oc) = on_change {
            oc(open, window, cx);
        }
        return;
    }
    if state.read(cx).open == open {
        return;
    }
    apply_open(state, open, window, cx);
    if let Some(oc) = on_change {
        oc(open, window, cx);
    }
}

/// Changes the state and moves focus into / back out of the popover.
fn apply_open(state: &Entity<FloatState>, open: bool, window: &mut Window, cx: &mut App) {
    let prev = if open { window.focused(cx) } else { None };
    let (focus, restore) = state.update(cx, |s, cx| {
        s.open = open;
        if open {
            s.prev_focus = prev;
        }
        cx.notify();
        (s.focus.clone(), if open { None } else { s.prev_focus.take() })
    });
    if open {
        // after the current mouse-down finished: focusable ancestors re-focus themselves while it bubbles
        window.defer(cx, move |window, cx| focus.focus(window, cx));
    } else if let Some(prev) = restore {
        window.focus(&prev, cx);
    }
}

#[derive(IntoElement)]
pub struct Popover {
    id: ElementId,
    trigger: Option<AnyElement>,
    content: Option<ContentFn>,
    side: Side,
    align: Align,
    side_offset: f32,
    align_offset: f32,
    open: Option<bool>,
    default_open: bool,
    on_open_change: Option<OpenChange>,
    width: Pixels,
    unstyled: bool,
    content_style: Option<Rc<dyn Fn(Div) -> Div>>,
    style: StyleRefinement,
    children: Vec<AnyElement>,
}

impl Popover {
    #[track_caller]
    pub fn new() -> Self {
        Self {
            id: element_id(None, Location::caller()),
            trigger: None,
            content: None,
            side: Side::Bottom,
            align: Align::Center,
            side_offset: 4.0,
            align_offset: 0.0,
            open: None,
            default_open: false,
            on_open_change: None,
            width: px(288.0),
            unstyled: false,
            content_style: None,
            style: StyleRefinement::default(),
            children: Vec::new(),
        }
    }

    pub fn id(mut self, id: impl Into<ElementId>) -> Self {
        self.id = id.into();
        self
    }

    /// Element that toggles the popover when pressed.
    pub fn trigger(mut self, trigger: impl IntoElement) -> Self {
        self.trigger = Some(trigger.into_any_element());
        self
    }

    /// Content of the popover (called every frame while open).
    pub fn content<E: IntoElement>(mut self, content: impl Fn(&mut Window, &mut App) -> E + 'static) -> Self {
        self.content = Some(Rc::new(move |_, window, cx| content(window, cx).into_any_element()));
        self
    }

    /// Like `content`, with a [`PopoverCtx`] to close the popover from inside.
    pub fn content_with<E: IntoElement>(
        mut self,
        content: impl Fn(&PopoverCtx, &mut Window, &mut App) -> E + 'static,
    ) -> Self {
        self.content = Some(Rc::new(move |ctx, window, cx| content(ctx, window, cx).into_any_element()));
        self
    }

    /// Which side of the trigger the popover opens on (default `Bottom`). It flips to the opposite side when it does
    /// not fit.
    pub fn side(mut self, side: Side) -> Self {
        self.side = side;
        self
    }

    /// How the popover lines up along the trigger's edge (default `Center`).
    pub fn align(mut self, align: Align) -> Self {
        self.align = align;
        self
    }

    /// Gap between the trigger and the popover in px (default 4).
    pub fn side_offset(mut self, offset: f32) -> Self {
        self.side_offset = offset;
        self
    }

    pub fn align_offset(mut self, offset: f32) -> Self {
        self.align_offset = offset;
        self
    }

    pub fn open(mut self, open: bool) -> Self {
        self.open = Some(open);
        self
    }

    pub fn default_open(mut self, open: bool) -> Self {
        self.default_open = open;
        self
    }

    pub fn on_open_change(mut self, handler: impl Fn(bool, &mut Window, &mut App) + 'static) -> Self {
        self.on_open_change = Some(Rc::new(handler));
        self
    }

    /// Width of the popover (default 288 px).
    pub fn content_width(mut self, width: Pixels) -> Self {
        self.width = width;
        self
    }

    /// Extra styling of the content box, applied after the defaults.
    pub fn content_style(mut self, f: impl Fn(Div) -> Div + 'static) -> Self {
        self.content_style = Some(Rc::new(f));
        self
    }

    /// Render the content without the bg/border/padding/width chrome (you supply your own).
    pub fn unstyled(mut self, unstyled: bool) -> Self {
        self.unstyled = unstyled;
        self
    }
}

impl Styled for Popover {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl ParentElement for Popover {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for Popover {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        ensure_keys(cx);
        let t = cx.tokens();
        let default_open = self.default_open;
        let state = window.use_keyed_state(self.id.clone(), cx, move |_, cx| FloatState::new(default_open, cx));
        let controlled = self.open.is_some();
        let on_change = self.on_open_change;

        // controlled sync (no change event: not a user interaction)
        if let Some(want) = self.open
            && state.read(cx).open != want
        {
            apply_open(&state, want, window, cx);
        }
        let (open, anchor, measured, focus) = {
            let s = state.read(cx);
            (s.open, s.anchor, s.measured.clone(), s.focus.clone())
        };
        let ctx = PopoverCtx { state: state.clone(), on_change: on_change.clone(), controlled };

        let toggle = {
            let ctx = ctx.clone();
            move |_: &gpui_kit::MouseDownEvent, window: &mut Window, cx: &mut App| {
                set_open(&ctx.state, &ctx.on_change, ctx.controlled, !open, window, cx);
            }
        };

        let content = if open {
            let mut chrome = div().when(!self.unstyled, |d| {
                d.w(self.width)
                    .p(px(16.0))
                    .rounded(radius::MD)
                    .border_1()
                    .border_color(t.border)
                    .bg(t.popover)
                    .text_color(t.popover_foreground)
                    .shadow_md()
            });
            if let Some(f) = &self.content_style {
                chrome = f(chrome);
            }
            let mut body = chrome
                .id("shadcn-popover")
                .key_context(POPOVER_CONTEXT)
                .track_focus(&focus)
                .on_action({
                    let ctx = ctx.clone();
                    move |_: &OverlayCancel, window, cx| ctx.close(window, cx)
                })
                .on_mouse_down_out({
                    let ctx = ctx.clone();
                    move |ev, window, cx| {
                        // a press on the trigger toggles through the trigger handler
                        if !anchor.contains(&ev.position) {
                            ctx.close(window, cx);
                        }
                    }
                });
            if let Some(content) = &self.content {
                body = body.child(content(&ctx, window, cx));
            }
            Some(Floating::new(
                anchor,
                measured,
                Placement::new(self.side, self.align, self.side_offset, self.align_offset),
                body,
            ))
        } else {
            None
        };

        div()
            .id(self.id)
            .merge_style(&self.style)
            .children(self.trigger)
            .children(self.children)
            .child(track_bounds(state))
            .on_mouse_down(MouseButton::Left, toggle)
            .children(content)
    }
}
