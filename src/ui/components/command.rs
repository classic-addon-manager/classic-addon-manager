//! Searchable command list, combobox and tags input.
//!
//! * [`Command`]: a search input (36px tall, search icon, bottom border) above a scrollable list (300px tall at most)
//!   with groups and headings, separators, items (small text, the selected row in the accent colour), an empty-state
//!   message, an optional leading check (`.checked(Some(bool))`), icons and shortcuts. It has a fuzzy filter and
//!   arrow / Enter keys. `Command::new(id).items(..).show_input(false).query(text)` is a plain list driven by your own
//!   `Input`.
//! * [`Combobox`]: trigger + popover + [`Command`]. Single or multi select (chips in the trigger, `X` to remove).
//! * [`TagsInput`]: a keyword token field (chips + inline text input), a stateful entity.
use std::{cmp::Ordering, rc::Rc};

use gpui_kit::component::{IconNamed, StyledExt as _, input::Input as RawInput};
use gpui_kit::{
    Anchor, AnyElement, App, AppContext as _, Bounds, ClickEvent, Context, ElementId, Entity, EventEmitter,
    FocusHandle, Focusable, InteractiveElement, IntoElement, KeyDownEvent, MouseButton, ParentElement, Pixels,
    Refineable as _, Render, RenderOnce, ScrollHandle, SharedString, StatefulInteractiveElement, StyleRefinement,
    Styled, Subscription, Window, anchored, deferred, div, point, prelude::FluentBuilder as _, px, svg,
};

use super::{
    input::{FocusVis, InputEvent, InputState, focus_vis, track_focus_vis},
    scroll_area::{ScrollbarAxis, scrollbar_overlay},
    select::{bounds_probe, opens_upward, popup_panel},
};
use crate::ui::{
    icons,
    theme::{Tokens, TokensExt, TwText, alpha, radius, shadow_xs},
};

// ---------------------------------------------------------------------------------------------------------
// Model
// ---------------------------------------------------------------------------------------------------------

/// One row of a [`Command`] list.
#[derive(Clone)]
pub struct CommandItem {
    pub value: SharedString,
    pub label: SharedString,
    /// Extra text matched by the search. The label is always matched.
    pub keywords: SharedString,
    pub icon: Option<SharedString>,
    pub shortcut: Option<SharedString>,
    pub disabled: bool,
    /// `Some(b)`: reserve a leading check column (16px wide), visible when `b` (multi-select lists).
    pub checked: Option<bool>,
}

impl CommandItem {
    pub fn new(value: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self {
            value: value.into(),
            label: label.into(),
            keywords: SharedString::default(),
            icon: None,
            shortcut: None,
            disabled: false,
            checked: None,
        }
    }
    pub fn keywords(mut self, keywords: impl Into<SharedString>) -> Self {
        self.keywords = keywords.into();
        self
    }
    /// Leading 16px icon in the muted colour.
    pub fn icon(mut self, icon: impl IconNamed) -> Self {
        self.icon = Some(icon.path());
        self
    }
    /// Shortcut text, pushed to the right edge in small muted text.
    pub fn shortcut(mut self, shortcut: impl Into<SharedString>) -> Self {
        self.shortcut = Some(shortcut.into());
        self
    }
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = Some(checked);
        self
    }
}

#[derive(Clone)]
pub enum CommandEntry {
    /// Group of items. The heading is small, medium-weight muted text.
    Group { heading: Option<SharedString>, items: Vec<CommandItem> },
    /// Separator line between groups (hidden while searching).
    Separator,
}

enum Row {
    Heading { text: SharedString, gap: f32 },
    Separator,
    Item { item: CommandItem, gap: f32 },
}

impl Row {
    fn enabled(&self) -> bool {
        matches!(self, Row::Item { item, .. } if !item.disabled)
    }
}

/// Lightweight fuzzy score in 0..=1 (0 = no match): prefix > word start > substring > in-order subsequence.
fn score(item: &CommandItem, query: &str) -> f32 {
    let q = query.to_lowercase();
    let hay = format!("{} {}", item.label, item.keywords).to_lowercase();
    if let Some(pos) = hay.find(&q) {
        return if pos == 0 {
            1.0
        } else if hay[..pos].ends_with(' ') {
            0.9
        } else {
            0.8
        };
    }
    let mut it = hay.chars();
    let mut matched = 0usize;
    for qc in q.chars().filter(|c| !c.is_whitespace()) {
        if it.any(|hc| hc == qc) {
            matched += 1;
        } else {
            return 0.0;
        }
    }
    if matched == 0 { 0.0 } else { 0.5 * matched as f32 / hay.chars().count().max(1) as f32 + 0.1 }
}

