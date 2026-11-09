use anyhow::{Context, Result};

use super::AddonDir;
use super::{InstallProgress, archive};
use crate::api::{self, AddonManifest, Release};
use crate::download;

impl AddonDir<'_> {
    pub fn install_addon(
        &self,
        manifest: &AddonManifest,
        version: &str,
        progress: &dyn Fn(&InstallProgress),
    ) -> Result<()> {
        self.install_or_update(manifest, version, "Installing", "installed", progress)
    }

    /// Replaces all files of an existing addon, preserving an existing `.data` folder.
    pub fn update_addon(
        &self,
        manifest: &AddonManifest,
        version: &str,
        progress: &dyn Fn(&InstallProgress),
    ) -> Result<()> {
        self.install_or_update(manifest, version, "Updating", "updated", progress)
    }

    fn install_or_update(
        &self,
        manifest: &AddonManifest,
        version: &str,
        verb: &str,
        done: &str,
        progress: &dyn Fn(&InstallProgress),
    ) -> Result<()> {
        self.ensure_addons_txt_exists()?;
        // Checked before the folder is replaced, so an addons.txt that can't be read fails the install with the
        // addon's files untouched.
        self.installed_names().context("reading addons.txt")?;

        log::info!("{verb} addon:{} from {} version: {version}", manifest.name, manifest.repo);

        let release_version = if version.is_empty() { "latest" } else { version };
        let release = api::release(&manifest.name, release_version)
            .with_context(|| format!("looking up release {release_version} of {}", manifest.name))?;
        self.download_and_install(manifest, &release, progress)?;
        self.add_to_addons_txt(&manifest.name)?;
        self.add_managed_addon(manifest, &release)?;

        log::info!("{} {done} successfully", manifest.name);
        Ok(())
    }

    /// Downloads this exact release and installs it only if its size and SHA-256 match. It asks for the tag, not
    /// "latest", because "latest" could change between the release lookup and the download. `progress` reports the
    /// download, then the extracted files.
    fn download_and_install(
        &self,
        manifest: &AddonManifest,
        release: &Release,
        progress: &dyn Fn(&InstallProgress),
    ) -> Result<()> {
        if release.sha256.trim().is_empty() || release.size_bytes == 0 {
            return Err(download::VerificationFailed(format!(
                "release {} has no published checksum",
                release.tag_name
            ))
            .into());
        }
        let zip = self.paths.cache_dir()?.join(format!("{}.zip", manifest.name));
        let dest = self.path.join(&manifest.name);
        let url = build_download_url(manifest, &release.tag_name);

        let mut event = InstallProgress {
            name: manifest.name.clone(),
            alias: if manifest.alias.is_empty() { manifest.name.clone() } else { manifest.alias.clone() },
            tag: release.tag_name.clone(),
            total: release.size_bytes,
            ..Default::default()
        };
        download::download_verified(
            &format!("{}{url}", api::API_URL),
            &zip,
            &download::ExpectedDownload { sha256: &release.sha256, size: release.size_bytes },
            &mut |received| {
                event.received = received;
                progress(&event);
            },
        )?;
        event.verified = true;
        progress(&event);
        log::info!("{} {} verified (SHA-256 {})", manifest.name, release.tag_name, release.sha256);

        let installed = archive::install_release(
            &zip,
            &dest,
            &mut |extracted, total| {
                event.files_extracted = extracted;
                event.files_total = total;
                progress(&event);
            },
            None,
        );
        if let Err(e) = std::fs::remove_file(&zip) {
            log::warn!("Could not remove downloaded archive {:?}: {e}", zip.display().to_string());
        }
        installed.with_context(|| format!("installing {} {}", manifest.name, release.tag_name))
    }
}

/// Download path for one exact release tag. A tag named `latest` is sent as is, because the server treats an
/// explicit `version=latest` as a tag and a missing `version` as the newest release.
fn build_download_url(manifest: &AddonManifest, tag: &str) -> String {
    let tag: String = form_urlencoded::byte_serialize(tag.as_bytes()).collect();
    format!("/addon/{}/download?version={tag}", manifest.name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn download_url() {
        let a = AddonManifest { name: "foo".into(), ..Default::default() };
        assert_eq!(build_download_url(&a, "latest"), "/addon/foo/download?version=latest");
        assert_eq!(build_download_url(&a, "v1.2"), "/addon/foo/download?version=v1.2");
        assert_eq!(build_download_url(&a, "v1.2+build&x"), "/addon/foo/download?version=v1.2%2Bbuild%26x");
    }
}
