//! Windows: download the new exe to the temp folder, then a batch script copies it over the running one once the
//! app has exited and starts it again.
use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};

use super::checksum::{remove_if_exists, verify_update_checksum};
use super::{Paths, UPDATE_FAILURE_FILE_NAME};
use crate::backend::{logger, util};

fn update_temp_dir() -> PathBuf {
    std::env::temp_dir().join("ClassicAddonManager")
}

/// Whether this installation can replace itself.
pub fn self_update_supported() -> bool {
    true
}

pub(super) fn failure_path(_paths: &Paths) -> Result<PathBuf> {
    Ok(update_temp_dir().join(UPDATE_FAILURE_FILE_NAME))
}

/// Downloads and installs the update, then exits the process. Returns only on failure.
pub fn self_update(paths: &Paths, update_url: &str, checksum: &str) -> Result<()> {
    let exe_path = std::env::current_exe().context("error getting executable path")?;

    let tmp_dir = update_temp_dir();
    fs::create_dir_all(&tmp_dir).context("error creating temporary directory")?;

    let failure_path = failure_path(paths)?;
    if let Err(e) = remove_if_exists(&failure_path) {
        log::error!("Error removing old update failure report: {e:#}");
    }

    log::info!("Downloading update from {update_url}");
    let new_exe_path = tmp_dir.join("ClassicAddonManager.new.exe");

    if new_exe_path.exists() {
        fs::remove_file(&new_exe_path).context("error removing existing update file")?;
        log::info!("Removed existing update file");
    }

    util::download_file(update_url, &new_exe_path).context("error downloading update")?;

    verify_update_checksum(&new_exe_path, checksum)?;

    let script_path = tmp_dir.join("update.bat");
    let script =
        update_script(&new_exe_path.to_string_lossy(), &exe_path.to_string_lossy(), &failure_path.to_string_lossy());
    fs::write(&script_path, script).context("error creating update script")?;

    util::hidden_command("cmd").arg("/C").arg(&script_path).spawn().context("error starting update script")?;

    log::info!("Update script started, exiting application.");
    logger::sync();
    std::process::exit(0);
}

/// Batch file that keeps trying to copy the downloaded version over the running one (Windows refuses until the
/// app has closed), up to 30 times a second apart. After it gives up it writes a report so the app can tell the
/// user next start. Either way it starts the app again.
fn update_script(source: &str, target: &str, failure: &str) -> String {
    let escape = |v: &str| v.replace('%', "%%");
    let lines = [
        "@echo off".to_string(),
        "setlocal".to_string(),
        format!("set \"SOURCE={}\"", escape(source)),
        format!("set \"TARGET={}\"", escape(target)),
        format!("set \"FAILURE={}\"", escape(failure)),
        "set /a ATTEMPT=0".to_string(),
        ":retry".to_string(),
        "copy /Y \"%SOURCE%\" \"%TARGET%\" > NUL 2>&1".to_string(),
        "if not errorlevel 1 goto success".to_string(),
        "set /a ATTEMPT+=1".to_string(),
        "if %ATTEMPT% GEQ 30 goto failed".to_string(),
        "ping -n 2 127.0.0.1 > NUL".to_string(),
        "goto retry".to_string(),
        ":success".to_string(),
        "del \"%SOURCE%\" > NUL 2>&1".to_string(),
        "start \"\" \"%TARGET%\"".to_string(),
        "(goto) 2>nul & del \"%~f0\"".to_string(),
        ":failed".to_string(),
        "> \"%FAILURE%\" echo The update could not replace \"%TARGET%\" after %ATTEMPT% attempts. The downloaded version was kept at \"%SOURCE%\" and can be copied over manually.".to_string(),
        "start \"\" \"%TARGET%\"".to_string(),
        "exit /b 1".to_string(),
    ];
    lines.join("\r\n") + "\r\n"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn script_escapes_percent_and_relaunches() {
        let s = update_script("C:\\a%b\\new.exe", "C:\\t.exe", "C:\\f.txt");
        assert!(s.contains("set \"SOURCE=C:\\a%%b\\new.exe\"\r\n"));
        assert!(s.contains("if %ATTEMPT% GEQ 30 goto failed"));
        assert!(s.contains("start \"\" \"%TARGET%\"\r\n(goto) 2>nul & del \"%~f0\""));
        assert!(s.ends_with("start \"\" \"%TARGET%\"\r\nexit /b 1\r\n"));
    }
}
