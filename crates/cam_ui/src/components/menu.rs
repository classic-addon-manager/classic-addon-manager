//! Menu model and view shared by `DropdownMenu` and `ContextMenu`.
//!
//! Build the content with [`Menu`] and [`MenuItem`]:
//!
//! ```ignore
//! Menu::new()
//!     .label("My account")
//!     .item(MenuItem::new("Profile").icon(icons::USER).shortcut("Ctrl+P").on_click(|window, cx| {}))
//!     .item(MenuItem::new("Settings").disabled(true))
//!     .separator()
//!     .checkbox("Show hidden", self.show_hidden, move |checked, window, cx| {})
//!     .radio_group("b", [("a", "Alpha"), ("b", "Beta")], |value, window, cx| {})
//!     .submenu("More", |m| m.item(MenuItem::new("Nested")))
//!     .separator()
//!     .item(MenuItem::new("Delete").destructive().icon(icons::TRASH_2).on_click(|window, cx| {}))
//! ```
use std::{cell::Cell, rc::Rc};

use gpui_kit::component::IconNamed;
use gpui_kit::{
    App, AppContext as _, Bounds, Context, DismissEvent, Entity, EventEmitter, FocusHandle, Focusable,
    InteractiveElement as _, IntoElement, MouseDownEvent, ParentElement, Pixels, Render, ScrollHandle, SharedString,
    StatefulInteractiveElement as _, Styled, WeakEntity, Window, canvas, div, prelude::FluentBuilder, px, svg,
};

use super::floating::{
    Align, Floating, MENU_CONTEXT, OverlayCancel, OverlayConfirm, OverlayDown, OverlayLeft, OverlayRight, OverlayUp,
    POPUP_PRIORITY, Placement, Side, ensure_keys,
};
use crate::{
    icons,
    theme::{TokensExt, Tw, TwText, alpha, radius, tw},
};

pub type MenuHandler = Rc<dyn Fn(&mut Window, &mut App)>;
/// Where a submenu's trigger row was painted, read when positioning the submenu.
type TriggerBounds = Rc<Cell<Bounds<Pixels>>>;
/// The submenu's measured size, `None` until it has been laid out once.
type MeasuredSize = Rc<Cell<Option<gpui_kit::Size<Pixels>>>>;

/// How a menu item is coloured.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum MenuItemVariant {
    #[default]
    Default,
    /// Red text, with a red highlight when selected.
    Destructive,
    /// Orange text, with an orange highlight when selected.
    Warning,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ItemKind {
    Normal,
    Checkbox(bool),
    Radio(bool),
}

/// One selectable row: a plain item, a checkbox item, a radio item or a submenu trigger.
#[derive(Clone)]
pub struct MenuItem {
    label: SharedString,
    icon: Option<SharedString>,
    shortcut: Option<SharedString>,
    variant: MenuItemVariant,
    disabled: bool,
    inset: bool,
    keep_open: bool,
    kind: ItemKind,
    on_click: Option<MenuHandler>,
}

impl MenuItem {
    /// Plain item.
    pub fn new(label: impl Into<SharedString>) -> Self {
        Self {
            label: label.into(),
            icon: None,
            shortcut: None,
            variant: MenuItemVariant::Default,
            disabled: false,
            inset: false,
            keep_open: false,
            kind: ItemKind::Normal,
            on_click: None,
        }
    }

    /// Checkbox item: shows a check on the left when `checked`. `on_click` runs on every toggle and the caller owns the
    /// state (the menu is rebuilt each time it opens).
    pub fn checkbox(label: impl Into<SharedString>, checked: bool) -> Self {
        Self { kind: ItemKind::Checkbox(checked), ..Self::new(label) }
    }

    /// Radio item: shows a dot on the left when `selected`.
    pub fn radio(label: impl Into<SharedString>, selected: bool) -> Self {
        Self { kind: ItemKind::Radio(selected), ..Self::new(label) }
    }

    /// Leading 16px icon (muted colour, or the variant colour).
    pub fn icon(mut self, icon: impl IconNamed) -> Self {
        self.icon = Some(icon.path());
        self
    }

