use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow};

pub fn file_exists(path: impl AsRef<Path>) -> bool {
    match fs::metadata(path.as_ref()) {
        Ok(_) => true,
        Err(e) if e.kind() == io::ErrorKind::NotFound => false,
        Err(e) => {
            log::error!("Error checking if file exists: {e:#}");
            false
        }
    }
}

/// Reads a file as lines: splits on `\n`, strips a trailing `\r` and drops the final empty line.
pub fn read_lines(path: impl AsRef<Path>) -> io::Result<Vec<String>> {
    let data = fs::read(path)?;
    let text = String::from_utf8_lossy(&data);
    Ok(text.lines().map(str::to_owned).collect())
}

/// Moves `path` aside to `<name>.invalid-<YYYYmmdd-HHMMSS>` in the same folder (`-2`, `-3`, ... when that name is
/// taken) and returns the new path. An earlier backup is never replaced.
pub fn quarantine(path: &Path) -> Result<PathBuf> {
    let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    quarantine_as(path, &format!("{name}.invalid-{}", chrono::Local::now().format("%Y%m%d-%H%M%S")))
}

/// [`quarantine`] with the backup name given (tests pin it to force collisions).
pub(super) fn quarantine_as(path: &Path, stem: &str) -> Result<PathBuf> {
    for n in 1..=100 {
        let name = if n == 1 { stem.to_string() } else { format!("{stem}-{n}") };
        let backup = path.with_file_name(name);
        // Reserve the name atomically, the rename then only replaces our own empty placeholder.
        match fs::OpenOptions::new().write(true).create_new(true).open(&backup) {
            Ok(_) => {}
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(anyhow::Error::new(e).context(format!("could not create {}", backup.display()))),
        }
        return match fs::rename(path, &backup) {
            Ok(()) => Ok(backup),
            Err(e) => {
                let _ = fs::remove_file(&backup);
                Err(anyhow::Error::new(e).context(format!("could not move {} to {}", path.display(), backup.display())))
            }
        };
    }
    Err(anyhow!("could not move {} aside: too many {stem}* backups", path.display()))
}

/// Permission bits to carry over when replacing an existing file, `None` where the platform has no mode bits.
#[cfg(unix)]
fn existing_mode(meta: &fs::Metadata) -> Option<u32> {
    use std::os::unix::fs::PermissionsExt;
    Some(meta.permissions().mode() & 0o777)
}

#[cfg(not(unix))]
fn existing_mode(_meta: &fs::Metadata) -> Option<u32> {
    None
}

/// Writes a temp file next to `path`, syncs it, then renames it over `path`, so readers see the old or the new
/// contents and never a part. An existing file keeps its permission bits, a new one gets `perm` (unix only).
pub fn write_atomic(path: impl AsRef<Path>, data: &[u8], perm: u32) -> Result<()> {
    let path = path.as_ref();
    let dir = path.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or(Path::new("."));
    fs::create_dir_all(dir).context("failed to create directories")?;

    let preserved_mode = match fs::metadata(path) {
        Ok(meta) => existing_mode(&meta),
        Err(e) if e.kind() == io::ErrorKind::NotFound => None,
        Err(e) => return Err(anyhow::Error::new(e).context("failed to inspect existing file")),
    };

    let prefix = format!(".{}.tmp-", path.file_name().map(|n| n.to_string_lossy()).unwrap_or_default());
    let mut builder = tempfile::Builder::new();
    builder.prefix(&prefix);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        builder.permissions(fs::Permissions::from_mode(preserved_mode.unwrap_or(perm)));
    }
    #[cfg(not(unix))]
    let _ = (perm, preserved_mode);
    // Dropping the temp file on any error below deletes it.
    let mut tmp = builder.tempfile_in(dir).context("failed to create temp file")?;
    // The creation mode is masked by umask, restore the original mode exactly.
    #[cfg(unix)]
    if let Some(mode) = preserved_mode {
        use std::os::unix::fs::PermissionsExt;
        tmp.as_file()
            .set_permissions(fs::Permissions::from_mode(mode))
            .context("failed to preserve file permissions")?;
    }
    tmp.write_all(data).context("failed to write temp file")?;
    tmp.as_file().sync_all().context("failed to sync temp file")?;
    tmp.persist(path).context("failed to replace file")?;
    Ok(())
}

