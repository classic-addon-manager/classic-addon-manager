use std::collections::HashMap;
use std::fmt::Write as _;

use anyhow::{Context, Result};

use super::{Addon, AddonDir};
use crate::backend::api::{self, Release};
use crate::backend::file;

static LUA_SCRIPT: &[u8] = include_bytes!("cam.lua");

/// Result of a successful update check.
pub struct UpdateCheck {
    /// Managed addons with a newer release, carrying the new version.
    pub updates: HashMap<String, Addon>,
    /// Managed addons the server has no release info for, sorted by name.
    pub missing: Vec<String>,
}

impl AddonDir<'_> {
    /// Checks every managed addon for a newer release. `Err` means the check could not run at all (the managed addons
    /// file is unreadable or the release request failed), so callers must not regenerate the notification and lose the
    /// previously reported updates.
    pub fn check_for_updates(&self) -> Result<UpdateCheck> {
        let addons = {
            let _lists = self.lock();
            self.load_managed().context("load managed addons")?.addons
        };
        let mut updates = HashMap::new();
        if addons.is_empty() {
            return Ok(UpdateCheck { updates, missing: Vec::new() });
        }

        let mut names: Vec<String> = addons.keys().cloned().collect();
        names.sort();

        let releases = api::latest_releases(&names).context("fetch latest releases")?;

        let mut missing = Vec::new();
        for (name, a) in &addons {
            match releases.get(name) {
                Some(r) if !r.tag_name.is_empty() => {
                    if has_update(a, r) {
                        let mut updated = a.clone();
                        updated.version = r.tag_name.clone();
                        updates.insert(name.clone(), updated);
                    }
                }
                _ => missing.push(name.clone()),
            }
        }
        missing.sort();
        Ok(UpdateCheck { updates, missing })
    }

    /// Writes the `AddonUpdateNotification` addon, which shows `updates` in the game, and lists it in addons.txt.
    pub fn generate_update_addon_lua(&self, updates: &HashMap<String, Addon>) -> Result<()> {
        let addon_path = self.path.join("AddonUpdateNotification");

        if !addon_path.exists() {
            std::fs::create_dir_all(&addon_path).context("create AddonUpdateNotification directory")?;
        }

        self.installed_names().context("read addons.txt")?;

        self.add_to_addons_txt("AddonUpdateNotification").context("add AddonUpdateNotification to addons.txt")?;

        file::write_atomic(addon_path.join("main.lua"), LUA_SCRIPT, 0o644)
            .context("write AddonUpdateNotification main.lua")?;

        if updates.is_empty() {
            return match std::fs::remove_file(addon_path.join("updates.lua")) {
                Ok(()) => Ok(()),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
                Err(e) => Err(anyhow::Error::new(e).context("remove AddonUpdateNotification updates.lua")),
            };
        }

        file::write_atomic(addon_path.join("updates.lua"), &generate_updates_lua(updates), 0o644)
            .context("write AddonUpdateNotification updates.lua")
    }
}

/// The update rule of the headless check and the UI. Only a managed addon gets updates, and only from a release with a
/// tag. A new tag always counts. The same tag counts only if re-published after the addon was installed. Addons saved
/// without a publish time are compared by tag alone.
pub fn has_update(a: &Addon, release: &Release) -> bool {
    if !a.is_managed || release.tag_name.is_empty() {
        return false;
    }
    if release.tag_name != a.version {
        return true;
    }
    a.updated_at.is_some_and(|installed| release.published_at > Some(installed))
}

pub fn generate_updates_lua(updates: &HashMap<String, Addon>) -> Vec<u8> {
    let mut sorted: Vec<&Addon> = updates.values().collect();
    sorted.sort_by(|a, b| a.name.cmp(&b.name));

    let mut out: Vec<u8> = b"{\n".to_vec();
    for addon in sorted {
        let display = if addon.alias.is_empty() { &addon.name } else { &addon.alias };
        out.extend_from_slice(b"    [");
        write_lua_string(&mut out, addon.name.as_bytes());
        out.extend_from_slice(b"] = {name=");
        write_lua_string(&mut out, display.as_bytes());
        out.extend_from_slice(b", version=");
        write_lua_string(&mut out, addon.version.as_bytes());
        out.extend_from_slice(b"}, \n");
    }
    out.extend_from_slice(b"}\n");
    out
}

/// Lua decimal escapes work in Lua 5.1 and stay unambiguous before digits.
fn write_lua_string(out: &mut Vec<u8>, value: &[u8]) {
    out.push(b'"');
    for &c in value {
        match c {
            b'"' | b'\\' => {
                out.push(b'\\');
                out.push(c);
            }
            c if c < b' ' || c == 0x7f => {
                let mut s = String::new();
                let _ = write!(s, "\\{:03}", c);
                out.extend_from_slice(s.as_bytes());
            }
            c => out.push(c),
        }
    }
    out.push(b'"');
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};

    fn addon(name: &str, alias: &str, version: &str) -> Addon {
        Addon { name: name.into(), alias: alias.into(), version: version.into(), ..Default::default() }
    }

    #[test]
    fn lua_quotes_every_field() {
        let mut m = HashMap::new();
        m.insert("a\"b".to_string(), addon("a\"b", "al\\ias\n1", "v\u{7f}"));
        let out = String::from_utf8(generate_updates_lua(&m)).unwrap();
        assert_eq!(out, "{\n    [\"a\\\"b\"] = {name=\"al\\\\ias\\0101\", version=\"v\\127\"}, \n}\n");
    }

    #[test]
    fn lua_uses_name_without_alias() {
        let mut m = HashMap::new();
        m.insert("x".to_string(), addon("x", "", "v2"));
        let out = String::from_utf8(generate_updates_lua(&m)).unwrap();
        assert!(out.contains("[\"x\"] = {name=\"x\", version=\"v2\"}"));
    }

    #[test]
    fn has_update_rules() {
        let t0 = Utc.with_ymd_and_hms(2023, 12, 1, 0, 0, 0).unwrap();
        let t1 = Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();
        let t2 = Utc.with_ymd_and_hms(2024, 2, 1, 0, 0, 0).unwrap();
        let mut a = Addon { is_managed: true, ..addon("x", "", "v1") };
        let mut r = Release { tag_name: "v2".into(), ..Default::default() };
        assert!(has_update(&a, &r));
        assert!(!has_update(&Addon { is_managed: false, ..a.clone() }, &r), "unmanaged");
        r.tag_name = "v1".into();
        r.published_at = Some(t2);
        assert!(!has_update(&a, &r), "no installed publish time: tag only");
        a.updated_at = Some(t1);
        assert!(has_update(&a, &r), "same tag republished");
        r.published_at = Some(t1);
        assert!(!has_update(&a, &r));
        r.published_at = Some(t0);
        assert!(!has_update(&a, &r), "same tag published before the install");
        r.tag_name = String::new();
        r.published_at = Some(t2);
        assert!(!has_update(&a, &r), "no tag");
    }
}
