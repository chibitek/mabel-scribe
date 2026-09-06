//! Local data root + upgrade survival.
//!
//! Product LOCK (hard): dictation / usage history in the current
//! Application Support store must survive a TestFlight version bump on
//! the **same install / container**. Do not wipe `stats.json`,
//! `clipboard-history.json`, or `config.json` on upgrade. Load-or-default
//! that overwrites those files is a lock break.
//!
//! Schema rewrite is soft later — keep serde defaults; do not invent a
//! new on-disk format on this tip.
//!
//! Optional import from an older *path* (unsandboxed DMG or `com.typr.app`)
//! only fills files the current store does not already hold. Never
//! overwrite dest history. Never delete the source. If a prior store
//! exists and cannot be read, surface a reason.

use serde::{Deserialize, Serialize};
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

pub const BUNDLE_DIR: &str = "com.mabel.app";
pub const LEGACY_BUNDLE_DIR: &str = "com.typr.app";
pub const MARKER_FILE: &str = ".migration-v1.done";

/// User-owned JSON/text. Models, wavs, and debug.log are not imported.
pub const USER_DATA_FILES: &[&str] = &[
    "config.json",
    "stats.json",
    "clipboard-history.json",
    "teams.json",
    "snippets.json",
    "style.json",
    "transforms.json",
    "scratchpad.txt",
];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MigrationReport {
    pub status: String,
    pub message: Option<String>,
    pub from: Option<String>,
    pub imported: Vec<String>,
    pub skipped: Vec<String>,
}

impl MigrationReport {
    pub fn fresh() -> Self {
        Self {
            status: "fresh".into(),
            message: None,
            from: None,
            imported: Vec::new(),
            skipped: Vec::new(),
        }
    }

    pub fn already_current() -> Self {
        Self {
            status: "already_current".into(),
            message: None,
            from: None,
            imported: Vec::new(),
            skipped: Vec::new(),
        }
    }

    pub fn migrated(from: &Path, imported: Vec<String>, skipped: Vec<String>) -> Self {
        Self {
            status: "migrated".into(),
            message: Some(format!(
                "Imported {} from {}.",
                imported.join(", "),
                from.display()
            )),
            from: Some(from.display().to_string()),
            imported,
            skipped,
        }
    }

    pub fn failed(message: impl Into<String>) -> Self {
        Self {
            status: "failed".into(),
            message: Some(message.into()),
            from: None,
            imported: Vec::new(),
            skipped: Vec::new(),
        }
    }

