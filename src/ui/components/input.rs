//! Single-line text `Input` on top of gpui-component's `InputState` editor, plus the helpers shared by all text
//! fields (keyboard-only focus ring tracking, field chrome, subscription helpers).
//!
//! ```ignore
//! // 1. create the state once (page constructor), `cx` may be `&mut Context<Page>`
//! self.search = text_input(window, cx, "Search addons...");
//! // 2. optionally react to changes (the subscription lives as long as the state, no need to store it)
//! on_input_change(&self.search, window, cx, |this, text, _window, cx| { this.query = text.to_string(); cx.notify(); });
//! // or, for a search box, only once typing pauses
//! on_input_settled(&self.search, window, cx, |this, text, cx| { this.query = text; cx.notify(); });
//! // 3. render
//! Input::search(&self.search).w(px(256.0))   // or Input::new(&self.name).placeholder via the state
//! // 4. read the text any time with `self.search.read(cx).value()`, set it with `self.search.update(cx, |s, cx| s.set_value("x", window, cx))`
//! ```
use std::{rc::Rc, time::Duration};

use gpui_kit::base::input::InputBaseState;
use gpui_kit::component::input::InputModeKind;
pub use gpui_kit::component::input::{InputEvent, InputState};
use gpui_kit::component::{IconNamed, StyledExt as _, input::Input as RawInput};
use gpui_kit::{
    AnyElement, App, AppContext as _, Context, DefiniteLength, ElementId, Entity, FocusHandle, Focusable,
    InteractiveElement, IntoElement, MouseButton, ParentElement, Refineable as _, RenderOnce, SharedString,
    StatefulInteractiveElement as _, StyleRefinement, Styled, Task, Window, div, prelude::FluentBuilder as _, px, svg,
};

use crate::ui::{
    icons,
    theme::{Tokens, TokensExt, TwText, alpha, radius, shadow_xs},
};

// ---------------------------------------------------------------------------------------------------------
// State helpers
// ---------------------------------------------------------------------------------------------------------

/// Creates a single-line text input state with a placeholder. Call it once (page constructor), keep the
/// `Entity<InputState>` in your struct and pass `&state` to [`Input::new`].
///
/// `cx` can be `&mut App` or a `&mut Context<YourPage>` (auto-deref).
pub fn text_input(window: &mut Window, cx: &mut App, placeholder: impl Into<SharedString>) -> Entity<InputState> {
    let placeholder = placeholder.into();
    cx.new(|cx| InputState::new(window, cx).placeholder(placeholder))
}

/// [`text_input`] with an initial value.
pub fn text_input_with_value(
    window: &mut Window,
    cx: &mut App,
    placeholder: impl Into<SharedString>,
    value: impl Into<SharedString>,
) -> Entity<InputState> {
    let (placeholder, value) = (placeholder.into(), value.into());
    cx.new(|cx| InputState::new(window, cx).placeholder(placeholder).default_value(value))
}

/// Calls `f(this, current_text, window, cx)` every time the text of `state` changes (typing, paste, the clear button).
/// `InputState::set_value` is silent (it never emits [`InputEvent::Change`]), so callers that set the text
/// programmatically must update their own state. The subscription is detached: it lives as long as the page and the state.
pub fn on_input_change<T: 'static, M: InputModeKind>(
    state: &Entity<InputBaseState<M>>,
    window: &mut Window,
    cx: &mut Context<T>,
    f: impl Fn(&mut T, SharedString, &mut Window, &mut Context<T>) + 'static,
) {
    cx.subscribe_in(state, window, move |this, state, event: &InputEvent, window, cx| {
        if matches!(event, InputEvent::Change) {
            let text = state.read(cx).value();
            f(this, text, window, cx);
        }
    })
    .detach();
}

/// How long the text must stay unchanged before [`on_input_settled`] reports it.
const SETTLE_DELAY: Duration = Duration::from_millis(50);

/// Like [`on_input_change`], but calls `f(this, current_text, cx)` only once the text has stayed unchanged for 50 ms,
/// so a search box filters once per pause in typing instead of once per key. The last text always wins: every change
/// drops the wait the change before started. Nothing runs once the view is gone.
pub fn on_input_settled<T: 'static, M: InputModeKind>(
    state: &Entity<InputBaseState<M>>,
    window: &mut Window,
    cx: &mut Context<T>,
    f: impl Fn(&mut T, String, &mut Context<T>) + 'static,
) {
    let f = Rc::new(f);
    let mut pending: Option<Task<()>> = None;
    cx.subscribe_in(state, window, move |_, state, event: &InputEvent, _window, cx| {
        if matches!(event, InputEvent::Change) {
            let text = state.read(cx).value().to_string();
            let f = f.clone();
            let wait = cx.spawn(async move |this, cx| {
                cx.background_executor().timer(SETTLE_DELAY).await;
                let _ = this.update(cx, |this, cx| f(this, text, cx));
            });
            // Dropping the wait of the change before cancels it.
            drop(pending.replace(wait));
        }
    })
    .detach();
}