fn build_rows(entries: &[CommandEntry], query: &str) -> Vec<Row> {
    let searching = !query.trim().is_empty();
    // (best score, heading, items)
    let mut groups: Vec<(f32, Option<SharedString>, Vec<CommandItem>)> = Vec::new();
    let mut seps_after: Vec<usize> = Vec::new(); // group index before which a separator sits
    for e in entries {
        match e {
            CommandEntry::Separator => seps_after.push(groups.len()),
            CommandEntry::Group { heading, items } => {
                if !searching {
                    groups.push((1.0, heading.clone(), items.clone()));
                    continue;
                }
                let mut scored: Vec<(f32, CommandItem)> =
                    items.iter().map(|i| (score(i, query.trim()), i.clone())).filter(|(s, _)| *s > 0.0).collect();
                scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(Ordering::Equal));
                if let Some(best) = scored.first().map(|s| s.0) {
                    groups.push((best, heading.clone(), scored.into_iter().map(|s| s.1).collect()));
                }
            }
        }
    }
    if searching {
        groups.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(Ordering::Equal));
    }
    let mut rows = Vec::new();
    for (gi, (_, heading, items)) in groups.into_iter().enumerate() {
        if !searching && seps_after.contains(&gi) && gi > 0 {
            rows.push(Row::Separator);
        }
        let gap = if rows.is_empty() || matches!(rows.last(), Some(Row::Separator)) { 4.0 } else { 8.0 };
        let mut gap = Some(gap);
        if let Some(h) = heading {
            rows.push(Row::Heading { text: h, gap: gap.take().unwrap_or(0.0) });
        }
        for item in items {
            rows.push(Row::Item { item, gap: gap.take().unwrap_or(0.0) });
        }
    }
    rows
}

fn step_row(rows: &[Row], from: Option<usize>, dir: isize) -> Option<usize> {
    let n = rows.len() as isize;
    let mut i = match from {
        Some(i) => i as isize + dir,
        None if dir > 0 => 0,
        None => n - 1,
    };
    while i >= 0 && i < n {
        if rows[i as usize].enabled() {
            return Some(i as usize);
        }
        i += dir;
    }
    from
}

// ---------------------------------------------------------------------------------------------------------
// Command
// ---------------------------------------------------------------------------------------------------------

type SelectFn = Rc<dyn Fn(&SharedString, &mut Window, &mut App) + 'static>;

struct CommandState {
    input: Entity<InputState>,
    focus: FocusHandle,
    highlighted: Option<usize>,
    last_query: String,
    scroll: ScrollHandle,
}

#[derive(IntoElement)]
pub struct Command {
    id: ElementId,
    entries: Vec<CommandEntry>,
    placeholder: SharedString,
    empty_text: SharedString,
    query: Option<SharedString>,
    show_input: bool,
    focus_on_mount: bool,
    bordered: bool,
    max_height: Pixels,
    on_select: Option<SelectFn>,
    style: StyleRefinement,
}

impl Command {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            entries: Vec::new(),
            placeholder: "Search...".into(),
            empty_text: "No results found.".into(),
            query: None,
            show_input: true,
            focus_on_mount: false,
            bordered: false,
            max_height: px(300.0),
            on_select: None,
            style: StyleRefinement::default(),
        }
    }

    /// Ungrouped rows (one headingless group).
    pub fn items(mut self, items: impl IntoIterator<Item = CommandItem>) -> Self {
        self.entries.push(CommandEntry::Group { heading: None, items: items.into_iter().collect() });
        self
    }

    /// Group of items under a heading.
    pub fn group(mut self, heading: impl Into<SharedString>, items: impl IntoIterator<Item = CommandItem>) -> Self {
        self.entries.push(CommandEntry::Group { heading: Some(heading.into()), items: items.into_iter().collect() });
        self
    }

    pub fn separator(mut self) -> Self {
        self.entries.push(CommandEntry::Separator);
        self
    }

    pub fn entries(mut self, entries: impl IntoIterator<Item = CommandEntry>) -> Self {
        self.entries.extend(entries);
        self
    }

    pub fn placeholder(mut self, placeholder: impl Into<SharedString>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    /// Text shown when nothing matches (default "No results found.").
    pub fn empty_text(mut self, text: impl Into<SharedString>) -> Self {
        self.empty_text = text.into();
        self
    }

    /// Hides the search field (then use [`Command::query`] to filter, or none for a plain keyboard list).
    pub fn show_input(mut self, show: bool) -> Self {
        self.show_input = show;
        self
    }

    /// External search text (overrides the built-in field): drive the filter from your own `Input`.
    pub fn query(mut self, query: impl Into<SharedString>) -> Self {
        self.query = Some(query.into());
        self
    }

    /// Focus the search field when the command is first rendered (popovers).
    pub fn focus_on_mount(mut self, focus: bool) -> Self {
        self.focus_on_mount = focus;
        self
    }

    /// Draws a border, rounded corners and a drop shadow around the whole command (standalone use, a popover already
    /// has them).
    pub fn bordered(mut self, bordered: bool) -> Self {
        self.bordered = bordered;
        self
    }

    /// Maximum height of the scrolling list (default 300px).
    pub fn max_height(mut self, h: Pixels) -> Self {
        self.max_height = h;
        self
    }

    /// Called with the item `value` on click / Enter. Works with `cx.listener(|this, v: &SharedString, w, cx| ..)`.
    pub fn on_select(mut self, f: impl Fn(&SharedString, &mut Window, &mut App) + 'static) -> Self {
        self.on_select = Some(Rc::new(f));
        self
    }
}