    pub fn is_failed(&self) -> bool {
        self.status == "failed"
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct StorageStatus {
    pub migration: MigrationReport,
    pub settings_error: Option<String>,
    pub stats_error: Option<String>,
    pub history_error: Option<String>,
}

impl StorageStatus {
    pub fn visible_error(&self) -> Option<String> {
        if self.migration.is_failed() {
            return self.migration.message.clone();
        }
        self.settings_error
            .clone()
            .or_else(|| self.stats_error.clone())
            .or_else(|| self.history_error.clone())
    }
}

/// Current Application Support store. Never falls back to cwd (`.`): that
/// path changes per launch and looks like a silent wipe on the next open.
pub fn app_dir() -> PathBuf {
    config_root()
        .unwrap_or_else(|| PathBuf::from("/tmp/mabel-app-support"))
        .join(BUNDLE_DIR)
}

fn config_root() -> Option<PathBuf> {
    dirs::config_dir()
        .or_else(|| real_home().map(|h| h.join("Library").join("Application Support")))
        .or_else(|| dirs::home_dir().map(|h| h.join("Library").join("Application Support")))
}

/// User's real home even when `$HOME` is the sandbox container.
pub fn real_home() -> Option<PathBuf> {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .or_else(dirs::home_dir)?;
    if let Some(real) = strip_container_prefix(&home) {
        return Some(real);
    }
    Some(home)
}

fn strip_container_prefix(path: &Path) -> Option<PathBuf> {
    let s = path.to_string_lossy();
    let idx = s.find("/Library/Containers/")?;
    let prefix = &s[..idx];
    if prefix.is_empty() {
        return None;
    }
    Some(PathBuf::from(prefix))
}

pub fn legacy_candidate_dirs() -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Some(home) = real_home() {
        let support = home.join("Library").join("Application Support");
        out.push(support.join(BUNDLE_DIR));
        out.push(support.join(LEGACY_BUNDLE_DIR));
    }
    if let Some(home) = dirs::home_dir() {
        out.push(
            home.join("Library")
                .join("Application Support")
                .join(BUNDLE_DIR),
        );
    }
    out.sort();
    out.dedup();
    out
}

/// One-time import of prior-version user files into `dest`.
pub fn migrate_into(dest: &Path, candidates: &[PathBuf]) -> MigrationReport {
    if let Err(e) = fs::create_dir_all(dest) {
        return MigrationReport::failed(format!(
            "Could not create the current Mabel data folder ({}): {e}. Prior files were not moved.",
            dest.display()
        ));
    }

    let marker = dest.join(MARKER_FILE);
    if marker.exists() {
        return MigrationReport::already_current();
    }

    let dest_key = normalize(dest);
    let mut blocked: Option<(PathBuf, String)> = None;

    for cand in candidates {
        if normalize(cand) == dest_key {
            continue;
        }
        match inspect_legacy(cand) {
            Ok(false) => continue,
            Ok(true) => {
                match import_missing_files(cand, dest) {
                    Ok((imported, skipped)) => {
                        if imported.is_empty() {
                            let report = if has_real_history(dest) {
                                MigrationReport::already_current()
                            } else {
                                MigrationReport::fresh()
                            };
                            let _ = write_marker(&marker, cand, &report);
                            return report;
                        }
                        let report = MigrationReport::migrated(cand, imported, skipped);
                        if let Err(e) = write_marker(&marker, cand, &report) {
                            return MigrationReport::failed(format!(
                                "Imported prior data from {} but could not write the migration marker: {e}. \
                                 The original files are still at that path.",
                                cand.display()
                            ));
                        }
                        return report;
                    }
                    Err(e) => {
                        return MigrationReport::failed(format!(
                            "Could not import previous Mabel data from {}. {e} \
                             Your old files are still there. Insights and history were not replaced.",
                            cand.display()
                        ));
                    }
                }
            }
            Err(e) => {
                if blocked.is_none() {
                    blocked = Some((cand.clone(), e));
                }
            }
        }
    }

    if let Some((path, err)) = blocked {
        if !has_real_history(dest) {
            return MigrationReport::failed(format!(
                "Could not import previous Mabel data from {}. {err} \
                 Your old files are still there. Insights and history were not replaced.",
                path.display()
            ));
        }
    }

    let report = if has_real_history(dest) || dest.join("config.json").exists() {
        MigrationReport::already_current()
    } else {
        MigrationReport::fresh()
    };
    let _ = write_marker(&marker, dest, &report);
    report
}

fn normalize(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

fn inspect_legacy(dir: &Path) -> Result<bool, String> {
    match fs::read_dir(dir) {
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(false),
        Err(e) => Err(format!("Cannot read {}: {e}.", dir.display())),
        Ok(_) => {
            for name in USER_DATA_FILES {
                let p = dir.join(name);
                match fs::metadata(&p) {
                    Ok(m) if m.len() > 0 => return Ok(true),
                    Ok(_) => continue,
                    Err(e) if e.kind() == ErrorKind::NotFound => continue,
                    Err(e) if e.kind() == ErrorKind::PermissionDenied => {
                        return Err(format!("Cannot read {}: {e}.", p.display()));
                    }
                    Err(_) => continue,
                }
            }
            Ok(false)
        }
    }
}

fn import_missing_files(src: &Path, dest: &Path) -> Result<(Vec<String>, Vec<String>), String> {
    let mut planned = Vec::new();
    let mut skipped = Vec::new();
    for name in USER_DATA_FILES {
        let from = src.join(name);
        let to = dest.join(name);
        if !from.exists() {
            continue;
        }
        if dest_file_holds_user_data(name, &to) {
            skipped.push((*name).to_string());
            continue;
        }
        if needs_json_verify(name) {
            verify_json(&from)?;
        }
        planned.push(*name);
    }
    let mut imported = Vec::new();
    for name in planned {
        let from = src.join(name);
        let to = dest.join(name);
        copy_file(&from, &to)?;
        if !to.exists() {
            return Err(format!("Copy of {name} did not produce a dest file."));
        }
        if needs_json_verify(name) {
            verify_json(&to)?;
        }
        imported.push(name.to_string());
    }
    Ok((imported, skipped))
}

fn needs_json_verify(name: &str) -> bool {
    matches!(
        name,
        "stats.json" | "config.json" | "clipboard-history.json" | "teams.json"
            | "snippets.json" | "style.json" | "transforms.json"
    )
}

fn dest_file_holds_user_data(name: &str, dest: &Path) -> bool {
    if !dest.exists() {
        return false;
    }
    match name {
        "stats.json" => stats_file_has_history(dest),
        "clipboard-history.json" => history_file_has_items(dest),
        _ => match fs::metadata(dest) {
            Ok(m) => m.len() > 0,
            Err(_) => false,
        },
    }
}

pub fn has_real_history(dir: &Path) -> bool {
    stats_file_has_history(&dir.join("stats.json"))
        || history_file_has_items(&dir.join("clipboard-history.json"))
}

fn stats_file_has_history(path: &Path) -> bool {
    let Ok(raw) = fs::read_to_string(path) else {
        return false;
    };
    let Ok(v) = serde_json::from_str::<serde_json::Value>(&raw) else {
        return false;
    };
    if v.get("total_dictations").and_then(|x| x.as_u64()).unwrap_or(0) > 0 {
        return true;
    }
    v.get("daily")
        .and_then(|d| d.as_object())
        .map(|o| !o.is_empty())
        .unwrap_or(false)
}

fn history_file_has_items(path: &Path) -> bool {
    let Ok(raw) = fs::read_to_string(path) else {
        return false;
    };
    let Ok(v) = serde_json::from_str::<serde_json::Value>(&raw) else {
        return false;
    };
    v.get("items")
        .and_then(|i| i.as_array())
        .map(|a| !a.is_empty())
        .unwrap_or(false)
}

fn copy_file(from: &Path, to: &Path) -> Result<(), String> {
    if let Some(parent) = to.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("create dest dir: {e}"))?;
    }
    if to.exists() {
        fs::remove_file(to).map_err(|e| format!("replace empty dest {}: {e}", to.display()))?;
    }
    fs::copy(from, to).map_err(|e| format!("copy {}: {e}", from.display()))?;
    Ok(())
}

