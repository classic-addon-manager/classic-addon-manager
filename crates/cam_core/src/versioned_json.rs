//! JSON files whose top-level object has a `version` number next to the data: `auth_session.json`, `app_state.json`
//! and `managed_addons.json`.
use std::io;
use std::marker::PhantomData;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow};
use serde::{Deserialize, Serialize, de::DeserializeOwned};

use super::file;

/// The file layout: `version` first, then the fields of `T`.
#[derive(Serialize, Deserialize)]
struct Versioned<T> {
    version: i64,
    #[serde(flatten)]
    data: T,
}

/// Error context for an unusable file that could not be moved aside. The file is still in place, so saving would
/// replace it without a backup. Test for it with [`not_moved_aside`].
#[derive(Debug)]
struct NotMovedAside(String);

impl std::fmt::Display for NotMovedAside {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Whether `error` reports an unusable file that is still in place (see [`VersionedJson::quarantine`]). Callers then
/// must not save over it.
pub fn not_moved_aside(error: &anyhow::Error) -> bool {
    error.downcast_ref::<NotMovedAside>().is_some()
}

/// One versioned JSON file holding a `T`. Saves are atomic and pretty-printed with a 2-space indent, the layout the Go
/// client uses for the session and app state. New files get `mode` on unix.
pub struct VersionedJson<T> {
    path: PathBuf,
    version: i64,
    mode: u32,
    data: PhantomData<fn() -> T>,
}

impl<T: Serialize + DeserializeOwned> VersionedJson<T> {
    pub fn new(path: PathBuf, version: i64, mode: u32) -> Self {
        Self { path, version, mode, data: PhantomData }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    fn name(&self) -> std::borrow::Cow<'_, str> {
        self.path.file_name().unwrap_or(self.path.as_os_str()).to_string_lossy()
    }

    /// The file's data, `None` when the file doesn't exist. A file that can't be read, isn't JSON for `T` or has
    /// another version is moved aside (see [`Self::quarantine`]), so the next save doesn't replace it. The error then
    /// names the backup, or is marked with [`not_moved_aside`] when the move failed.
    pub fn load(&self) -> Result<Option<T>> {
        match self.read()? {
            Some(data) => self.decode(&data).map(Some),
            None => Ok(None),
        }
    }

    /// The file's bytes, `None` when it doesn't exist. A file that can't be read is moved aside.
    pub fn read(&self) -> Result<Option<Vec<u8>>> {
        match std::fs::read(&self.path) {
            Ok(data) => Ok(Some(data)),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(self.quarantine(anyhow::Error::new(e).context("could not read it"))),
        }
    }

    /// Decodes bytes [`Self::read`] returned. Data that isn't JSON for `T` or has another version is moved aside.
    pub fn decode(&self, data: &[u8]) -> Result<T> {
        let file: Versioned<T> = match serde_json::from_slice(data) {
            Ok(file) => file,
            Err(e) => return Err(self.quarantine(anyhow::Error::new(e).context("invalid JSON"))),
        };
        if file.version != self.version {
            return Err(self.quarantine(anyhow!("unsupported version {}", file.version)));
        }
        Ok(file.data)
    }

    /// Moves the file to `<name>.invalid-<YYYYmmdd-HHMMSS>` because of `reason` and returns the error to report: the
    /// reason with the backup path, or with the failure to move the file. The second one is marked, see
    /// [`not_moved_aside`].
    pub fn quarantine(&self, reason: anyhow::Error) -> anyhow::Error {
        match file::quarantine(&self.path) {
            Ok(backup) => reason.context(format!("{} was moved to {}", self.name(), backup.display())),
            Err(e) => reason.context(NotMovedAside(format!("{} could not be moved aside ({e:#})", self.name()))),
        }
    }

    pub fn save(&self, data: &T) -> Result<()> {
        let bytes = serde_json::to_vec_pretty(&Versioned { version: self.version, data })
            .with_context(|| format!("could not encode {}", self.name()))?;
        file::write_atomic(&self.path, &bytes, self.mode).with_context(|| format!("could not write {}", self.name()))
    }
}
