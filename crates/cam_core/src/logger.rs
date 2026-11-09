//! JSON-lines log in `<config dir>/app.log`, the same location the Go client uses. It is installed as the logger behind
//! the `log` macros, so the app's lines and those of its libraries land in the same file.
use std::borrow::Cow;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use log::{Level, LevelFilter, Log, Metadata, Record};

const MAX_SIZE_BYTES: u64 = 10 * 1024 * 1024;
const MAX_BACKUPS: usize = 3;
const HEADLESS_MAX_BYTES: u64 = 1 << 20;

enum Sink {
    Rotating { path: PathBuf, file: File, size: u64 },
    Append(File),
    Stderr,
}

/// Records whose target starts with one of these crate names come from the app itself: the crates of this workspace.
const APP_CRATES: [&str; 3] = ["cam_core", "cam_ui", "classic_addon_manager"];

struct FileLogger {
    /// The folder of the log files, `None` when it couldn't be resolved: lines go to stderr then.
    dir: Option<PathBuf>,
    /// Logs to `check-updates.log` instead of the rotating `app.log`.
    headless: bool,
    /// Opened on the first line.
    sink: OnceLock<Mutex<Sink>>,
}

/// The `log` facade needs a logger that lives as long as the process.
static LOGGER: OnceLock<FileLogger> = OnceLock::new();

/// Installs the file logger behind the `log` macros, logging into `dir`. Lines logged before this call are dropped, so
/// `main` calls it first. `headless` routes the lines to `check-updates.log` instead of the rotating `app.log`.
pub fn init(dir: Option<PathBuf>, headless: bool) {
    let logger = LOGGER.get_or_init(|| FileLogger { dir, headless, sink: OnceLock::new() });
    if log::set_logger(logger).is_ok() {
        log::set_max_level(LevelFilter::Info);
    }
}

impl Log for FileLogger {
    /// The app logs from info up. Libraries log only warnings and errors, their info lines are noise here.
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Warn || is_app_target(metadata.target())
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            write_entry(self, record);
        }
    }

    fn flush(&self) {
        let Some(sink) = self.sink.get() else { return };
        let Ok(mut g) = sink.lock() else { return };
        match &mut *g {
            Sink::Rotating { file, .. } => {
                let _ = file.flush();
            }
            Sink::Append(f) => {
                let _ = f.flush();
            }
            Sink::Stderr => {}
        }
    }
}

fn is_app_target(target: &str) -> bool {
    APP_CRATES
        .iter()
        .any(|name| target.strip_prefix(name).is_some_and(|rest| rest.is_empty() || rest.starts_with("::")))
}

fn open_sink(dir: Option<&Path>, headless: bool) -> Sink {
    let Some(dir) = dir else {
        eprintln!("file logging unavailable: no config directory");
        return Sink::Stderr;
    };
    if let Err(e) = super::paths::mkdir_private(dir) {
        eprintln!("file logging unavailable: create log directory: {e}");
        return Sink::Stderr;
    }
    if headless {
        let path = dir.join("check-updates.log");
        if let Ok(meta) = std::fs::metadata(&path)
            && meta.len() > HEADLESS_MAX_BYTES
        {
            let mut bak = path.clone().into_os_string();
            bak.push(".1");
            let _ = std::fs::rename(&path, PathBuf::from(bak));
        }
        match open_append(&path) {
            Ok(f) => Sink::Append(f),
            Err(e) => {
                eprintln!("file logging unavailable: open log file: {e}");
                Sink::Stderr
            }
        }
    } else {
        let app_log = dir.join("app.log");
        match open_append(&app_log) {
            Ok(file) => {
                let size = file.metadata().map(|m| m.len()).unwrap_or(0);
                Sink::Rotating { path: app_log, file, size }
            }
            Err(e) => {
                eprintln!("file logging unavailable: open log file: {e}");
                Sink::Stderr
            }
        }
    }
}

fn open_append(path: &Path) -> std::io::Result<File> {
    let mut o = OpenOptions::new();
    o.create(true).append(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        o.mode(0o600);
    }
    o.open(path)
}

/// Rotates the log: app.log is renamed to app-<timestamp>.log and only the newest MAX_BACKUPS backups are kept.
fn rotate(path: &Path) -> std::io::Result<(File, u64)> {
    let dir = path.parent().unwrap_or(Path::new("."));
    let stamp = chrono::Utc::now().format("%Y-%m-%dT%H-%M-%S%.3f");
    let backup = dir.join(format!("app-{stamp}.log"));
    std::fs::rename(path, backup)?;
    let mut backups: Vec<PathBuf> = std::fs::read_dir(dir)?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            p.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.starts_with("app-") && n.ends_with(".log"))
        })
        .collect();
    backups.sort();
    while backups.len() > MAX_BACKUPS {
        let _ = std::fs::remove_file(backups.remove(0));
    }
    Ok((open_append(path)?, 0))
}

