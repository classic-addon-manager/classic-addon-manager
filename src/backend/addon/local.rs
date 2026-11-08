//! `managed_addons.json`: the addons installed from the catalog, with the release each one is at.
use std::collections::HashMap;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use super::{Addon, AddonDir};
use crate::backend::api::{self, Release};
use crate::backend::file;
use crate::backend::shared::AddonManifest;
use crate::backend::versioned_json::{VersionedJson, not_moved_aside};

#[derive(Serialize, Deserialize)]
struct ManagedAddonsFile {
    #[serde(default, deserialize_with = "crate::backend::serde_util::null_default")]
    addons: Vec<Addon>,
}

fn normalize_addon(mut a: Addon) -> Addon {
    // A `Vec` is never null, so `dependencies` always serializes as `[]`.
    a.dependencies.shrink_to_fit();
    a
}

fn by_name(addons: Vec<Addon>) -> HashMap<String, Addon> {
    addons.into_iter().map(|a| (a.name.clone(), normalize_addon(a))).collect()
}

/// Fills each addon's dependencies from its catalog entry. The legacy file format didn't store them.
fn backfill_dependencies(addons: Vec<Addon>, manifests: &[AddonManifest]) -> Vec<Addon> {
    let by_name: HashMap<&str, &AddonManifest> = manifests.iter().map(|m| (m.name.as_str(), m)).collect();
    addons
        .into_iter()
        .map(|mut a| {
            match by_name.get(a.name.as_str()) {
                Some(m) => a.dependencies = m.dependencies.clone(),
                None => log::warn!("backfillDependencies: manifest not found for addon: {}", a.name),
            }
            a
        })
        .collect()
}

/// Whether `data` is the legacy `managed_addons.json`: a bare array of addons, without a version.
fn is_legacy(data: &[u8]) -> bool {
    data.trim_ascii_start().starts_with(b"[")
}

/// The managed addons by name, and the format of their file.
pub(super) struct Managed {
    pub addons: HashMap<String, Addon>,
    /// The file is still in the legacy format: [`AddonDir::migrate_legacy_managed`] didn't convert it yet. Saves keep
    /// that format, so the migration can still back the file up and fill in the dependencies.
    legacy: bool,
}

