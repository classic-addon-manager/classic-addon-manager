//! Flow-level tests against the sandbox: zip safety, addons.txt, load order, install/uninstall, update notification,
//! config.
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use super::addon::{self, Addon};
use super::config::{AccentColorId, AddonSort, AddonViewMode, ConfigError};
use super::shared::AddonManifest;
use super::testutil::{Sandbox, make_zip, sandbox};
use super::{appstate, auth, file, util};

fn write(p: impl AsRef<Path>, s: &str) {
    let p = p.as_ref();
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(p, s).unwrap();
}

fn txt(sb: &Sandbox) -> String {
    fs::read_to_string(sb.addon_dir().join("addons.txt")).unwrap()
}

fn config_file(sb: &Sandbox) -> PathBuf {
    sb.paths().config.join("config.toml")
}

fn managed_file(sb: &Sandbox) -> PathBuf {
    sb.paths().config.join("managed_addons.json")
}

/// Writes `managed_addons.json` with `addons`.
fn write_managed(sb: &Sandbox, addons: &[Addon]) {
    let json = serde_json::json!({ "version": 1, "addons": addons });
    write(managed_file(sb), &json.to_string());
}

// ------------------------------------------------------------------ config

#[test]
fn config_roundtrip_and_override_path() {
    let sb = sandbox();
    sb.config()
        .update(|general, ui| {
            general.auto_detect_path = false;
            ui.accent_color = AccentColorId::Violet;
            ui.addon_view_mode = AddonViewMode::Grid;
            ui.addon_sort = AddonSort::DownloadsAsc;
        })
        .unwrap();
    // The values keep the spelling earlier versions wrote, so their files still load.
    let saved: toml::Table = fs::read_to_string(config_file(&sb)).unwrap().parse().unwrap();
    assert_eq!(saved["general"]["autodetectpath"].as_bool(), Some(false));
    assert_eq!(saved["ui"]["accentcolor"].as_str(), Some("violet"));
    assert_eq!(saved["ui"]["addonviewmode"].as_str(), Some("grid"));
    assert_eq!(saved["ui"]["addonsort"].as_str(), Some("downloads-asc"));

    // Reload from disk, override keeps the stored path.
    sb.config().load().unwrap();
    let ui = sb.config().ui();
    assert_eq!(
        (ui.accent_color, ui.addon_view_mode, ui.addon_sort),
        (AccentColorId::Violet, AddonViewMode::Grid, AddonSort::DownloadsAsc)
    );
    assert!(!sb.config().general().auto_detect_path);
}

#[test]
fn sort_orders_load_from_the_names_earlier_versions_wrote() {
    let sb = sandbox();
    let names = [
        (AddonSort::Default, "default"),
        (AddonSort::NameAsc, "name-asc"),
        (AddonSort::NameDesc, "name-desc"),
        (AddonSort::DownloadsAsc, "downloads-asc"),
        (AddonSort::DownloadsDesc, "downloads-desc"),
        (AddonSort::RatingAsc, "rating-asc"),
        (AddonSort::RatingDesc, "rating-desc"),
    ];
    for (sort, name) in names {
        fs::write(config_file(&sb), format!("[general]\nautodetectpath = false\n[ui]\naddonsort = '{name}'\n"))
            .unwrap();
        sb.config().load().unwrap();
        assert_eq!(sb.config().ui().addon_sort, sort, "{name}");
    }
}

#[test]
fn unknown_preference_values_load_as_defaults() {
    let sb = sandbox();
    let files = [
        "accentcolor = 'Violet'\naddonviewmode = 2\naddonsort = 'downloads'\n",
        "accentcolor = { blue = {} }\naddonviewmode = { grid = [] }\naddonsort = { downloads-desc = {} }\n",
    ];
    for ui in files {
        fs::write(config_file(&sb), format!("[general]\nautodetectpath = false\n[ui]\n{ui}")).unwrap();
        sb.config().load().unwrap();
        let ui = sb.config().ui();
        assert_eq!(
            (ui.accent_color, ui.addon_view_mode, ui.addon_sort),
            (AccentColorId::Gold, AddonViewMode::List, AddonSort::Default)
        );
    }
}

#[test]
fn legacy_viper_config_loads_leniently_and_saving_keeps_unknown_keys() {
    let sb = sandbox();
    let aac = sb.root().join("Documents");
    let legacy = format!(
        "Extra = 1\nSeen = 2026-11-02\nDates = [2026-11-02T10:00:00Z]\n[General]\nAutoDetectPath = \"false\"\nAACPath = {:?}\nLegacy = 'keep'\n[UI]\nAddonViewMode = 2026-11-02\n[Other]\nk = 'v'\n",
        aac.to_str().unwrap()
    );
    fs::write(config_file(&sb), legacy).unwrap();
    sb.config().load().unwrap();
    let general = sb.config().general();
    assert!(!general.auto_detect_path);
    assert_eq!(general.aac_path, aac.to_str().unwrap());
    // A date value names no view mode.
    assert_eq!(sb.config().ui().addon_view_mode, AddonViewMode::List);

    sb.config().update(|_, ui| ui.accent_color = AccentColorId::Blue).unwrap();
    let saved: toml::Table = fs::read_to_string(config_file(&sb)).unwrap().parse().unwrap();
    assert_eq!(saved["extra"].as_integer(), Some(1));
    assert_eq!(saved["seen"].as_datetime().map(|d| d.to_string()).as_deref(), Some("2026-11-02"));
    assert!(saved["dates"][0].as_datetime().is_some(), "{saved:?}");
    assert_eq!(saved["general"]["legacy"].as_str(), Some("keep"));
    assert_eq!(saved["general"]["autodetectpath"].as_bool(), Some(false));
    assert_eq!(saved["other"]["k"].as_str(), Some("v"));
    assert_eq!(saved["ui"]["accentcolor"].as_str(), Some("blue"));
}

