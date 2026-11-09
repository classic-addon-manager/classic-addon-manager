//! Toasts for dependency installs and the description of a failed install.
use cam_core::addon::{AddonInstallStatus, InstallError, InstallWithDependenciesResult};
use cam_ui::{icons, toast};
use gpui_kit::App;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FailureKind {
    /// A dependency failed.
    Dependencies,
    /// The parent addon itself failed.
    Addon,
    /// An archive of the addon or of a dependency failed its size or SHA-256 check and was discarded.
    Verification,
}

impl FailureKind {
    /// Title for the failure alert or toast. `addon_title` is used for [`FailureKind::Addon`].
    pub fn title(self, addon_title: &'static str) -> &'static str {
        match self {
            FailureKind::Dependencies => "Dependency installation failed",
            FailureKind::Addon => addon_title,
            FailureKind::Verification => "Download verification failed",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DependencyFailure {
    pub kind: FailureKind,
    pub description: String,
}

/// Toasts dependency warnings and each dependency that was newly installed.
pub fn notify_dependency_progress(result: &InstallWithDependenciesResult, cx: &mut App) {
    if let Some(warning) = result.dependency_warnings.first() {
        toast::show(
            cx,
            toast::Toast::new("Dependency resolution warning", warning.clone()).icon(icons::TRIANGLE_ALERT),
        );
    }
    for dep in result.dependencies.iter().filter(|d| d.success && !d.skipped) {
        toast::show(cx, toast::Toast::new("Dependency installed", format!("{} installed successfully", dep.alias)));
    }
}

/// Describes why an install failed, or `None` when it succeeded.
pub fn dependency_failure(
    result: &InstallWithDependenciesResult,
    parent_failure_fallback: &str,
) -> Option<DependencyFailure> {
    if result.success {
        return None;
    }
    let failed: Vec<&AddonInstallStatus> = result.dependencies.iter().filter(|d| !d.success).collect();
    if let Some(failure) = failed.iter().find_map(|d| verification_failure(d)) {
        return Some(failure);
    }
    if !failed.is_empty() {
        let names: Vec<&str> = failed.iter().map(|d| display_name(d)).collect();
        return Some(DependencyFailure {
            kind: FailureKind::Dependencies,
            description: format!("Failed: {}", names.join(", ")),
        });
    }
    if let Some(failure) = verification_failure(&result.main_addon) {
        return Some(failure);
    }
    Some(DependencyFailure {
        kind: FailureKind::Addon,
        description: match &result.main_addon.error {
            None => parent_failure_fallback.to_string(),
            Some(error) => error.to_string(),
        },
    })
}

fn display_name(status: &AddonInstallStatus) -> &str {
    if status.alias.is_empty() { &status.name } else { &status.alias }
}

/// Only names the addon whose download was discarded. Dependencies installed before it stay installed.
fn verification_failure(status: &AddonInstallStatus) -> Option<DependencyFailure> {
    let Some(InstallError::Verification(detail)) = &status.error else { return None };
    Some(DependencyFailure {
        kind: FailureKind::Verification,
        description: format!("The download of {} was discarded: {detail}.", display_name(status)),
    })
}

/// Progress toasts plus an error toast on failure. Returns `true` on success.
pub fn notify_dependency_result(
    result: &InstallWithDependenciesResult,
    parent_failure_fallback: &str,
    cx: &mut App,
) -> bool {
    notify_dependency_progress(result, cx);
    let failure = dependency_failure(result, parent_failure_fallback);
    if let Some(failure) = &failure {
        toast::show(
            cx,
            toast::Toast::new(failure.kind.title("Error"), failure.description.clone()).icon(icons::TRIANGLE_ALERT),
        );
    }
    failure.is_none()
}

#[cfg(test)]
mod tests {
    use super::*;
    use cam_core::addon::AddonInstallStatus;

    fn status(name: &str, alias: &str, success: bool, error: Option<InstallError>) -> AddonInstallStatus {
        AddonInstallStatus { name: name.into(), alias: alias.into(), success, skipped: false, error }
    }

    #[test]
    fn success_has_no_failure() {
        let r = InstallWithDependenciesResult { success: true, ..Default::default() };
        assert_eq!(dependency_failure(&r, "x"), None);
    }

    #[test]
    fn failed_dependencies_are_listed_by_alias_or_name() {
        let r = InstallWithDependenciesResult {
            success: false,
            dependencies: vec![
                status("a", "Alpha", false, None),
                status("b", "", false, None),
                status("c", "C", true, None),
            ],
            ..Default::default()
        };
        let f = dependency_failure(&r, "x").unwrap();
        assert_eq!(f.kind, FailureKind::Dependencies);
        assert_eq!(f.description, "Failed: Alpha, b");
    }

    #[test]
    fn main_addon_error_or_fallback() {
        let mut r = InstallWithDependenciesResult { success: false, ..Default::default() };
        let f = dependency_failure(&r, "Failed to update Foo").unwrap();
        assert_eq!((f.kind, f.description.as_str()), (FailureKind::Addon, "Failed to update Foo"));
        r.main_addon = status("m", "M", false, Some(InstallError::Failed("boom".into())));
        assert_eq!(dependency_failure(&r, "fb").unwrap().description, "boom");
    }

    #[test]
    fn verification_errors_name_the_discarded_addon() {
        let mismatch = || Some(InstallError::Verification("SHA-256 mismatch (expected a…, got b…)".into()));
        let mut r = InstallWithDependenciesResult { success: false, ..Default::default() };
        r.main_addon = status("m", "Main", false, mismatch());
        let f = dependency_failure(&r, "fb").unwrap();
        assert_eq!(f.kind, FailureKind::Verification);
        assert_eq!(f.description, "The download of Main was discarded: SHA-256 mismatch (expected a…, got b…).");

        // A dependency that fails the check keeps its details instead of a bare "Failed: lib".
        r.dependencies = vec![status("lib", "", false, mismatch())];
        r.main_addon = status("m", "Main", false, Some(InstallError::Dependency));
        let f = dependency_failure(&r, "fb").unwrap();
        assert_eq!(f.kind, FailureKind::Verification);
        assert_eq!(f.description, "The download of lib was discarded: SHA-256 mismatch (expected a…, got b…).");
    }
}
