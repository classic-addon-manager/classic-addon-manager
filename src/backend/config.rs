//! `<config dir>/config.toml` as typed settings. Keys are matched case-insensitively because files written by the Go
//! client may use any casing.
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

use anyhow::{Context, Result, anyhow};
use serde::{Deserialize, Deserializer, Serialize};
use thiserror::Error;

use super::file;
use super::paths::{Paths, ensure_dir};

pub const ERR_AAC_PATH_NOT_SET: &str = "ArcheAge Classic path is not set, open Settings, disable automatic path detection and choose the ArcheAge Classic Documents path";

#[derive(Debug, Error)]
pub enum ConfigError {
    /// No ArcheAge Classic installation was found. Carries the detection error text.
    #[error("ArcheAge Classic installation was not detected: {0}")]
    AacNotDetected(String),
    /// `config.toml` exists but is not valid UTF-8 TOML, [`Config::quarantine`] moves it aside.
    #[error("could not read config: {}: {error}", .path.display())]
    Malformed { path: PathBuf, error: String },
    /// The config loaded, but persisting the auto-detected AAC path failed (saving is not blocked).
    #[error("{0}")]
    SaveFailed(String),
    #[error("{0}")]
    Other(String),
}

/// `[general]` values.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct General {
    #[serde(rename = "aacpath", deserialize_with = "lenient_string")]
    pub aac_path: String,
    #[serde(rename = "autodetectpath", deserialize_with = "lenient_bool")]
    pub auto_detect_path: bool,
}

impl Default for General {
    fn default() -> Self {
        Self { aac_path: String::new(), auto_detect_path: true }
    }
}

/// `[ui]` values.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Ui {
    #[serde(rename = "accentcolor", deserialize_with = "lenient_enum")]
    pub accent_color: AccentColorId,
    #[serde(rename = "addonviewmode", deserialize_with = "lenient_enum")]
    pub addon_view_mode: AddonViewMode,
    #[serde(rename = "addonsort", deserialize_with = "lenient_enum")]
    pub addon_sort: AddonSort,
}

/// Accent colour of the theme, stored as `gold`, `amber`, ...
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AccentColorId {
    #[default]
    Gold,
    Amber,
    Rose,
    Violet,
    Blue,
    Cyan,
    Emerald,
    Slate,
}

/// Layout of the addon catalog, stored as `list` or `grid`.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AddonViewMode {
    #[default]
    List,
    Grid,
}

/// What the addon catalog is ordered by.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum AddonSortKey {
    /// New addons first (newest first), then by name, shown as two sections.
    #[default]
    Default,
    Name,
    Downloads,
    Rating,
}

/// Catalog order: a key plus a direction, stored as `default`, `name-asc`, `name-desc`, ...
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AddonSort {
    /// [`AddonSortKey::Default`], which has no direction.
    #[default]
    Default,
    NameAsc,
    NameDesc,
    DownloadsAsc,
    DownloadsDesc,
    RatingAsc,
    RatingDesc,
}

impl AddonSort {
    /// Every order, in menu order.
    pub const ALL: [Self; 7] = [
        Self::Default,
        Self::NameAsc,
        Self::NameDesc,
        Self::DownloadsDesc,
        Self::DownloadsAsc,
        Self::RatingDesc,
        Self::RatingAsc,
    ];

    /// `key` in its natural direction: names A to Z, downloads and ratings highest first.
    pub const fn by(key: AddonSortKey) -> Self {
        match key {
            AddonSortKey::Default => Self::Default,
            AddonSortKey::Name => Self::NameAsc,
            AddonSortKey::Downloads => Self::DownloadsDesc,
            AddonSortKey::Rating => Self::RatingDesc,
        }
    }

    pub const fn key(self) -> AddonSortKey {
        match self {
            Self::Default => AddonSortKey::Default,
            Self::NameAsc | Self::NameDesc => AddonSortKey::Name,
            Self::DownloadsAsc | Self::DownloadsDesc => AddonSortKey::Downloads,
            Self::RatingAsc | Self::RatingDesc => AddonSortKey::Rating,
        }
    }

    /// Whether the order runs from the highest value down. False for [`AddonSort::Default`].
    pub const fn descending(self) -> bool {
        matches!(self, Self::NameDesc | Self::DownloadsDesc | Self::RatingDesc)
    }