#[test]
fn autodetect_failure_clears_path_without_persisting() {
    let sb = sandbox();
    sb.config().update(|general, _| general.auto_detect_path = true).unwrap();
    let before = fs::read_to_string(config_file(&sb)).unwrap();
    assert!(before.contains("aacpath"));
    match sb.config().load() {
        Err(ConfigError::AacNotDetected(_)) => {}
        other => panic!("expected AacNotDetected, got {other:?}"),
    }
    assert_eq!(sb.config().general().aac_path, "");
    assert!(sb.config().aac_dir().is_err());
    assert_eq!(fs::read_to_string(config_file(&sb)).unwrap(), before);
}

#[test]
fn malformed_config_is_never_overwritten_and_recovery_restores_defaults() {
    let sb = sandbox();
    let bad = "[general\naacpath = 1";
    fs::write(config_file(&sb), bad).unwrap();
    let parse_error = match sb.config().load() {
        Err(ConfigError::Malformed { error, .. }) => error,
        other => panic!("expected Malformed, got {other:?}"),
    };
    // Saving the in-memory config would replace the user's file with defaults.
    assert!(sb.config().update(|general, _| general.auto_detect_path = false).is_err());
    assert_eq!(fs::read_to_string(config_file(&sb)).unwrap(), bad);

    // The defaults load, then autodetect finds nothing in the sandbox and gets its own entry.
    let errors = sb.config().load_or_recover();
    let titles: Vec<_> = errors.iter().map(|e| e.title.as_str()).collect();
    assert_eq!(titles, ["Settings were reset", "ArcheAge Classic was not found"]);
    let backups: Vec<_> = fs::read_dir(&sb.paths().config)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.file_name().unwrap().to_string_lossy().starts_with("config.toml.invalid-"))
        .collect();
    assert_eq!(backups.len(), 1);
    assert_eq!(fs::read_to_string(&backups[0]).unwrap(), bad);
    assert!(errors[0].message.contains(&backups[0].display().to_string()), "{}", errors[0].message);
    assert_eq!(errors[0].details.as_deref(), Some(parse_error.as_str()));
    sb.config().update(|general, _| general.auto_detect_path = false).unwrap();
    assert!(fs::read_to_string(config_file(&sb)).unwrap().contains("autodetectpath = false"));

    // A config that loads reports nothing.
    assert!(sb.config().load_or_recover().is_empty());
}

#[test]
fn quarantine_never_replaces_an_earlier_backup() {
    let sb = sandbox();
    let config_file = config_file(&sb);
    let stem = "config.toml.invalid-20260101-000000";
    let taken = config_file.with_file_name(stem);
    fs::write(&taken, "[earlier").unwrap();

    fs::write(&config_file, "[first").unwrap();
    let first = file::quarantine_as(&config_file, stem).unwrap();
    fs::write(&config_file, "[second").unwrap();
    let second = file::quarantine_as(&config_file, stem).unwrap();

    assert_eq!(first, config_file.with_file_name(format!("{stem}-2")));
    assert_eq!(second, config_file.with_file_name(format!("{stem}-3")));
    assert_eq!(fs::read_to_string(&taken).unwrap(), "[earlier");
    assert_eq!(fs::read_to_string(&first).unwrap(), "[first");
    assert_eq!(fs::read_to_string(&second).unwrap(), "[second");
}

#[test]
fn non_utf8_config_is_malformed() {
    let sb = sandbox();
    fs::write(config_file(&sb), b"\xff\xfe").unwrap();
    match sb.config().load() {
        Err(ConfigError::Malformed { .. }) => {}
        other => panic!("expected Malformed, got {other:?}"),
    }
}

#[cfg(target_os = "linux")]
#[test]
fn autodetect_finds_linux_proton_documents_dir() {
    let sb = sandbox();
    sb.config().update(|general, _| general.auto_detect_path = true).unwrap();
    let docs = sb
        .install_root()
        .join(".local/share/AAClassic/steam-compatdata/pfx/drive_c/users/steamuser/Documents/AAClassic");
    fs::create_dir_all(&docs).unwrap();
    sb.config().load().unwrap();
    assert_eq!(sb.config().general().aac_path, docs.to_str().unwrap());
}

#[cfg(windows)]
#[test]
fn autodetect_finds_highest_windows_install_documents_dir() {
    let sb = sandbox();
    sb.config().update(|general, _| general.auto_detect_path = true).unwrap();
    let drive = sb.install_root();
    fs::create_dir_all(drive.join("AAClassic/Documents")).unwrap();
    fs::create_dir_all(drive.join("AAClassic2/Documents")).unwrap();
    sb.config().load().unwrap();
    assert_eq!(sb.config().general().aac_path, drive.join("AAClassic2").join("Documents").to_str().unwrap());
}

#[test]
fn aac_dir_must_be_absolute() {
    let sb = sandbox();
    sb.config().update(|general, _| general.aac_path = "relative/dir".to_string()).unwrap();
    let e = sb.config().aac_dir().unwrap_err().to_string();
    assert!(e.contains("is not absolute"), "{e}");
}

// ------------------------------------------------------------------ addons.txt

#[test]
fn addons_txt_skips_blank_lines_and_hides_update_notification() {
    let sb = sandbox();
    write(sb.addon_dir().join("addons.txt"), "a\n\n  \nAddonUpdateNotification\nb\n");
    assert_eq!(sb.addons().installed_names().unwrap(), ["a", "b"]);
}

