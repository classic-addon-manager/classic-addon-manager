//! Self-update: the failure report and, in `checksum`, download verification, shared by the platform updaters that
//! replace the running installation (`windows`: the exe, `appimage`: a Linux AppImage). Other platforms update
//! manually.
#[cfg(target_os = "linux")]
mod appimage;
#[cfg(any(windows, target_os = "linux"))]
mod checksum;
#[cfg(windows)]
mod windows;

#[cfg(target_os = "linux")]
use appimage as platform;
#[cfg(windows)]
use windows as platform;

use std::fs;
use std::path::Path;

use anyhow::Result;

use super::paths::Paths;

pub use platform::{self_update, self_update_supported};

#[cfg(not(windows))]
pub const ERR_SELF_UPDATE_UNSUPPORTED: &str =
    "automatic updates require Windows or a Linux AppImage, please update this installation manually";

const UPDATE_FAILURE_FILE_NAME: &str = "update-failed.txt";

#[cfg(not(any(windows, target_os = "linux")))]
mod platform {
    use std::path::PathBuf;

    use anyhow::{Result, anyhow};

    use super::{ERR_SELF_UPDATE_UNSUPPORTED, Paths, UPDATE_FAILURE_FILE_NAME};

    pub fn self_update_supported() -> bool {
        false
    }

    pub fn self_update(_paths: &Paths, _update_url: &str, _checksum: &str) -> Result<()> {
        Err(anyhow!(ERR_SELF_UPDATE_UNSUPPORTED))
    }

    pub(super) fn failure_path(_paths: &Paths) -> Result<PathBuf> {
        Ok(std::env::temp_dir().join("ClassicAddonManager").join(UPDATE_FAILURE_FILE_NAME))
    }
}

/// Returns the report left behind by a failed update, if any, and removes it so it is shown once.
pub fn consume_update_failure(paths: &Paths) -> Result<String> {
    consume_update_failure_at(&platform::failure_path(paths)?)
}

fn consume_update_failure_at(path: &Path) -> Result<String> {
    let data = match fs::read(path) {
        Ok(d) => d,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(String::new()),
        Err(e) => return Err(e.into()),
    };
    if let Err(e) = fs::remove_file(path) {
        log::error!("Error removing update failure report: {e:#}");
    }
    Ok(String::from_utf8_lossy(&data).trim().to_string())
}

#[cfg(test)]
fn test_dir(name: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("cam-upd-{}-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&d);
    fs::create_dir_all(&d).unwrap();
    d
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failure_report_is_consumed_once() {
        let d = test_dir("fail");
        let f = d.join("update-failed.txt");
        assert_eq!(consume_update_failure_at(&f).unwrap(), "");
        fs::write(&f, "  broke\n").unwrap();
        assert_eq!(consume_update_failure_at(&f).unwrap(), "broke");
        assert!(!f.exists());
        fs::remove_dir_all(d).unwrap();
    }
}