    /// Column-header click: the active column flips direction, any other column starts in its natural direction.
    pub const fn clicked(self, key: AddonSortKey) -> Self {
        match (self, key) {
            (Self::NameAsc, AddonSortKey::Name) => Self::NameDesc,
            (Self::NameDesc, AddonSortKey::Name) => Self::NameAsc,
            (Self::DownloadsAsc, AddonSortKey::Downloads) => Self::DownloadsDesc,
            (Self::DownloadsDesc, AddonSortKey::Downloads) => Self::DownloadsAsc,
            (Self::RatingAsc, AddonSortKey::Rating) => Self::RatingDesc,
            (Self::RatingDesc, AddonSortKey::Rating) => Self::RatingAsc,
            _ => Self::by(key),
        }
    }
}

/// A config section's known values plus its unknown keys, kept apart so getters copy only the known values and
/// saving never drops the unknown ones.
#[derive(Debug, Default, Serialize, Deserialize)]
struct Section<T> {
    #[serde(flatten)]
    known: T,
    #[serde(flatten)]
    extra: toml::Table,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default)]
struct Settings {
    general: Section<General>,
    ui: Section<Ui>,
    #[serde(flatten)]
    extra: toml::Table,
}

/// Reads a bool leniently: `"true"` or `"1"` (and the other usual spellings) and non-zero integers count as true,
/// anything else as false.
fn lenient_bool<'de, D: Deserializer<'de>>(d: D) -> Result<bool, D::Error> {
    Ok(match toml::Value::deserialize(d)? {
        toml::Value::Boolean(b) => b,
        toml::Value::String(s) => matches!(s.as_str(), "1" | "t" | "T" | "true" | "TRUE" | "True"),
        toml::Value::Integer(i) => i != 0,
        _ => false,
    })
}

/// Reads a string leniently: scalars are converted to text, tables, arrays and dates become empty.
fn lenient_string<'de, D: Deserializer<'de>>(d: D) -> Result<String, D::Error> {
    Ok(match toml::Value::deserialize(d)? {
        toml::Value::String(s) => s,
        toml::Value::Boolean(b) => b.to_string(),
        toml::Value::Integer(i) => i.to_string(),
        toml::Value::Float(f) => f.to_string(),
        _ => String::new(),
    })
}

/// Reads an enum leniently: a value that names no variant (another type, an unknown or differently cased name) gives
/// the default.
fn lenient_enum<'de, D: Deserializer<'de>, T: Deserialize<'de> + Default>(d: D) -> Result<T, D::Error> {
    Ok(match toml::Value::deserialize(d)? {
        // Only a string names a variant: TOML would also read a one-entry table such as `{ grid = [] }` as one.
        name @ toml::Value::String(_) => name.try_into().unwrap_or_default(),
        _ => T::default(),
    })
}

/// The settings of one process, kept in memory and saved to `config.toml` on every change.
pub struct Config {
    /// The config folder, which holds `config.toml`.
    dir: PathBuf,
    /// Where [`Config::load`] looks for game installs.
    install_root: PathBuf,
    state: Mutex<State>,
}

#[derive(Default)]
struct State {
    settings: Settings,
    /// Set while `config.toml` could not be loaded: saving the in-memory defaults would overwrite the user's file.
    save_blocked: bool,
}

impl Config {
    /// Default settings until [`Config::load`] reads the file in `paths.config`.
    pub fn new(paths: &Paths) -> Self {
        Self { dir: paths.config.clone(), install_root: paths.install_root.clone(), state: Mutex::default() }
    }

    fn lock(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(|p| p.into_inner())
    }

    /// The ArcheAge Classic Documents folder. Fails when the path isn't set or isn't absolute.
    pub fn aac_dir(&self) -> Result<PathBuf> {
        let path = self.lock().settings.general.known.aac_path.clone();
        if path.is_empty() {
            return Err(anyhow!(ERR_AAC_PATH_NOT_SET));
        }
        let p = PathBuf::from(&path);
        if !p.is_absolute() {
            return Err(anyhow!(
                "ArcheAge Classic path {:?} is not absolute, choose the ArcheAge Classic Documents path in Settings",
                path
            ));
        }
        Ok(p)
    }

    fn file(&self) -> PathBuf {
        self.dir.join("config.toml")
    }

    fn read(&self) -> Result<(), ConfigError> {
        let dir = ensure_dir(&self.dir, "config")
            .map_err(|e| ConfigError::Other(format!("could not create config directory: {e:#}")))?;
        let path = dir.join("config.toml");
        let (settings, created) = match fs::read_to_string(&path) {
            Ok(text) => match parse_settings(&text) {
                Ok(settings) => (settings, false),
                Err(e) => return Err(ConfigError::Malformed { path, error: format!("{e:#}") }),
            },
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => (Settings::default(), true),
            Err(e) if e.kind() == std::io::ErrorKind::InvalidData => {
                return Err(ConfigError::Malformed { path, error: e.to_string() });
            }
            Err(e) => return Err(ConfigError::Other(format!("could not read config: {e}"))),
        };
        let mut state = self.lock();
        *state = State { settings, save_blocked: false };

        if created {
            self.save(&state).map_err(|e| ConfigError::Other(format!("could not write config file: {e:#}")))?;
        }
        Ok(())
    }