#[test]
fn addons_txt_add_remove_preserve_update_notification() {
    let sb = sandbox();
    write(sb.addon_dir().join("addons.txt"), "a\nAddonUpdateNotification\n");
    let addons = sb.addons();
    addons.add_to_addons_txt("b").unwrap();
    addons.add_to_addons_txt("b").unwrap();
    assert_eq!(txt(&sb), "a\nAddonUpdateNotification\nb");
    addons.remove_from_addons_txt("a").unwrap();
    assert_eq!(txt(&sb), "AddonUpdateNotification\nb");
    addons.remove_from_addons_txt("missing").unwrap();
    assert_eq!(txt(&sb), "AddonUpdateNotification\nb");
}

#[test]
fn add_to_missing_addons_txt_creates_it() {
    let sb = sandbox();
    sb.addons().add_to_addons_txt("a").unwrap();
    assert_eq!(txt(&sb), "a");
}

/// A Documents folder picked in Settings may have no addons.txt yet. Resolving the dependencies of an install creates
/// it, so the install can go on.
#[test]
fn resolve_dependencies_creates_missing_addons_txt() {
    let sb = sandbox();
    let lib = AddonManifest { name: "lib".into(), ..Default::default() };
    let app = AddonManifest { name: "app".into(), dependencies: vec!["lib".into()], ..Default::default() };
    let resolution = sb.addons().resolve_dependencies(&app, &[lib, app.clone()]).unwrap();
    let missing: Vec<_> =
        resolution.dependencies.iter().filter(|d| !d.is_installed).map(|d| &d.manifest.name).collect();
    assert_eq!(missing, ["lib"]);
    assert_eq!(txt(&sb), "");
}

#[test]
fn ensure_addons_txt_keeps_an_existing_file() {
    let sb = sandbox();
    write(sb.addon_dir().join("addons.txt"), "a\nb\n");
    sb.addons().ensure_addons_txt_exists().unwrap();
    assert_eq!(txt(&sb), "a\nb\n");
}

/// An install replaces the addon's folder before it lists the addon, so an addons.txt that can't be read must fail
/// the install first. Otherwise the files are gone and the install still fails.
#[test]
fn install_with_unreadable_addons_txt_leaves_the_addon_untouched() {
    let sb = sandbox();
    write(sb.addon_dir().join("a/main.lua"), "mine");
    fs::create_dir(sb.addon_dir().join("addons.txt")).unwrap();
    let manifest = AddonManifest { name: "a".into(), ..Default::default() };
    let e = format!("{:#}", sb.addons().install_addon(&manifest, "", &|_| {}).unwrap_err());
    assert!(e.starts_with("reading addons.txt:"), "{e}");
    assert_eq!(fs::read_to_string(sb.addon_dir().join("a/main.lua")).unwrap(), "mine");
}

/// Every change reads the file, edits it and writes it back under the lock, so changes at the same moment all land.
#[test]
fn concurrent_adds_keep_every_entry() {
    let sb = sandbox();
    let names: Vec<String> = (0..16).map(|i| format!("addon{i}")).collect();
    std::thread::scope(|s| {
        for name in &names {
            let addons = sb.addons();
            s.spawn(move || addons.add_to_addons_txt(name).unwrap());
        }
    });
    let mut listed = sb.addons().installed_names().unwrap();
    listed.sort();
    let mut expected = names.clone();
    expected.sort();
    assert_eq!(listed, expected);
}

#[test]
fn sort_addons_txt_orders_by_dependencies_and_keeps_notification() {
    let sb = sandbox();
    write(sb.addon_dir().join("addons.txt"), "app\nAddonUpdateNotification\nlib\n");
    write_managed(&sb, &[Addon { name: "app".into(), dependencies: vec!["lib".into()], ..Default::default() }]);
    sb.addons().sort_addons_txt().unwrap();
    assert_eq!(txt(&sb), "lib\napp\nAddonUpdateNotification");
}

#[test]
fn sort_addons_txt_cycle_leaves_file_untouched() {
    let sb = sandbox();
    write(sb.addon_dir().join("addons.txt"), "a\nb\n");
    write_managed(
        &sb,
        &[
            Addon { name: "a".into(), dependencies: vec!["b".into()], ..Default::default() },
            Addon { name: "b".into(), dependencies: vec!["a".into()], ..Default::default() },
        ],
    );
    let e = sb.addons().sort_addons_txt().unwrap_err().to_string();
    assert!(e.starts_with("circular dependency detected"), "{e}");
    assert_eq!(txt(&sb), "a\nb\n");
}

// ------------------------------------------------------------------ managed_addons.json

#[test]
fn managed_addons_load_save_roundtrip() {
    let sb = sandbox();
    let path = managed_file(&sb);
    // A missing file is an empty list, and reading doesn't create it.
    assert!(sb.addons().managed_addons().unwrap().is_empty() && !path.exists());

    write(
        &path,
        r#"{"version":1,"addons":[{"name":"x","alias":"X","description":"d","version":"v1","commit":"c","author":"a","repo":"r/x","isManaged":true,"updatedAt":"2026-01-24T07:36:52Z","branch":"main","dependencies":null}]}"#,
    );
    let a = sb.addons().managed_addons().unwrap().remove("x").unwrap();
    assert!(a.is_managed && a.dependencies.is_empty());
    sb.addons().remove_managed_addon("x").unwrap();
    let saved: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    assert_eq!(saved, serde_json::json!({"version": 1, "addons": []}));
}

