use anyhow::Result;
use chrono::{DateTime, Months, Utc};
use serde::{Deserialize, Serialize};

use super::paths::Paths;
use super::versioned_json::{VersionedJson, not_moved_aside};

#[derive(Serialize, Deserialize, Default)]
struct StateFile {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    kofi_modal_last_shown_at: Option<DateTime<Utc>>,
}

fn state_file(paths: &Paths) -> Result<VersionedJson<StateFile>> {
    Ok(VersionedJson::new(paths.data_dir()?.join("app_state.json"), 1, 0o600))
}

/// Whether the Ko-fi modal is due: it never was shown, or last was more than 3 months ago. On the first launch,
/// without `app_state.json`, it isn't due, and the file is created so that the next launch shows it. A file that can't
/// be used is moved aside and counts as a first launch. When it can't be moved aside, the error is returned and the
/// file is left alone.
pub fn kofi_modal_due(paths: &Paths) -> Result<bool> {
    let file = state_file(paths)?;
    let state = match file.load() {
        Ok(state) => state,
        Err(e) if not_moved_aside(&e) => return Err(e),
        Err(e) => {
            log::warn!("App state: {e:#}");
            None
        }
    };
    let Some(state) = state else {
        file.save(&StateFile::default())?;
        log::info!("App state: initialized app_state.json (first launch)");
        return Ok(false);
    };
    Ok(match state.kofi_modal_last_shown_at {
        None => true,
        Some(t) => Utc::now() > t.checked_add_months(Months::new(3)).unwrap_or(t),
    })
}

pub fn record_kofi_modal_shown(paths: &Paths) -> Result<()> {
    state_file(paths)?.save(&StateFile { kofi_modal_last_shown_at: Some(Utc::now()) })?;
    log::info!("App state: kofi modal last shown recorded");
    Ok(())
}