impl Styled for Command {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for Command {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = cx.tokens();
        let placeholder = self.placeholder.clone();
        let focus_on_mount = self.focus_on_mount;
        let state =
            window.use_keyed_state((self.id.clone(), "command"), cx, move |window, cx: &mut Context<CommandState>| {
                let input = cx.new(|cx| InputState::new(window, cx).placeholder(placeholder));
                if focus_on_mount {
                    input.update(cx, |i, cx| i.focus(window, cx));
                }
                CommandState {
                    input,
                    focus: cx.focus_handle().tab_stop(true),
                    highlighted: None,
                    last_query: String::new(),
                    scroll: ScrollHandle::new(),
                }
            });
        let (input, focus, scroll) = {
            let s = state.read(cx);
            (s.input.clone(), s.focus.clone(), s.scroll.clone())
        };
        let query: String = match (&self.query, self.show_input) {
            (Some(q), _) => q.to_string(),
            (None, true) => input.read(cx).value().to_string(),
            (None, false) => String::new(),
        };
        let entries = Rc::new(self.entries);
        let rows = build_rows(&entries, &query);

        // reset the highlight when the query changed; keep it valid otherwise
        let highlighted = {
            let s = state.read(cx);
            let valid = s.highlighted.filter(|&i| rows.get(i).is_some_and(Row::enabled));
            if s.last_query != query || valid.is_none() {
                step_row(&rows, None, 1).filter(|&i| rows[i].enabled())
            } else {
                valid
            }
        };
        state.update(cx, |s, _| {
            s.highlighted = highlighted;
            s.last_query = query.clone();
        });

        let select: SelectFn = {
            let on_select = self.on_select.clone();
            Rc::new(move |v, window, cx| {
                if let Some(f) = &on_select {
                    f(v, window, cx);
                }
            })
        };

        let key_handler = {
            let (state, entries, input, select) = (state.clone(), entries.clone(), input.clone(), select.clone());
            let (show_input, ext_query) = (self.show_input, self.query.clone());
            move |ev: &KeyDownEvent, window: &mut Window, cx: &mut App| {
                let key = ev.keystroke.key.as_str();
                if !matches!(key, "down" | "up" | "home" | "end" | "enter") {
                    return;
                }
                let q = match (&ext_query, show_input) {
                    (Some(q), _) => q.to_string(),
                    (None, true) => input.read(cx).value().to_string(),
                    (None, false) => String::new(),
                };
                let rows = build_rows(&entries, &q);
                let cur = state.read(cx).highlighted;
                let next = match key {
                    "down" => step_row(&rows, cur, 1),
                    "up" => step_row(&rows, cur, -1),
                    "home" => step_row(&rows, None, 1),
                    "end" => step_row(&rows, None, -1),
                    _ => {
                        if let Some(Row::Item { item, .. }) = cur.and_then(|i| rows.get(i)) {
                            cx.stop_propagation();
                            select(&item.value, window, cx);
                        }
                        return;
                    }
                };
                cx.stop_propagation();
                state.update(cx, |s, cx| {
                    s.highlighted = next;
                    if let Some(i) = next {
                        s.scroll.scroll_to_item(i);
                    }
                    cx.notify();
                });
            }
        };