/// The migration of a legacy file writes its backup and fills in the dependencies. Until it succeeds, a change keeps
/// the legacy format, so the migration can still run.
#[test]
fn change_to_unmigrated_legacy_managed_file_keeps_its_format() {
    let sb = sandbox();
    let path = managed_file(&sb);
    write(&path, r#"[{"name":"a","version":"v1","isManaged":true},{"name":"b","version":"v2","isManaged":true}]"#);
    assert_eq!(sb.addons().managed_addons().unwrap().len(), 2);
    sb.addons().remove_managed_addon("a").unwrap();
    let saved: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    let names: Vec<&str> =
        saved.as_array().expect("legacy array").iter().map(|a| a["name"].as_str().unwrap()).collect();
    assert_eq!(names, ["b"]);
    assert!(!path.with_file_name("managed_addons.json.bak").exists());
}

#[test]
fn unusable_managed_addons_file_is_moved_aside_not_overwritten() {
    let sb = sandbox();
    let path = managed_file(&sb);
    let bad = ["  ", "{\"version\":1,\"addons\":[", r#"{"version":2,"addons":[]}"#];
    for bad in bad {
        write(&path, bad);
        // A change logs the failure, starts from an empty list and writes a new file.
        sb.addons().remove_managed_addon("x").unwrap();
        let saved: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        assert_eq!(saved, serde_json::json!({"version": 1, "addons": []}), "{bad}");
    }
    assert_eq!(backups_of(&path), bad);

    // The update check fails instead, so the notification of the last check stays.
    write(&path, "{");
    let err = format!("{:#}", sb.addons().check_for_updates().err().unwrap());
    assert!(err.contains("was moved to") && err.contains("managed_addons.json.invalid-"), "{err}");
    assert!(!path.exists());
}

/// Contents of the `<name>.invalid-*` backups next to `path`, sorted.
fn backups_of(path: &Path) -> Vec<String> {
    let prefix = format!("{}.invalid-", path.file_name().unwrap().to_string_lossy());
    let mut found: Vec<String> = fs::read_dir(path.parent().unwrap())
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.file_name().unwrap().to_string_lossy().starts_with(&prefix))
        .map(|p| fs::read_to_string(p).unwrap())
        .collect();
    found.sort();
    found
}

#[test]
fn unusable_file_that_cannot_be_moved_aside_is_never_overwritten() {
    let sb = sandbox();
    let dir = sb.paths().data_dir().unwrap();
    let state = dir.join("app_state.json");
    let managed = dir.join("managed_addons.json");
    for p in [&state, &managed] {
        fs::write(p, "{").unwrap();
        take_backup_names(p);
    }

    assert!(appstate::kofi_modal_due(sb.paths()).is_err());
    let err = format!("{:#}", sb.addons().managed_addons().unwrap_err());
    assert!(err.contains("could not be moved aside"), "{err}");
    // An addon change can't save over the file.
    assert!(sb.addons().remove_managed_addon("x").is_err());
    // The installed list shows the addons as unmanaged.
    write(sb.addon_dir().join("addons.txt"), "x");
    assert!(!sb.addons().addons().unwrap()[0].is_managed);

    assert_eq!(fs::read_to_string(&state).unwrap(), "{");
    assert_eq!(fs::read_to_string(&managed).unwrap(), "{");
}

/// Takes every backup name [`file::quarantine`] could pick for `path` in the next 3 seconds, so moving it aside fails.
fn take_backup_names(path: &Path) {
    let name = path.file_name().unwrap().to_string_lossy();
    let now = chrono::Local::now();
    for s in 0..3 {
        let stem = format!("{name}.invalid-{}", (now + chrono::Duration::seconds(s)).format("%Y%m%d-%H%M%S"));
        for n in 1..=100 {
            let backup = if n == 1 { stem.clone() } else { format!("{stem}-{n}") };
            fs::write(path.with_file_name(backup), "").unwrap();
        }
    }
}

#[test]
fn managed_addon_json_shape_matches_go() {
    let a = Addon {
        name: "n".into(),
        updated_at: Some(chrono::DateTime::parse_from_rfc3339("2026-01-24T07:36:52Z").unwrap().into()),
        ..Default::default()
    };
    assert_eq!(
        serde_json::to_string(&a).unwrap(),
        r#"{"name":"n","alias":"","description":"","version":"","commit":"","author":"","repo":"","isManaged":false,"updatedAt":"2026-01-24T07:36:52Z","dependencies":[]}"#
    );
}

/// The Go client has no `null` time: it writes the zero time for an addon without a publish time and reads it back
/// as unset.
#[test]
fn managed_addon_unset_time_is_go_zero_time() {
    let saved = serde_json::to_value(Addon { name: "n".into(), ..Default::default() }).unwrap();
    assert_eq!(saved["updatedAt"], "0001-01-01T00:00:00Z");
    let zero = r#","updatedAt":"0001-01-01T00:00:00Z""#;
    let epoch = r#","updatedAt":"1970-01-01T00:00:00Z""#;
    for updated_at in [zero, r#","updatedAt":null"#, "", epoch] {
        let a: Addon = serde_json::from_str(&format!(r#"{{"name":"n"{updated_at}}}"#)).unwrap();
        assert_eq!(a.updated_at, None, "{updated_at}");
    }
    let a: Addon = serde_json::from_str(r#"{"name":"n","updatedAt":"1970-01-01T00:00:00.001Z"}"#).unwrap();
    assert_eq!(a.updated_at.map(|t| t.timestamp_millis()), Some(1));
}

// ------------------------------------------------------------------ zip extraction / install

/// Installs a zip of `entries` as the addon folder `name`, with no commit step.
fn install_entries(sb: &super::testutil::Sandbox, name: &str, entries: &[(&str, &[u8])]) -> anyhow::Result<()> {
    let zip = sb.aac.join(format!("{name}.zip"));
    make_zip(&zip, entries);
    util::install_release(&zip, &sb.addon_dir().join(name), &mut |_, _| {}, None)
}

/// Sorted names in the addon folder.
fn addon_dir_names(sb: &super::testutil::Sandbox) -> Vec<String> {
    let mut names: Vec<String> =
        fs::read_dir(sb.addon_dir()).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().into_owned()).collect();
    names.sort();
    names
}

#[test]
fn install_rejects_path_traversal_and_keeps_existing() {
    let sb = sandbox();
    write(sb.addon_dir().join("ev/main.lua"), "old");
    for evil in ["../evil.txt", "root/../../evil.txt", "..\\evil.txt"] {
        let e = install_entries(&sb, "ev", &[("root/main.lua", b"new"), (evil, b"x")]).unwrap_err().to_string();
        assert!(e.ends_with("invalid file path"), "{evil}: {e}");
        assert_eq!(addon_dir_names(&sb), ["ev"], "{evil}");
        assert!(!sb.aac.join("evil.txt").exists(), "{evil}");
        assert_eq!(fs::read_to_string(sb.addon_dir().join("ev/main.lua")).unwrap(), "old");
    }
}

/// On Windows a drive letter inside an entry name (`root/K:dir/file`) replaces the path it is joined onto, so the file
/// would land under the current directory of that drive. The target is a folder this test owns.
#[cfg(windows)]
#[test]
fn install_rejects_embedded_drive_prefix() {
    use std::path::{Component, Prefix};
    let sb = sandbox();
    let cwd = std::env::current_dir().unwrap();
    let Some(Component::Prefix(prefix)) = cwd.components().next() else { panic!("no prefix in {cwd:?}") };
    let Prefix::Disk(drive) = prefix.kind() else {
        eprintln!("skipped: the current directory {cwd:?} is not on a drive letter");
        return;
    };
    let owned = tempfile::tempdir_in(&cwd).unwrap();
    let owned_name = owned.path().file_name().unwrap().to_str().unwrap();
    let entry = format!("root/{}:{owned_name}/escaped.txt", drive as char);
    write(sb.addon_dir().join("ev/main.lua"), "old");
    let result = install_entries(&sb, "ev", &[("root/main.lua", b"new"), (entry.as_str(), b"x")]);
    assert!(!owned.path().join("escaped.txt").exists(), "wrote outside the staging folder");
    let e = result.unwrap_err().to_string();
    assert!(e.ends_with("invalid file path"), "{e}");
    assert_eq!(fs::read_to_string(sb.addon_dir().join("ev/main.lua")).unwrap(), "old");
    assert_eq!(addon_dir_names(&sb), ["ev"]);
}

/// On NTFS `main.lua:payload` would write a stream and leave an empty `main.lua`, which would pass as a release.
#[cfg(windows)]
#[test]
fn install_rejects_ntfs_stream_names() {
    let sb = sandbox();
    write(sb.addon_dir().join("ads/main.lua"), "old");
    let e = install_entries(&sb, "ads", &[("root/main.lua:payload", b"x")]).unwrap_err().to_string();
    assert!(e.ends_with("invalid file path"), "{e}");
    assert_eq!(fs::read_to_string(sb.addon_dir().join("ads/main.lua")).unwrap(), "old");
    assert_eq!(addon_dir_names(&sb), ["ads"]);
}

/// Symlink entries are left out, also when their name ends in a separator, so a `main.lua` that is only a symlink
/// doesn't make a release.
#[test]
fn install_skips_symlink_entries() {
    let sb = sandbox();
    let dest = sb.addon_dir().join("ln");
    write(dest.join("main.lua"), "old");
    let zip = sb.aac.join("ln.zip");

    let mut w = zip::ZipWriter::new(fs::File::create(&zip).unwrap());
    w.add_symlink("root/main.lua", "scripts/entry.lua", zip::write::SimpleFileOptions::default()).unwrap();
    w.finish().unwrap();
    let e = util::install_release(&zip, &dest, &mut |_, _| {}, None).unwrap_err().to_string();
    assert!(e.starts_with("invalid archive:"), "{e}");
    assert_eq!(fs::read_to_string(dest.join("main.lua")).unwrap(), "old");
    assert_eq!(addon_dir_names(&sb), ["ln"]);

    let mut w = zip::ZipWriter::new(fs::File::create(&zip).unwrap());
    w.add_symlink("root/main.lua/", "elsewhere", zip::write::SimpleFileOptions::default()).unwrap();
    w.add_symlink("root/unused/", "elsewhere", zip::write::SimpleFileOptions::default()).unwrap();
    w.start_file("root/main.lua", zip::write::SimpleFileOptions::default()).unwrap();
    std::io::Write::write_all(&mut w, b"new").unwrap();
    w.add_symlink("root/link.lua", "main.lua", zip::write::SimpleFileOptions::default()).unwrap();
    w.finish().unwrap();
    let mut steps = Vec::new();
    util::install_release(&zip, &dest, &mut |n, total| steps.push((n, total)), None).unwrap();
    assert_eq!(fs::read_to_string(dest.join("main.lua")).unwrap(), "new");
    assert!(fs::symlink_metadata(dest.join("link.lua")).is_err());
    assert!(fs::symlink_metadata(dest.join("unused")).is_err());
    assert_eq!(steps.last(), Some(&(1, 1)));
}

#[test]
fn install_failed_commit_restores_previous_addon() {
    let sb = sandbox();
    let dest = sb.addon_dir().join("ad");
    write(dest.join("main.lua"), "old");
    write(dest.join(".data/state"), "user");
    let zip = sb.aac.join("ad.zip");
    make_zip(&zip, &[("root/main.lua", b"new"), ("root/extra.lua", b"x")]);
    let fail = || -> anyhow::Result<()> { Err(anyhow::anyhow!("injected")) };
    let e = format!("{:#}", util::install_release(&zip, &dest, &mut |_, _| {}, Some(&fail)).unwrap_err());
    assert!(e.contains("injected"), "{e}");
    assert_eq!(fs::read_to_string(dest.join("main.lua")).unwrap(), "old");
    assert_eq!(fs::read_to_string(dest.join(".data/state")).unwrap(), "user");
    assert!(!dest.join("extra.lua").exists());
    assert_eq!(addon_dir_names(&sb), ["ad"]);
}

#[test]
fn install_failed_commit_removes_fresh_addon() {
    let sb = sandbox();
    let dest = sb.addon_dir().join("ad");
    let zip = sb.aac.join("ad.zip");
    make_zip(&zip, &[("root/main.lua", b"new")]);
    let fail = || -> anyhow::Result<()> { Err(anyhow::anyhow!("injected")) };
    let e = format!("{:#}", util::install_release(&zip, &dest, &mut |_, _| {}, Some(&fail)).unwrap_err());
    assert!(e.contains("injected"), "{e}");
    assert!(addon_dir_names(&sb).is_empty(), "{:?}", addon_dir_names(&sb));
}

#[test]
fn install_reports_file_progress() {
    let sb = sandbox();
    let zip = sb.aac.join("a.zip");
    make_zip(&zip, &[("root/", b""), ("root/main.lua", b"1"), ("root/sub/x.lua", b"2")]);
    let mut steps = Vec::new();
    util::install_release(&zip, &sb.addon_dir().join("a"), &mut |n, total| steps.push((n, total)), None).unwrap();
    assert_eq!(fs::read_to_string(sb.addon_dir().join("a/sub/x.lua")).unwrap(), "2");
    // Folders are not counted. The count starts at 0 and ends at the total.
    assert_eq!((steps.first(), steps.last()), (Some(&(0, 2)), Some(&(2, 2))));
}

#[test]
fn install_preserves_existing_data_dir() {
    let sb = sandbox();
    write(sb.addon_dir().join("ad/main.lua"), "old");
    write(sb.addon_dir().join("ad/.data/state"), "user");
    let entries: [(&str, &[u8]); 3] =
        [("root/main.lua", b"new"), ("root/.data/state", b"release"), ("README.md", b"outside the root")];
    install_entries(&sb, "ad", &entries).unwrap();
    assert_eq!(fs::read_to_string(sb.addon_dir().join("ad/main.lua")).unwrap(), "new");
    assert_eq!(fs::read_to_string(sb.addon_dir().join("ad/.data/state")).unwrap(), "user");
    assert!(!sb.addon_dir().join("ad/README.md").exists());
    // No staging or backup folder is left.
    assert_eq!(addon_dir_names(&sb), ["ad"]);
}

#[test]
fn install_fresh_keeps_release_data() {
    let sb = sandbox();
    install_entries(&sb, "ad", &[("./root/main.lua", b"new"), ("root/.data/state", b"release")]).unwrap();
    assert_eq!(fs::read_to_string(sb.addon_dir().join("ad/.data/state")).unwrap(), "release");
    assert_eq!(addon_dir_names(&sb), ["ad"]);
}

#[test]
fn install_requires_exactly_one_top_level_folder_with_main_lua() {
    let sb = sandbox();
    write(sb.addon_dir().join("ad/keep"), "k");
    let cases: [&[(&str, &[u8])]; 5] = [
        &[("main.lua", b"x")],
        &[("a/b/main.lua", b"x")],
        &[("root/main.lua/", b"")],
        &[("root/readme", b"x")],
        &[("a/main.lua", b"1"), ("b/main.lua", b"2")],
    ];
    for (i, entries) in cases.into_iter().enumerate() {
        let e = install_entries(&sb, "ad", entries).unwrap_err().to_string();
        assert!(e.starts_with("invalid archive:"), "{i}: {e}");
        assert!(sb.addon_dir().join("ad/keep").exists(), "{i}: existing addon untouched");
        assert_eq!(addon_dir_names(&sb), ["ad"], "{i}");
    }
}

#[cfg(unix)]
#[test]
fn install_keeps_executable_bit() {
    use std::io::Write;
    use std::os::unix::fs::PermissionsExt;
    let sb = sandbox();
    let zip = sb.aac.join("ex.zip");
    let mut w = zip::ZipWriter::new(fs::File::create(&zip).unwrap());
    w.start_file("root/main.lua", zip::write::SimpleFileOptions::default()).unwrap();
    w.write_all(b"x").unwrap();
    w.start_file("root/run.sh", zip::write::SimpleFileOptions::default().unix_permissions(0o755)).unwrap();
    w.write_all(b"#!/bin/sh").unwrap();
    w.finish().unwrap();
    util::install_release(&zip, &sb.addon_dir().join("ex"), &mut |_, _| {}, None).unwrap();
    let mode = fs::metadata(sb.addon_dir().join("ex/run.sh")).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o755);
}

#[test]
fn install_over_non_directory_data_fails_and_keeps_dest() {
    let sb = sandbox();
    write(sb.addon_dir().join("d/main.lua"), "old");
    write(sb.addon_dir().join("d/.data"), "not a dir");
    let e = install_entries(&sb, "d", &[("root/main.lua", b"new")]).unwrap_err().to_string();
    assert!(e.contains("is not a directory"), "{e}");
    assert_eq!(fs::read_to_string(sb.addon_dir().join("d/main.lua")).unwrap(), "old");
    assert_eq!(addon_dir_names(&sb), ["d"]);
}

#[test]
fn install_zip_installs_and_registers() {
    let sb = sandbox();
    write(sb.addon_dir().join("addons.txt"), "other\n");
    write_managed(&sb, &[Addon { name: "my_addon".into(), is_managed: true, ..Default::default() }]);

    let zip = sb.aac.join("my_addon.zip");
    make_zip(&zip, &[("whatever/main.lua", b"print(1)"), ("whatever/lib/x.lua", b"2")]);
    let name = sb.addons().zip_addon_name(&zip).unwrap();
    assert_eq!(name, "my_addon");
    sb.addons().install_zip(&zip, &name).unwrap();
    assert_eq!(fs::read_to_string(sb.addon_dir().join("my_addon/main.lua")).unwrap(), "print(1)");
    assert_eq!(txt(&sb), "other\nmy_addon");
    assert!(!sb.addons().managed_addons().unwrap().contains_key("my_addon"), "manual install drops managed metadata");
    assert_eq!(addon_dir_names(&sb), ["addons.txt", "my_addon"]);
}

#[test]
fn zip_addon_name_rejects_bad_names() {
    let sb = sandbox();
    for name in [".zip", "x..zip", "x .zip", "AddonUpdateNotification.zip", "addonupdatenotification.zip"] {
        let zip = sb.aac.join(name);
        make_zip(&zip, &[("r/main.lua", b"x")]);
        assert!(sb.addons().zip_addon_name(&zip).is_err(), "{name}");
    }
}

#[test]
fn install_zip_invalid_archive() {
    let sb = sandbox();
    let zip = sb.aac.join("bad.zip");
    make_zip(&zip, &[("r/readme", b"x")]);
    let e = sb.addons().install_zip(&zip, "bad").unwrap_err().to_string();
    assert!(e.starts_with("invalid archive:"), "{e}");
    assert!(!sb.addon_dir().join("bad").exists());
}

// ------------------------------------------------------------------ addon folder

#[test]
fn open_directory_rejects_escapes() {
    let sb = sandbox();
    fs::create_dir_all(sb.addon_dir().join("ok")).unwrap();
    let opened = std::cell::RefCell::new(None);
    let open = |p: &Path| {
        *opened.borrow_mut() = Some(p.to_path_buf());
        Ok(())
    };
    for bad in [".", "..", "../x", "a/b", "a\\b", "", "/abs"] {
        assert!(sb.addons().open_directory_with(bad, &open).is_err(), "{bad:?}");
    }
    assert!(opened.borrow().is_none());
    sb.addons().open_directory_with("ok", &open).unwrap();
    assert!(opened.borrow().as_ref().unwrap().ends_with("Addon/ok"));
}

#[cfg(unix)]
#[test]
fn open_directory_rejects_symlink_escape() {
    let sb = sandbox();
    let outside = sb.aac.join("outside");
    fs::create_dir_all(&outside).unwrap();
    std::os::unix::fs::symlink(&outside, sb.addon_dir().join("link")).unwrap();
    let e = sb.addons().open_directory_with("link", &|_| Ok(())).unwrap_err().to_string();
    assert!(e.contains("escapes addon root"), "{e}");
}

#[test]
fn uninstall_unmanaged_addon_removes_dir_and_entry() {
    let sb = sandbox();
    write(sb.addon_dir().join("addons.txt"), "keep\ngone\n");
    write(sb.addon_dir().join("gone/main.lua"), "x");
    write(sb.addon_dir().join("keep/main.lua"), "x");
    assert!(sb.addons().uninstall_addon("not_installed").is_err());
    sb.addons().uninstall_addon("gone").unwrap();
    assert!(!sb.addon_dir().join("gone").exists());
    assert_eq!(txt(&sb), "keep");
}

#[test]
fn addons_merge_managed_metadata() {
    let sb = sandbox();
    write(sb.addon_dir().join("addons.txt"), "plain_one\nmanaged\n");
    write_managed(
        &sb,
        &[Addon { name: "managed".into(), version: "v1".into(), is_managed: true, ..Default::default() }],
    );
    let list = sb.addons().addons().unwrap();
    assert_eq!(list[0].alias, "plain one");
    assert!(!list[0].is_managed);
    assert_eq!(list[1].alias, "managed");
    assert_eq!(list[1].version, "v1");
}

#[test]
fn reset_settings_truncates_addon_settings() {
    let sb = sandbox();
    write(sb.aac.join("addon_settings"), "data");
    addon::reset_addon_settings(&sb.aac).unwrap();
    assert_eq!(fs::metadata(sb.aac.join("addon_settings")).unwrap().len(), 0);
    fs::remove_file(sb.aac.join("addon_settings")).unwrap();
    assert!(addon::reset_addon_settings(&sb.aac).is_err());
}

#[test]
fn diagnose_reads_aac_log() {
    let sb = sandbox();
    write(sb.aac.join("ArcheAge.log"), "[Lua Error] x C:/G/Addon/foo/main.lua:3: oops\n");
    let r = util::diagnose_issues(&sb.aac).unwrap();
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].addon, "foo");
}

