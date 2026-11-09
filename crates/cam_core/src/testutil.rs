//! Test sandbox: every backend test runs against its own config/cache/AAC dirs in a temporary folder, never the real
//! ones, so tests run in parallel.
use std::path::{Path, PathBuf};

use super::Backend;
use super::addon::AddonDir;
use super::config::Config;
use super::paths::Paths;

pub(crate) struct Sandbox {
    /// Removed with the sandbox.
    root: tempfile::TempDir,
    pub backend: Backend,
    pub aac: PathBuf,
}

impl Sandbox {
    pub fn paths(&self) -> &Paths {
        &self.backend.paths
    }

    pub fn config(&self) -> &Config {
        &self.backend.config
    }

    /// The folder where AAC path detection looks for installs, empty unless a test populates it.
    pub fn install_root(&self) -> &Path {
        &self.backend.paths.install_root
    }

    /// `<sandbox>/AAC/Addon`.
    pub fn addon_dir(&self) -> PathBuf {
        self.aac.join("Addon")
    }

    /// The addon folder the sandboxed backend works on.
    pub fn addons(&self) -> AddonDir<'_> {
        self.backend.addon_dir().unwrap()
    }

    pub fn root(&self) -> &Path {
        self.root.path()
    }
}

/// A fresh sandbox: empty config, cache and install folders, and an `aacpath` pointing at `<sandbox>/AAC`, which holds
/// an empty `Addon` folder.
pub(crate) fn sandbox() -> Sandbox {
    let root = tempfile::tempdir().unwrap();
    let paths = Paths {
        config: root.path().join("config"),
        cache: root.path().join("cache"),
        install_root: root.path().join("installs"),
        #[cfg(unix)]
        socket: root.path().join("instance.sock"),
        sandboxed: true,
    };
    let aac = root.path().join("AAC");
    std::fs::create_dir_all(aac.join("Addon")).unwrap();
    std::fs::create_dir_all(&paths.install_root).unwrap();
    let backend = Backend::new(paths);
    // Detection finds nothing in the empty install folder, so the result is ignored.
    let _ = backend.config.load();
    backend.config.update(|general, _| general.aac_path = aac.to_str().unwrap().to_string()).unwrap();
    Sandbox { root, backend, aac }
}

/// Builds a zip at `path` from (name, content) entries.
pub(crate) fn make_zip(path: &std::path::Path, entries: &[(&str, &[u8])]) {
    use std::io::Write;
    let f = std::fs::File::create(path).unwrap();
    let mut w = zip::ZipWriter::new(f);
    let opts = zip::write::SimpleFileOptions::default();
    for (name, data) in entries {
        if name.ends_with('/') {
            w.add_directory(*name, opts).unwrap();
        } else {
            w.start_file(*name, opts).unwrap();
            w.write_all(data).unwrap();
        }
    }
    w.finish().unwrap();
}
