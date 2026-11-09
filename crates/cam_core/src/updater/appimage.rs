//! Linux AppImage: download and verify the new image next to the running one, then a detached helper script
//! replaces it once the app has exited and starts it again.
use std::ffi::{OsStr, OsString};
use std::fs;
use std::io::{Read, Write};
use std::os::unix::ffi::{OsStrExt, OsStringExt};
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::{Context, Result, anyhow};

use super::checksum::{ERR_UPDATE_VERIFICATION_FAILED, remove_if_exists, verify_update_checksum};
use super::{ERR_SELF_UPDATE_UNSUPPORTED, Paths, UPDATE_FAILURE_FILE_NAME};
use crate::{download, logger};

/// Whether this installation can replace itself.
pub fn self_update_supported() -> bool {
    app_image_target().is_ok()
}

pub(super) fn failure_path(paths: &Paths) -> Result<PathBuf> {
    Ok(paths.cache_dir()?.join(UPDATE_FAILURE_FILE_NAME))
}

/// Downloads and installs the update, then exits the process. Returns only on failure.
pub fn self_update(paths: &Paths, update_url: &str, checksum: &str) -> Result<()> {
    let update = prepare_update(update_url, checksum)?;
    if let Err(e) = start_update_helper(paths, &update) {
        let _ = fs::remove_dir_all(&update.staging_dir);
        return Err(e);
    }
    log::info!("AppImage update helper started, exiting application.");
    logger::sync();
    std::process::exit(0);
}

/// `APPIMAGE` identifies the outer image (`current_exe` points inside its temporary mount). Symlinks are resolved so
/// updating the image does not replace a user's launcher symlink.
fn app_image_target() -> Result<PathBuf> {
    let env = std::env::var_os("APPIMAGE").unwrap_or_default();
    if !Path::new(&env).is_absolute() {
        return Err(anyhow!(ERR_SELF_UPDATE_UNSUPPORTED));
    }
    let path = fs::canonicalize(&env).context("locating AppImage")?;
    let info = fs::metadata(&path)?;
    if !info.is_file() || info.permissions().mode() & 0o111 == 0 {
        return Err(anyhow!(ERR_SELF_UPDATE_UNSUPPORTED));
    }
    validate_app_image(&path)?;
    // Replacement needs a writable directory, not write access to the running inode.
    let dir = path.parent().unwrap_or(Path::new("/"));
    staging_dir(dir).context("the AppImage directory is not writable")?;
    Ok(path)
}

fn validate_app_image(path: &Path) -> Result<()> {
    let mut f = fs::File::open(path)?;
    let mut header = [0u8; 11];
    if f.read_exact(&mut header).is_err() || &header[..4] != b"\x7fELF" || &header[8..11] != b"AI\x02" {
        return Err(anyhow!("the update must be a type 2 AppImage"));
    }
    Ok(())
}

struct AppImageUpdate {
    target_path: PathBuf,
    source_path: PathBuf,
    staging_dir: PathBuf,
}

/// Stage on the target filesystem so the helper's rename cannot become a cross-device copy.
/// Every failure before the helper starts leaves the installed image untouched.
fn prepare_update(update_url: &str, checksum: &str) -> Result<AppImageUpdate> {
    let target = app_image_target()?;
    let checksum = checksum.trim();
    match hex::decode(checksum) {
        Ok(d) if d.len() == 32 => {}
        _ => return Err(anyhow!(ERR_UPDATE_VERIFICATION_FAILED)),
    }
    let info = fs::metadata(&target)?;
    // Dropping the staging directory on any error below removes it.
    let staging = staging_dir(target.parent().unwrap_or(Path::new("/")))?;
    let source = staging.path().join("update.AppImage");
    download::download_file(update_url, &source)?;
    verify_update_checksum(&source, checksum)?;
    validate_app_image(&source)?;
    let f = fs::OpenOptions::new().read(true).write(true).open(&source)?;
    f.set_permissions(info.permissions())?;
    f.sync_all()?;
    // From here on the helper script owns the directory and removes it.
    Ok(AppImageUpdate { target_path: target, source_path: source, staging_dir: staging.keep() })
}

/// A new private directory next to the AppImage, on the same filesystem.
fn staging_dir(dir: &Path) -> std::io::Result<tempfile::TempDir> {
    tempfile::Builder::new()
        .prefix(".classic-addon-manager-update-")
        .permissions(fs::Permissions::from_mode(0o700))
        .tempdir_in(dir)
}

/// Paths are passed as positional arguments, never interpolated into shell source. The helper waits for the old
/// process to exit, then atomically replaces the image without a backup.
const UPDATE_SCRIPT: &str = r#"#!/bin/sh
parent=$1
target=$2
source=$3
staging=$4
failure=$5
while kill -0 "$parent" 2>/dev/null; do
    sleep 0.1
done
if ! mv -f -- "$source" "$target"; then
    printf 'The update could not replace "%s". The verified download remains at "%s".\n' "$target" "$source" > "$failure"
    "$target"
    exit 1
