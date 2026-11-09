//! Installed addons: addons.txt, load order, managed addon metadata, dependencies, installs, updates, uninstalls.
mod addons_txt;
pub(crate) mod archive;
mod dependencies;
mod install;
mod install_zip;
mod load_order;
mod local;
mod remote;
mod updates;

pub use dependencies::resolve_dependencies_with;
pub use install::{
    AddonInstallStatus, DependencyInfo, DependencyResolutionResult, InstallError, InstallProgress,
    InstallWithDependenciesResult,
};
pub use updates::has_update;

use std::path::{Component, Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

use anyhow::{Context as _, Result, anyhow};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::paths::Paths;
use super::serde_util::{go_time, null_default};
use super::{api, file};

/// The game's addon folder and the two lists that describe it: `addons.txt` in the folder, the installed addons in
/// load order, and `managed_addons.json` in the config folder, the addons installed from the catalog. Both lists are
/// read from disk on every call. Each call that reads or changes a list holds the lock on both while it does, so two
/// changes at the same moment can't lose one. Installs and uninstalls are several such calls in a row.
pub struct AddonDir<'a> {
    path: PathBuf,
    paths: &'a Paths,
    lists: &'a Mutex<()>,
}

/// An installed addon. Its JSON shape is the one the Go client uses in `managed_addons.json`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Addon {
    pub name: String,
    pub alias: String,
    pub description: String,
    pub version: String,
    pub commit: String,
    pub author: String,
    pub repo: String,
    #[serde(rename = "isManaged")]
    pub is_managed: bool,
    /// Publish time of the installed release, used to notice when the same tag is re-published. `None` for addons
    /// installed without one, which are compared by tag alone.
    #[serde(with = "go_time", rename = "updatedAt")]
    pub updated_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub branch: String,
    #[serde(deserialize_with = "null_default")]
    pub dependencies: Vec<String>,
}

impl<'a> AddonDir<'a> {
    pub(super) fn new(path: PathBuf, paths: &'a Paths, lists: &'a Mutex<()>) -> Self {
        Self { path, paths, lists }
    }

    /// Held while a list is read or changed. The functions that take it don't call each other while they hold it.
    fn lock(&self) -> MutexGuard<'a, ()> {
        self.lists.lock().unwrap_or_else(|p| p.into_inner())
    }

    /// Installed addons in addons.txt order, with their managed metadata. An unusable `managed_addons.json` is logged,
    /// and its addons are listed as unmanaged then.
    pub fn addons(&self) -> Result<Vec<Addon>> {
        let _lists = self.lock();
        let names = self.installed_names_locked()?;
        let mut managed = self.read_managed().map(|m| m.addons).unwrap_or_else(|e| {
            log::error!("Error loading managed_addons.json: {e:#}");
            Default::default()
        });
        Ok(names
            .into_iter()
            .map(|name| {
                let alias = name.replace('_', " ");
                match managed.remove(&name) {
                    Some(addon) if addon.alias.is_empty() => Addon { alias, ..addon },
                    Some(addon) => addon,
                    None => Addon { name, alias, ..Addon::default() },
                }
            })
            .collect())
    }

    /// Removes an installed addon: its addons.txt entry, directory and managed record, then unsubscribes from the
    /// server when it was managed.
    pub fn uninstall_addon(&self, name: &str) -> Result<()> {
        if !self.installed_names()?.iter().any(|n| n == name) {
            return Err(anyhow!("{name} is not installed"));
        }

        let was_managed = self.managed_addons()?.contains_key(name);

        self.remove_from_addons_txt(name).context("remove addon from addons.txt")?;

        file::remove_all(&self.path.join(name)).context("remove addon directory")?;

        if was_managed {
            self.remove_managed_addon(name).context("remove managed addon record")?;
        }

        // The removal itself succeeded: a load-order cycle or write failure only leaves addons.txt unsorted.
        if let Err(e) = self.sort_addons_txt() {
            log::error!("Error sorting addons.txt: {e:#}");
        }

        // Best effort: the addon is already gone locally, so a server failure must not fail the uninstall.
        if was_managed && let Err(e) = api::unsubscribe(name) {
            log::warn!("Could not unsubscribe from {name}: {e}");
        }

        Ok(())
    }

    /// Opens `<addons>/<name>` in the file manager; `name` must be a single, non-escaping path element.
    pub fn open_directory(&self, name: &str) -> Result<()> {
        self.open_directory_with(name, &|p| file::open_directory(p))
    }

    pub(crate) fn open_directory_with(&self, name: &str, open: &dyn Fn(&Path) -> Result<()>) -> Result<()> {
        if name.is_empty() || name == "." || name == ".." || name.contains(['/', '\\']) {
            return Err(anyhow!("invalid addon directory: {:?}", name));
        }
        let root = resolve_addon_directory(&self.path)?;
        let target = resolve_addon_directory(&root.join(name))?;
        match target.strip_prefix(&root) {
            Ok(rel) if rel.components().count() > 0 && rel.components().all(|c| matches!(c, Component::Normal(_))) => {
                open(&target)
            }
            _ => Err(anyhow!("addon directory escapes addon root: {:?}", name)),
        }
    }
}

/// Empties `addon_settings` in the ArcheAge Classic Documents folder `aac_dir`.
pub fn reset_addon_settings(aac_dir: &Path) -> Result<()> {
    let f = std::fs::OpenOptions::new().write(true).open(aac_dir.join("addon_settings"))?;
    f.set_len(0)?;
    Ok(())
}

fn resolve_addon_directory(path: &Path) -> std::io::Result<PathBuf> {
    let resolved = std::fs::canonicalize(path)?;
    #[cfg(windows)]
    {
        let s = resolved.to_string_lossy().into_owned();
        if let Some(rest) = s.strip_prefix(r"\\?\UNC\") {
            return Ok(PathBuf::from(format!(r"\\{rest}")));
        }
        if let Some(rest) = s.strip_prefix(r"\\?\") {
            return Ok(PathBuf::from(rest));
        }
    }
    Ok(resolved)
}
