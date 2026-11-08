// Everything that is not UI: addon management, GitHub and server access, configuration, logging, self-update.
#![allow(dead_code, unused_imports)]

pub mod addon;
pub mod api;
pub mod appstate;
pub mod auth;
pub mod config;
pub mod file;
pub mod headless;
pub mod ipc;
pub mod logger;
pub mod paths;
pub mod shared;
pub mod startup;
pub mod updater;
pub mod util;
pub mod versioned_json;

#[cfg(test)]
mod tests;
#[cfg(test)]
pub(crate) mod testutil;

use std::sync::Mutex;

use anyhow::Result;

use addon::AddonDir;
use config::Config;
use paths::Paths;

/// The backend state of one process: its folders, the settings, and the lock on the addon folder's lists. The app
/// makes one at startup and shares it, tests make one each.
pub struct Backend {
    pub paths: Paths,
    pub config: Config,
    /// Held while `addons.txt` or `managed_addons.json` is read, changed and written back, so two changes at the same
    /// moment can't lose one.
    addon_lists: Mutex<()>,
}

impl Backend {
    pub fn new(paths: Paths) -> Self {
        Self { config: Config::new(&paths), paths, addon_lists: Mutex::new(()) }
    }

    /// The game's addon folder, `<ArcheAge Classic Documents>/Addon`. Fails while the settings hold no usable path.
    pub fn addon_dir(&self) -> Result<AddonDir<'_>> {
        Ok(AddonDir::new(self.config.aac_dir()?.join("Addon"), &self.paths, &self.addon_lists))
    }
}

/// serde helpers for JSON in the shapes the Go client reads and writes.
pub mod serde_util {
    use serde::{Deserialize, Deserializer};

    /// Reads `null` as the type's default, as Go decodes `null` into a zero value.
    pub fn null_default<'de, D, T>(d: D) -> Result<T, D::Error>
    where
        D: Deserializer<'de>,
        T: Deserialize<'de> + Default,
    {
        Ok(Option::<T>::deserialize(d)?.unwrap_or_default())
    }

    /// An optional timestamp in the form the Go client reads and writes: it has no `null` for times and writes the
    /// zero time (0001-01-01T00:00:00Z) for an unset one. Use with `#[serde(default, with = "go_time")]`.
    ///
    /// Reading gives `None` for `null`, a missing field and any time at or before the Unix epoch, which covers the
    /// zero time. Writing gives the zero time for `None`, so the Go client can read the file.
    pub mod go_time {
        use chrono::{DateTime, TimeZone, Utc};
        use serde::{Deserialize, Deserializer, Serialize, Serializer};

        pub fn serialize<S: Serializer>(time: &Option<DateTime<Utc>>, s: S) -> Result<S::Ok, S::Error> {
            time.unwrap_or_else(|| Utc.with_ymd_and_hms(1, 1, 1, 0, 0, 0).unwrap()).serialize(s)
        }

        pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Option<DateTime<Utc>>, D::Error> {
            Ok(Option::<DateTime<Utc>>::deserialize(d)?.filter(|t| t.timestamp_millis() > 0))
        }
    }
}
