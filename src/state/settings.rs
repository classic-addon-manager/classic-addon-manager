//! The `[general]` values of config.toml.
use gpui_kit::Context;

use super::backend;
use super::tasks::{error_message, run_blocking};
use super::toast::{self, Toast};

pub struct SettingsState {
    pub auto_path_detection: bool,
    pub aac_path: String,
    pub is_initialized: bool,
    pub is_loading: bool,
    pub load_error: Option<String>,
}

impl SettingsState {
    pub fn new() -> Self {
        Self {
            auto_path_detection: true,
            aac_path: String::new(),
            is_initialized: false,
            is_loading: false,
            load_error: None,
        }
    }

    /// Reads `[general] autodetectpath/aacpath`. Does nothing once loaded or while loading.
    pub fn load_config(&mut self, cx: &mut Context<Self>) {
        if self.is_initialized || self.is_loading {
            return;
        }
        self.is_loading = true;
        self.load_error = None;
        cx.notify();
        let backend = backend(cx);
        cx.spawn(async move |this, cx| {
            let general = run_blocking(cx, move || backend.config.general()).await;
            this.update(cx, |this, cx| {
                this.auto_path_detection = general.auto_detect_path;
                this.aac_path = general.aac_path;
                this.is_initialized = true;
                this.is_loading = false;
                cx.notify();
            })
        })
        .detach();
    }

    /// Updates the value immediately, then persists it and shows a toast if saving fails.
    pub fn set_auto_path_detection(&mut self, enabled: bool, cx: &mut Context<Self>) {
        self.auto_path_detection = enabled;
        cx.notify();
        let backend = backend(cx);
        cx.spawn(async move |_, cx| {
            let save = move || backend.config.update(|general, _| general.auto_detect_path = enabled);
            if let Err(e) = run_blocking(cx, save).await {
                log::error!("Failed to save settings: {e:#}");
                cx.update(|cx| toast::show(cx, Toast::new("Failed to save settings", error_message(&e))));
            }
        })
        .detach();
    }

    /// Sets the path in memory only: `Config::set_aac_path_checked` persists it.
    pub fn set_aac_path(&mut self, path: String, cx: &mut Context<Self>) {
        self.aac_path = path;
        cx.notify();
    }
}