        let list_children: Vec<AnyElement> = rows
            .iter()
            .enumerate()
            .map(|(ix, row)| match row {
                Row::Heading { text, gap } => div()
                    .px(px(8.0))
                    .py(px(6.0))
                    .mt(px(*gap))
                    .mx(px(4.0))
                    .text_xs_tw()
                    .font_weight(gpui_kit::FontWeight::MEDIUM)
                    .text_color(t.muted_foreground)
                    .child(text.clone())
                    .into_any_element(),
                Row::Separator => div().h(px(1.0)).bg(t.border).into_any_element(),
                Row::Item { item, gap } => {
                    let is_hl = highlighted == Some(ix);
                    let (state, select, value) = (state.clone(), select.clone(), item.value.clone());
                    div()
                        .id((self.id.clone(), SharedString::from(format!("row-{ix}"))))
                        .relative()
                        .flex()
                        .items_center()
                        .gap(px(8.0))
                        .mt(px(*gap))
                        .mx(px(4.0))
                        .rounded(radius::SM)
                        .px(px(8.0))
                        .py(px(6.0))
                        .text_sm_tw()
                        .when(is_hl, |d| d.bg(t.accent).text_color(t.accent_foreground))
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
                                .on_click(move |_: &ClickEvent, window, cx| select(&value, window, cx))
                        })
                        .when_some(item.checked, |d, checked| {
                            d.child(svg().path(icons::CHECK.path()).size(px(16.0)).flex_none().text_color(if checked {
                                t.primary
                            } else {
                                alpha(t.primary, 0.0)
                            }))
                        })
                        .when_some(item.icon.clone(), |d, path| {
                            d.child(svg().path(path).size(px(16.0)).flex_none().text_color(t.muted_foreground))
                        })
                        .child(div().min_w_0().flex_1().truncate().child(item.label.clone()))
                        .when_some(item.shortcut.clone(), |d, s| {
                            d.child(div().ml_auto().text_xs_tw().text_color(t.muted_foreground).child(s))
                        })
                        .into_any_element()
                }
            })
            .collect();
        let empty = rows.is_empty();

        let mut root = div()
            .id((self.id.clone(), "root"))
            .flex()
            .flex_col()
            .w_full()
            .overflow_hidden()
            .rounded(radius::MD)
            .bg(t.popover)
            .text_color(t.popover_foreground)
            .when(self.bordered, |d| d.refine_style(&popup_panel(&t)));
        root.style().refine(&self.style);
        let root = if self.show_input { root } else { root.track_focus(&focus) };
        root.on_key_down(key_handler)
            .when(self.show_input, |d| {
                d.child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(8.0))
                        .h(px(36.0))
                        .px(px(12.0))
                        .border_b_1()
                        .border_color(t.border)
                        .child(
                            svg()
                                .path(icons::SEARCH.path())
                                .size(px(16.0))
                                .flex_none()
                                .text_color(alpha(t.muted_foreground, 0.5)),
                        )
                        .child(
                            RawInput::new(&input)
                                .appearance(false)
                                .bordered(false)
                                .focus_bordered(false)
                                .h(px(36.0))
                                .p_0()
                                .text_sm_tw()
                                .text_color(t.popover_foreground),
                        ),
                )
            })
            .child(if empty {
                div().py(px(24.0)).text_center().text_sm_tw().child(self.empty_text).into_any_element()
            } else {
                div()
                    .relative()
                    .child(
                        div()
                            .id((self.id.clone(), "list"))
                            .pb(px(4.0))
                            .max_h(self.max_height)
                            .overflow_y_scroll()
                            .track_scroll(&scroll)
                            .children(list_children),
                    )
                    .child(scrollbar_overlay((self.id.clone(), "scrollbar"), &scroll, ScrollbarAxis::Vertical))
                    .into_any_element()
            })
    }
}

// ---------------------------------------------------------------------------------------------------------
// Chips (rounded labels with a remove button)
// ---------------------------------------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq)]
enum ChipVariant {
    Default,
    Secondary,
}

fn chip(
    id: ElementId,
    label: SharedString,
    variant: ChipVariant,
    disabled: bool,
    t: &Tokens,
    on_remove: impl Fn(&mut Window, &mut App) + 'static,
) -> AnyElement {
    let (bg, fg, hover) = match variant {
        ChipVariant::Default => (t.primary, t.primary_foreground, alpha(t.primary_foreground, 0.2)),
        ChipVariant::Secondary => (t.secondary, t.secondary_foreground, alpha(t.background, 0.5)),
    };
    div()
        .flex()
        .flex_none()
        .items_center()
        .max_w_full()
        .gap(px(4.0))
        .rounded_full()
        .border_1()
        .border_color(gpui_kit::transparent_black())
        .bg(bg)
        .text_color(fg)
        .pl(px(8.0))
        .pr(px(4.0))
        .py(px(2.0))
        .text_xs_tw()
        .font_weight(gpui_kit::FontWeight::MEDIUM)
        .whitespace_nowrap()
        .child(div().min_w_0().truncate().child(label))
        .child(
            div()
                .id(id)
                .flex()
                .items_center()
                .justify_center()
                .rounded_full()
                .p(px(2.0))
                .when(!disabled, |d| {
                    d.hover(move |s| s.bg(hover))
                        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                        .on_click(move |_, window, cx| {
                            cx.stop_propagation();
                            on_remove(window, cx)
                        })
                })
                .child(svg().path(icons::X.path()).size(px(12.0)).flex_none().text_color(fg)),
        )
        .into_any_element()
}

// ---------------------------------------------------------------------------------------------------------
// Combobox
// ---------------------------------------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ComboboxSide {
    /// Popover below the trigger (default).
    #[default]
    Bottom,
    /// Popover above the trigger.
    Top,
}

