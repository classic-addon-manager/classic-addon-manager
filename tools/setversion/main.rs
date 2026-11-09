//! Sets or bumps `[workspace.package] version` in ./Cargo.toml (run from the project root via `just version`).
//! usage: setversion [major|minor|patch|X.Y.Z]   (default: patch, a leading `v` is accepted)
use std::{env, fs, process::ExitCode};

fn main() -> ExitCode {
    let target = env::args().nth(1).unwrap_or_else(|| "patch".into());
    match run(&target) {
        Ok((old, new)) => {
            println!("version: {old} -> {new}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}

fn run(target: &str) -> Result<(String, String), String> {
    let src = fs::read_to_string("Cargo.toml")
        .map_err(|e| format!("cannot read Cargo.toml (run from the project root): {e}"))?;
    let (old, new, out) = update(&src, target)?;
    fs::write("Cargo.toml", out).map_err(|e| format!("cannot write Cargo.toml: {e}"))?;
    Ok((old, new))
}

fn parse(v: &str) -> Option<[u64; 3]> {
    let parts: Vec<&str> = v.split('.').collect();
    if parts.len() != 3 {
        return None;
    }
    let mut out = [0; 3];
    for (slot, p) in out.iter_mut().zip(parts) {
        let digits = !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit());
        if !digits || (p.len() > 1 && p.starts_with('0')) {
            return None;
        }
        *slot = p.parse().ok()?;
    }
    Some(out)
}

/// Returns (old version, new version, new file contents). Only the `version` key of the `[workspace.package]` table
/// changes, the version every crate of the workspace inherits.
fn update(src: &str, target: &str) -> Result<(String, String, String), String> {
    let mut lines: Vec<String> = src.split_inclusive('\n').map(str::to_owned).collect();
    let mut in_package = false;
    let mut found = None;
    for (i, line) in lines.iter().enumerate() {
        let t = line.trim();
        if t.starts_with('[') {
            in_package = t == "[workspace.package]";
        } else if in_package
            && let Some(rest) = t.strip_prefix("version")
            && let Some(value) = rest.trim_start().strip_prefix('=')
            && let Some(old) = value.trim().strip_prefix('"').and_then(|v| v.split('"').next())
        {
            found = Some((i, old.to_owned()));
            break;
        }
    }
    let (idx, old) = found.ok_or("no [workspace.package] version = \"X.Y.Z\" found in Cargo.toml")?;
    let [major, minor, patch] =
        parse(&old).ok_or_else(|| format!("current version '{old}' is not X.Y.Z"))?;
    let new = match target {
        "major" => format!("{}.0.0", major + 1),
        "minor" => format!("{major}.{}.0", minor + 1),
        "patch" => format!("{major}.{minor}.{}", patch + 1),
        _ => {
            let v = target.strip_prefix('v').unwrap_or(target);
            parse(v).ok_or_else(|| format!("expected major, minor, patch, or X.Y.Z, got '{target}'"))?;
            v.to_owned()
        }
    };
    let eol = if lines[idx].ends_with("\r\n") { "\r\n" } else if lines[idx].ends_with('\n') { "\n" } else { "" };
    lines[idx] = format!("version = \"{new}\"{eol}");
    Ok((old, new, lines.concat()))
}

#[cfg(test)]
mod tests {
    use super::update;

    #[test]
    fn only_workspace_package_version_changes() {
        let src = "[workspace]\nmembers = []\n\n[workspace.package]\nversion = \"1.2.3\"\n\n\
                   [workspace.dependencies.foo]\nversion = \"9.9.9\"\n";
        let (old, new, out) = update(src, "minor").unwrap();
        assert_eq!((old.as_str(), new.as_str()), ("1.2.3", "1.3.0"));
        assert_eq!(out, src.replace("1.2.3", "1.3.0"));
        assert!(update("[package]\nversion = \"1.2.3\"\n", "minor").is_err());
    }

    #[test]
    fn keeps_crlf_and_rejects_bad_targets() {
        let src = "[workspace.package]\r\nversion = \"0.1.0\"\r\n";
        assert_eq!(update(src, "v2.0.1").unwrap().2, "[workspace.package]\r\nversion = \"2.0.1\"\r\n");
        for bad in ["1.2", "01.2.3", "foo", "1.2.3.4", ""] {
            assert!(update(src, bad).is_err(), "{bad}");
        }
    }
}
