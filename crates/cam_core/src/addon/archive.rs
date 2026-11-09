use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};

use anyhow::{Context, Result, anyhow};

use crate::file;
use crate::throttle::Throttle;

/// Installs the addon in the archive `zip` as the addon folder `dest`. The archive is extracted into a staging folder
/// next to `dest`. Its one top-level folder with a `main.lua` gets the existing addon's `.data` and is then swapped
/// into place. `commit` runs after the swap, and when it fails the previous addon is put back. `progress` is as in
/// [`extract`].
pub fn install_release(
    zip: &Path,
    dest: &Path,
    progress: &mut dyn FnMut(u64, u64),
    commit: Option<&dyn Fn() -> Result<()>>,
) -> Result<()> {
    let parent = dest.parent().unwrap_or(Path::new("."));
    fs::create_dir_all(parent).context("failed to create addon directory")?;

    let base = dest.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    // Dropping the staging directory removes it with whatever the swap left in it.
    let stage = tempfile::Builder::new()
        .prefix(&format!(".{base}.staging-"))
        .tempdir_in(parent)
        .context("failed to create staging directory")?;

    extract(zip, stage.path(), progress)?;
    let root = release_root(stage.path())?;
    carry_over_addon_data(dest, &root)?;
    swap_directories_with_commit(&root, dest, &|a, b| fs::rename(a, b), commit)
}

/// Extracts the archive `zip` into the existing directory `into`. An entry whose name would land outside `into` fails
/// the whole extraction. Symlinks and other special entries are left out. `progress(extracted, total)` counts files,
/// not folders. It runs once before the first file, at most every 50 ms while extracting, and once at the end.
fn extract(zip: &Path, into: &Path, progress: &mut dyn FnMut(u64, u64)) -> Result<()> {
    let shown = || zip.display().to_string();
    let f = fs::File::open(zip).with_context(|| format!("open archive {:?}", shown()))?;
    let mut archive = zip::ZipArchive::new(f).with_context(|| format!("read archive {:?}", shown()))?;

    // Count the files first. This only reads the zip's file list, not the file contents.
    let mut files_total = 0u64;
    for i in 0..archive.len() {
        if entry_kind(&archive.by_index_raw(i)?) == EntryKind::File {
            files_total += 1;
        }
    }
    let (mut files, mut directories, mut skipped) = (0u64, 0usize, 0usize);
    let mut throttle = Throttle::new();
    progress(0, files_total);

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        let Some(path) = entry_path(&entry, into) else {
            return Err(anyhow!("{:?}: invalid file path", String::from_utf8_lossy(entry.name_raw())));
        };

        match entry_kind(&entry) {
            EntryKind::Dir => {
                fs::create_dir_all(&path)
                    .with_context(|| format!("create directory {:?}", path.display().to_string()))?;
                directories += 1;
                continue;
            }
            EntryKind::Other => {
                skipped += 1;
                continue;
            }
            EntryKind::File => {}
        }

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("create parent directory for {:?}", path.display().to_string()))?;
        }

        let mode = entry.unix_mode().unwrap_or(0o666) & 0o777;
        extract_file(&mut entry, &path, mode).with_context(|| format!("extract {:?}", path.display().to_string()))?;
        files += 1;
        if throttle.ready() {
            progress(files, files_total);
        }
    }
    progress(files, files_total);

    log::info!(
        "Extracted archive {:?}: {files} files, {directories} directories, {skipped} symlinks or special files left out",
        shown()
    );
    Ok(())
}

/// Where `entry` goes under `into`, or `None` when its name would leave `into`. `enclosed_name` alone is not enough on
/// Windows: a drive letter inside the name (`root/K:file`) comes back as a path with a prefix, and joining that
/// replaces `into`. So every element of the result must be a plain name. On Windows it also must not contain a colon:
/// `main.lua:payload` writes an NTFS stream and leaves an empty `main.lua` behind.
fn entry_path<R: io::Read>(entry: &zip::read::ZipFile<'_, R>, into: &Path) -> Option<PathBuf> {
    let rel = entry.enclosed_name()?;
    let plain = |c: Component| match c {
        Component::Normal(name) => !(cfg!(windows) && name.as_encoded_bytes().contains(&b':')),
        _ => false,
    };
    rel.components().all(plain).then(|| into.join(rel))
}

#[derive(PartialEq)]
enum EntryKind {
    Dir,
    File,
    /// A symlink, device or other special file.
    Other,
}

