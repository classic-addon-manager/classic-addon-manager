//! Root view: title bar, 220 px sidebar and the active page, plus the dialog backdrop. It also owns the sidebar's
//! account area and the dialogs shown at startup (app update, Ko-fi).
mod app_update_dialog;
mod sidebar;
mod support_dialog;
pub mod titlebar;
mod user_bar;
mod widgets;

use std::rc::Rc;

use cam_ui::{components::dialog, theme::TokensExt};
use gpui_kit::base::Root;
use gpui_kit::{
    AnyView, App, AppContext as _, Context, IntoElement, ParentElement, Render, Styled, Window, deferred, div,
};

use self::sidebar::{OpenUpdateHandler, PageHandler, SidebarProps, sidebar};
use self::titlebar::{TitleBar, TitleBarSlot, clear_slot};
use self::user_bar::UserBar;
use super::pages::{self, PageId};
use crate::state;

pub struct Shell {
    active: PageId,
    started: bool,
    page: Option<AnyView>,
    user_bar: Option<AnyView>,
}

impl Shell {
    pub fn new(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        // re-render when auth/preferences/addon/app state changes (sidebar badge, developer item, ready flag)
        cx.observe(&state::user(cx), |_, _, cx| cx.notify()).detach();
        cx.observe(&state::preferences(cx), |_, _, cx| cx.notify()).detach();
        cx.observe(&state::addons(cx), |_, _, cx| cx.notify()).detach();
        cx.observe(&state::app_info(cx), |_, _, cx| cx.notify()).detach();
        cx.observe_global::<TitleBarSlot>(|_, cx| cx.notify()).detach();
        Self { active: PageId::Dashboard, started: false, page: None, user_bar: None }
    }

    /// Switches page. The title-bar slot is cleared and the page is created fresh.
    pub fn set_active(&mut self, id: PageId, window: &mut Window, cx: &mut Context<Self>) {
        if self.page.is_some() && self.active == id {
            return;
        }
        clear_slot(cx);
        self.active = id;
        self.page = Some(pages::build_page(id, window, cx));
        cx.notify();
    }

    fn start(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.started = true;
        self.page = Some(pages::build_page(self.active, window, cx));
        self.user_bar = Some(cx.new(|cx| UserBar::new(window, cx)).into());
        show_startup_dialogs(window, cx);
    }
}

impl Render for Shell {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = cx.tokens();
        // Render nothing until the stored session is restored and the preferences are loaded.
        if !state::is_ready(cx) {
            // the startup error alert can open before this
            return div().size_full().bg(t.background).children(dialog::backdrop(window, cx)).into_any_element();
        }
        if !self.started {
            self.start(window, cx);
        }
        let authenticated = state::user(cx).read(cx).is_authenticated();
        // sidebar: leave the developer page when logged out
        if !authenticated && self.active == PageId::Developer {
            self.set_active(PageId::Dashboard, window, cx);
        }
        let addons = state::addons(cx).read(cx);
        let app_info = state::app_info(cx).read(cx);
        let slot = cx.try_global::<TitleBarSlot>().and_then(|s| s.0.clone());

        let this = cx.entity();
        let on_select: PageHandler = {
            let this = this.clone();
            Rc::new(move |id, window, cx| this.update(cx, |s, cx| s.set_active(id, window, cx)))
        };
        let on_open_update: OpenUpdateHandler = Rc::new(app_update_dialog::open);

        let sidebar = sidebar(
            SidebarProps {
                active: self.active,
                authenticated,
                updates_available_count: addons.updates_available_count,
                update_available: app_info.update_available(),
                version: app_info.version.clone(),
                user_bar: self.user_bar.clone().unwrap(),
                on_select,
                on_open_update,
            },
            cx,
        );

        let shell = div()
            .size_full()
            .relative()
            .flex()
            .flex_col()
            .overflow_hidden()
            .bg(t.background)
            .text_color(t.foreground)
            .child(TitleBar::new(slot))
            .child(div().flex().flex_1().min_h_0().w_full().child(sidebar).child(
                div().flex().flex_col().flex_1().min_w_0().min_h_0().overflow_hidden().children(self.page.clone()),
            ))
            // dims the page while a dialog is open, dialogs draw above it
            .children(dialog::backdrop(window, cx));
        // Dialogs, sheets, menus and tooltips come from gpui-kit's `Root`, which wraps this view. Our toasts are a
        // `Root` plugin (`toast::init`) drawn above dialogs and sheets.
        // gpui-kit also draws empty full-window layers over our page, so the inspector would pick those instead of
        // our elements. While picking, we draw the page last to put it back on top. Dialogs and toasts still draw above.
        if window.is_inspector_picking(cx) {
            return deferred(shell).with_priority(0).into_any_element();
        }
        shell.into_any_element()
    }
}

/// Checks for an app update and opens its dialog when there is one. Otherwise opens the Ko-fi dialog when it is due.
fn show_startup_dialogs(window: &mut Window, cx: &mut Context<Shell>) {
    let update_found = app_update_dialog::check(cx);
    cx.spawn_in(window, async move |_, cx| {
        if update_found.await {
            let _ = cx.update(app_update_dialog::open);
        } else if let Ok(due) = cx.update(|_, cx| support_dialog::is_due(cx))
            && due.await
        {
            let _ = cx.update(support_dialog::open);
        }
    })
    .detach();
}

/// Switches the shell to `id` from anywhere in the app. The page is created fresh.
pub fn navigate(id: PageId, window: &mut Window, cx: &mut App) {
    let Some(Some(root)) = window.root::<Root>() else { return };
    let Ok(shell) = root.read(cx).view().clone().downcast::<Shell>() else { return };
    shell.update(cx, |shell, cx| shell.set_active(id, window, cx));
}