type SelectionFn = Rc<dyn Fn(&Vec<SharedString>, &mut Window, &mut App) + 'static>;
type ActionFn = Rc<dyn Fn(&mut Window, &mut App) + 'static>;
type CommitFn = Rc<dyn Fn(Vec<SharedString>, bool, &mut Window, &mut App)>;

struct ComboState {
    open: bool,
    bounds: Option<Bounds<Pixels>>,
    selected: Vec<SharedString>,
}

/// Popover + search + list. Multi-select shows removable chips in the trigger and single-select shows the selected
/// label.
///
/// ```ignore
/// Combobox::new("deps").multiple(true)
///     .items(self.addons.iter().map(|a| CommandItem::new(a.name.clone(), a.alias.clone())))
///     .selected(self.deps.clone())                    // controlled (omit for uncontrolled)
///     .placeholder("Select addons").search_placeholder("Search addons…").side(ComboboxSide::Top)
///     .on_change(cx.listener(|this, sel: &Vec<SharedString>, _, cx| { this.deps = sel.clone(); cx.notify(); }))
/// ```
#[derive(IntoElement)]
pub struct Combobox {
    id: ElementId,
    items: Vec<CommandItem>,
    selected: Option<Vec<SharedString>>,
    default_selected: Vec<SharedString>,
    multiple: bool,
    placeholder: SharedString,
    search_placeholder: SharedString,
    empty_text: SharedString,
    status: Option<SharedString>,
    status_action: Option<(SharedString, ActionFn)>,
    side: ComboboxSide,
    disabled: bool,
    invalid: bool,
    default_open: bool,
    on_change: Option<SelectionFn>,
    style: StyleRefinement,
}

impl Combobox {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            items: Vec::new(),
            selected: None,
            default_selected: Vec::new(),
            multiple: false,
            placeholder: "Select...".into(),
            search_placeholder: "Search...".into(),
            empty_text: "No results found.".into(),
            status: None,
            status_action: None,
            side: ComboboxSide::Bottom,
            disabled: false,
            invalid: false,
            default_open: false,
            on_change: None,
            style: StyleRefinement::default(),
        }
    }

    pub fn items(mut self, items: impl IntoIterator<Item = CommandItem>) -> Self {
        self.items.extend(items);
        self
    }

    /// Multi-select with chips (default: single select).
    pub fn multiple(mut self, multiple: bool) -> Self {
        self.multiple = multiple;
        self
    }

    /// Controlled selection (values). Update your field in `on_change`.
    pub fn selected(mut self, selected: impl IntoIterator<Item = SharedString>) -> Self {
        self.selected = Some(selected.into_iter().collect());
        self
    }

    /// Initial selection of the uncontrolled mode.
    pub fn default_selected(mut self, selected: impl IntoIterator<Item = SharedString>) -> Self {
        self.default_selected = selected.into_iter().collect();
        self
    }

    /// Trigger text when nothing is selected (`Select addons`).
    pub fn placeholder(mut self, p: impl Into<SharedString>) -> Self {
        self.placeholder = p.into();
        self
    }

    /// Placeholder of the search field (`Search addons…`).
    pub fn search_placeholder(mut self, p: impl Into<SharedString>) -> Self {
        self.search_placeholder = p.into();
        self
    }

    pub fn empty_text(mut self, p: impl Into<SharedString>) -> Self {
        self.empty_text = p.into();
        self
    }

    /// Replaces the trigger by a muted status row (`Loading addons…`, `No addons available.`); chips stay below.
    pub fn status(mut self, text: impl Into<SharedString>) -> Self {
        self.status = Some(text.into());
        self
    }

    /// Small outline action at the right of the status row (`Retry`).
    pub fn status_action(
        mut self,
        label: impl Into<SharedString>,
        f: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        self.status_action = Some((label.into(), Rc::new(f)));
        self
    }

    pub fn side(mut self, side: ComboboxSide) -> Self {
        self.side = side;
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

    /// Starts with the popover open (screenshots / demos).
    pub fn default_open(mut self, open: bool) -> Self {
        self.default_open = open;
        self
    }

    /// Called with the NEW selection (multi: toggled list; single: 0 or 1 values). `cx.listener(|this, v: &Vec<SharedString>, w, cx| ..)`.
    pub fn on_change(mut self, f: impl Fn(&Vec<SharedString>, &mut Window, &mut App) + 'static) -> Self {
        self.on_change = Some(Rc::new(f));
        self
    }
}

