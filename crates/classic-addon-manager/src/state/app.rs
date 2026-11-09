//! Application version and app self-update information.
use cam_core::{VERSION, api::ApplicationRelease};
use gpui_kit::Context;

pub struct AppInfoState {
    pub version: String,
    /// The newer release found by the startup check, if any.
    pub update_information: Option<ApplicationRelease>,
    /// Whether this installation can replace itself (Windows or a Linux AppImage).
    pub self_update_supported: bool,
    /// True while a self update downloads and installs.
    pub self_updating: bool,
}

impl AppInfoState {
    pub fn new() -> Self {
        Self {
            version: VERSION.to_string(),
            update_information: None,
            self_update_supported: false,
            self_updating: false,
        }
    }

    pub fn update_available(&self) -> bool {
        self.update_information.is_some()
    }

    pub fn set_update_available(&mut self, info: ApplicationRelease, cx: &mut Context<Self>) {
        self.update_information = Some(info);
        cx.notify();
    }

    pub fn set_self_update_supported(&mut self, supported: bool, cx: &mut Context<Self>) {
        self.self_update_supported = supported;
        cx.notify();
    }

    pub fn set_self_updating(&mut self, updating: bool, cx: &mut Context<Self>) {
        self.self_updating = updating;
        cx.notify();
    }
}
