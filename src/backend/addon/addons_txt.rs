//! `addons.txt`: the installed addons in load order, one name per line. It also lists the update notification addon,
//! which the app hides.
use std::io;
use std::path::PathBuf;

use anyhow::{Context, Result};

use super::AddonDir;
use crate::backend::file;

const UPDATE_NOTIFICATION: &str = "AddonUpdateNotification";

impl AddonDir<'_> {
    pub fn addons_txt(&self) -> PathBuf {
        self.path.join("addons.txt")
    }

    /// The names in addons.txt, the update notification included. Blank lines are skipped.
    fn lines(&self) -> io::Result<Vec<String>> {
        let lines = file::read_lines(self.addons_txt())?;
        Ok(lines.into_iter().filter(|l| !l.trim().is_empty()).collect())
    }

    /// [`Self::lines`], failing when addons.txt is missing. The caller holds the lock.
    pub(super) fn read_lines(&self) -> Result<Vec<String>> {
        self.lines().with_context(|| format!("read {}", self.addons_txt().display()))
    }

    /// [`Self::lines`], empty when addons.txt is missing. The caller holds the lock.
    fn read_lines_or_empty(&self) -> Result<Vec<String>> {
        match self.lines() {
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(Vec::new()),
            result => result.with_context(|| format!("read {}", self.addons_txt().display())),
        }
    }

    /// Writes the full list, including any update notification. The caller holds the lock.
    pub(super) fn write_lines(&self, names: &[String]) -> Result<()> {
        let path = self.addons_txt();
        file::write_atomic(&path, names.join("\n").as_bytes(), 0o644)
            .with_context(|| format!("write {}", path.display()))
    }

    /// The installed addons in load order, without the update notification. Fails when addons.txt is missing.
    pub fn installed_names(&self) -> Result<Vec<String>> {
        let _lists = self.lock();
        self.installed_names_locked()
    }

    /// [`Self::installed_names`] for a caller that holds the lock.
    pub(super) fn installed_names_locked(&self) -> Result<Vec<String>> {
        Ok(self.read_lines()?.into_iter().filter(|n| n != UPDATE_NOTIFICATION).collect())
    }

    /// Appends `addon_name` unless addons.txt lists it. A missing addons.txt is created.
    pub fn add_to_addons_txt(&self, addon_name: &str) -> Result<()> {
        let _lists = self.lock();
        let mut names = self.read_lines_or_empty()?;
        if names.iter().any(|n| n == addon_name) {
            return Ok(());
        }
        names.push(addon_name.to_string());
        self.write_lines(&names)
    }

    pub fn remove_from_addons_txt(&self, addon_name: &str) -> Result<()> {
        let _lists = self.lock();
        let mut names = self.read_lines_or_empty()?;
        let Some(idx) = names.iter().position(|n| n == addon_name) else {
            return Ok(());
        };
        names.remove(idx);
        self.write_lines(&names)
    }

    /// Creates an empty addons.txt when there is none. The check and the write happen under the lock, so a file
    /// another change just wrote is never replaced. A file that can't be checked fails, it isn't replaced either.
    pub fn ensure_addons_txt_exists(&self) -> Result<()> {
        let _lists = self.lock();
        let path = self.addons_txt();
        match std::fs::metadata(&path) {
            Err(e) if e.kind() == io::ErrorKind::NotFound => {}
            result => return result.map(drop).with_context(|| format!("check {}", path.display())),
        }
        log::info!("addons.txt not found, creating it: {}", path.display());
        file::write_atomic(&path, b"", 0o644).with_context(|| format!("create {}", path.display()))
    }
}