    /// Writes `state` to `config.toml`. Callers hold the config lock, so two saves can't write in the wrong order.
    fn save(&self, state: &State) -> Result<()> {
        if state.save_blocked {
            return Err(anyhow!("could not save config: config.toml failed to load, restart the app after fixing it"));
        }
        let text = toml::to_string(&state.settings).context("could not save config")?;
        file::write_atomic(self.file(), text.as_bytes(), 0o644).context("could not save config")
    }

    /// Moves an unparseable `config.toml` aside with [`super::file::quarantine`] so the next [`Config::load`] starts
    /// from defaults. Returns the backup path.
    pub fn quarantine(&self) -> Result<PathBuf> {
        file::quarantine(&self.file())
    }

    /// Loads `config.toml` and detects the game directory. Returns `ConfigError::AacNotDetected` when detection fails
    /// (callers decide whether that is fatal).
    /// When `config.toml` cannot be loaded, the settings in memory are left as they are and [`Config::update`] refuses
    /// to write.
    pub fn load(&self) -> Result<(), ConfigError> {
        if let Err(e) = self.read() {
            self.lock().save_blocked = true;
            return Err(e);
        }

        if !self.lock().settings.general.known.auto_detect_path {
            log::info!("Settings: Override path is enabled, will not detect AAC path");
            return Ok(());
        }

        match detect_aac_path(&self.install_root) {
            Err(e) => {
                // Intentionally not persisted: a temporary startup miss must not overwrite the user's config.
                self.lock().settings.general.known.aac_path.clear();
                Err(ConfigError::AacNotDetected(format!("{e:#}")))
            }
            Ok(aac_path) => self
                .update(|general, _| general.aac_path = aac_path.to_string_lossy().into_owned())
                .map_err(|e| ConfigError::SaveFailed(format!("{e:#}"))),
        }
    }

    pub fn general(&self) -> General {
        self.lock().settings.general.known.clone()
    }

    pub fn ui(&self) -> Ui {
        self.lock().settings.ui.known
    }

    /// Applies `apply` to the known values in memory, saves the file and logs each value that changed. The config lock
    /// is held until the file is written, so two updates at the same moment both reach the file. The in-memory change
    /// stays when saving fails.
    pub fn update(&self, apply: impl FnOnce(&mut General, &mut Ui)) -> Result<()> {
        let mut state = self.lock();
        let (general_before, ui_before) = (state.settings.general.known.clone(), state.settings.ui.known);
        let Settings { general, ui, .. } = &mut state.settings;
        apply(&mut general.known, &mut ui.known);
        self.save(&state)?;
        log_changes("general", &general_before, &state.settings.general.known);
        log_changes("ui", &ui_before, &state.settings.ui.known);
        Ok(())
    }

    /// Stores a user-picked ArcheAge Classic Documents folder, after checking that it holds the `Addon` folder.
    pub fn set_aac_path_checked(&self, path: &str) -> Result<()> {
        if !file::file_exists(Path::new(path).join("Addon")) {
            return Err(anyhow!("invalid AAC documents path, try a different path"));
        }
        self.update(|general, _| general.aac_path = path.to_string())
    }
}

// ---------------------------------------------------------------- parsing

fn lowercase_keys(t: toml::Table) -> toml::Table {
    t.into_iter()
        .map(|(k, v)| {
            let v = match v {
                toml::Value::Table(inner) => toml::Value::Table(lowercase_keys(inner)),
                other => other,
            };
            (k.to_lowercase(), v)
        })
        .collect()
}

/// Keys are matched case-insensitively (the Go client may have written any casing), so they are lower-cased before
/// mapping onto [`Settings`]. The lower-cased table goes back through TOML text because `toml::Value::try_into` would
/// turn datetimes into strings.
fn parse_settings(text: &str) -> Result<Settings> {
    let table: toml::Table = text.parse()?;
    let lowered = toml::to_string(&lowercase_keys(table))?;
    Ok(toml::from_str(&lowered)?)
}

