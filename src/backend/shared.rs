use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::serde_util::{go_time, null_default};

/// Application version: single source of truth is `[package] version` in Cargo.toml (`just version`).
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct AddonManifest {
    #[serde(deserialize_with = "null_default")]
    pub name: String,
    #[serde(deserialize_with = "null_default")]
    pub alias: String,
    #[serde(deserialize_with = "null_default")]
    pub dependencies: Vec<String>,
    #[serde(deserialize_with = "null_default")]
    pub description: String,
    #[serde(deserialize_with = "null_default")]
    pub author: String,
    #[serde(deserialize_with = "null_default")]
    pub repo: String,
    #[serde(deserialize_with = "null_default")]
    pub branch: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(deserialize_with = "null_default")]
    pub tags: Vec<String>,
    pub downloads: i64,
    #[serde(rename = "like_percentage")]
    pub like_percent: Option<i8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub votes: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kofi: Option<String>,
    #[serde(with = "go_time")]
    pub added_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warning: Option<String>,
    pub library: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DependencyInfo {
    pub manifest: AddonManifest,
    #[serde(rename = "isInstalled")]
    pub is_installed: bool,
    pub depth: i32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct DependencyResolutionResult {
    pub dependencies: Vec<DependencyInfo>,
    pub errors: Vec<String>,
}

/// Why one addon of an install failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstallError {
    /// The server has no such addon, or no release of it to download.
    NoRelease,
    /// The archive did not match its published size or SHA-256 and was discarded. The text says how it differed.
    Verification(String),
    /// A dependency failed first, so this addon was not attempted.
    Dependency,
    /// Anything else, with its full cause chain.
    Failed(String),
}

impl InstallError {
    pub fn from_error(e: &anyhow::Error) -> Self {
        for cause in e.chain() {
            if let Some(v) = cause.downcast_ref::<super::util::VerificationFailed>() {
                return InstallError::Verification(v.0.clone());
            }
            if let Some(super::api::ApiError::NotFound) = cause.downcast_ref() {
                return InstallError::NoRelease;
            }
            if let Some(e) = cause.downcast_ref::<InstallError>() {
                return e.clone();
            }
        }
        InstallError::Failed(format!("{e:#}"))
    }
}

impl std::fmt::Display for InstallError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InstallError::NoRelease => f.write_str("no release found"),
            InstallError::Verification(detail) => write!(f, "download verification failed: {detail}"),
            InstallError::Dependency => f.write_str("dependency installation failed"),
            InstallError::Failed(message) => f.write_str(message),
        }
    }
}

impl std::error::Error for InstallError {}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct AddonInstallStatus {
    pub name: String,
    pub alias: String,
    pub success: bool,
    pub skipped: bool,
    pub error: Option<InstallError>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct InstallWithDependenciesResult {
    pub success: bool,
    pub dependency_warnings: Vec<String>,
    pub dependencies: Vec<AddonInstallStatus>,
    pub main_addon: AddonInstallStatus,
}

/// Progress of one archive during an install or update: the download, then the extraction. Dependencies are
/// installed before the addon itself.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InstallProgress {
    pub name: String,
    pub alias: String,
    pub tag: String,
    pub received: u64,
    /// Published archive size (`release.size_bytes`).
    pub total: u64,
    /// The whole archive arrived and matched its published size and SHA-256.
    pub verified: bool,
    /// Files written so far. `files_total` stays 0 until extraction starts.
    pub files_extracted: u64,
    pub files_total: u64,
}

impl InstallProgress {
    /// Share of the current stage that is done: bytes while downloading, files while extracting. `None` after the
    /// download passed its checks and before extraction starts.
    pub fn fraction(&self) -> Option<f64> {
        let (done, total) = if !self.verified {
            (self.received, self.total)
        } else if self.files_total > 0 {
            (self.files_extracted, self.files_total)
        } else {
            return None;
        };
        Some(if total == 0 { 0.0 } else { (done as f64 / total as f64).clamp(0.0, 1.0) })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_decodes_nulls_and_optionals() {
        let m: AddonManifest = serde_json::from_str(
            r#"{"name":"a","dependencies":null,"tags":null,"like_percentage":null,"added_at":"2024-01-02T03:04:05Z"}"#,
        )
        .unwrap();
        assert!(m.dependencies.is_empty());
        assert_eq!(m.like_percent, None);
        let out = serde_json::to_value(&m).unwrap();
        assert!(out.get("icon").is_none());
        assert_eq!(out["like_percentage"], serde_json::Value::Null);
    }

    #[test]
    fn catalog_with_null_branch_and_strings_still_parses() {
        // Documented `GET /addons` example, one entry with `branch: null` and null string fields.
        let body = r#"[
          {"name":"OpenAddon","alias":"Open Addon","dependencies":[],"author":"dev","description":"Example addon.",
           "repo":"example/OpenAddon","branch":null,"tags":["UI"],"downloads":42,"like_percentage":100,"votes":3,
           "library":false,"added_at":"2026-09-20T10:00:00Z"},
          {"name":"Other","alias":null,"dependencies":null,"author":null,"description":null,"repo":null,
           "branch":"main","tags":null,"downloads":1,"like_percentage":null,"votes":0,"library":true,
           "added_at":"2026-09-21T10:00:00Z"}
        ]"#;
        let list: Option<Vec<AddonManifest>> = serde_json::from_str(body).unwrap();
        let list = list.unwrap();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].name, "OpenAddon");
        assert_eq!(list[0].branch, "");
        assert_eq!(list[0].repo, "example/OpenAddon");
        assert_eq!(list[1].branch, "main");
        assert_eq!((list[1].alias.as_str(), list[1].author.as_str(), list[1].repo.as_str()), ("", "", ""));
    }
}
