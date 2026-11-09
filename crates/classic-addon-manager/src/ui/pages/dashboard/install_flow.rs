//! Helpers of the update dialog and the version picker: the failure an update through the addon store ends with, its
//! text, and colour blending.
use anyhow::{Error, Result};
use cam_core::addon::{InstallError, InstallWithDependenciesResult};
use gpui_kit::App;

use super::phase_ui::Failure;
use crate::{
    state::{self, tasks::error_message},
    ui::{
        dependency_notify::{dependency_failure, notify_dependency_progress},
        format::truncate_chars,
    },
};

/// What the dialog did: updated an installed addon, or installed a version picked from the list.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verb {
    Update,
    Install,
}

impl Verb {
    fn word(self) -> &'static str {
        match self {
            Verb::Update => "update",
            Verb::Install => "install",
        }
    }

    fn failed(self) -> &'static str {
        match self {
            Verb::Update => "Update failed",
            Verb::Install => "Install failed",
        }
    }

    fn doing(self) -> &'static str {
        match self {
            Verb::Update => "Updating",
            Verb::Install => "Installing",
        }
    }
}

/// The failure to show after the store updated `name` to `tag`, `None` after a success. Logs a failure the store
/// returned and toasts dependency warnings and newly installed dependencies.
pub fn outcome_failure(
    outcome: Result<InstallWithDependenciesResult>,
    name: &str,
    alias: &str,
    tag: &str,
    verb: Verb,
    cx: &mut App,
) -> Option<Failure> {
    match outcome {
        Err(e) => {
            state::log_failure(&format!("{} {name} {tag} failed:", verb.doing()), &e);
            Some(Failure { title: verb.failed().to_string(), description: failure_description(&e, verb.word()) })
        }
        Ok(result) => {
            notify_dependency_progress(&result, cx);
            dependency_failure(&result, &format!("Failed to {} {alias}", verb.word())).map(|failure| Failure {
                title: failure.kind.title(verb.failed()).to_string(),
                description: failure.description,
            })
        }
    }
}

/// Error text of an update or install that failed before the store recorded a per-addon result.
pub fn failure_description(error: &Error, verb: &str) -> String {
    if InstallError::from_error(error) == InstallError::NoRelease {
        "No release found for this addon".to_string()
    } else {
        format!("Failed to {verb} addon: {}", truncate_chars(&error_message(error), 100))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failure_text() {
        let not_found = Error::new(cam_core::api::ApiError::NotFound).context("looking up release");
        assert_eq!(failure_description(&not_found, "install"), "No release found for this addon");
        assert_eq!(failure_description(&anyhow::anyhow!("boom"), "install"), "Failed to install addon: Boom");
        assert_eq!(
            failure_description(&anyhow::anyhow!("x".repeat(150)), "update").len(),
            "Failed to update addon: ".len() + 100
        );
    }
}