/// The unix file type decides first, so a symlink stays one even when its name ends in a separator. An entry with no
/// type or a regular type is a folder when its name ends in a separator, and a file otherwise.
fn entry_kind<R: io::Read>(entry: &zip::read::ZipFile<'_, R>) -> EntryKind {
    match entry.unix_mode().map(|m| m & 0o170000) {
        Some(0o040000) => EntryKind::Dir,
        None | Some(0) | Some(0o100000) if entry.is_dir() => EntryKind::Dir,
        None | Some(0) | Some(0o100000) => EntryKind::File,
        Some(_) => EntryKind::Other,
    }
}

fn extract_file(entry: &mut impl io::Read, dest: &Path, mode: u32) -> io::Result<()> {
    let mut o = fs::OpenOptions::new();
    o.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        o.mode(mode);
    }
    #[cfg(not(unix))]
    let _ = mode;
    let mut dst = o.open(dest)?;
    io::copy(entry, &mut dst)?;
    Ok(())
}

/// The one top-level folder of the extracted archive at `dir` that holds a `main.lua` file.
fn release_root(dir: &Path) -> Result<PathBuf> {
    let entries = fs::read_dir(dir).context("error reading extracted archive")?;
    let mut entries: Vec<fs::DirEntry> =
        entries.collect::<io::Result<_>>().context("error reading extracted archive")?;
    entries.sort_by_key(|e| e.file_name());

    let mut root: Option<fs::DirEntry> = None;
    for entry in entries {
        // `file_type` does not follow symlinks, so a symlink to a directory is skipped.
        if !entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            continue;
        }
        match fs::metadata(entry.path().join("main.lua")) {
            Ok(info) if info.is_file() => {}
            Ok(_) => continue,
            Err(e) if e.kind() == io::ErrorKind::NotFound => continue,
            Err(e) => {
                return Err(
                    anyhow::Error::new(e).context(format!("error inspecting archive folder {:?}", entry.file_name()))
                );
            }
        }
        if let Some(prev) = &root {
            return Err(anyhow!(
                "invalid archive: more than one top-level folder contains main.lua: {:?} and {:?}",
                prev.file_name(),
                entry.file_name()
            ));
        }
        root = Some(entry);
    }
    root.map(|e| e.path()).ok_or_else(|| anyhow!("invalid archive: no top-level folder contains main.lua"))
}

/// Replaces the staged release's `.data` with a copy of the existing install's `.data`.
fn carry_over_addon_data(dest: &Path, stage: &Path) -> Result<()> {
    let existing = dest.join(".data");
    let info = match fs::metadata(&existing) {
        Ok(i) => i,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(anyhow::Error::new(e).context("failed to inspect existing .data")),
    };
    if !info.is_dir() {
        return Err(anyhow!("existing .data at {:?} is not a directory", existing.display().to_string()));
    }
    let staged_data = stage.join(".data");
    file::remove_all(&staged_data).context("failed to replace staged .data")?;
    file::copy_dir(&existing, &staged_data).context("failed to preserve existing .data")?;
    Ok(())
}

type RenameFn<'a> = &'a dyn Fn(&Path, &Path) -> io::Result<()>;

/// Creates an empty directory `dir/<prefix><random>` that stays, so its name is taken.
fn reserve_path(dir: &Path, prefix: &str) -> io::Result<PathBuf> {
    Ok(tempfile::Builder::new().prefix(prefix).tempdir_in(dir)?.keep())
}

/// The commit failure `err`, noting the follow-up failure `extra` that left the rollback incomplete.
fn commit_failed(err: anyhow::Error, extra: impl std::fmt::Display) -> anyhow::Error {
    err.context(format!("commit addon installation (additionally {extra})"))
}