impl Styled for Combobox {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for Combobox {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = cx.tokens();
        let (default_open, default_selected) = (self.default_open, self.default_selected.clone());
        let state = window.use_keyed_state((self.id.clone(), "combo"), cx, move |_, _| ComboState {
            open: default_open,
            bounds: None,
            selected: default_selected,
        });
        if let Some(sel) = &self.selected
            && state.read(cx).selected != *sel
        {
            state.update(cx, |s, _| s.selected = sel.clone());
        }
        let vis = focus_vis((self.id.clone(), "focus"), window, cx);
        let handle = vis.read(cx).handle.clone();
        let (open, bounds, selected) = {
            let s = state.read(cx);
            (s.open && !self.disabled, s.bounds, s.selected.clone())
        };
        let ring = !self.disabled && !open && vis.read(cx).visible(window);
        let label_of = |v: &SharedString| -> SharedString {
            self.items.iter().find(|i| i.value == *v).map(|i| i.label.clone()).unwrap_or_else(|| v.clone())
        };

        let commit: CommitFn = {
            let (state, on_change, handle) = (state.clone(), self.on_change.clone(), handle.clone());
            Rc::new(move |next, close, window, cx| {
                state.update(cx, |s, cx| {
                    s.selected = next.clone();
                    if close {
                        s.open = false;
                    }
                    cx.notify();
                });
                if let Some(f) = &on_change {
                    f(&next, window, cx);
                }
                if close {
                    window.focus(&handle, cx);
                }
            })
        };

        let (border, shadows) = match (self.invalid, ring) {
            (true, _) => (t.destructive, shadow_xs()),
            (false, true) => (t.ring, shadow_xs()),
            (false, false) => (t.input, shadow_xs()),
        };

        let chips: Vec<AnyElement> = if self.multiple {
            selected
                .iter()
                .enumerate()
                .map(|(i, v)| {
                    let (commit, selected) = (commit.clone(), selected.clone());
                    let v = v.clone();
                    chip(
                        (self.id.clone(), SharedString::from(format!("chip-{i}"))).into(),
                        label_of(&v),
                        ChipVariant::Default,
                        self.disabled,
                        &t,
                        move |window, cx| {
                            commit(selected.iter().filter(|s| **s != v).cloned().collect(), false, window, cx)
                        },
                    )
                })
                .collect()
        } else {
            Vec::new()
        };

        let toggle_open = {
            let state = state.clone();
            move |_: &mut Window, cx: &mut App| {
                state.update(cx, |s, cx| {
                    s.open = !open;
                    cx.notify();
                })
            }
        };

        let status_row = self.status.clone().map(|text| {
            div()
                .flex()
                .h(px(36.0))
                .items_center()
                .justify_between()
                .gap(px(8.0))
                .px(px(12.0))
                .child(div().text_sm_tw().text_color(t.muted_foreground).child(text))
                .when_some(self.status_action.clone(), |d, (label, f)| {
                    d.child(
                        div()
                            .id((self.id.clone(), "status-action"))
                            .flex()
                            .items_center()
                            .h(px(32.0))
                            .px(px(12.0))
                            .rounded(radius::MD)
                            .border_1()
                            .border_color(t.input)
                            .bg(alpha(t.input, 0.3))
                            .text_sm_tw()
                            .font_weight(gpui_kit::FontWeight::MEDIUM)
                            .hover(move |s| s.bg(alpha(t.input, 0.5)))
                            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                            .on_click(move |_, window, cx| f(window, cx))
                            .child(label),
                    )
                })
        });
        let has_status = status_row.is_some();

        let body: AnyElement = if has_status {
            div()
                .flex()
                .flex_col()
                .w_full()
                .children(status_row)
                .when(!chips.is_empty(), |d| {
                    d.child(div().flex().flex_wrap().gap(px(6.0)).px(px(12.0)).pb(px(8.0)).children(chips))
                })
                .into_any_element()
        } else {
            let display_empty = selected.is_empty();
            div()
                .relative()
                .flex()
                .w_full()
                .min_h(px(36.0))
                .child(
                    div()
                        .flex()
                        .flex_1()
                        .min_w_0()
                        .flex_wrap()
                        .items_center()
                        .gap(px(6.0))
                        .py(px(if self.multiple && !display_empty { 8.0 } else { 0.0 }))
                        .pl(px(12.0))
                        .pr(px(36.0))
                        .text_sm_tw()
                        .when(display_empty, |d| {
                            d.h(px(36.0)).text_color(t.muted_foreground).child(self.placeholder.clone())
                        })
                        .when(!display_empty && !self.multiple, |d| {
                            d.h(px(36.0)).child(div().truncate().child(label_of(&selected[0])))
                        })
                        .children(chips),
                )
                .child(
                    div()
                        .absolute()
                        .top_0()
                        .right_0()
                        .w(px(36.0))
                        .h(px(36.0))
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(
                            svg()
                                .path(icons::CHEVRONS_UP_DOWN.path())
                                .size(px(16.0))
                                .flex_none()
                                .text_color(alpha(t.foreground, 0.5)),
                        ),
                )
                .into_any_element()
        };