/// Where a line was logged: `cam_core/config.rs:42` for the app (the crate and the path below its `src/`), the target
/// and line for a library, whose file path would point into the cargo registry. A library record without a target
/// (gpui's `log_err`) gives its file from the crate directory on: `gpui-pre-windows-0.3.11/src/window.rs:2202`.
fn caller(record: &Record) -> String {
    let line = record.line().unwrap_or(0);
    let target = record.target();
    let Some(file) = record.file() else { return format!("{target}:{line}") };
    let file = file.replace('\\', "/");
    if is_app_target(target) {
        // `crates/cam_core/src/config.rs` -> `cam_core/config.rs`
        let file = file.strip_prefix("crates/").unwrap_or(&file);
        format!("{}:{line}", file.replacen("/src/", "/", 1))
    } else if target.is_empty() {
        let crate_dir = file.rfind("/src/").and_then(|i| file[..i].rfind('/')).map_or(0, |i| i + 1);
        format!("{}:{line}", &file[crate_dir..])
    } else {
        format!("{target}:{line}")
    }
}

fn level_name(level: Level) -> &'static str {
    match level {
        Level::Error => "error",
        Level::Warn => "warn",
        Level::Info => "info",
        Level::Debug => "debug",
        Level::Trace => "trace",
    }
}

#[derive(serde::Serialize)]
struct Entry<'a> {
    level: &'static str,
    ts: &'a str,
    caller: &'a str,
    msg: &'a str,
}

fn write_entry(logger: &FileLogger, record: &Record) {
    let ts = chrono::Local::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let caller = caller(record);
    let args = record.args();
    let msg = args.as_str().map_or_else(|| Cow::Owned(args.to_string()), Cow::Borrowed);
    let entry = Entry { level: level_name(record.level()), ts: &ts, caller: &caller, msg: &msg };
    let Ok(mut line) = serde_json::to_string(&entry) else { return };
    line.push('\n');

    if cfg!(debug_assertions) {
        let _ = writeln!(std::io::stdout(), "{ts}\t{}\t{caller}\t{msg}", record.level());
    }

    let sink = logger.sink.get_or_init(|| Mutex::new(open_sink(logger.dir.as_deref(), logger.headless)));
    let mut guard = match sink.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    match &mut *guard {
        Sink::Rotating { path, file, size } => {
            if *size + line.len() as u64 > MAX_SIZE_BYTES
                && let Ok((f, s)) = rotate(path)
            {
                *file = f;
                *size = s;
            }
            if file.write_all(line.as_bytes()).is_ok() {
                *size += line.len() as u64;
            }
        }
        Sink::Append(f) => {
            let _ = f.write_all(line.as_bytes());
        }
        Sink::Stderr => {
            let _ = std::io::stderr().write_all(line.as_bytes());
        }
    }
}

/// Flushes the log file. Writes are not buffered, so this only matters for callers that want a flush before exit.
pub fn sync() {
    log::logger().flush();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn caller_of(target: &str, file: &str) -> String {
        caller(&Record::builder().target(target).file(Some(file)).line(Some(42)).build())
    }

    #[test]
    fn caller_names_app_files_by_crate_and_libraries_by_target() {
        assert_eq!(caller_of("cam_core::config", "crates/cam_core/src/config.rs"), "cam_core/config.rs:42");
        assert_eq!(
            caller_of("classic_addon_manager::state::addons", "crates/classic-addon-manager/src/state/addons.rs"),
            "classic-addon-manager/state/addons.rs:42"
        );
        assert_eq!(caller_of("cam_ui", "crates\\cam_ui\\src\\lib.rs"), "cam_ui/lib.rs:42");
        assert_eq!(caller_of("ureq::unversioned", "/home/u/.cargo/registry/ureq-3/src/run.rs"), "ureq::unversioned:42");
        assert_eq!(caller_of("", "/r/gpui-0.3.11/src/window.rs"), "gpui-0.3.11/src/window.rs:42");
        // A crate whose name only starts with an app crate's name is a library.
        assert_eq!(caller_of("cam_core_extra", "/r/cam_core_extra/src/lib.rs"), "cam_core_extra:42");
    }
}