fn verify_json(path: &Path) -> Result<(), String> {
    let raw = fs::read_to_string(path).map_err(|e| format!("verify {}: {e}", path.display()))?;
    let value: serde_json::Value =
        serde_json::from_str(&raw).map_err(|e| format!("{} is not valid JSON: {e}", path.display()))?;
    if value.is_null() {
        return Err(format!("{} decoded as null", path.display()));
    }
    Ok(())
}

fn write_marker(marker: &Path, from: &Path, report: &MigrationReport) -> Result<(), String> {
    let payload = serde_json::json!({
        "from": from.display().to_string(),
        "status": report.status,
        "imported": report.imported,
    });
    fs::write(
        marker,
        serde_json::to_string_pretty(&payload).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())
}

/// Parse-or-error for an existing file. Missing → `Ok(None)`. Present but
/// unreadable/unparseable → `Err` and the file is left untouched.
pub fn read_existing_json<T: for<'de> serde::Deserialize<'de>>(
    path: &Path,
    label: &str,
) -> Result<Option<T>, String> {
    match fs::read_to_string(path) {
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(None),
        Err(e) => Err(format!(
            "Could not read {label} in {}. The file was left untouched so nothing was wiped. ({e})",
            path.display()
        )),
        Ok(raw) if raw.trim().is_empty() => Ok(None),
        Ok(raw) => match serde_json::from_str::<T>(&raw) {
            Ok(v) => Ok(Some(v)),
            Err(e) => Err(format!(
                "Could not read {label} in {}. The file was left untouched so nothing was wiped. ({e})",
                path.display()
            )),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    fn tmp() -> PathBuf {
        static N: AtomicU64 = AtomicU64::new(0);
        let dir = std::env::temp_dir().join(format!(
            "mabel-migrate-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write(dir: &Path, name: &str, body: &str) {
        fs::create_dir_all(dir).unwrap();
        fs::write(dir.join(name), body).unwrap();
    }

    fn sample_stats(n: u64) -> String {
        format!(
            r#"{{"daily":{{"2026-09-01":{{"dictations":{n},"words":10,"seconds":1.0}}}},"total_dictations":{n},"total_words":10,"total_seconds":1.0}}"#
        )
    }

    fn sample_config() -> &'static str {
        r#"{
            "microphone": "BuiltIn",
            "engine": "local",
            "whisperModel": "large-v3",
            "recordingMode": "toggle",
            "hotkey": "CmdOrCtrl+D"
        }"#
    }

    #[test]
    fn empty_dest_imports_legacy_stats_and_config() {
        let root = tmp();
        let dest = root.join("container");
        let legacy = root.join("Application Support").join(BUNDLE_DIR);
        write(&legacy, "config.json", sample_config());
        write(&legacy, "stats.json", &sample_stats(7));
        write(
            &legacy,
            "clipboard-history.json",
            r#"{"items":[{"id":"a","text":"hello","createdAt":1}]}"#,
        );

        let report = migrate_into(&dest, &[legacy.clone()]);
        assert_eq!(report.status, "migrated", "{report:?}");
        assert!(report.imported.contains(&"stats.json".into()));
        assert!(report.imported.contains(&"config.json".into()));
        assert!(report.imported.contains(&"clipboard-history.json".into()));
        assert_eq!(
            fs::read_to_string(dest.join("stats.json")).unwrap(),
            sample_stats(7)
        );
        assert!(dest.join(MARKER_FILE).exists());
        assert!(legacy.join("stats.json").exists(), "legacy must not be deleted");
    }

    #[test]
    fn dest_history_is_never_overwritten() {
        let root = tmp();
        let dest = root.join("dest");
        let legacy = root.join("legacy");
        write(&dest, "stats.json", &sample_stats(3));
        write(&legacy, "stats.json", &sample_stats(99));
        write(&legacy, "config.json", sample_config());

        let report = migrate_into(&dest, &[legacy]);
        assert_eq!(report.status, "migrated", "{report:?}");
        assert!(report.skipped.contains(&"stats.json".into()));
        assert!(report.imported.contains(&"config.json".into()));
        assert!(fs::read_to_string(dest.join("stats.json"))
            .unwrap()
            .contains("\"total_dictations\":3"));
    }

    #[test]
    fn empty_placeholder_stats_are_replaced_from_legacy() {
        let root = tmp();
        let dest = root.join("dest");
        let legacy = root.join("legacy");
        write(
            &dest,
            "stats.json",
            r#"{"daily":{},"total_dictations":0,"total_words":0,"total_seconds":0.0}"#,
        );
        write(&legacy, "stats.json", &sample_stats(12));

        let report = migrate_into(&dest, &[legacy]);
        assert_eq!(report.status, "migrated", "{report:?}");
        assert!(report.imported.contains(&"stats.json".into()));
        assert!(fs::read_to_string(dest.join("stats.json"))
            .unwrap()
            .contains("\"total_dictations\":12"));
    }

    #[test]
    fn typr_bundle_is_a_legacy_candidate() {
        let root = tmp();
        let dest = root.join("dest");
        let typr = root.join(LEGACY_BUNDLE_DIR);
        write(&typr, "stats.json", &sample_stats(2));
        write(&typr, "config.json", sample_config());

        let report = migrate_into(&dest, &[typr]);
        assert_eq!(report.status, "migrated");
        assert!(dest.join("stats.json").exists());
    }

    #[test]
    fn corrupt_legacy_json_fails_closed_without_writing_dest() {
        let root = tmp();
        let dest = root.join("dest");
        let legacy = root.join("legacy");
        write(&legacy, "stats.json", "{not-json");
        write(&legacy, "config.json", sample_config());

        let report = migrate_into(&dest, &[legacy.clone()]);
        assert_eq!(report.status, "failed", "{report:?}");
        assert!(report
            .message
            .as_deref()
            .unwrap_or("")
            .contains("still there"));
        assert!(!dest.join("stats.json").exists());
        assert!(legacy.join("stats.json").exists());
        assert!(!dest.join(MARKER_FILE).exists());
    }

    #[test]
    fn marker_prevents_second_import() {
        let root = tmp();
        let dest = root.join("dest");
        let legacy = root.join("legacy");
        write(&legacy, "stats.json", &sample_stats(4));
        let first = migrate_into(&dest, &[legacy.clone()]);
        assert_eq!(first.status, "migrated");

        write(&legacy, "stats.json", &sample_stats(50));
        let second = migrate_into(&dest, &[legacy]);
        assert_eq!(second.status, "already_current");
        assert!(fs::read_to_string(dest.join("stats.json"))
            .unwrap()
            .contains("\"total_dictations\":4"));
    }

    #[test]
    fn same_path_is_skipped_as_legacy() {
        let root = tmp();
        write(&root, "stats.json", &sample_stats(1));
        let report = migrate_into(&root, &[root.clone()]);
        assert_eq!(report.status, "already_current");
    }

    #[test]
    fn tf_1303_same_container_survives_1401() {
        // Same sandbox container as TF 1.3.x/1303: config + Insights stats,
        // no clipboard-history.json (that file is #14). 1.4.0/1401 must
        // keep those bytes. Schema rewrite is later.
        let dest = tmp();
        let config_1303 = r#"{
            "microphone": "BuiltIn",
            "engine": "local",
            "localEngine": "parakeet",
            "whisperModel": "large-v3",
            "recordingMode": "toggle",
            "hotkey": "CmdOrCtrl+D",
            "lastSeenVersion": "1.3.0"
        }"#;
        write(&dest, "config.json", config_1303);
        write(&dest, "stats.json", &sample_stats(1303));
        let before_stats = fs::read_to_string(dest.join("stats.json")).unwrap();
        let report = migrate_into(&dest, &[dest.clone()]);
        assert_eq!(report.status, "already_current", "{report:?}");
        assert_eq!(fs::read_to_string(dest.join("stats.json")).unwrap(), before_stats);
        assert!(fs::read_to_string(dest.join("config.json"))
            .unwrap()
            .contains("1.3.0"));
        assert!(!dest.join("clipboard-history.json").exists());
    }

    #[test]
    fn same_container_upgrade_does_not_wipe_history() {
        let dest = tmp();
        write(&dest, "stats.json", &sample_stats(44));
        write(
            &dest,
            "clipboard-history.json",
            r#"{"items":[{"id":"a","text":"kept","createdAt":1}]}"#,
        );
        write(&dest, "config.json", sample_config());
        let before_stats = fs::read_to_string(dest.join("stats.json")).unwrap();
        let before_clip = fs::read_to_string(dest.join("clipboard-history.json")).unwrap();
        let before_cfg = fs::read_to_string(dest.join("config.json")).unwrap();

        let report = migrate_into(&dest, &[dest.clone()]);
        assert_eq!(report.status, "already_current", "{report:?}");
        assert_eq!(fs::read_to_string(dest.join("stats.json")).unwrap(), before_stats);
        assert_eq!(
            fs::read_to_string(dest.join("clipboard-history.json")).unwrap(),
            before_clip
        );
        assert_eq!(fs::read_to_string(dest.join("config.json")).unwrap(), before_cfg);
    }

    #[test]
    fn polish_and_clipboard_locks_are_not_regressed() {
        let polish = include_str!("polish.rs");
        assert!(polish.contains("ENFORCER_BOUND"));
        assert!(polish.contains("default OFF"));
        assert!(polish.contains("clipboardHistoryEnabled != Polish"));
        assert!(polish.contains("never invent"));
        let clip = include_str!("clipboard_history.rs");
        assert!(clip.contains("Default OFF"));
        assert!(clip.contains("local slot counts"));
        assert!(clip.contains("Fail closed when opt-in is off"));
        assert!(clip.contains("Not company memory"));
        let dict = include_str!("dictionary.rs");
        assert!(dict.contains("ENFORCER_BOUND"));
        assert!(dict.contains("Pro-gated"));
        assert!(dict.contains("no Nexus/SIEM write"));
        assert!(dict.contains("no auto-promote"));
        assert!(dict.contains("no HIPAA/BAA"));
        assert!(dict.contains("Scratchpad/Insights local-only default"));
        assert!(dict.contains("fail closed if ACL missing"));
        let snip = include_str!("snippets.rs");
        assert!(snip.contains("ENFORCER_BOUND"));
        assert!(snip.contains("Pro-gated + Stiki session"));
        assert!(snip.contains("no Nexus/SIEM write"));
        assert!(snip.contains("no HIPAA/BAA"));
        assert!(snip.contains("fail closed if ACL missing"));
    }

    #[cfg(unix)]
    #[test]
    fn unreadable_legacy_with_empty_dest_is_visible_failure() {
        use std::os::unix::fs::PermissionsExt;
        let root = tmp();
        let dest = root.join("dest");
        let legacy = root.join("legacy");
        fs::create_dir_all(&legacy).unwrap();
        write(&legacy, "stats.json", &sample_stats(5));
        let mut perms = fs::metadata(&legacy).unwrap().permissions();
        perms.set_mode(0o000);
        fs::set_permissions(&legacy, perms).unwrap();

        let report = migrate_into(&dest, &[legacy.clone()]);

        let mut restore = fs::metadata(&legacy)
            .map(|m| m.permissions())
            .unwrap_or_else(|_| fs::Permissions::from_mode(0o755));
        restore.set_mode(0o755);
        let _ = fs::set_permissions(&legacy, restore);

        if report.status != "failed" {
            // Some CI users (root) can still read mode 000. Skip rather than flake.
            if std::process::id() == 0 || report.status == "migrated" {
                return;
            }
        }
        assert_eq!(report.status, "failed", "{report:?}");
        assert!(report
            .message
            .as_deref()
            .unwrap_or("")
            .contains("still there"));
    }

    #[test]
    fn container_home_strips_to_real_home() {
        let container = PathBuf::from("/Users/dev/Library/Containers/com.mabel.app/Data");
        assert_eq!(
            strip_container_prefix(&container).as_deref(),
            Some(Path::new("/Users/dev"))
        );
        assert_eq!(strip_container_prefix(Path::new("/Users/dev")), None);
    }

    #[test]
    fn read_existing_json_does_not_default_on_garbage() {
        let dir = tmp();
        let path = dir.join("stats.json");
        fs::write(&path, "???") .unwrap();
        let err = read_existing_json::<serde_json::Value>(&path, "Insights history").unwrap_err();
        assert!(err.contains("left untouched"), "{err}");
        assert_eq!(fs::read_to_string(&path).unwrap(), "???");
    }

    #[test]
    fn read_existing_json_missing_is_none() {
        let dir = tmp();
        assert_eq!(
            read_existing_json::<serde_json::Value>(&dir.join("nope.json"), "x").unwrap(),
            None
        );
    }

    #[test]
    fn main_imports_prior_store_before_loading() {
        let main = include_str!("main.rs");
        let migrate = main.find("storage::migrate_into").expect("migrate_into");
        let settings = main
            .find("Settings::load_with_status")
            .expect("load_with_status");
        let stats = main
            .find("StatsStore::load_with_status")
            .expect("stats load_with_status");
        assert!(migrate < settings && migrate < stats);
        assert!(main.contains("get_storage_status"));
        assert!(!main.contains("PathBuf::from(\".\")"));
    }
}