        let mut root = div()
            .id((self.id.clone(), "trigger"))
            .relative()
            .flex()
            .flex_col()
            .w_full()
            .min_h(px(36.0))
            .rounded(radius::MD)
            .border_1()
            .border_color(border)
            .bg(alpha(t.input, 0.3))
            .shadow(shadows)
            .child(body)
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
        root.style().refine(&self.style);
        if self.disabled {
            root = root.opacity(0.5).cursor_not_allowed();
        } else if !has_status {
            root = track_focus_vis(root, &vis, window, cx).on_mouse_down(MouseButton::Left, {
                let toggle_open = toggle_open.clone();
                move |_, window, cx| toggle_open(window, cx)
            });
            root = root.on_key_down({
                let toggle_open = toggle_open.clone();
                move |ev: &KeyDownEvent, window, cx| {
                    if !open && matches!(ev.keystroke.key.as_str(), "enter" | "space" | "down") {
                        cx.stop_propagation();
                        toggle_open(window, cx);
                    }
                }
            });
        }

        let popup = (open && !has_status && bounds.is_some()).then(|| {
            let b = bounds.unwrap();
            let outside = state.clone();
            let esc = (state.clone(), handle.clone());
            let items: Vec<CommandItem> = self
                .items
                .iter()
                .cloned()
                .map(|mut i| {
                    if self.multiple {
                        i.checked = Some(selected.contains(&i.value));
                    } else if selected.contains(&i.value) {
                        i.checked = Some(true);
                    } else {
                        i.checked = Some(false);
                    }
                    i
                })
                .collect();
            let (commit, multiple, sel_now) = (commit.clone(), self.multiple, selected.clone());
            let panel = div()
                .id((self.id.clone(), "popup"))
                .occlude()
                .w(b.size.width)
                .overflow_hidden()
                .refine_style(&popup_panel(&t))
                .on_mouse_down_out(move |_, _, cx| {
                    outside.update(cx, |s, cx| {
                        s.open = false;
                        cx.notify();
                    });
                })
                .on_key_down(move |ev: &KeyDownEvent, window, cx| {
                    if ev.keystroke.key == "escape" {
                        cx.stop_propagation();
                        esc.0.update(cx, |s, cx| {
                            s.open = false;
                            cx.notify();
                        });
                        window.focus(&esc.1, cx);
                    }
                })
                .child(
                    Command::new((self.id.clone(), "cmd"))
                        .items(items)
                        .placeholder(self.search_placeholder.clone())
                        .empty_text(self.empty_text.clone())
                        .focus_on_mount(true)
                        .on_select(move |value, window, cx| {
                            let mut next = sel_now.clone();
                            if let Some(pos) = next.iter().position(|v| v == value) {
                                next.remove(pos);
                            } else if multiple {
                                next.push(value.clone());
                            } else {
                                next = vec![value.clone()];
                            }
                            commit(next, !multiple, window, cx);
                        }),
                );
            let est = px(36.0 + (self.items.len() as f32 * 32.0 + 12.0).min(300.0));
            let up = match self.side {
                ComboboxSide::Top => true,
                ComboboxSide::Bottom => opens_upward(b, est, window),
            };
            let (corner, pos) = if up {
                (Anchor::BottomLeft, point(b.left(), b.top() - px(4.0)))
            } else {
                (Anchor::TopLeft, point(b.left(), b.bottom() + px(4.0)))
            };
            deferred(anchored().anchor(corner).position(pos).snap_to_window_with_margin(px(8.0)).child(panel))
                .with_priority(super::floating::POPUP_PRIORITY)
        });

        div().w_full().relative().child(root).children(popup)
    }
}

// ---------------------------------------------------------------------------------------------------------
// TagsInput
// ---------------------------------------------------------------------------------------------------------

/// Emitted by [`TagsInput`] when the token list changes.
pub struct TagsChanged(pub Vec<SharedString>);

/// Keyword token field: secondary chips with an `X`, followed by a borderless text input at least 128px wide.
/// Space, Enter and blur commit the draft, Backspace on an empty draft removes the last chip.
///
/// A stateful entity: create once, render with `.child(entity.clone())`, subscribe to [`TagsChanged`]:
/// ```ignore
/// self.tags = cx.new(|cx| TagsInput::new(window, cx, "Add keyword"));
/// cx.subscribe(&self.tags, |this, _, ev: &TagsChanged, cx| { this.keywords = ev.0.clone(); cx.notify(); }).detach();
/// // read/set: self.tags.read(cx).values(); self.tags.update(cx, |t, cx| t.set_values(vec![..], cx));
/// ```
pub struct TagsInput {
    values: Vec<SharedString>,
    input: Entity<InputState>,
    disabled: bool,
    invalid: bool,
    style: StyleRefinement,
    _subs: Vec<Subscription>,
}

