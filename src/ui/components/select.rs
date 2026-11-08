//! Select: a dropdown that picks one value from a list. The trigger is 36px tall (32px for the small size) with a
//! rounded border, 12px horizontal padding, small text, a faint input-coloured background and a half transparent
//! chevron. The menu is a popup panel with 4px padding, rows that show a check on the right when selected, labels,
//! separators and groups.
//!
//! ```ignore
//! Select::new("tag")
//!     .items(vec![("all", "All"), ("ui", "UI"), ("raid", "Raid")])      // (value, label)
//!     .value(self.tag.clone())                                           // controlled; or .default_value("all")
//!     .placeholder("Select tag")
//!     .width(px(140.0)).size(SelectSize::Sm)
//!     .on_change(cx.listener(|this, value: &SharedString, _, cx| { this.tag = value.clone(); cx.notify(); }))
//! ```
//! Open/closed state, the highlighted row and (uncontrolled) value live in `window.use_keyed_state` keyed by the id,
//! so ids must be unique among siblings. Keyboard: Tab to focus, Space/Enter/Down opens, arrows navigate, Enter picks,
//! Escape closes.
use std::rc::Rc;

use gpui_kit::component::{IconNamed, StyledExt as _};
use gpui_kit::{
    Anchor, AnyElement, App, Bounds, ElementId, Entity, InteractiveElement, IntoElement, KeyDownEvent, MouseButton,
    ParentElement, Pixels, Refineable as _, RenderOnce, ScrollHandle, SharedString, StatefulInteractiveElement,
    StyleRefinement, Styled, Window, anchored, canvas, deferred, div, point, prelude::FluentBuilder as _, px, svg,
};

use super::floating::POPUP_PRIORITY;
use super::input::{focus_vis, track_focus_vis};
use super::scroll_area::{ScrollbarAxis, scrollbar_overlay};
use crate::ui::{
    icons,
    theme::{Tokens, TokensExt, TwText, alpha, radius, shadow_md, shadow_xs},
};

// ---------------------------------------------------------------------------------------------------------
// Shared popup helpers (also used by `Combobox`)
// ---------------------------------------------------------------------------------------------------------

/// Popup panel box: popover colours, a 1px border, rounded corners and a medium drop shadow.
pub(crate) fn popup_panel(t: &Tokens) -> StyleRefinement {
    StyleRefinement::default()
        .bg(t.popover)
        .text_color(t.popover_foreground)
        .border_1()
        .border_color(t.border)
        .rounded(radius::MD)
        .shadow(shadow_md())
}

/// Whether a popup of about `est_height` px should open above `trigger` (not enough room below, more above).
pub(crate) fn opens_upward(trigger: Bounds<Pixels>, est_height: Pixels, window: &Window) -> bool {
    let below = window.viewport_size().height - trigger.bottom() - px(12.0);
    below < est_height && trigger.top() > below
}

/// Invisible absolute element reporting the bounds of its parent after layout.
pub(crate) fn bounds_probe(on_bounds: impl Fn(Bounds<Pixels>, &mut App) + 'static) -> impl IntoElement {
    canvas(move |bounds, _, cx| on_bounds(bounds, cx), |_, _, _, _| {}).absolute().size_full()
}

// ---------------------------------------------------------------------------------------------------------
// Items
// ---------------------------------------------------------------------------------------------------------

/// A selectable row of a [`Select`].
#[derive(Clone)]
pub struct SelectItem {
    pub value: SharedString,
    pub label: SharedString,
    pub icon: Option<SharedString>,
    pub disabled: bool,
}

