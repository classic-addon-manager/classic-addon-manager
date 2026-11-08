use std::path::Path;

use anyhow::{Context, Result, anyhow};

use super::AddonDir;
use crate::backend::util;

impl AddonDir<'_> {
    /// `name` spelled like the installed addon folder it names, which a case-insensitive file system may spell
    /// differently.
    fn existing_addon_name(&self, name: &str) -> Result<String> {
        let installed = match std::fs::metadata(self.path.join(name)) {
            Ok(m) => m,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(name.to_string()),
            Err(e) => return Err(e.into()),
        };
        for entry in std::fs::read_dir(&self.path)? {
            let entry = entry?;
            if !entry.file_type()?.is_dir() {
                continue;
            }
            let info = entry.metadata()?;
            if same_file(&installed, &info) {
                return Ok(entry.file_name().to_string_lossy().into_owned());
            }
        }
        Ok(name.to_string())
    }

    /// The addon a ZIP archive installs as: the archive's file name without its extension, spelled like the installed
    /// addon folder it names. Rejects names that can't be an addon folder.
    pub fn zip_addon_name(&self, zip_path: &Path) -> Result<String> {
        let file_name = zip_path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        // Everything before the last dot, so `.zip` gives an empty name.
        let addon_name = file_name.rsplit_once('.').map_or(file_name.as_str(), |(stem, _)| stem).to_string();

        if addon_name.is_empty()
            || addon_name == "."
            || addon_name == ".."
            || addon_name.ends_with('.')
            || addon_name.ends_with(' ')
        {
            return Err(anyhow!("invalid addon name derived from zip file name {:?}", file_name));
        }
        let reserved = |n: &str| anyhow!("addon name {:?} is reserved for update notifications", n);
        if addon_name.eq_ignore_ascii_case("AddonUpdateNotification") {
            return Err(reserved(&addon_name));
        }
        let addon_name = self.existing_addon_name(&addon_name).context("resolve existing addon name")?;
        if addon_name.eq_ignore_ascii_case("AddonUpdateNotification") {
            return Err(reserved(&addon_name));
        }
        Ok(addon_name)
    }

    /// Installs the archive at `zip_path` as the addon `addon_name` (see [`Self::zip_addon_name`]) and adds it to
    /// addons.txt.
    pub fn install_zip(&self, zip_path: &Path, addon_name: &str) -> Result<()> {
        let dest = self.path.join(addon_name);
        let commit = || self.remove_managed_addon_after_zip_install(addon_name);
        util::install_release(zip_path, &dest, &mut |_, _| {}, Some(&commit))?;

        self.add_to_addons_txt(addon_name)?;

        // The import itself succeeded: a load-order cycle or write failure only leaves addons.txt unsorted.
        if let Err(e) = self.sort_addons_txt() {
            log::error!("Error sorting addons.txt: {e:#}");
        }

        Ok(())
    }
}

#[cfg(unix)]
fn same_file(a: &std::fs::Metadata, b: &std::fs::Metadata) -> bool {
    use std::os::unix::fs::MetadataExt;
    a.dev() == b.dev() && a.ino() == b.ino()
}

#[cfg(not(unix))]
fn same_file(_a: &std::fs::Metadata, _b: &std::fs::Metadata) -> bool {
    false
}