impl EventEmitter<TagsChanged> for TagsInput {}

impl TagsInput {
    pub fn new(window: &mut Window, cx: &mut Context<Self>, placeholder: impl Into<SharedString>) -> Self {
        let placeholder = placeholder.into();
        let input = cx.new(|cx| InputState::new(window, cx).placeholder(placeholder));
        let sub = cx.subscribe_in(&input, window, |this: &mut Self, input, ev: &InputEvent, window, cx| match ev {
            InputEvent::Change => {
                let text = input.read(cx).value().to_string();
                if text.chars().any(char::is_whitespace) {
                    let ends_ws = text.ends_with(char::is_whitespace);
                    let mut parts: Vec<&str> = text.split_whitespace().collect();
                    let pending = if ends_ws { "" } else { parts.pop().unwrap_or("") }.to_string();
                    let added: Vec<SharedString> =
                        parts.into_iter().map(|s| SharedString::from(s.to_string())).collect();
                    this.values.extend(added);
                    input.update(cx, |i, cx| i.set_value(pending, window, cx));
                    cx.emit(TagsChanged(this.values.clone()));
                    cx.notify();
                }
            }
            InputEvent::PressEnter { .. } | InputEvent::Blur => this.commit_draft(window, cx),
            _ => {}
        });
        Self {
            values: Vec::new(),
            input,
            disabled: false,
            invalid: false,
            style: StyleRefinement::default(),
            _subs: vec![sub],
        }
    }

    pub fn values(&self) -> &[SharedString] {
        &self.values
    }

    pub fn set_values(&mut self, values: Vec<SharedString>, cx: &mut Context<Self>) {
        self.values = values;
        cx.notify();
    }

    pub fn set_disabled(&mut self, disabled: bool, cx: &mut Context<Self>) {
        self.disabled = disabled;
        cx.notify();
    }

    pub fn set_invalid(&mut self, invalid: bool, cx: &mut Context<Self>) {
        self.invalid = invalid;
        cx.notify();
    }

    fn commit_draft(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let draft = self.input.read(cx).value().trim().to_string();
        if draft.is_empty() {
            return;
        }
        self.values.push(draft.into());
        self.input.update(cx, |i, cx| i.set_value("", window, cx));
        cx.emit(TagsChanged(self.values.clone()));
        cx.notify();
    }

    fn remove(&mut self, ix: usize, cx: &mut Context<Self>) {
        if ix < self.values.len() {
            self.values.remove(ix);
            cx.emit(TagsChanged(self.values.clone()));
            cx.notify();
        }
    }
}

impl Styled for TagsInput {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl Render for TagsInput {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = cx.tokens();
        let focused = self.input.focus_handle(cx).is_focused(window);
        let (border, shadows) = match (self.invalid, focused && !self.disabled) {
            (true, _) => (t.destructive, shadow_xs()),
            (false, true) => (t.ring, shadow_xs()),
            (false, false) => (t.input, shadow_xs()),
        };
        let entity = cx.entity();
        let input_state = self.input.clone();
        let disabled = self.disabled;
        let mut root = div()
            .id("tags-input")
            .flex()
            .flex_wrap()
            .items_center()
            .gap(px(6.0))
            .w_full()
            .min_h(px(36.0))
            .rounded(radius::MD)
            .border_1()
            .border_color(border)
            .bg(alpha(t.input, 0.3))
            .px(px(12.0))
            .py(px(4.0))
            .shadow(shadows)
            .when(disabled, |d| d.opacity(0.5).cursor_not_allowed())
            // the editor consumes Backspace as an action, so intercept it in the capture phase
            .capture_action(cx.listener(|this, _: &gpui_kit::component::input::Backspace, _window, cx| {
                if this.input.read(cx).value().is_empty() && !this.values.is_empty() && !this.disabled {
                    let last = this.values.len() - 1;
                    this.remove(last, cx);
                    cx.stop_propagation();
                }
            }));
        root.style().refine(&self.style);
        let chips = self.values.iter().cloned().enumerate().map(|(i, v)| {
            let entity = entity.clone();
            chip(
                SharedString::from(format!("tag-{i}")).into(),
                v,
                ChipVariant::Secondary,
                disabled,
                &t,
                move |_, cx| entity.update(cx, |this, cx| this.remove(i, cx)),
            )
        });
        root.children(chips).child(
            div().min_w(px(128.0)).flex_1().child(
                RawInput::new(&input_state)
                    .appearance(false)
                    .bordered(false)
                    .focus_bordered(false)
                    .disabled(disabled)
                    .h(px(28.0))
                    .p_0()
                    .text_sm_tw()
                    .text_color(t.foreground),
            ),
        )
    }
}