fn swap_directories_with_commit(
    stage: &Path,
    dest: &Path,
    rename: RenameFn,
    commit: Option<&dyn Fn() -> Result<()>>,
) -> Result<()> {
    let dest_parent = dest.parent().unwrap_or(Path::new("."));
    let base = dest.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let mut backup: Option<PathBuf> = None;

    match fs::metadata(dest) {
        Ok(_) => {
            let placeholder =
                reserve_path(dest_parent, &format!("{base}.backup-")).context("failed to reserve backup path")?;
            fs::remove_dir(&placeholder).context("failed to release backup path")?;
            rename(dest, &placeholder).context("failed to move existing addon aside")?;
            backup = Some(placeholder);
        }
        Err(e) if e.kind() == io::ErrorKind::NotFound => {}
        Err(e) => return Err(anyhow::Error::new(e).context("failed to inspect existing addon directory")),
    }

    if let Err(err) = rename(stage, dest) {
        if let Some(b) = &backup
            && let Err(restore_err) = rename(b, dest)
        {
            return Err(anyhow::Error::new(err).context(format!(
                "failed to move staged addon into place (additionally failed to restore the previous addon from {:?}: {restore_err})",
                b.display().to_string()
            )));
        }
        let _ = file::remove_all(stage);
        return Err(anyhow::Error::new(err).context("failed to move staged addon into place"));
    }

    if let Some(commit) = commit
        && let Err(err) = commit()
    {
        match &backup {
            None => {
                if let Err(remove_err) = file::remove_all(dest) {
                    return Err(commit_failed(err, format!("failed to remove new addon: {remove_err}")));
                }
            }
            Some(b) => {
                let bs = b.display().to_string();
                let placeholder = match reserve_path(dest_parent, &format!("{base}.failed-")) {
                    Ok(p) => p,
                    Err(e) => {
                        return Err(commit_failed(
                            err,
                            format!("failed to reserve rollback path (previous addon at {bs:?}): {e}"),
                        ));
                    }
                };
                if let Err(e) = fs::remove_dir(&placeholder) {
                    return Err(commit_failed(
                        err,
                        format!("failed to release rollback path (previous addon at {bs:?}): {e}"),
                    ));
                }
                if let Err(e) = rename(dest, &placeholder) {
                    return Err(commit_failed(
                        err,
                        format!("failed to move new addon aside (previous addon at {bs:?}): {e}"),
                    ));
                }
                if let Err(restore_err) = rename(b, dest) {
                    return Err(match rename(&placeholder, dest) {
                        Err(move_err) => commit_failed(
                            err,
                            format!(
                                "failed to restore previous addon from {bs:?}: {restore_err}; new addon remains at {:?}: {move_err}",
                                placeholder.display().to_string()
                            ),
                        ),
                        Ok(()) => {
                            commit_failed(err, format!("failed to restore previous addon from {bs:?}: {restore_err}"))
                        }
                    });
                }
                if let Err(remove_err) = file::remove_all(&placeholder) {
                    log::error!("Failed to remove replaced addon after rollback: {remove_err:#}");
                }
            }
        }
        return Err(err.context("commit addon installation"));
    }

    if let Some(b) = &backup
        && let Err(e) = file::remove_all(b)
    {
        log::error!("Failed to remove addon backup after successful swap: {e:#}");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A tempdir with `dest/f` holding "old" (unless `fresh`) and `stage/f` holding "new".
    fn folders(fresh: bool) -> (tempfile::TempDir, PathBuf, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let (dest, stage) = (dir.path().join("d"), dir.path().join("stage"));
        if !fresh {
            fs::create_dir_all(&dest).unwrap();
            fs::write(dest.join("f"), "old").unwrap();
        }
        fs::create_dir_all(&stage).unwrap();
        fs::write(stage.join("f"), "new").unwrap();
        (dir, dest, stage)
    }

    #[test]
    fn swap_second_rename_failure_restores_previous() {
        let (_dir, dest, stage) = folders(false);
        let calls = std::cell::Cell::new(0);
        let rename = |a: &Path, b: &Path| {
            calls.set(calls.get() + 1);
            if calls.get() == 2 {
                return Err(io::Error::other("injected"));
            }
            fs::rename(a, b)
        };
        let e = format!("{:#}", swap_directories_with_commit(&stage, &dest, &rename, None).unwrap_err());
        assert!(e.starts_with("failed to move staged addon into place: injected"), "{e}");
        assert_eq!(fs::read_to_string(dest.join("f")).unwrap(), "old");
        assert!(!stage.exists());
    }

    #[test]
    fn swap_restore_failure_retains_backup() {
        let (dir, dest, stage) = folders(false);
        let calls = std::cell::Cell::new(0);
        let rename = |a: &Path, b: &Path| {
            calls.set(calls.get() + 1);
            if calls.get() >= 2 {
                return Err(io::Error::other("injected"));
            }
            fs::rename(a, b)
        };
        let e = swap_directories_with_commit(&stage, &dest, &rename, None).unwrap_err().to_string();
        assert!(e.contains("additionally failed to restore the previous addon from"), "{e}");
        let backups: Vec<_> = fs::read_dir(dir.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().starts_with("d.backup-"))
            .collect();
        assert_eq!(backups.len(), 1);
    }

    #[test]
    fn swap_fresh_install() {
        let (_dir, dest, stage) = folders(true);
        swap_directories_with_commit(&stage, &dest, &|a, b| fs::rename(a, b), None).unwrap();
        assert_eq!(fs::read_to_string(dest.join("f")).unwrap(), "new");
    }
}