// ------------------------------------------------------------------ update notification

#[test]
fn generate_update_lua_writes_files_and_registers() {
    let sb = sandbox();
    write(sb.addon_dir().join("addons.txt"), "a\n");
    let mut updates = HashMap::new();
    updates.insert(
        "a".to_string(),
        Addon { name: "a".into(), alias: "A".into(), version: "v2".into(), ..Default::default() },
    );
    let addons = sb.addons();
    addons.generate_update_addon_lua(&updates).unwrap();
    let dir = sb.addon_dir().join("AddonUpdateNotification");
    assert!(fs::read_to_string(dir.join("main.lua")).unwrap().contains("Classic Addon Manager"));
    assert_eq!(
        fs::read_to_string(dir.join("updates.lua")).unwrap(),
        "{\n    [\"a\"] = {name=\"A\", version=\"v2\"}, \n}\n"
    );
    assert_eq!(txt(&sb), "a\nAddonUpdateNotification");
    // hidden from the user-visible list
    assert_eq!(addons.installed_names().unwrap(), ["a"]);

    addons.generate_update_addon_lua(&HashMap::new()).unwrap();
    assert!(!dir.join("updates.lua").exists());
    addons.generate_update_addon_lua(&HashMap::new()).unwrap();
    assert_eq!(txt(&sb), "a\nAddonUpdateNotification");
}

