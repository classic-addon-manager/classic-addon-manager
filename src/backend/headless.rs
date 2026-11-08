//! `--check-updates` headless mode: checks for addon updates without opening a window.
use super::{Backend, logger};

/// Runs the headless update check. Returns the process exit code (0 ok, 1 failure). `main` set up the logger for it.
pub fn run(backend: &Backend) -> i32 {
    let code = run_inner(backend);
    logger::sync();
    code
}

fn run_inner(backend: &Backend) -> i32 {
    if let Err(e) = backend.config.load() {
        log::error!("Error loading config: {e:#}");
        return 1;
    }

    let addon_dir = match backend.addon_dir() {
        Ok(d) => d,
        Err(e) => {
            log::error!("Cannot check for updates: {e:#}");
            return 1;
        }
    };

    if let Err(e) = addon_dir.ensure_addons_txt_exists() {
        log::error!("Error creating addons.txt: {e:#}");
        return 1;
    }

    addon_dir.migrate_legacy_managed();
    let check = match addon_dir.check_for_updates() {
        Ok(c) => c,
        Err(e) => {
            // Total failure: leave the previous notification in place.
            log::error!("Error checking for addon updates: {e:#}");
            return 1;
        }
    };
    if let Err(e) = addon_dir.generate_update_addon_lua(&check.updates) {
        log::error!("Error generating addon update notification: {e:#}");
        return 1;
    }
    if !check.missing.is_empty() {
        log::error!("Error checking for addon updates: no release info for: {}", check.missing.join(", "));
        return 1;
    }
    0
}