/// Calls `f(this, current_text, window, cx)` when Enter is pressed inside `state`.
pub fn on_input_enter<T: 'static, M: InputModeKind>(
    state: &Entity<InputBaseState<M>>,
    window: &mut Window,
    cx: &mut Context<T>,
    f: impl Fn(&mut T, SharedString, &mut Window, &mut Context<T>) + 'static,
) {
    cx.subscribe_in(state, window, move |this, state, event: &InputEvent, window, cx| {
        if matches!(event, InputEvent::PressEnter { .. }) {
            let text = state.read(cx).value();
            f(this, text, window, cx);
        }
    })
    .detach();
}

/// Calls `f(this, window, cx)` when `state` gains (`true`) or loses (`false`) keyboard focus.
pub fn on_input_focus<T: 'static, M: InputModeKind>(
    state: &Entity<InputBaseState<M>>,
    window: &mut Window,
    cx: &mut Context<T>,
    f: impl Fn(&mut T, bool, &mut Window, &mut Context<T>) + 'static,
) {
    cx.subscribe_in(state, window, move |this, _state, event: &InputEvent, window, cx| match event {
        InputEvent::Focus => f(this, true, window, cx),
        InputEvent::Blur => f(this, false, window, cx),
        _ => {}
    })
    .detach();
}

// ---------------------------------------------------------------------------------------------------------
// Shared chrome
// ---------------------------------------------------------------------------------------------------------

/// Shared look of the text fields: rounded border, translucent fill, small text, plus the focus, invalid and
/// disabled states.
pub(crate) fn field_style(t: &Tokens, focused: bool, invalid: bool, disabled: bool) -> StyleRefinement {
    let border = if invalid { t.destructive } else { t.input };
    let mut s = StyleRefinement::default()
        .rounded(radius::MD)
        .bg(alpha(t.input, 0.3))
        .border_1()
        .border_color(border)
        .text_sm_tw()
        .text_color(t.foreground);
    s = if focused && !disabled {
        let border = if invalid { t.destructive } else { t.ring };
        s.border_color(border).shadow(shadow_xs())
    } else {
        s.shadow(shadow_xs())
    };
    if disabled {
        s = s.opacity(0.5).cursor_not_allowed();
    }
    s
}

/// Focus handle plus a "was focused by mouse" flag. The focus ring only shows for keyboard focus, so clicking a field
/// does not draw it.
pub(crate) struct FocusVis {
    pub handle: FocusHandle,
    pub mouse: bool,
}

pub(crate) fn focus_vis(id: impl Into<ElementId>, window: &mut Window, cx: &mut App) -> Entity<FocusVis> {
    window.use_keyed_state(id, cx, |_, cx| FocusVis { handle: cx.focus_handle().tab_stop(true), mouse: false })
}

impl FocusVis {
    /// Whether the focus ring should be drawn now.
    pub fn visible(&self, window: &Window) -> bool {
        self.handle.is_focused(window) && !self.mouse
    }
}

/// Makes `el` focusable through `vis` (tab stop, click focuses without ring, any key press re-enables the ring).
pub(crate) fn track_focus_vis<E: InteractiveElement + Styled>(
    el: E,
    vis: &Entity<FocusVis>,
    window: &mut Window,
    cx: &mut App,
) -> E {
    let (handle, stale) = {
        let v = vis.read(cx);
        (v.handle.clone(), v.mouse && !v.handle.is_focused(window))
    };
    if stale {
        vis.update(cx, |v, _| v.mouse = false);
    }
    let (down, key) = (vis.clone(), vis.clone());
    el.track_focus(&handle)
        .on_mouse_down(MouseButton::Left, move |_, _, cx| down.update(cx, |v, _| v.mouse = true))
        .on_key_down(move |_, _, cx| {
            if key.read(cx).mouse {
                key.update(cx, |v, cx| {
                    v.mouse = false;
                    cx.notify();
                });
            }
        })
}

// ---------------------------------------------------------------------------------------------------------
// Input
// ---------------------------------------------------------------------------------------------------------

/// Single-line text field: 36 px tall and full width by default, with focus, invalid and disabled states.
///
/// * Create the state with [`text_input`]. `Input::new(&state)` only draws it.
/// * Implements [`Styled`], so callers can chain `.w(px(256.))`, `.h(px(32.))`, `.text_xs_tw()`, `.font_mono_tw()`,
///   `.pl(..)`, `.flex_1()`... to override the defaults.
/// * [`Input::search`] / `.inline_search()`: toolbar search box with a leading search icon and no shadow until it
///   is focused.
#[derive(IntoElement)]
pub struct Input {
    state: Entity<InputState>,
    style: StyleRefinement,
    prefix_icon: Option<SharedString>,
    prefix: Option<AnyElement>,
    suffix: Option<AnyElement>,
    disabled: bool,
    invalid: bool,
    cleanable: bool,
    search: bool,
}

impl Input {
    pub fn new(state: &Entity<InputState>) -> Self {
        Self {
            state: state.clone(),
            style: StyleRefinement::default(),
            prefix_icon: None,
            prefix: None,
            suffix: None,
            disabled: false,
            invalid: false,
            cleanable: false,
            search: false,
        }
    }

