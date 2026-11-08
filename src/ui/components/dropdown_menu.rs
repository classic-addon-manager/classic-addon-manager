//! Dropdown menu: a menu of actions that opens next to a trigger element.
//!
//! ```ignore
//! DropdownMenu::new()
//!     .trigger(Button::new("more").variant(ButtonVariant::Outline).child("Actions"))
//!     .side(Side::Bottom).align(Align::End).side_offset(4.0)
//!     .menu(|m| m
//!         .label("Addon")
//!         .item(MenuItem::new("Open folder").icon(icons::FOLDER_OPEN).on_click(|window, cx| {}))
//!         .separator()
//!         .item(MenuItem::new("Uninstall").destructive().on_click(|window, cx| {})))
//! ```
//! Uncontrolled by default: clicking the trigger toggles, click outside, Escape or selecting an item closes the menu
//! and focus returns to where it was. Controlled: `.open(bool)` + `.on_open_change(|open, window, cx| ..)`.
//! `.default_open(true)` starts open. Give `.id(..)` when several menus are created from the same code location
//! (loops). Otherwise the id is derived from the constructor's call site.
use std::{panic::Location, rc::Rc};

use super::style_ext::StyleMergeExt as _;
use gpui_kit::{
    AnyElement, App, AppContext as _, DismissEvent, ElementId, Entity, InteractiveElement as _, IntoElement,
    MouseButton, ParentElement, RenderOnce, StyleRefinement, Styled, Window, div, prelude::FluentBuilder as _,
};

use super::{
    floating::{Align, FloatState, Floating, Placement, Side, element_id, ensure_keys, track_bounds},
    menu::{Menu, MenuView},
};

pub(crate) type MenuBuilder = Rc<dyn Fn(Menu) -> Menu>;
pub(crate) type OpenChange = Rc<dyn Fn(bool, &mut Window, &mut App)>;

/// Builds the menu entity, subscribes to its dismissal and focuses it.
pub(crate) fn open_menu(
    state: &Entity<FloatState>,
    build: &MenuBuilder,
    on_change: &Option<OpenChange>,
    emit: bool,
    window: &mut Window,
    cx: &mut App,
) {
    let view = cx.new(|cx| MenuView::new(build(Menu::new()), None, cx));
    let (st, oc) = (state.clone(), on_change.clone());
    let sub = window.subscribe(&view, cx, move |_, _: &DismissEvent, window, cx| {
        close_menu(&st, &oc, true, window, cx);
    });
    let prev = window.focused(cx);
    // focus once the current mouse-down finished (focusable ancestors re-focus themselves while it bubbles)
    let focus_view = view.clone();
    window.defer(cx, move |window, cx| focus_view.update(cx, |m, cx| m.focus(window, cx)));
    state.update(cx, |s, cx| {
        s.open = true;
        s.menu = Some(view);
        s.prev_focus = prev;
        s._subscription = Some(sub);
        cx.notify();
    });
    if emit && let Some(oc) = on_change {
        oc(true, window, cx);
    }
}

/// Drops the menu entity, restores focus and notifies `on_change`.
pub(crate) fn close_menu(
    state: &Entity<FloatState>,
    on_change: &Option<OpenChange>,
    emit: bool,
    window: &mut Window,
    cx: &mut App,
) {
    let (was_open, prev) = state.update(cx, |s, cx| {
        let was_open = s.open;
        s.open = false;
        s.menu = None;
        s._subscription = None;
        cx.notify();
        (was_open, s.prev_focus.take())
    });
    if !was_open {
        return;
    }
    if let Some(prev) = prev {
        window.focus(&prev, cx);
    }
    if emit && let Some(oc) = on_change {
        oc(false, window, cx);
    }
}

#[derive(IntoElement)]
pub struct DropdownMenu {
    id: ElementId,
    trigger: Option<AnyElement>,
    menu: Option<MenuBuilder>,
    side: Side,
    align: Align,
    side_offset: f32,
    align_offset: f32,
    open: Option<bool>,
    default_open: bool,
    on_open_change: Option<OpenChange>,
    style: StyleRefinement,
    children: Vec<AnyElement>,
}