impl SelectItem {
    pub fn new(value: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self { value: value.into(), label: label.into(), icon: None, disabled: false }
    }
    /// Leading 16px icon in the muted colour.
    pub fn icon(mut self, icon: impl IconNamed) -> Self {
        self.icon = Some(icon.path());
        self
    }
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

/// Content of the select menu: rows, group labels and separators.
#[derive(Clone)]
pub enum SelectEntry {
    Item(SelectItem),
    Label(SharedString),
    Separator,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum SelectSize {
    /// 36px tall.
    #[default]
    Default,
    /// 32px tall.
    Sm,
}

type ChangeFn = Rc<dyn Fn(&SharedString, &mut Window, &mut App) + 'static>;

struct SelectState {
    open: bool,
    value: Option<SharedString>,
    highlighted: Option<usize>,
    bounds: Option<Bounds<Pixels>>,
    scroll: ScrollHandle,
}

#[derive(IntoElement)]
pub struct Select {
    id: ElementId,
    entries: Vec<SelectEntry>,
    value: Option<Option<SharedString>>,
    default_value: Option<SharedString>,
    placeholder: SharedString,
    trigger_label: Option<SharedString>,
    trigger_content: Option<AnyElement>,
    size: SelectSize,
    disabled: bool,
    invalid: bool,
    default_open: bool,
    on_change: Option<ChangeFn>,
    style: StyleRefinement,
}

impl Select {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            entries: Vec::new(),
            value: None,
            default_value: None,
            placeholder: SharedString::default(),
            trigger_label: None,
            trigger_content: None,
            size: SelectSize::Default,
            disabled: false,
            invalid: false,
            default_open: false,
            on_change: None,
            style: StyleRefinement::default(),
        }
    }

    /// Adds `(value, label)` rows.
    pub fn items<V: Into<SharedString>, L: Into<SharedString>>(
        mut self,
        items: impl IntoIterator<Item = (V, L)>,
    ) -> Self {
        self.entries.extend(items.into_iter().map(|(v, l)| SelectEntry::Item(SelectItem::new(v, l))));
        self
    }

    /// Adds one fully configured row.
    pub fn item(mut self, item: SelectItem) -> Self {
        self.entries.push(SelectEntry::Item(item));
        self
    }

    /// Replaces/extends the menu content with explicit entries (rows, labels, separators).
    pub fn entries(mut self, entries: impl IntoIterator<Item = SelectEntry>) -> Self {
        self.entries.extend(entries);
        self
    }