impl AddonDir<'_> {
    fn managed_file(&self) -> Result<VersionedJson<ManagedAddonsFile>> {
        Ok(VersionedJson::new(self.paths.data_dir()?.join("managed_addons.json"), 1, 0o644))
    }

    /// The managed addons. A missing file means none. A file that can't be used is moved aside, and the error says
    /// where. A legacy file is read as it is, its addons have no dependencies. The caller holds the lock.
    pub(super) fn load_managed(&self) -> Result<Managed> {
        let file = self.managed_file()?;
        let Some(data) = file.read()? else { return Ok(Managed { addons: HashMap::new(), legacy: false }) };
        if !is_legacy(&data) {
            return Ok(Managed { addons: by_name(file.decode(&data)?.addons), legacy: false });
        }
        let addons: Vec<Addon> = serde_json::from_slice(&data)
            .map_err(|e| file.quarantine(anyhow::Error::new(e).context("invalid JSON")))?;
        Ok(Managed { addons: by_name(addons), legacy: true })
    }

    /// [`Self::load_managed`] for a change to the list. A file that was moved aside is logged and the list starts
    /// empty, so the next save writes a new file. A file that couldn't be moved aside fails the change, so no save
    /// replaces it. The caller holds the lock.
    pub(super) fn read_managed(&self) -> Result<Managed> {
        match self.load_managed() {
            Err(e) if !not_moved_aside(&e) => {
                log::error!("Error loading managed_addons.json: {e:#}");
                Ok(Managed { addons: HashMap::new(), legacy: false })
            }
            result => result,
        }
    }

    /// Converts a legacy `managed_addons.json` to version 1, keeping the old file as `managed_addons.json.bak`. The
    /// legacy format has no dependencies, so they come from the catalog. The catalog is fetched without the lock,
    /// and the file is converted only when it didn't change meanwhile. Failures are logged, and the file then stays
    /// as it is. Startup and the headless check call this once.
    pub fn migrate_legacy_managed(&self) {
        // Plain reads: a file that can't be read or isn't legacy is left to the next regular read, which moves an
        // unusable one aside and logs why.
        let Ok(path) = self.managed_file().map(|file| file.path().to_path_buf()) else { return };
        let data = match std::fs::read(&path) {
            Ok(data) if is_legacy(&data) => data,
            _ => return,
        };
        let Ok(addons) = serde_json::from_slice::<Vec<Addon>>(&data) else { return };
        // The migration runs once, so it asks the server directly instead of going through the app's catalog.
        let manifests = match api::catalog() {
            Ok(manifests) => manifests,
            Err(e) => {
                log::error!("Migrating managed_addons.json failed: fetch addon manifest: {e:#}");
                return;
            }
        };
        let _lists = self.lock();
        if !matches!(std::fs::read(&path), Ok(now) if now == data) {
            log::warn!("Migrating managed_addons.json skipped: the file changed while the catalog was fetched");
            return;
        }
        let backup = path.with_file_name("managed_addons.json.bak");
        if let Err(e) = std::fs::write(&backup, &data) {
            log::error!("Migrating managed_addons.json failed: write {}: {e}", backup.display());
            return;
        }
        let managed = Managed { addons: by_name(backfill_dependencies(addons, &manifests)), legacy: false };
        match self.save_managed(&managed) {
            Ok(()) => log::info!("Migrated managed_addons.json from legacy format to version 1"),
            Err(e) => log::error!("Error saving migrated managed_addons.json: {e:#}"),
        }
    }

    /// The managed addons by name, see [`Self::read_managed`].
    pub fn managed_addons(&self) -> Result<HashMap<String, Addon>> {
        let _lists = self.lock();
        Ok(self.read_managed()?.addons)
    }

    pub fn add_managed_addon(&self, manifest: &AddonManifest, release: &Release) -> Result<()> {
        let alias = if manifest.alias.is_empty() { manifest.name.replace('_', " ") } else { manifest.alias.clone() };
        let addon = Addon {
            name: manifest.name.clone(),
            alias,
            description: manifest.description.clone(),
            version: release.tag_name.clone(),
            commit: release.tag.sha.clone(),
            author: manifest.author.clone(),
            repo: manifest.repo.clone(),
            is_managed: true,
            updated_at: release.published_at,
            branch: manifest.branch.clone(),
            dependencies: manifest.dependencies.clone(),
        };

        let _lists = self.lock();
        let mut managed = self.read_managed().context("refusing to record managed addon")?;
        managed.addons.insert(manifest.name.clone(), addon);
        self.save_managed(&managed)
    }

    pub fn remove_managed_addon(&self, name: &str) -> Result<()> {
        let _lists = self.lock();
        let mut managed = self.read_managed().context("refusing to save managed addons")?;
        managed.addons.remove(name);
        self.save_managed(&managed)
    }

    /// Writes `managed`, sorted by name, in the format it was read in. The caller holds the lock.
    fn save_managed(&self, managed: &Managed) -> Result<()> {
        let mut addons: Vec<Addon> = managed.addons.values().cloned().map(normalize_addon).collect();
        addons.sort_by(|a, b| a.name.cmp(&b.name));
        let file = self.managed_file().context("resolve managed addons file path")?;
        if managed.legacy {
            let data = serde_json::to_vec_pretty(&addons).context("could not encode managed_addons.json")?;
            file::write_atomic(file.path(), &data, 0o644).context("could not write managed_addons.json")?;
        } else {
            file.save(&ManagedAddonsFile { addons })?;
        }
        log::info!("Managed addons saved to disk");
        Ok(())
    }

    /// Drops the managed record of `name`, whose folder a ZIP archive replaces.
    pub(super) fn remove_managed_addon_after_zip_install(&self, name: &str) -> Result<()> {
        let _lists = self.lock();
        let mut managed = self.read_managed()?;
        if managed.addons.remove(name).is_none() {
            return Ok(());
        }
        self.save_managed(&managed).with_context(|| format!("save managed metadata after replacing {:?}", name))
    }
}
