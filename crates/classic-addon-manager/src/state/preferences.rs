//! Accent colour, addon view mode and addon sort order (persisted via `[ui]` in config.toml).
use cam_core::config::{AccentColorId, AddonSort, AddonViewMode, Ui};
use cam_ui::theme;
use cam_ui::toast::{self, Toast};
use gpui_kit::Context;

use super::backend;
use super::tasks::{error_message, run_blocking};
use crate::ui::accent;

pub struct PreferencesState {
    pub addon_view_mode: AddonViewMode,
    pub addon_sort: AddonSort,
    pub accent_color: AccentColorId,
    pub hydrated: bool,
}

impl PreferencesState {
    pub fn new() -> Self {
        Self {
            addon_view_mode: AddonViewMode::default(),
            addon_sort: AddonSort::default(),
            accent_color: AccentColorId::default(),
            hydrated: false,
        }
    }

    pub fn set_addon_view_mode(&mut self, mode: AddonViewMode, cx: &mut Context<Self>) {
        self.addon_view_mode = mode;
        cx.notify();
        save(cx, move |ui| ui.addon_view_mode = mode);
    }

    pub fn set_addon_sort(&mut self, sort: AddonSort, cx: &mut Context<Self>) {
        if self.addon_sort == sort {
            return;
        }
        self.addon_sort = sort;
        cx.notify();
        save(cx, move |ui| ui.addon_sort = sort);
    }

    /// Applies the accent immediately (theme) and persists it.
    pub fn set_accent_color(&mut self, id: AccentColorId, cx: &mut Context<Self>) {
        self.accent_color = id;
        theme::apply_accent(cx, accent(id));
        cx.notify();
        save(cx, move |ui| ui.accent_color = id);
    }

    /// Loads persisted preferences and applies the accent.
    pub fn hydrate(&mut self, cx: &mut Context<Self>) {
        if self.hydrated {
            return;
        }
        let backend = backend(cx);
        cx.spawn(async move |this, cx| {
            let prefs = run_blocking(cx, move || backend.config.ui()).await;
            this.update(cx, |this, cx| {
                this.accent_color = prefs.accent_color;
                this.addon_view_mode = prefs.addon_view_mode;
                this.addon_sort = prefs.addon_sort;
                this.hydrated = true;
                theme::apply_accent(cx, accent(this.accent_color));
                cx.notify();
            })
        })
        .detach();
    }
}

/// Writes one preference to config.toml in the background and toasts its failure.
fn save(cx: &mut Context<PreferencesState>, apply: impl FnOnce(&mut Ui) + Send + 'static) {
    let backend = backend(cx);
    cx.spawn(async move |_, cx| {
        if let Err(e) = run_blocking(cx, move || backend.config.update(|_, ui| apply(ui))).await {
            log::error!("Failed to save preferences: {e:#}");
            cx.update(|cx| toast::show(cx, Toast::new("Failed to save preferences", error_message(&e))));
        }
    })
    .detach();
}
