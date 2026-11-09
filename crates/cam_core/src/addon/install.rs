//! What an install reports: its progress, the result of each addon and its dependencies, and why one failed.
use serde::{Deserialize, Serialize};

use crate::api::AddonManifest;

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
            if let Some(v) = cause.downcast_ref::<crate::download::VerificationFailed>() {
                return InstallError::Verification(v.0.clone());
            }
            if let Some(crate::api::ApiError::NotFound) = cause.downcast_ref() {
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