pub fn open_directory(path: impl AsRef<Path>) -> Result<()> {
    let path = path.as_ref();
    let mut cmd = if cfg!(windows) {
        let mut c = std::process::Command::new("explorer");
        c.arg(path);
        c
    } else if cfg!(target_os = "linux") {
        let mut c = std::process::Command::new("xdg-open");
        c.arg(path);
        c
    } else {
        return Err(anyhow!("unsupported platform"));
    };
    cmd.spawn()?;
    Ok(())
}

pub fn copy_file(src: impl AsRef<Path>, dest: impl AsRef<Path>) -> Result<()> {
    let (src, dest) = (src.as_ref(), dest.as_ref());
    let src_info = fs::metadata(src).with_context(|| format!("source file does not exist: {}", src.display()))?;
    if !src_info.is_file() {
        return Err(anyhow!("cannot copy non-regular file: {}", src.display()));
    }

    let dest_dir = dest.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or(Path::new("."));
    match fs::metadata(dest_dir) {
        Ok(m) if !m.is_dir() => return Err(anyhow!("destination is not a directory: {}", dest_dir.display())),
        Ok(_) => {}
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            return Err(anyhow!("destination directory does not exist: {}", dest_dir.display()));
        }
        Err(e) => {
            return Err(anyhow::Error::new(e)
                .context(format!("error getting info for destination directory: {}", dest_dir.display())));
        }
    }

    let mut src_file = fs::File::open(src).with_context(|| format!("error opening source file: {}", src.display()))?;
    let mut o = fs::OpenOptions::new();
    o.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
        o.mode(src_info.mode() & 0o777);
    }
    let mut dest_file = o.open(dest).with_context(|| format!("error opening destination file: {}", dest.display()))?;

    let copy_res = io::copy(&mut src_file, &mut dest_file);
    let sync_res = dest_file.flush();
    drop(dest_file);
    if let Err(e) = copy_res {
        let _ = fs::remove_file(dest);
        return Err(anyhow::Error::new(e).context("error copying file"));
    }
    if let Err(e) = sync_res {
        let _ = fs::remove_file(dest);
        return Err(anyhow::Error::new(e).context("error closing destination file"));
    }

    if let Ok(mtime) = src_info.modified() {
        let f = fs::OpenOptions::new().write(true).open(dest).context("error setting modified time")?;
        f.set_modified(mtime).context("error setting modified time")?;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::{MetadataExt, PermissionsExt};
        fs::set_permissions(dest, fs::Permissions::from_mode(src_info.mode() & 0o777))
            .context("error setting destination permissions")?;
    }
    Ok(())
}

pub fn copy_dir(src: impl AsRef<Path>, dest: impl AsRef<Path>) -> Result<()> {
    let (src, dest) = (src.as_ref(), dest.as_ref());
    let src_info = fs::metadata(src)?;
    let entries: Vec<fs::DirEntry> = fs::read_dir(src)?.collect::<io::Result<_>>()?;
    fs::create_dir_all(dest)?;
    for entry in entries {
        let src_path = entry.path();
        let dest_path = dest.join(entry.file_name());
        // `file_type` does not follow symlinks, so a symlink to a directory is copied as a file.
        if entry.file_type()?.is_dir() {
            copy_dir(&src_path, &dest_path)?;
        } else {
            copy_file(&src_path, &dest_path)?;
        }
    }
    fs::set_permissions(dest, src_info.permissions())?;
    Ok(())
}

pub fn move_dir(src: impl AsRef<Path>, dest: impl AsRef<Path>) -> Result<()> {
    let (src, dest) = (src.as_ref(), dest.as_ref());
    if fs::metadata(dest).is_ok() {
        fs::remove_dir_all(dest).or_else(|_| fs::remove_file(dest)).context("failed to remove existing destination")?;
    }
    if fs::rename(src, dest).is_ok() {
        return Ok(());
    }
    copy_dir(src, dest).context("failed to copy directory")?;
    fs::remove_dir_all(src).context("failed to remove source directory")?;
    Ok(())
}

pub fn remove_all(path: &Path) -> io::Result<()> {
    match fs::symlink_metadata(path) {
        Ok(m) if m.is_dir() => fs::remove_dir_all(path),
        Ok(_) => fs::remove_file(path),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e),
    }
}