    /// Toolbar search input: [`Input::new`] + [`Input::inline_search`].
    pub fn search(state: &Entity<InputState>) -> Self {
        Self::new(state).inline_search()
    }

    /// Leading search icon, 40 px left and 16 px right padding, no shadow until focused: the search boxes of the
    /// Dashboard and Addons toolbars.
    pub fn inline_search(mut self) -> Self {
        self.search = true;
        self.prefix_icon = Some(icons::SEARCH.path());
        self
    }

    /// Leading icon drawn 12 px from the left edge. The text starts at the left padding, so set one wide enough for
    /// the icon with `.pl(..)`, or use [`Input::inline_search`] which does it.
    pub fn prefix_icon(mut self, icon: impl IconNamed) -> Self {
        self.prefix_icon = Some(icon.path());
        self
    }

    /// Arbitrary element laid out before the text (inside the border).
    pub fn prefix(mut self, el: impl IntoElement) -> Self {
        self.prefix = Some(el.into_any_element());
        self
    }

    /// Arbitrary element laid out after the text (inside the border), e.g. a unit label or a small button.
    pub fn suffix(mut self, el: impl IntoElement) -> Self {
        self.suffix = Some(el.into_any_element());
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Marks the field as invalid: the border is destructive-coloured, also while focused.
    pub fn invalid(mut self, invalid: bool) -> Self {
        self.invalid = invalid;
        self
    }

    /// Shows a trailing x button when the field is not empty. Clicking it empties the field (emits
    /// [`InputEvent::Change`]) and focuses it.
    pub fn cleanable(mut self, cleanable: bool) -> Self {
        self.cleanable = cleanable;
        self
    }
}

impl Styled for Input {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for Input {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = cx.tokens();
        let focused = self.state.read(cx).focus_handle(cx).is_focused(window);
        let mut base = field_style(&t, focused, self.invalid, self.disabled).h(px(36.0)).px(px(12.0)).py(px(4.0));
        if self.search {
            base = base.pl(px(40.0)).pr(px(16.0));
            if !focused || self.disabled {
                base = base.shadow(vec![]);
            }
        }
        base.refine(&self.style);

        // gpui-component wraps `.prefix()` in an in-flow, zero-width `div` and lays it out with a column gap (6px for
        // the default size). An `absolute()` icon would therefore be placed against that wrapper, after the left
        // padding, and the wrapper would add 6px before the text. To put the icon 12px from the left edge and start
        // the text at the requested left padding, the left padding becomes a plain 12px and the prefix slot is an
        // in-flow box that spans the rest of the requested padding, minus the frame gap.
        const FRAME_GAP: f32 = 6.0;
        let icon = self.prefix_icon.map(|path| {
            let text_start = match base.padding.left {
                Some(DefiniteLength::Absolute(left)) => left.to_pixels(window.rem_size()),
                _ => px(12.0),
            };
            base.padding.left = Some(px(12.0).into());
            div()
                .flex()
                .flex_none()
                .items_center()
                .w((text_start - px(12.0 + FRAME_GAP)).max(px(0.0)))
                .child(svg().path(path).size(px(16.0)).flex_none().text_color(t.muted_foreground))
        });

        let clear = (self.cleanable && !self.disabled && !self.state.read(cx).value().is_empty()).then(|| {
            let state = self.state.clone();
            let group = SharedString::from(format!("input-clear-{}", state.entity_id()));
            div()
                .id(("input-clear", state.entity_id()))
                .group(group.clone())
                .flex()
                .flex_none()
                .items_center()
                .justify_center()
                .size(px(20.0))
                .rounded(radius::SM)
                .cursor_pointer()
                .child(
                    svg()
                        .path(icons::X.path())
                        .size(px(16.0))
                        .text_color(t.muted_foreground)
                        .group_hover(group, move |s| s.text_color(t.foreground)),
                )
                // Keep the editor from moving the caret / starting a selection under the button.
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .on_click(move |_, window, cx| {
                    state.update(cx, |s, cx| {
                        // `clean` goes through the editor's replace path, which emits `InputEvent::Change` exactly as
                        // typing does (and as gpui-component's own clear button does); `set_value` is silent.
                        s.clean(window, cx);
                        s.focus(window, cx);
                    })
                })
        });

        // `appearance(false)`: `field_style` draws the whole chrome. With appearance on, gpui-component paints the
        // suffix slot with an opaque `theme.background` square that clashes with the translucent field fill.
        RawInput::new(&self.state)
            .appearance(false)
            .disabled(self.disabled)
            .focus_bordered(false)
            .when_some(icon, |i, icon| i.prefix(icon))
            .when_some(self.prefix, |i, el| i.prefix(el))
            .when(self.suffix.is_some() || clear.is_some(), |i| {
                i.suffix(div().flex().items_center().gap(px(6.0)).children(clear).children(self.suffix))
            })
            .refine_style(&base)
    }
}
