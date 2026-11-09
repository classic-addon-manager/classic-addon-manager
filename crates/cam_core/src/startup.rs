//! Non-UI startup: argument handling, config/addons.txt bootstrap, single-instance check, persisted state.
use super::config::{Config, ConfigError};
use super::{Backend, auth, ipc, logger};

pub const FATAL_ERROR_TITLE: &str = "Classic Addon Manager Error";
pub const DEEPLINK_SCHEME: &str = "classicaddonmanager";

/// Whether the command line asks for headless update checking. Accepts `-check-updates`, `--check-updates` and
/// `--check-updates=true`, with the usual boolean spellings after `=`.
pub fn is_check_updates_mode(args: &[String]) -> bool {
    for a in args {
        if a == "--" {
            break;
        }
        let stripped = a.strip_prefix("--").or_else(|| a.strip_prefix('-'));
        let Some(flag) = stripped else { break };
        match flag.split_once('=') {
            None if flag == "check-updates" => return true,
            Some(("check-updates", v)) => return matches!(v, "1" | "t" | "T" | "true" | "TRUE" | "True"),
            _ => {}
        }
    }
    false
}

/// One problem for the startup error dialog.
#[derive(Debug)]
pub struct StartupError {
    /// Short headline.
    pub title: String,
    /// What happened and what the user can do.
    pub message: String,
    /// Raw error text (parse errors, OS errors), shown verbatim in a monospace block.
    pub details: Option<String>,
}

impl StartupError {
    fn new(title: &str, message: impl Into<String>, details: Option<String>) -> Self {
        Self { title: title.to_string(), message: message.into(), details }
    }
}

/// What the UI must do after [`run_startup`].
#[derive(Default)]
pub struct StartupOutcome {
    /// This process as the running instance. `None` when another instance answered: exit with code 0 then (`errors`
    /// only go to stderr).
    pub instance: Option<ipc::Instance>,
    /// Problems to show in one error dialog once the window is up.
    pub errors: Vec<StartupError>,
}

impl Config {
    /// [`Config::load`] for startup: a malformed `config.toml` is moved aside and the defaults are loaded instead.
    /// Returns the problems for the startup error dialog, empty when the settings loaded and automatic path detection,
    /// if enabled, found the game folder.
    pub fn load_or_recover(&self) -> Vec<StartupError> {
        let (path, error) = match self.load() {
            Ok(()) => return Vec::new(),
            Err(ConfigError::Malformed { path, error }) => (path, error),
            Err(e) => return vec![load_failure(&e, None)],
        };
        log::error!("Invalid config.toml: {error}");
        let backup = match self.quarantine() {
            Ok(backup) => backup,
            Err(move_error) => {
                log::error!("Could not move config.toml aside: {move_error:#}");
                return vec![load_failure(&ConfigError::Malformed { path, error }, Some(&move_error))];
            }
        };
        let reloaded = self.load();
        // Defaults are only claimed once they loaded. A failed reload gets its own entry.
        let (title, outcome) = match reloaded {
            Err(ConfigError::Malformed { .. } | ConfigError::Other(_)) => ("Settings file was moved aside", ""),
            _ => ("Settings were reset", " and default settings were loaded"),
        };
        let mut errors = vec![StartupError::new(
            title,
            format!("config.toml could not be read, so it was moved to {}{outcome}.", backup.display()),
            Some(error),
        )];
        errors.extend(reloaded.err().map(|e| load_failure(&e, None)));
        errors
    }
}

/// The dialog entry for a failed [`Config::load`]. `move_error` is why a malformed file could not be moved aside.
fn load_failure(e: &ConfigError, move_error: Option<&anyhow::Error>) -> StartupError {
    match e {
        ConfigError::AacNotDetected(reason) => StartupError::new(
            "ArcheAge Classic was not found",
            "Open Settings, disable automatic path detection, and choose the ArcheAge Classic Documents path.",
            Some(reason.clone()),
        ),
        ConfigError::SaveFailed(reason) => {
            log::error!("Error saving the detected ArcheAge Classic path: {reason:#}");
            StartupError::new(
                "Settings could not be saved",
                "The detected ArcheAge Classic path is used for now, but it was not saved and will be detected again next time.",
                Some(reason.clone()),
            )
        }
        // Not fatal: without a window the error would be invisible (no console on Windows / desktop launch).
        ConfigError::Malformed { .. } | ConfigError::Other(_) => {
            log::error!("Error loading config: {e:#}");
            let details = match move_error {
                Some(move_error) => format!("{e}\n\n{move_error:#}"),
                None => e.to_string(),
            };
            StartupError::new(
                "Settings could not be loaded",
                "Changes to settings will not be saved until this is fixed and the app is restarted.",
                Some(details),
            )
        }
    }
}

/// GUI-mode startup: everything that has to happen before the window is built. `first_arg` is the first command line
/// argument after the program name: an auth deeplink is forwarded to an already running instance.
pub fn run_startup(backend: &Backend, first_arg: Option<&str>) -> StartupOutcome {
    let mut out = StartupOutcome { errors: backend.config.load_or_recover(), ..Default::default() };

    match backend.addon_dir() {
        // A settings problem already explains a missing or wrong path.
        Err(e) if out.errors.is_empty() => {
            out.errors.push(StartupError::new("Check the ArcheAge Classic path", format!("{e:#}"), None));
        }
        Err(_) => {}
        Ok(addon_dir) => {
            if let Err(e) = addon_dir.ensure_addons_txt_exists() {
                log::error!("Error creating addons.txt: {e:#}");
                out.errors.push(StartupError::new(
                    "addons.txt could not be created",
                    "Open Settings and verify the ArcheAge Classic path.",
                    Some(format!("{e:#}")),
                ));
            }
        }
    }

    out.instance = ipc::claim_instance(&backend.paths, first_arg);
    if out.instance.is_none() {
        logger::sync();
        return out;
    }

    load_persisted_state(backend, &mut out.errors);
    out
}

/// Must run before the UI's first service calls so they see the auth session.
pub fn load_persisted_state(backend: &Backend, errors: &mut Vec<StartupError>) {
    if let Err(e) = backend.paths.data_dir() {
        log::error!("Cannot access the data directory: {e:#}");
        errors.push(StartupError::new(
            "Data folder is not accessible",
            "Your sign-in and the list of managed addons could not be loaded.",
            Some(format!("{e:#}")),
        ));
        return;
    }

    auth::load_from_disk(&backend.paths);

    if let Ok(addon_dir) = backend.addon_dir() {
        addon_dir.migrate_legacy_managed();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn a(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn check_updates_flag_forms() {
        assert!(is_check_updates_mode(&a(&["--check-updates"])));
        assert!(is_check_updates_mode(&a(&["-check-updates"])));
        assert!(is_check_updates_mode(&a(&["--check-updates=true"])));
        assert!(!is_check_updates_mode(&a(&["--check-updates=false"])));
        assert!(!is_check_updates_mode(&a(&["classicaddonmanager://auth?t=x"])));
        assert!(!is_check_updates_mode(&a(&[])));
    }
}
