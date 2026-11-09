//! Everything Classic Addon Manager does that is not UI: the installed addons, the server API, the session, the
//! settings, self-update, the single-instance IPC and the log. Nothing here depends on GPUI: the app runs these calls
//! off the UI thread, and the `--check-updates` mode runs them without a window.
pub mod addon;
pub mod api;
pub mod app_state;
pub mod auth;
pub mod config;
pub mod diagnostics;
pub mod file;
pub mod headless;
pub mod ipc;
pub mod logger;
pub mod paths;
pub mod startup;
pub mod updater;
pub mod version;

mod download;
#[cfg(windows)]
mod process;
mod serde_util;
mod throttle;
mod versioned_json;

#[cfg(test)]
mod tests;
#[cfg(test)]
mod testutil;

use std::sync::Mutex;

use addon::AddonDir;
use anyhow::Result;
use config::Config;
use paths::Paths;

/// Application version: single source of truth is `[workspace.package] version` in Cargo.toml (`just version`).
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// The non-UI state of one process: its folders, the settings, and the lock on the addon folder's lists. The app
/// makes one at startup and shares it, tests make one each.
pub struct Backend {
    pub paths: Paths,
    pub config: Config,
    /// Held while `addons.txt` or `managed_addons.json` is read, changed and written back, so two changes at the same
    /// moment can't lose one.
    addon_lists: Mutex<()>,
}

impl Backend {
    pub fn new(paths: Paths) -> Self {
        Self { config: Config::new(&paths), paths, addon_lists: Mutex::new(()) }
    }

    /// The game's addon folder, `<ArcheAge Classic Documents>/Addon`. Fails while the settings hold no usable path.
    pub fn addon_dir(&self) -> Result<AddonDir<'_>> {
        Ok(AddonDir::new(self.config.aac_dir()?.join("Addon"), &self.paths, &self.addon_lists))
    }
}