/// Picks the highest numbered `AAClassic<N>` directory name among `names` (empty suffix counts as 0 but loses ties).
pub fn pick_highest_install(names: &[String]) -> Option<String> {
    fn suffix(m: &str) -> Option<&str> {
        let rest = m.strip_prefix("AAClassic")?;
        rest.bytes().all(|b| b.is_ascii_digit()).then_some(rest)
    }
    let mut highest: Option<String> = None;
    for m in names {
        let Some(h) = highest.clone() else {
            highest = Some(m.clone());
            continue;
        };
        let (Some(cur), Some(hi)) = (suffix(m), suffix(&h)) else {
            if *m > h {
                highest = Some(m.clone());
            }
            continue;
        };
        let cur_n: i64 = cur.parse().unwrap_or(0);
        let hi_n: i64 = hi.parse().unwrap_or(0);
        if cur_n > hi_n || (cur_n == hi_n && !cur.is_empty() && hi.is_empty()) {
            highest = Some(m.clone());
        }
    }
    highest
}

/// Documents dir of the Steam/Proton ArcheAge Classic install, relative to the home directory.
#[cfg(target_os = "linux")]
const LINUX_DOCS_SUBPATH: &str =
    ".local/share/AAClassic/steam-compatdata/pfx/drive_c/users/steamuser/Documents/AAClassic";

/// Returns the detected ArcheAge Classic Documents directory. Windows installs are folders named `AAClassic*` in
/// `install_root`, the drive root. The Linux install is the Steam/Proton one in `install_root`, the home directory.
#[cfg_attr(not(any(windows, target_os = "linux")), allow(unused_variables))]
fn detect_aac_path(install_root: &Path) -> Result<PathBuf> {
    const NOT_FOUND: &str = "could not find any ArcheAge Classic installations";

    #[cfg(windows)]
    {
        let mut names: Vec<String> = fs::read_dir(install_root)
            .with_context(|| format!("could not read {}", install_root.display()))?
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|n| n.starts_with("AAClassic"))
            .collect();
        names.sort();
        pick_highest_install(&names).map(|n| install_root.join(n).join("Documents")).ok_or_else(|| anyhow!(NOT_FOUND))
    }

    #[cfg(target_os = "linux")]
    {
        let docs = install_root.join(LINUX_DOCS_SUBPATH);
        if docs.is_dir() { Ok(docs) } else { Err(anyhow!("{NOT_FOUND} (looked for {})", docs.display())) }
    }

    #[cfg(not(any(windows, target_os = "linux")))]
    Err(anyhow!(NOT_FOUND))
}

/// Logs `Config update: <section>.<key> -> <value>` for each key whose value differs between `before` and `after`.
fn log_changes(section: &str, before: &impl Serialize, after: &impl Serialize) {
    let (Ok(toml::Value::Table(before)), Ok(toml::Value::Table(after))) =
        (toml::Value::try_from(before), toml::Value::try_from(after))
    else {
        return;
    };
    for (key, value) in after.iter().filter(|(key, value)| before.get(*key) != Some(value)) {
        match value {
            toml::Value::String(s) => log::info!("Config update: {section}.{key} -> {s}"),
            other => log::info!("Config update: {section}.{key} -> {other}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn m(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn highest_install_prefers_numeric_suffix() {
        assert_eq!(pick_highest_install(&m(&["AAClassic", "AAClassic2", "AAClassic10"])).unwrap(), "AAClassic10");
        assert_eq!(pick_highest_install(&m(&["AAClassic0", "AAClassic"])).unwrap(), "AAClassic0");
        assert_eq!(pick_highest_install(&m(&["AAClassic", "AAClassic0"])).unwrap(), "AAClassic0");
        assert_eq!(pick_highest_install(&m(&["AAClassicBeta", "AAClassic2"])).unwrap(), "AAClassicBeta");
        assert!(pick_highest_install(&[]).is_none());
    }

    #[test]
    fn header_click_flips_active_column_and_resets_others() {
        let downloads = AddonSort::by(AddonSortKey::Downloads);
        assert_eq!(downloads, AddonSort::DownloadsDesc);
        assert_eq!(downloads.clicked(AddonSortKey::Downloads), AddonSort::DownloadsAsc);
        assert_eq!(AddonSort::DownloadsAsc.clicked(AddonSortKey::Name), AddonSort::NameAsc);
        assert_eq!(AddonSort::NameDesc.clicked(AddonSortKey::Name), AddonSort::NameAsc);
        assert_eq!(AddonSort::default().clicked(AddonSortKey::Rating), AddonSort::RatingDesc);
        assert_eq!(AddonSort::default().clicked(AddonSortKey::Default), AddonSort::Default);
    }
}
