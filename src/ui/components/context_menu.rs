//! Context menu: a menu of actions that opens at the pointer when its children are right-clicked.
//!
//! ```ignore
//! ContextMenu::new()
//!     .id(("row", ix))
//!     .menu(|m| m.item(MenuItem::new("Update").on_click(|window, cx| {})).separator()
//!                .item(MenuItem::new("Uninstall").destructive().on_click(|window, cx| {})))
//!     .child(row_element)
//! ```
//! The wrapper div hosts the children and is the right-click region (style it with `.w_full()` etc.). The menu opens at
//! the pointer and flips up or left when it does not fit. Clicking elsewhere, Escape or selecting an item closes it.
//! Menu content is the same [`Menu`] as `DropdownMenu` (items, icons, shortcuts, separators, checkbox/radio, submenus).
//! Give `.id(..)` when created in a loop (otherwise the id is the constructor's call site).
use std::{panic::Location, rc::Rc};

use super::style_ext::StyleMergeExt as _;
use gpui_kit::{
    AnyElement, App, Bounds, ElementId, InteractiveElement as _, IntoElement, MouseButton, ParentElement, Pixels,
    Point, RenderOnce, Size, StyleRefinement, Styled, Window, div, prelude::FluentBuilder as _,
};

use super::{
    dropdown_menu::{MenuBuilder, OpenChange, close_menu, open_menu},
    floating::{Align, FloatState, Floating, Placement, Side, element_id, ensure_keys},
    menu::Menu,
};

#[derive(IntoElement)]
pub struct ContextMenu {
    id: ElementId,
    menu: Option<MenuBuilder>,
    disabled: bool,
    default_open_at: Option<Point<Pixels>>,
    on_open_change: Option<OpenChange>,
    style: StyleRefinement,
    children: Vec<AnyElement>,
}

impl ContextMenu {
    #[track_caller]
    pub fn new() -> Self {
        Self {
            id: element_id(None, Location::caller()),
            menu: None,
            disabled: false,
            default_open_at: None,
            on_open_change: None,
            style: StyleRefinement::default(),
            children: Vec::new(),
        }
    }

    pub fn id(mut self, id: impl Into<ElementId>) -> Self {
        self.id = id.into();
        self
    }

    /// Menu content, built on every right click (so it can reflect the current state).
    pub fn menu(mut self, build: impl Fn(Menu) -> Menu + 'static) -> Self {
        self.menu = Some(Rc::new(build));
        self
    }

    /// Disables the menu: right clicks are ignored and the children are still shown.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Starts open with the menu at this window position (visual tests).
    pub fn default_open_at(mut self, position: Point<Pixels>) -> Self {
        self.default_open_at = Some(position);
        self
    }

    pub fn on_open_change(mut self, handler: impl Fn(bool, &mut Window, &mut App) + 'static) -> Self {
        self.on_open_change = Some(Rc::new(handler));
        self
    }
}

impl Styled for ContextMenu {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl ParentElement for ContextMenu {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for ContextMenu {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        ensure_keys(cx);
        let initial = self.default_open_at;
        let state = window.use_keyed_state(self.id.clone(), cx, move |_, cx| {
            let mut s = FloatState::new(initial.is_some(), cx);
            if let Some(origin) = initial {
                s.anchor = Bounds { origin, size: Size::default() };
            }
            s
        });
        let build: MenuBuilder = self.menu.unwrap_or_else(|| Rc::new(|m| m));
        let on_change = self.on_open_change;
        if state.read(cx).open && state.read(cx).menu.is_none() {
            open_menu(&state, &build, &on_change, false, window, cx);
        }
        let (open, anchor, measured, menu) = {
            let s = state.read(cx);
            (s.open, s.anchor, s.measured.clone(), s.menu.clone())
        };
        let disabled = self.disabled;

        div()
            .id(self.id)
            .merge_style(&self.style)
            .children(self.children)
            .when(!disabled, |d| {
                let state = state.clone();
                let build = build.clone();
                let on_change = on_change.clone();
                d.on_mouse_down(MouseButton::Right, move |ev, window, cx| {
                    if open {
                        close_menu(&state, &on_change, true, window, cx);
                    }
                    state.update(cx, |s, _| s.anchor = Bounds { origin: ev.position, size: Size::default() });
                    open_menu(&state, &build, &on_change, true, window, cx);
                })
            })
            .when_some(menu.filter(|_| open), |d, menu| {
                d.child(Floating::new(anchor, measured, Placement::new(Side::Bottom, Align::Start, 0.0, 0.0), menu))
            })
    }
}