    /// Keyboard shortcut text: muted and pushed to the right edge.
    pub fn shortcut(mut self, shortcut: impl Into<SharedString>) -> Self {
        self.shortcut = Some(shortcut.into());
        self
    }

    pub fn variant(mut self, variant: MenuItemVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Shorthand for [`MenuItemVariant::Destructive`].
    pub fn destructive(self) -> Self {
        self.variant(MenuItemVariant::Destructive)
    }

    /// Shorthand for [`MenuItemVariant::Warning`].
    pub fn warning(self) -> Self {
        self.variant(MenuItemVariant::Warning)
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Indents the label by 32px so it lines up with checkbox and radio items.
    pub fn inset(mut self, inset: bool) -> Self {
        self.inset = inset;
        self
    }

    /// Do not close the menu when selected. Checkbox items toggle in place.
    pub fn keep_open(mut self, keep_open: bool) -> Self {
        self.keep_open = keep_open;
        self
    }

    /// Selection handler. Runs after the menu closed, so it may open dialogs.
    pub fn on_click(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_click = Some(Rc::new(handler));
        self
    }

    fn selectable(&self) -> bool {
        !self.disabled
    }
}

enum Entry {
    Item(MenuItem),
    Label { text: SharedString, inset: bool },
    TwoLine { title: SharedString, subtitle: SharedString },
    Separator,
    Sub(MenuItem, Menu),
}

/// Menu content description: the entries of a dropdown or context menu. See the module docs.
pub struct Menu {
    entries: Vec<Entry>,
    min_width: Pixels,
    width: Option<Pixels>,
    max_height: Option<Pixels>,
}

impl Default for Menu {
    fn default() -> Self {
        Self::new()
    }
}

impl FluentBuilder for Menu {}

impl Menu {
    pub fn new() -> Self {
        Self { entries: Vec::new(), min_width: px(128.0), width: None, max_height: None }
    }

    pub fn item(mut self, item: MenuItem) -> Self {
        self.entries.push(Entry::Item(item));
        self
    }

    /// Section label: small medium-weight text, usually above a group of items.
    pub fn label(mut self, text: impl Into<SharedString>) -> Self {
        self.entries.push(Entry::Label { text: text.into(), inset: false });
        self
    }

    /// Like `label`, indented by 32px so it lines up with checkbox and radio items.
    pub fn inset_label(mut self, text: impl Into<SharedString>) -> Self {
        self.entries.push(Entry::Label { text: text.into(), inset: true });
        self
    }

    /// Label with a medium-weight title and a small muted subtitle below it (used for the account label of the user
    /// bar).
    pub fn label_with_subtitle(mut self, title: impl Into<SharedString>, subtitle: impl Into<SharedString>) -> Self {
        self.entries.push(Entry::TwoLine { title: title.into(), subtitle: subtitle.into() });
        self
    }

    /// Thin horizontal line between groups of items.
    pub fn separator(mut self) -> Self {
        self.entries.push(Entry::Separator);
        self
    }

    /// Checkbox item. `on_change` receives the new checked value.
    pub fn checkbox(
        self,
        label: impl Into<SharedString>,
        checked: bool,
        on_change: impl Fn(bool, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.item(MenuItem::checkbox(label, checked).on_click(move |window, cx| on_change(!checked, window, cx)))
    }

    /// Group of radio items, one per `(value, label)`, where `value` is the selected one. `on_change` receives the
    /// chosen value.
    pub fn radio_group<V: Into<SharedString>, L: Into<SharedString>>(
        mut self,
        value: impl Into<SharedString>,
        options: impl IntoIterator<Item = (V, L)>,
        on_change: impl Fn(SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        let value: SharedString = value.into();
        let on_change = Rc::new(on_change);
        for (v, label) in options {
            let v: SharedString = v.into();
            let selected = v == value;
            let on_change = on_change.clone();
            self.entries.push(Entry::Item(
                MenuItem::radio(label, selected).on_click(move |window, cx| on_change(v.clone(), window, cx)),
            ));
        }
        self
    }

    /// Submenu with a plain trigger label.
    pub fn submenu(self, label: impl Into<SharedString>, build: impl FnOnce(Menu) -> Menu) -> Self {
        self.submenu_item(MenuItem::new(label), build)
    }

    /// Submenu with a customised trigger item (icon, disabled, inset...).
    pub fn submenu_item(mut self, trigger: MenuItem, build: impl FnOnce(Menu) -> Menu) -> Self {
        self.entries.push(Entry::Sub(trigger, build(Menu::new())));
        self
    }

    /// Minimum width (default 128px).
    pub fn min_width(mut self, width: Pixels) -> Self {
        self.min_width = width;
        self
    }

    /// Fixed width (for example `px(208.)`).
    pub fn width(mut self, width: Pixels) -> Self {
        self.width = Some(width);
        self
    }

    /// Makes the (root) menu scrollable above this height. Submenus are never scrollable.
    pub fn max_height(mut self, height: Pixels) -> Self {
        self.max_height = Some(height);
        self
    }
}

enum ViewEntry {
    Item(MenuItem),
    Label { text: SharedString, inset: bool },
    TwoLine { title: SharedString, subtitle: SharedString },
    Separator,
    Sub { item: MenuItem, menu: Entity<MenuView>, trigger: TriggerBounds, measured: MeasuredSize },
}

/// The rendered menu (an entity so it keeps hover/keyboard selection). Created by `DropdownMenu`/`ContextMenu`.
pub struct MenuView {
    focus_handle: FocusHandle,
    entries: Vec<ViewEntry>,
    selected: Option<usize>,
    open_sub: Option<usize>,
    parent: Option<WeakEntity<MenuView>>,
    bounds: Bounds<Pixels>,
    /// A press inside this rectangle (the trigger) does not count as "outside" (the trigger toggles itself).
    pub(crate) ignore_outside: Option<Bounds<Pixels>>,
    min_width: Pixels,
    width: Option<Pixels>,
    max_height: Option<Pixels>,
    scroll: ScrollHandle,
}

impl EventEmitter<DismissEvent> for MenuView {}

impl Focusable for MenuView {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl MenuView {
    pub(crate) fn new(menu: Menu, parent: Option<WeakEntity<MenuView>>, cx: &mut Context<Self>) -> Self {
        ensure_keys(cx);
        let this = cx.weak_entity();
        let entries: Vec<ViewEntry> = menu
            .entries
            .into_iter()
            .map(|e| match e {
                Entry::Item(i) => ViewEntry::Item(i),
                Entry::Label { text, inset } => ViewEntry::Label { text, inset },
                Entry::TwoLine { title, subtitle } => ViewEntry::TwoLine { title, subtitle },
                Entry::Separator => ViewEntry::Separator,
                Entry::Sub(item, sub) => {
                    let parent = this.clone();
                    let menu = cx.new(|cx| MenuView::new(sub, Some(parent), cx));
                    ViewEntry::Sub {
                        item,
                        menu,
                        trigger: Rc::new(Cell::new(Bounds::default())),
                        measured: Rc::new(Cell::new(None)),
                    }
                }
            })
            .collect();
        Self {
            focus_handle: cx.focus_handle(),
            entries,
            selected: None,
            open_sub: None,
            parent,
            bounds: Bounds::default(),
            ignore_outside: None,
            min_width: menu.min_width,
            width: menu.width,
            max_height: menu.max_height,
            scroll: ScrollHandle::new(),
        }
    }

    pub(crate) fn focus(&self, window: &mut Window, cx: &mut App) {
        self.focus_handle.focus(window, cx);
    }

    fn is_selectable(&self, ix: usize) -> bool {
        match &self.entries[ix] {
            ViewEntry::Item(i) => i.selectable(),
            ViewEntry::Sub { item, .. } => item.selectable(),
            _ => false,
        }
    }

    fn select(&mut self, ix: Option<usize>, cx: &mut Context<Self>) {
        if self.selected != ix {
            self.selected = ix;
            if self.open_sub.is_some() && self.open_sub != ix {
                self.open_sub = None;
            }
            cx.notify();
        }
    }

    fn move_selection(&mut self, forward: bool, cx: &mut Context<Self>) {
        let n = self.entries.len();
        if n == 0 {
            return;
        }
        let start = match (self.selected, forward) {
            (Some(i), _) => i as isize,
            (None, true) => -1,
            (None, false) => n as isize,
        };
        let step: isize = if forward { 1 } else { -1 };
        let mut i = start;
        for _ in 0..n {
            i = (i + step).rem_euclid(n as isize);
            if self.is_selectable(i as usize) {
                self.select(Some(i as usize), cx);
                return;
            }
        }
    }

    fn open_sub_at(&mut self, ix: usize, focus: Option<&mut Window>, cx: &mut Context<Self>) {
        if let Some(ViewEntry::Sub { menu, .. }) = self.entries.get(ix) {
            self.selected = Some(ix);
            self.open_sub = Some(ix);
            if let Some(window) = focus {
                menu.update(cx, |m, cx| {
                    m.focus(window, cx);
                    m.select(None, cx);
                    m.move_selection(true, cx);
                });
            }
            cx.notify();
        }
    }

    /// Closes the whole menu tree (the root emits `DismissEvent`).
    fn dismiss_all(&mut self, cx: &mut Context<Self>) {
        match self.parent.as_ref().and_then(|p| p.upgrade()) {
            Some(parent) => parent.update(cx, |p, cx| p.dismiss_all(cx)),
            None => cx.emit(DismissEvent),
        }
    }

    fn activate(&mut self, ix: usize, window: &mut Window, cx: &mut Context<Self>) {
        match self.entries.get_mut(ix) {
            Some(ViewEntry::Item(item)) if item.selectable() => {
                let handler = item.on_click.clone();
                if item.keep_open {
                    if let ItemKind::Checkbox(c) = &mut item.kind {
                        *c = !*c;
                    }
                    cx.notify();
                    if let Some(h) = handler {
                        h(window, cx);
                    }
                } else {
                    self.dismiss_all(cx);
                    // after the dismiss effect (focus restore) so handlers may open dialogs and take focus
                    if let Some(h) = handler {
                        window.defer(cx, move |window, cx| h(window, cx));
                    }
                }
            }
            Some(ViewEntry::Sub { item, .. }) if item.selectable() => self.open_sub_at(ix, Some(window), cx),
            _ => {}
        }
    }

    fn contains_point(&self, p: gpui_kit::Point<Pixels>, cx: &App) -> bool {
        if self.bounds.contains(&p) {
            return true;
        }
        match self.open_sub.and_then(|ix| self.entries.get(ix)) {
            Some(ViewEntry::Sub { menu, .. }) => menu.read(cx).contains_point(p, cx),
            _ => false,
        }
    }

    fn on_up(&mut self, _: &OverlayUp, _: &mut Window, cx: &mut Context<Self>) {
        self.move_selection(false, cx);
    }

    fn on_down(&mut self, _: &OverlayDown, _: &mut Window, cx: &mut Context<Self>) {
        self.move_selection(true, cx);
    }

    fn on_right(&mut self, _: &OverlayRight, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(ix) = self.selected
            && matches!(self.entries.get(ix), Some(ViewEntry::Sub { item, .. }) if item.selectable())
        {
            self.open_sub_at(ix, Some(window), cx);
        }
    }

    fn on_left(&mut self, _: &OverlayLeft, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(parent) = self.parent.as_ref().and_then(|p| p.upgrade()) {
            self.selected = None;
            parent.update(cx, |p, cx| {
                p.open_sub = None;
                p.focus(window, cx);
                cx.notify();
            });
        }
    }

    fn on_confirm(&mut self, _: &OverlayConfirm, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(ix) = self.selected {
            self.activate(ix, window, cx);
        }
    }

    fn on_cancel(&mut self, _: &OverlayCancel, _: &mut Window, cx: &mut Context<Self>) {
        self.dismiss_all(cx);
    }

    fn render_item(
        &self,
        ix: usize,
        item: &MenuItem,
        sub: Option<(&Entity<MenuView>, &TriggerBounds, &MeasuredSize)>,
        cx: &mut Context<Self>,
    ) -> gpui_kit::AnyElement {
        let t = cx.tokens();
        let selected = self.selected == Some(ix);
        let is_sub = sub.is_some();
        let sub_open = is_sub && self.open_sub == Some(ix);
        let highlighted = selected || sub_open;

        let (fg, hl_bg, icon_color) = match item.variant {
            MenuItemVariant::Default => (None, t.accent, t.muted_foreground),
            MenuItemVariant::Destructive => (Some(t.destructive), alpha(t.destructive, 0.2), t.destructive),
            MenuItemVariant::Warning => {
                let o = tw(Tw::Orange, 500);
                (Some(o), alpha(tw(Tw::Orange, 400), 0.2), o)
            }
        };
        let hl_fg = fg.unwrap_or(t.accent_foreground);

        let has_indicator = !matches!(item.kind, ItemKind::Normal);
        let left_pad = if has_indicator || item.inset { px(32.0) } else { px(8.0) };

        let mut row = div()
            .id(("shadcn-menu-item", ix))
            .relative()
            .flex()
            .items_center()
            .gap(px(8.0))
            .rounded(radius::SM)
            .pl(left_pad)
            .pr(px(8.0))
            .py(px(6.0))
            .text_sm_tw()
            .whitespace_nowrap()
            .cursor_default()
            .text_color(if highlighted { hl_fg } else { fg.unwrap_or(t.popover_foreground) })
            .when(highlighted, |d| d.bg(hl_bg))
            .when(item.disabled, |d| d.opacity(0.5))
            .when(!item.disabled, |d| {
                d.on_hover(cx.listener(move |this, hovered: &bool, _, cx| {
                    if *hovered {
                        this.select(Some(ix), cx);
                        if this.entries.get(ix).is_some_and(|e| matches!(e, ViewEntry::Sub { .. })) {
                            this.open_sub_at(ix, None, cx);
                        }
                    } else if this.selected == Some(ix) && this.open_sub != Some(ix) {
                        this.select(None, cx);
                    }
                }))
                .on_click(cx.listener(move |this, _, window, cx| {
                    cx.stop_propagation();
                    this.activate(ix, window, cx);
                }))
            });

        // checkbox / radio indicator, absolutely placed in the left padding (8px from the left edge, 14px wide)
        let indicator = match item.kind {
            ItemKind::Checkbox(true) => Some(
                svg()
                    .path(icons::CHECK.path())
                    .size(px(16.0))
                    .flex_none()
                    .text_color(hl_fg_or(highlighted, hl_fg, t.popover_foreground))
                    .into_any_element(),
            ),
            ItemKind::Radio(true) => Some(
                div()
                    .size(px(7.5))
                    .flex_none()
                    .rounded_full()
                    .bg(hl_fg_or(highlighted, hl_fg, t.popover_foreground))
                    .into_any_element(),
            ),
            _ => None,
        };
        if has_indicator {
            row = row.child(
                div()
                    .absolute()
                    .left(px(8.0))
                    .top_0()
                    .h_full()
                    .w(px(14.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .children(indicator),
            );
        }
        if let Some(icon) = &item.icon {
            row = row.child(svg().path(icon.clone()).size(px(16.0)).flex_none().text_color(icon_color));
        }
        row = row.child(div().flex_none().child(item.label.clone()));
        if let Some(shortcut) = &item.shortcut {
            row = row
                .child(div().flex_1().min_w(px(16.0)))
                .child(div().flex_none().text_xs_tw().text_color(t.muted_foreground).child(shortcut.clone()));
        }
        if let Some((menu, trigger, measured)) = sub {
            row = row.child(div().flex_1().min_w(px(8.0))).child(
                svg().path(icons::CHEVRON_RIGHT.path()).size(px(16.0)).flex_none().text_color(if highlighted {
                    hl_fg
                } else {
                    fg.unwrap_or(t.popover_foreground)
                }),
            );
            let trigger_cell = trigger.clone();
            row = row.child(canvas(move |bounds, _, _| trigger_cell.set(bounds), |_, _, _, _| {}).absolute().inset_0());
            if sub_open {
                row = row.child(
                    Floating::new(
                        trigger.get(),
                        measured.clone(),
                        Placement::new(Side::Right, Align::Start, 0.0, -5.0),
                        menu.clone(),
                    )
                    .priority(POPUP_PRIORITY + 1),
                );
            }
        }
        row.into_any_element()
    }
}

fn hl_fg_or(highlighted: bool, hl: gpui_kit::Hsla, normal: gpui_kit::Hsla) -> gpui_kit::Hsla {
    if highlighted { hl } else { normal }
}

impl Render for MenuView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = cx.tokens();
        let is_root = self.parent.is_none();
        let view = cx.entity();
        let max_h = self.max_height;

        let mut items = Vec::with_capacity(self.entries.len());
        for (ix, entry) in self.entries.iter().enumerate() {
            let el = match entry {
                ViewEntry::Item(item) => self.render_item(ix, item, None, cx),
                ViewEntry::Sub { item, menu, trigger, measured } => {
                    self.render_item(ix, item, Some((menu, trigger, measured)), cx)
                }
                ViewEntry::Label { text, inset } => div()
                    .px(px(8.0))
                    .when(*inset, |d| d.pl(px(32.0)))
                    .py(px(6.0))
                    .text_sm_tw()
                    .font_weight(gpui_kit::FontWeight::MEDIUM)
                    .whitespace_nowrap()
                    .text_color(t.popover_foreground)
                    .child(text.clone())
                    .into_any_element(),
                ViewEntry::TwoLine { title, subtitle } => div()
                    .px(px(8.0))
                    .py(px(6.0))
                    .flex()
                    .flex_col()
                    .gap(px(2.0))
                    .whitespace_nowrap()
                    .text_color(t.popover_foreground)
                    .child(div().text_sm_tw().font_weight(gpui_kit::FontWeight::MEDIUM).child(title.clone()))
                    .child(div().text_xs_tw().text_color(t.muted_foreground).child(subtitle.clone()))
                    .into_any_element(),
                ViewEntry::Separator => div().h(px(1.0)).mx(px(-4.0)).my(px(4.0)).bg(t.border).into_any_element(),
            };
            items.push(el);
        }

        let _ = window;
        div()
            .id("shadcn-menu")
            .key_context(MENU_CONTEXT)
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(Self::on_up))
            .on_action(cx.listener(Self::on_down))
            .on_action(cx.listener(Self::on_left))
            .on_action(cx.listener(Self::on_right))
            .on_action(cx.listener(Self::on_confirm))
            .on_action(cx.listener(Self::on_cancel))
            .when(is_root, |d| {
                d.on_mouse_down_out(cx.listener(|this, ev: &MouseDownEvent, _, cx| {
                    if this.ignore_outside.is_some_and(|b| b.contains(&ev.position))
                        || this.contains_point(ev.position, cx)
                    {
                        return;
                    }
                    cx.stop_propagation();
                    this.dismiss_all(cx);
                }))
            })
            .relative()
            .flex()
            .flex_col()
            .min_w(self.min_width)
            .when_some(self.width, |d, w| d.w(w))
            .p(px(4.0))
            .bg(t.popover)
            .text_color(t.popover_foreground)
            .border_1()
            .border_color(t.border)
            .rounded(radius::MD)
            .when(is_root, |d| d.shadow_md())
            .when(!is_root, |d| d.shadow_lg())
            .when_some(max_h, |d, h| d.max_h(h).overflow_y_scroll().track_scroll(&self.scroll))
            .children(items)
            .child(
                canvas(move |bounds, _, cx| view.update(cx, |v, _| v.bounds = bounds), |_, _, _, _| {})
                    .absolute()
                    .inset_0(),
            )
    }
}