#[test]
fn generate_update_lua_reports_missing_addons_txt() {
    let sb = sandbox();
    let e = format!("{:#}", sb.addons().generate_update_addon_lua(&HashMap::new()).unwrap_err());
    assert!(e.starts_with("read addons.txt:"), "{e}");
}

// ------------------------------------------------------------------ misc file helpers

#[test]
fn write_atomic_preserves_mode_and_leaves_no_temp() {
    let sb = sandbox();
    let p = sb.aac.join("f.txt");
    file::write_atomic(&p, b"one", 0o600).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&p, fs::Permissions::from_mode(0o640)).unwrap();
    }
    file::write_atomic(&p, b"two", 0o600).unwrap();
    assert_eq!(fs::read_to_string(&p).unwrap(), "two");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(fs::metadata(&p).unwrap().permissions().mode() & 0o777, 0o640);
    }
    let names: Vec<_> =
        fs::read_dir(&sb.aac).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().into_owned()).collect();
    assert!(names.iter().all(|n| !n.contains(".tmp-")), "{names:?}");
}

#[test]
fn copy_file_errors_and_preserves_mtime() {
    let sb = sandbox();
    let src = sb.aac.join("s");
    fs::write(&src, "x").unwrap();
    assert!(
        file::copy_file(sb.aac.join("missing"), sb.aac.join("d"))
            .unwrap_err()
            .to_string()
            .starts_with("source file does not exist")
    );
    assert!(
        file::copy_file(&src, sb.aac.join("nodir/d"))
            .unwrap_err()
            .to_string()
            .starts_with("destination directory does not exist")
    );
    assert!(
        file::copy_file(&sb.aac, sb.aac.join("d")).unwrap_err().to_string().starts_with("cannot copy non-regular file")
    );
    file::copy_file(&src, sb.aac.join("d")).unwrap();
    assert_eq!(
        fs::metadata(&src).unwrap().modified().unwrap(),
        fs::metadata(sb.aac.join("d")).unwrap().modified().unwrap()
    );
}

