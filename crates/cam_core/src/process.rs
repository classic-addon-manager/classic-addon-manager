//! Child processes started by the app on Windows.
use std::{os::windows::process::CommandExt, process::Command};

/// Builds a `Command` for a console program (`reg`, `cmd`, ...) that must not open a console window. The release
/// build is a GUI-subsystem app with no console to inherit, so Windows would otherwise allocate a visible one per
/// child.
pub fn hidden_command(program: &str) -> Command {
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let mut cmd = Command::new(program);
    cmd.creation_flags(CREATE_NO_WINDOW);
    cmd
}
