//! Toasts: the short notifications shown over the window. This module holds the queue and its API, `layer` draws
//! them.
//!
//! ```ignore
//! toast::success(cx, "Addon installed");            // title "Success", check icon
//! toast::error(cx, "Failed to save");               // title "Error", alert-triangle icon
//! toast::info(cx, "Heads up");                      // title "Info"
//! toast::show(cx, Toast::new("Update failed", "details").icon(icons::ALERT_TRIANGLE)
//!     .button("Retry", |window, cx| { ... }));      // fully custom, returns ToastId
//! let id = toast::loading(cx, "Installing...");      // spinner, never auto-dismisses
//! toast::success_update(cx, id, "Installed");        // turn a loading toast into a result (auto-dismiss again)
//! toast::dismiss(cx, id);
//! ```
//! Toasts appear at the bottom right, 364 px wide, dismiss themselves after 4 s and have a close button.
mod layer;

use std::{rc::Rc, time::Duration};

use gpui_kit::base::Root;
use gpui_kit::{App, AppContext, Context, Entity, Global, SharedString, Window};
use layer::ToastLayer;

use crate::icons::{self, Lucide};

pub const DEFAULT_DURATION: Duration = Duration::from_millis(4000);
const LEAVE_DURATION: Duration = Duration::from_millis(200);

pub type ToastId = u64;

type ClickFn = Rc<dyn Fn(&mut Window, &mut App)>;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ToastKind {
    Normal,
    Loading,
}

#[derive(Clone)]
pub struct ToastButton {
    pub label: SharedString,
    pub on_click: ClickFn,
}

/// Builder for a toast: `Toast::new(title, description).icon(..).button(..)`.
#[derive(Clone)]
pub struct Toast {
    pub title: SharedString,
    pub description: SharedString,
    pub icon: Option<Lucide>,
    pub button: Option<ToastButton>,
    pub kind: ToastKind,
    pub duration: Option<Duration>,
}

impl Toast {
    pub fn new(title: impl Into<SharedString>, description: impl Into<SharedString>) -> Self {
        Self {
            title: title.into(),
            description: description.into(),
            icon: None,
            button: None,
            kind: ToastKind::Normal,
            duration: Some(DEFAULT_DURATION),
        }
    }
    pub fn icon(mut self, icon: Lucide) -> Self {
        self.icon = Some(icon);
        self
    }
    pub fn button(
        mut self,
        label: impl Into<SharedString>,
        on_click: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        self.button = Some(ToastButton { label: label.into(), on_click: Rc::new(on_click) });
        self
    }
    /// `None` keeps the toast until dismissed.
    pub fn duration(mut self, d: Option<Duration>) -> Self {
        self.duration = d;
        self
    }
}

#[derive(Clone)]
pub struct ToastItem {
    pub id: ToastId,
    pub toast: Toast,
    pub leaving: bool,
}

#[derive(Default)]
pub struct ToastStore {
    pub items: Vec<ToastItem>,
    next_id: ToastId,
}

struct ToastStoreGlobal(Entity<ToastStore>);
impl Global for ToastStoreGlobal {}

/// Creates the queue and mounts the layer that draws it in every window's `Root`. Call after `gpui_kit::init`, before
/// the first window is opened.
pub(crate) fn init(cx: &mut App) {
    let store = cx.new(|_| ToastStore::default());
    cx.set_global(ToastStoreGlobal(store));
    Root::register_plugin::<ToastLayer>(cx, |_, cx| ToastLayer::new(cx));
}

/// The queue, rendered by [`ToastLayer`].
fn store(cx: &App) -> Entity<ToastStore> {
    cx.global::<ToastStoreGlobal>().0.clone()
}

impl ToastStore {
    fn push(&mut self, toast: Toast, cx: &mut Context<Self>) -> ToastId {
        self.next_id += 1;
        let id = self.next_id;
        let duration = toast.duration;
        self.items.push(ToastItem { id, toast, leaving: false });
        self.schedule_dismiss(id, duration, cx);
        cx.notify();
        id
    }

    fn schedule_dismiss(&mut self, id: ToastId, duration: Option<Duration>, cx: &mut Context<Self>) {
        if let Some(d) = duration {
            cx.spawn(async move |this, cx| {
                cx.background_executor().timer(d).await;
                let _ = this.update(cx, |s, cx| s.dismiss(id, cx));
            })
            .detach();
        }
    }

    pub fn dismiss(&mut self, id: ToastId, cx: &mut Context<Self>) {
        let Some(item) = self.items.iter_mut().find(|i| i.id == id) else { return };
        if item.leaving {
            return;
        }
        item.leaving = true;
        cx.notify();
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(LEAVE_DURATION).await;
            let _ = this.update(cx, |s, cx| {
                s.items.retain(|i| i.id != id);
                cx.notify();
            });
        })
        .detach();
    }

    fn replace(&mut self, id: ToastId, toast: Toast, cx: &mut Context<Self>) {
        let duration = toast.duration;
        if let Some(item) = self.items.iter_mut().find(|i| i.id == id && !i.leaving) {
            item.toast = toast;
            self.schedule_dismiss(id, duration, cx);
            cx.notify();
        } else {
            self.push(toast, cx);
        }
    }
}

pub fn show(cx: &mut App, toast: Toast) -> ToastId {
    store(cx).update(cx, |s, cx| s.push(toast, cx))
}

pub fn dismiss(cx: &mut App, id: ToastId) {
    store(cx).update(cx, |s, cx| s.dismiss(id, cx));
}

/// Replace the content of toast `id` (e.g. loading -> result). If it is gone a new toast is shown.
pub fn update(cx: &mut App, id: ToastId, toast: Toast) {
    store(cx).update(cx, |s, cx| s.replace(id, toast, cx));
}

pub fn success(cx: &mut App, description: impl Into<SharedString>) -> ToastId {
    show(cx, Toast::new("Success", description).icon(icons::CHECK))
}

pub fn error(cx: &mut App, description: impl Into<SharedString>) -> ToastId {
    show(cx, Toast::new("Error", description).icon(icons::TRIANGLE_ALERT))
}

pub fn info(cx: &mut App, description: impl Into<SharedString>) -> ToastId {
    show(cx, Toast::new("Info", description).icon(icons::INFO))
}

/// Toast with a custom title and a check icon.
pub fn success_titled(cx: &mut App, title: impl Into<SharedString>, description: impl Into<SharedString>) -> ToastId {
    show(cx, Toast::new(title, description).icon(icons::CHECK))
}

pub fn error_titled(cx: &mut App, title: impl Into<SharedString>, description: impl Into<SharedString>) -> ToastId {
    show(cx, Toast::new(title, description).icon(icons::TRIANGLE_ALERT))
}

/// Spinner toast that stays until `dismiss`/`update`d.
pub fn loading(cx: &mut App, description: impl Into<SharedString>) -> ToastId {
    let mut t = Toast::new("Loading", description).duration(None);
    t.kind = ToastKind::Loading;
    show(cx, t)
}

pub fn success_update(cx: &mut App, id: ToastId, description: impl Into<SharedString>) {
    update(cx, id, Toast::new("Success", description).icon(icons::CHECK));
}

pub fn error_update(cx: &mut App, id: ToastId, description: impl Into<SharedString>) {
    update(cx, id, Toast::new("Error", description).icon(icons::TRIANGLE_ALERT));
}