#[test]
fn move_dir_replaces_destination() {
    let sb = sandbox();
    write(sb.aac.join("src/a"), "1");
    write(sb.aac.join("dst/old"), "x");
    file::move_dir(sb.aac.join("src"), sb.aac.join("dst")).unwrap();
    assert!(sb.aac.join("dst/a").exists() && !sb.aac.join("dst/old").exists() && !sb.aac.join("src").exists());
}

#[test]
fn auth_and_appstate_roundtrip() {
    let sb = sandbox();
    let paths = sb.paths();
    auth::save_to_disk(paths, "tok").unwrap();
    assert_eq!(auth::get_token(), "tok");
    assert!(auth::save_to_disk(paths, "").is_err());
    let dir = paths.data_dir().unwrap();
    let p = dir.join("auth_session.json");
    let raw: serde_json::Value = serde_json::from_slice(&fs::read(&p).unwrap()).unwrap();
    assert_eq!(raw["version"], 1);
    #[cfg(windows)]
    {
        assert_eq!(raw["encoding"], "dpapi");
        assert_ne!(raw["token"], "tok");
    }
    #[cfg(not(windows))]
    {
        assert_eq!(raw["encoding"], "plain");
        assert_eq!(raw["token"], "tok");
    }
    auth::clear_token();
    auth::load_from_disk(paths);
    assert_eq!(auth::get_token(), "tok");
    auth::delete_from_disk(paths).unwrap();
    assert_eq!(auth::get_token(), "");

    // An unusable session is moved aside.
    let bad = [
        r#"{"version":1,"encoding":"plain","token":"","updated_at":"2026-01-01T00:00:00Z"}"#,
        r#"{"version":2,"encoding":"plain","token":"x","updated_at":"2026-01-01T00:00:00Z"}"#,
    ];
    for bad in bad {
        fs::write(&p, bad).unwrap();
        // A token already in memory must not survive an unusable session file.
        auth::set_token("stale");
        auth::load_from_disk(paths);
        assert!(!p.exists());
        assert_eq!(auth::get_token(), "");
    }
    assert_eq!(backups_of(&p), bad);

    // First launch: not due, and the file is created so the next launch shows it. Once shown, it waits.
    assert!(!appstate::kofi_modal_due(paths).unwrap());
    assert!(appstate::kofi_modal_due(paths).unwrap());
    appstate::record_kofi_modal_shown(paths).unwrap();
    assert!(!appstate::kofi_modal_due(paths).unwrap());

    // An unusable app state counts as a first launch.
    fs::write(dir.join("app_state.json"), "{").unwrap();
    assert!(!appstate::kofi_modal_due(paths).unwrap());
    assert!(appstate::kofi_modal_due(paths).unwrap());
}