impl DropdownMenu {
    #[track_caller]
    pub fn new() -> Self {
        Self {
            id: element_id(None, Location::caller()),
            trigger: None,
            menu: None,
            side: Side::Bottom,
            align: Align::Start,
            side_offset: 4.0,
            align_offset: 0.0,
            open: None,
            default_open: false,
            on_open_change: None,
            style: StyleRefinement::default(),
            children: Vec::new(),
        }
    }

    pub fn id(mut self, id: impl Into<ElementId>) -> Self {
        self.id = id.into();
        self
    }

    /// The element that opens the menu when pressed (a `Button`, an icon, any element).
    pub fn trigger(mut self, trigger: impl IntoElement) -> Self {
        self.trigger = Some(trigger.into_any_element());
        self
    }

    /// Menu content, built every time the menu opens.
    pub fn menu(mut self, build: impl Fn(Menu) -> Menu + 'static) -> Self {
        self.menu = Some(Rc::new(build));
        self
    }

    /// Which side of the trigger the menu opens on (default `Bottom`). It flips automatically when it does not fit.
    pub fn side(mut self, side: Side) -> Self {
        self.side = side;
        self
    }

    /// How the menu lines up along the trigger's edge (default `Start`).
    pub fn align(mut self, align: Align) -> Self {
        self.align = align;
        self
    }

    /// Gap between the trigger and the menu in px (default 4).
    pub fn side_offset(mut self, offset: f32) -> Self {
        self.side_offset = offset;
        self
    }

    pub fn align_offset(mut self, offset: f32) -> Self {
        self.align_offset = offset;
        self
    }

    /// Controlled open state (pair with `on_open_change`).
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
}

impl Styled for DropdownMenu {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl ParentElement for DropdownMenu {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for DropdownMenu {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        ensure_keys(cx);
        let default_open = self.default_open;
        let state = window.use_keyed_state(self.id.clone(), cx, move |_, cx| FloatState::new(default_open, cx));
        let build: MenuBuilder = self.menu.unwrap_or_else(|| Rc::new(|m| m));
        let on_change = self.on_open_change;
        let controlled = self.open.is_some();

        // controlled / default-open sync (no change events: these are not user interactions)
        let (is_open, has_menu) = {
            let s = state.read(cx);
            (s.open, s.menu.is_some())
        };
        let want = self.open.unwrap_or(is_open);
        if want && !has_menu {
            open_menu(&state, &build, &on_change, false, window, cx);
        } else if !want && is_open {
            close_menu(&state, &on_change, false, window, cx);
        }

        let (open, anchor, measured, menu) = {
            let s = state.read(cx);
            (s.open, s.anchor, s.measured.clone(), s.menu.clone())
        };
        if let Some(menu) = &menu {
            menu.update(cx, |m, _| m.ignore_outside = Some(anchor));
        }

        let on_mouse_down = {
            let (state, build, on_change) = (state.clone(), build.clone(), on_change.clone());
            move |_: &gpui_kit::MouseDownEvent, window: &mut Window, cx: &mut App| {
                if controlled {
                    if let Some(oc) = &on_change {
                        oc(!open, window, cx);
                    }
                } else if open {
                    close_menu(&state, &on_change, true, window, cx);
                } else {
                    open_menu(&state, &build, &on_change, true, window, cx);
                }
            }
        };

        div()
            .id(self.id)
            .merge_style(&self.style)
            .children(self.trigger)
            .children(self.children)
            .child(track_bounds(state.clone()))
            .on_mouse_down(MouseButton::Left, on_mouse_down)
            .when_some(menu.filter(|_| open), |d, menu| {
                d.child(Floating::new(
                    anchor,
                    measured,
                    Placement::new(self.side, self.align, self.side_offset, self.align_offset),
                    menu,
                ))
            })
    }
}