fi
rm -f -- "$staging/update.sh"
rmdir -- "$staging"
"$target"
status=$?
if [ "$status" -ne 0 ]; then
    printf 'The updated AppImage "%s" exited with status %s. Please download a working version manually, no backup was retained.\n' "$target" "$status" > "$failure"
fi
exit "$status"
"#;

/// AppRun can add mounted libraries/tools to the environment. They must not leak into the helper or the next
/// AppImage after the old mount has disappeared.
fn relaunch_environment(
    environ: impl IntoIterator<Item = (OsString, OsString)>,
    app_dir: &OsStr,
) -> Vec<(OsString, OsString)> {
    const SEP: u8 = b':';
    let app_dir = app_dir.as_bytes();
    let mount_prefix = [app_dir, b"/"].concat();
    let is_mount_path = |part: &[u8]| part == app_dir || part.starts_with(&mount_prefix);
    let mut result = Vec::new();
    for (key, value) in environ {
        if matches!(key.as_bytes(), b"APPIMAGE" | b"APPDIR" | b"ARGV0" | b"OWD") {
            continue;
        }
        if app_dir.is_empty() || !value.as_bytes().split(|&b| b == SEP).any(is_mount_path) {
            result.push((key, value));
            continue;
        }
        let kept: Vec<&[u8]> = value.as_bytes().split(|&b| b == SEP).filter(|part| !is_mount_path(part)).collect();
        if kept.is_empty() {
            continue;
        }
        result.push((key, OsString::from_vec(kept.join(&SEP))));
    }
    result
}

fn start_update_helper(paths: &Paths, update: &AppImageUpdate) -> Result<()> {
    let failure = failure_path(paths)?;
    remove_if_exists(&failure).context("clearing previous update failure")?;
    let script = update.staging_dir.join("update.sh");
    fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(&script)?
        .write_all(UPDATE_SCRIPT.as_bytes())?;

    let app_dir = std::env::var_os("APPDIR").unwrap_or_default();
    let mut cmd = Command::new("/bin/sh");
    cmd.arg(&script)
        .arg(std::process::id().to_string())
        .arg(&update.target_path)
        .arg(&update.source_path)
        .arg(&update.staging_dir)
        .arg(&failure);
    cmd.env_clear();
    cmd.envs(relaunch_environment(std::env::vars_os(), &app_dir));
    let mut dir = match std::env::var_os("OWD") {
        Some(d) if !d.is_empty() => PathBuf::from(d),
        _ => std::env::current_dir()?,
    };
    if !app_dir.is_empty() && (dir == Path::new(&app_dir) || dir.starts_with(&app_dir)) {
        dir = update.target_path.parent().map(Path::to_path_buf).unwrap_or_default();
    }
    cmd.current_dir(dir);
    cmd.stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
    // Survive termination of the old app's session.
    // SAFETY: setsid is async-signal-safe and runs in the forked child before exec.
    unsafe {
        cmd.pre_exec(|| {
            libc::setsid();
            Ok(())
        });
    }
    cmd.spawn().context("starting AppImage update helper")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::super::test_dir;
    use super::*;

    #[test]
    fn relaunch_environment_strips_appimage_vars_and_mount_paths() {
        let os = |s: &str| OsString::from(s);
        let raw = |b: &[u8]| OsString::from_vec(b.to_vec());
        let env = vec![
            (os("APPIMAGE"), os("/x.AppImage")),
            (os("APPDIR"), os("/tmp/.mount_x")),
            (os("PATH"), os("/tmp/.mount_x/usr/bin:/usr/bin")),
            (os("LD_LIBRARY_PATH"), os("/tmp/.mount_x/usr/lib")),
            (os("XDG_DATA_DIRS"), os("/tmp/.mount_xy/share:/tmp/.mount_x")),
            (os("HOME"), os("/home/u")),
            (raw(b"CAM_\xff"), raw(b"\xfe/tmp/.mount_x")),
            (os("NON_UTF8_PATH"), raw(b"/tmp/.mount_x/bin:/opt/\xff/bin")),
        ];
        let out = relaunch_environment(env, OsStr::new("/tmp/.mount_x"));
        assert_eq!(
            out,
            vec![
                (os("PATH"), os("/usr/bin")),
                (os("XDG_DATA_DIRS"), os("/tmp/.mount_xy/share")),
                (os("HOME"), os("/home/u")),
                (raw(b"CAM_\xff"), raw(b"\xfe/tmp/.mount_x")),
                (os("NON_UTF8_PATH"), raw(b"/opt/\xff/bin")),
            ]
        );
    }

    #[test]
    fn app_image_header_validation() {
        let d = test_dir("ai");
        let good = d.join("good");
        let mut h = b"\x7fELF".to_vec();
        h.extend_from_slice(&[0; 4]);
        h.extend_from_slice(b"AI\x02");
        fs::write(&good, &h).unwrap();
        validate_app_image(&good).unwrap();
        let bad = d.join("bad");
        fs::write(&bad, b"not an appimage").unwrap();
        assert!(validate_app_image(&bad).is_err());
        fs::remove_dir_all(d).unwrap();
    }
}
