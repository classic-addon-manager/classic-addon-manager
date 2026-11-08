mod download;
mod log_parser;
mod throttle;
mod zip_extract;

pub use download::{ExpectedDownload, VerificationFailed, download_file, download_verified};
pub use log_parser::{LogParseResult, diagnose_issues, parse_log, parse_log_path};
pub use zip_extract::install_release;

use std::process::Command;

/// Builds a `Command` for a console program (`reg`, `cmd`, ...) that must not open a console window. The release
/// build is a GUI-subsystem app with no console to inherit, so Windows would otherwise allocate a visible one per
/// child. No-op elsewhere.
pub fn hidden_command(program: &str) -> Command {
    #[allow(unused_mut)]
    let mut cmd = Command::new(program);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd
}