    /// Group label row: small muted text.
    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.entries.push(SelectEntry::Label(label.into()));
        self
    }

    /// Thin horizontal line between rows.
    pub fn separator(mut self) -> Self {
        self.entries.push(SelectEntry::Separator);
        self
    }

    /// Group with a label: a label row followed by `(value, label)` rows.
    pub fn group<V: Into<SharedString>, L: Into<SharedString>>(
        mut self,
        label: impl Into<SharedString>,
        items: impl IntoIterator<Item = (V, L)>,
    ) -> Self {
        self.entries.push(SelectEntry::Label(label.into()));
        self.items(items)
    }

    /// Controlled selected value (`None`-like values that match no row show the placeholder). Update your field in `on_change`.
    pub fn value(mut self, value: impl Into<SharedString>) -> Self {
        self.value = Some(Some(value.into()));
        self
    }

    /// Controlled selected value that may be empty (shows the placeholder).
    pub fn value_opt(mut self, value: Option<SharedString>) -> Self {
        self.value = Some(value);
        self
    }

    /// Initial value of the uncontrolled mode.
    pub fn default_value(mut self, value: impl Into<SharedString>) -> Self {
        self.default_value = Some(value.into());
        self
    }

    pub fn placeholder(mut self, placeholder: impl Into<SharedString>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    /// Text shown in the trigger instead of the selected row's label.
    pub fn trigger_label(mut self, label: impl Into<SharedString>) -> Self {
        self.trigger_label = Some(label.into());
        self
    }

    /// Element shown in the trigger instead of any text (icon-only triggers), takes precedence over `trigger_label`.
    pub fn trigger_content(mut self, content: impl IntoElement) -> Self {
        self.trigger_content = Some(content.into_any_element());
        self
    }

    pub fn size(mut self, size: SelectSize) -> Self {
        self.size = size;
        self
    }

    /// Fixed trigger width (for example `px(140.)`). The menu is at least this wide. Default is content width.
    pub fn width(mut self, width: Pixels) -> Self {
        self.style.size.width = Some(width.into());
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Invalid state: draws the trigger border in the destructive colour.
    pub fn invalid(mut self, invalid: bool) -> Self {
        self.invalid = invalid;
        self
    }

    /// Starts with the menu open (screenshots / demos).
    pub fn default_open(mut self, open: bool) -> Self {
        self.default_open = open;
        self
    }

    /// Called with the picked value. Works with `cx.listener(|this, v: &SharedString, window, cx| ..)`.
    pub fn on_change(mut self, f: impl Fn(&SharedString, &mut Window, &mut App) + 'static) -> Self {
        self.on_change = Some(Rc::new(f));
        self
    }
}

impl Styled for Select {
    /// Styles apply to the trigger: `.w(..)`, `.h(..)`, `.flex_1()`, `.mt_2()`...
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

fn is_enabled_item(e: &SelectEntry) -> bool {
    matches!(e, SelectEntry::Item(i) if !i.disabled)
}

/// Next enabled row index from `from` in direction `dir` (+1/-1). It does not wrap around at either end.
fn step(entries: &[SelectEntry], from: Option<usize>, dir: isize) -> Option<usize> {
    let n = entries.len() as isize;
    let mut i = match from {
        Some(i) => i as isize + dir,
        None if dir > 0 => 0,
        None => n - 1,
    };
    while i >= 0 && i < n {
        if is_enabled_item(&entries[i as usize]) {
            return Some(i as usize);
        }
        i += dir;
    }
    from
}

impl RenderOnce for Select {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = cx.tokens();
        let default_value = self.default_value.clone();
        let default_open = self.default_open;
        let state = window.use_keyed_state((self.id.clone(), "select"), cx, move |_, _| SelectState {
            open: default_open,
            value: default_value,
            highlighted: None,
            bounds: None,
            scroll: ScrollHandle::new(),
        });
        if let Some(v) = &self.value
            && state.read(cx).value != *v
        {
            state.update(cx, |s, _| s.value = v.clone());
        }
        let vis = focus_vis((self.id.clone(), "focus"), window, cx);
        let handle = vis.read(cx).handle.clone();

        let (open, value, highlighted, bounds, scroll) = {
            let s = state.read(cx);
            (s.open && !self.disabled, s.value.clone(), s.highlighted, s.bounds, s.scroll.clone())
        };
        let entries = Rc::new(self.entries);
        let on_change = self.on_change.clone();
        let ring = !self.disabled && !open && vis.read(cx).visible(window);

        let selected_label = value.as_ref().and_then(|v| {
            entries.iter().find_map(|e| match e {
                SelectEntry::Item(i) if i.value == *v => Some(i.label.clone()),
                _ => None,
            })
        });
        let shown = self.trigger_label.clone().or(selected_label).or_else(|| value.clone().filter(|v| !v.is_empty()));
        let placeholder_shown = shown.is_none() && self.trigger_content.is_none();
        let trigger_text = self
            .trigger_content
            .unwrap_or_else(|| shown.unwrap_or_else(|| self.placeholder.clone()).into_any_element());

        let pick: ChangeFn = {
            let (state, on_change, handle) = (state.clone(), on_change, handle.clone());
            Rc::new(move |v, window, cx| {
                state.update(cx, |s, cx| {
                    s.value = Some(v.clone());
                    s.open = false;
                    cx.notify();
                });
                if let Some(f) = &on_change {
                    f(v, window, cx);
                }
                window.focus(&handle, cx);
            })
        };

        let was_open = open;
        let initial_highlight = {
            let entries = entries.clone();
            let value = value.clone();
            move || {
                value
                    .as_ref()
                    .and_then(|v| {
                        entries.iter().position(|e| matches!(e, SelectEntry::Item(i) if i.value == *v && !i.disabled))
                    })
                    .or_else(|| step(&entries, None, 1))
            }
        };

        let height = match self.size {
            SelectSize::Default => px(36.0),
            SelectSize::Sm => px(32.0),
        };
        let fill = self.style.size.width.is_some() || self.style.flex_grow.is_some();
        let grows = self.style.flex_grow.is_some();
        let (border, shadows) = match (self.invalid, ring) {
            (true, _) => (t.destructive, shadow_xs()),
            (false, true) => (t.ring, shadow_xs()),
            (false, false) => (t.input, shadow_xs()),
        };

        let mut trigger = div()
            .id((self.id.clone(), "trigger"))
            .relative()
            .flex()
            .flex_none()
            .items_center()
            .justify_between()
            .gap(px(8.0))
            .h(height)
            .px(px(12.0))
            .rounded(radius::MD)
            .border_1()
            .border_color(border)
            .bg(alpha(t.input, 0.3))
            .text_sm_tw()
            .text_color(t.foreground)
            .whitespace_nowrap()
            .shadow(shadows)
            .when(fill, |d| d.w_full())
            .when(grows, |d| d.flex_shrink(1.0).min_w_0());
        // the caller's style goes last so it overrides the defaults
        trigger.style().refine(&self.style);
        let mut trigger = trigger
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .overflow_hidden()
                    .text_ellipsis()
                    .when(placeholder_shown, |d| d.text_color(t.muted_foreground))
                    .child(trigger_text),
            )
            .child(
                svg()
                    .path(icons::CHEVRON_DOWN.path())
                    .size(px(16.0))
                    .flex_none()
                    .text_color(alpha(t.muted_foreground, 0.5)),
            )
            .child(bounds_probe({
                let state = state.clone();
                move |b, cx| {
                    if state.read(cx).bounds != Some(b) {
                        state.update(cx, |s, cx| {
                            s.bounds = Some(b);
                            cx.notify();
                        });
                    }
                }
            }));

        if self.disabled {
            trigger = trigger.opacity(0.5).cursor_not_allowed();
        } else {
            trigger = track_focus_vis(trigger, &vis, window, cx)
                .hover(move |s| s.bg(alpha(t.input, 0.5)))
                .on_mouse_down(MouseButton::Left, {
                    let (state, init) = (state.clone(), initial_highlight.clone());
                    move |_, _, cx| {
                        let init = init();
                        state.update(cx, |s, cx| {
                            s.open = !was_open;
                            if s.open {
                                s.highlighted = init;
                                if let Some(ix) = init {
                                    s.scroll.scroll_to_item(ix);
                                }
                            }
                            cx.notify();
                        });
                    }
                })
                .on_key_down({
                    let (state, entries, pick, init) =
                        (state.clone(), entries.clone(), pick.clone(), initial_highlight.clone());
                    move |ev: &KeyDownEvent, window, cx| {
                        let key = ev.keystroke.key.as_str();
                        let (is_open, hl) = {
                            let s = state.read(cx);
                            (s.open, s.highlighted)
                        };
                        let set_hl = |ix: Option<usize>, cx: &mut App| {
                            state.update(cx, |s, cx| {
                                s.highlighted = ix;
                                if let Some(ix) = ix {
                                    s.scroll.scroll_to_item(ix);
                                }
                                cx.notify();
                            });
                        };
                        match key {
                            "escape" if is_open => {
                                cx.stop_propagation();
                                state.update(cx, |s, cx| {
                                    s.open = false;
                                    cx.notify();
                                });
                            }
                            "tab" if is_open => state.update(cx, |s, cx| {
                                s.open = false;
                                cx.notify();
                            }),
                            "down" | "up" | "home" | "end" | "enter" | "space" => {
                                cx.stop_propagation();
                                if !is_open {
                                    if matches!(key, "enter" | "space" | "down" | "up") {
                                        let init = init();
                                        state.update(cx, |s, cx| {
                                            s.open = true;
                                            s.highlighted = init;
                                            if let Some(ix) = init {
                                                s.scroll.scroll_to_item(ix);
                                            }
                                            cx.notify();
                                        });
                                    }
                                    return;
                                }
                                match key {
                                    "down" => set_hl(step(&entries, hl, 1), cx),
                                    "up" => set_hl(step(&entries, hl, -1), cx),
                                    "home" => set_hl(step(&entries, None, 1), cx),
                                    "end" => set_hl(step(&entries, None, -1), cx),
                                    _ => {
                                        if let Some(SelectEntry::Item(i)) = hl.and_then(|ix| entries.get(ix)) {
                                            pick(&i.value, window, cx);
                                        }
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                });
        }

        let menu = (open && bounds.is_some()).then(|| {
            let b = bounds.unwrap();
            let rows: Vec<AnyElement> = entries
                .iter()
                .enumerate()
                .map(|(ix, entry)| match entry {
                    SelectEntry::Label(text) => div()
                        .px(px(8.0))
                        .py(px(6.0))
                        .text_xs_tw()
                        .text_color(t.muted_foreground)
                        .child(text.clone())
                        .into_any_element(),
                    SelectEntry::Separator => div().h(px(1.0)).mx(px(-4.0)).my(px(4.0)).bg(t.border).into_any_element(),
                    SelectEntry::Item(item) => {
                        let is_selected = value.as_ref() == Some(&item.value);
                        let is_hl = highlighted == Some(ix);
                        let (state, pick, item_value) = (state.clone(), pick.clone(), item.value.clone());
                        div()
                            .id((self.id.clone(), SharedString::from(format!("item-{ix}"))))
                            .relative()
                            .flex()
                            .items_center()
                            .gap(px(8.0))
                            .w_full()
                            .rounded(radius::SM)
                            .py(px(6.0))
                            .pr(px(32.0))
                            .pl(px(8.0))
                            .text_sm_tw()
                            .when(is_hl && !item.disabled, |d| d.bg(t.accent).text_color(t.accent_foreground))
                            .when(item.disabled, |d| d.opacity(0.5))
                            .when(!item.disabled, |d| {
                                d.cursor_default()
                                    .on_mouse_move(move |_, _, cx| {
                                        if state.read(cx).highlighted != Some(ix) {
                                            state.update(cx, |s, cx| {
                                                s.highlighted = Some(ix);
                                                cx.notify();
                                            });
                                        }
                                    })
                                    .on_click(move |_, window, cx| pick(&item_value, window, cx))
                            })
                            .when_some(item.icon.clone(), |d, path| {
                                d.child(svg().path(path).size(px(16.0)).flex_none().text_color(t.muted_foreground))
                            })
                            .child(div().flex_1().min_w_0().child(item.label.clone()))
                            .when(is_selected, |d| {
                                d.child(
                                    div()
                                        .absolute()
                                        .right(px(8.0))
                                        .top_0()
                                        .bottom_0()
                                        .w(px(14.0))
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .child(
                                            svg()
                                                .path(icons::CHECK.path())
                                                .size(px(16.0))
                                                .flex_none()
                                                .text_color(t.muted_foreground),
                                        ),
                                )
                            })
                            .into_any_element()
                    }
                })
                .collect();

            let outside = state.clone();
            let panel = div()
                .id((self.id.clone(), "menu"))
                .occlude()
                .relative()
                .flex()
                .flex_col()
                .min_w(px(128.0).max(b.size.width))
                .max_h(px(320.0))
                .refine_style(&popup_panel(&t))
                .on_mouse_down_out(move |_, _, cx| {
                    outside.update(cx, |s, cx| {
                        s.open = false;
                        cx.notify();
                    });
                })
                .child(
                    div()
                        .id((self.id.clone(), "menu-scroll"))
                        .max_h(px(318.0))
                        .overflow_y_scroll()
                        .track_scroll(&scroll)
                        .p(px(4.0))
                        .children(rows),
                )
                .child(scrollbar_overlay((self.id.clone(), "menu-scrollbar"), &scroll, ScrollbarAxis::Vertical));
            let est: f32 = entries
                .iter()
                .map(|e| match e {
                    SelectEntry::Item(_) => 32.0,
                    SelectEntry::Label(_) => 28.0,
                    SelectEntry::Separator => 9.0,
                })
                .sum::<f32>()
                + 10.0;
            let (corner, pos) = if opens_upward(b, px(est.min(320.0)), window) {
                (Anchor::BottomLeft, point(b.left(), b.top() - px(4.0)))
            } else {
                (Anchor::TopLeft, point(b.left(), b.bottom() + px(4.0)))
            };
            deferred(anchored().anchor(corner).position(pos).snap_to_window_with_margin(px(8.0)).child(panel))
                .with_priority(POPUP_PRIORITY)
        });

        div().flex().when(grows, |d| d.flex_1().min_w_0()).child(trigger).children(menu)
    }
}
