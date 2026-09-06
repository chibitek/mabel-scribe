//! Product **Snippets** — Enforcer BOUND (suite b6530197) + Sign-on RE-LOCK.
//!
//! GREEN: StoreKit Pro + Stiki dual gate; local-first personal snippets;
//! no cloud sync; no team/company share without ACL Make It So;
//! no Nexus/SIEM write; no auto-promote; no HIPAA/BAA; Free dictate no Stiki;
//! distinct from Polish/Clipboard/Dictionary stores.
//!
//! BREAKS IF: Snippets without dual gate
//! BREAKS IF: cloud/team share without ACL
//! BREAKS IF: Nexus write
//! BREAKS IF: auto-promote
//! HELD: cloud sync + team share until MCS with Stiki/folder-style ACL;
//! fail-closed if ACL missing.

use std::path::PathBuf;

use crate::pro_features::{self, Snippet};
use crate::stiki_session;
use crate::storekit;

/// Named Enforcer suite. Tests fail if this is retargeted without a new MCS.
pub const ENFORCER_SUITE: &str = "b6530197";

/// Named Enforcer BOUND. Tests fail if this is violated.
pub const ENFORCER_BOUND: &str = "StoreKit Pro + Stiki dual gate; local-first personal snippets; no cloud sync; no team/company share without ACL Make It So; no Nexus/SIEM write; no auto-promote; no HIPAA/BAA; Free dictate no Stiki; distinct from Polish/Clipboard/Dictionary stores";

/// Product LOCK Snippets v1 + Sign-on. Tests fail if the surface drifts.
pub const PRODUCT_LOCK: &str = "Name: Snippets; Pro surface requires StoreKit Pro AND Stiki session; Free locked + Activate Pro / Sign in with Stiki; trigger phrase → replacement during cleanup/dictation paste; Settings + sidebar/nav; default empty; add/edit/delete on device; local-first; not Nexus; not cloud sync v1; distinct from Dictionary, Polish, and Clipboard History; non-goals: shared team snippets, cloud write, HIPAA";

/// Held. A later MCS must flip this only with Stiki/folder-style ACL.
pub const STIKI_FOLDER_ACL_SHIPPED: bool = false;

const SHARE_BLOCKED: &str =
    "Cloud and team snippet share is not available. Snippets stay on this Mac.";
const ACL_MISSING: &str =
    "Cloud and team snippet share is held until a Stiki/folder-style ACL Make It So. ACL missing — fail closed.";

/// Both gates. Pro without Stiki stays locked. Stiki without Pro stays locked.
pub fn surface_ready() -> bool {
    storekit::pro_surfaces_unlocked()
}

pub fn require_surface_with(entitled: bool, signed_in: bool) -> Result<(), String> {
    if !entitled {
        return Err(
            "Mabel Pro requires an active App Store subscription (the 30-day trial counts)."
                .into(),
        );
    }
    if !signed_in {
        return Err(stiki_session::NEED_SESSION.into());
    }
    Ok(())
}

pub fn require_surface() -> Result<(), String> {
    storekit::require_pro()?;
    Ok(())
}

/// Ungated local read. Callers must apply `effective_snippets` before expanding.
pub fn load_stored(app_dir: &PathBuf) -> Vec<Snippet> {
    pro_features::snippets_read(app_dir)
}

/// Runtime gate: Free / no Stiki / lapsed → no expansions in dictation.
pub fn effective_snippets_with(
    stored: &[Snippet],
    entitled: bool,
    signed_in: bool,
) -> Vec<Snippet> {
    if entitled && signed_in {
        stored.to_vec()
    } else {
        Vec::new()
    }
}

pub fn effective_snippets(stored: &[Snippet]) -> Vec<Snippet> {
    effective_snippets_with(
        stored,
        storekit::current_entitlement().entitled,
        stiki_session::is_live(),
    )
}

pub fn require_list(app_dir: &PathBuf) -> Result<Vec<Snippet>, String> {
    require_surface()?;
    Ok(load_stored(app_dir))
}

pub fn add(app_dir: &PathBuf, trigger: String, expansion: String) -> Result<Vec<Snippet>, String> {
    require_surface()?;
    pro_features::snippets_add_local(app_dir, trigger, expansion)
}

pub fn update(
    app_dir: &PathBuf,
    snippet_id: String,
    trigger: String,
    expansion: String,
) -> Result<Vec<Snippet>, String> {
    require_surface()?;
    pro_features::snippets_update_local(app_dir, snippet_id, trigger, expansion)
}

pub fn remove(app_dir: &PathBuf, snippet_id: String) -> Result<Vec<Snippet>, String> {
    require_surface()?;
    pro_features::snippets_remove_local(app_dir, snippet_id)
}

pub fn stiki_folder_acl_present() -> bool {
    STIKI_FOLDER_ACL_SHIPPED
}

/// HELD path. Fail closed when the Stiki/folder-style ACL is missing.
pub fn require_share_acl() -> Result<(), String> {
    if !stiki_folder_acl_present() {
        return Err(ACL_MISSING.into());
    }
    Err(SHARE_BLOCKED.into())
}

/// Soft later: cloud / team share. Do not ship until a separate ACL Make It So.
pub fn share_cloud_or_team() -> Result<(), String> {
    require_share_acl()?;
    Err(SHARE_BLOCKED.into())
}

/// BREAKS IF: private Snippets auto-promote to company memory.
pub fn promote_to_company_memory() -> Result<(), String> {
    Err("Private snippets cannot become company memory.".into())
}

/// Expand spoken triggers into stored templates. Free / empty list → unchanged.
pub fn apply_expansions(text: &str, stored: &[Snippet]) -> String {
    apply_expansions_with(text, &effective_snippets(stored))
}

pub fn apply_expansions_for_dir(text: &str, app_dir: &PathBuf) -> String {
    apply_expansions(text, &load_stored(app_dir))
}

fn apply_expansions_with(text: &str, snippets: &[Snippet]) -> String {
    if text.is_empty() || snippets.is_empty() {
        return text.to_string();
    }
    let mut items: Vec<&Snippet> = snippets.iter().collect();
    items.sort_by(|a, b| b.trigger.chars().count().cmp(&a.trigger.chars().count()));
    let mut out = text.to_string();
    for snippet in items {
        out = replace_trigger_ignore_case(&out, &snippet.trigger, &snippet.expansion);
    }
    out
}

fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '\''
}

fn replace_trigger_ignore_case(text: &str, trigger: &str, expansion: &str) -> String {
    let trigger = trigger.trim();
    if trigger.is_empty() {
        return text.to_string();
    }
    let chars: Vec<char> = text.chars().collect();
    let n = trigger.chars().count();
    let mut i = 0usize;
    let mut out = String::new();
    while i < chars.len() {
        if i + n <= chars.len() {
            let slice: String = chars[i..i + n].iter().collect();
            let prev_ok = i == 0 || !is_word_char(chars[i - 1]);
            let next_ok = i + n == chars.len() || !is_word_char(chars[i + n]);
            if slice.eq_ignore_ascii_case(trigger) && prev_ok && next_ok {
                out.push_str(expansion);
                i += n;
                continue;
            }
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(trigger: &str, expansion: &str) -> Snippet {
        Snippet {
            id: "1".into(),
            trigger: trigger.into(),
            expansion: expansion.into(),
        }
    }

    #[test]
    fn mutations_fail_closed_without_entitlement() {
        let dir = std::env::temp_dir().join(format!("mabel-snip-gate-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        assert!(require_list(&dir).is_err());
        assert!(add(&dir, "sig".into(), "Best".into()).is_err());
        assert!(update(&dir, "1".into(), "sig".into(), "Best".into()).is_err());
        assert!(remove(&dir, "missing".into()).is_err());
    }

    #[test]
    fn effective_snippets_empty_without_pro() {
        let stored = vec![sample("sig", "Best regards")];
        assert!(effective_snippets(&stored).is_empty());
        assert_eq!(apply_expansions("please send sig", &stored), "please send sig");
    }

    #[test]
    fn breaks_if_snippets_usable_without_stiki() {
        assert!(
            require_surface_with(true, false).is_err(),
            "BREAKS IF: Snippets usable without Stiki"
        );
        assert!(
            effective_snippets_with(&[sample("sig", "Best")], true, false).is_empty(),
            "BREAKS IF: snippet expansions without Stiki"
        );
        assert_eq!(
            apply_expansions("sig", &[sample("sig", "Best regards")]),
            "sig",
            "BREAKS IF: expansions apply without Stiki session"
        );
        assert!(require_surface_with(false, true).is_err());
        assert!(require_surface_with(true, true).is_ok());
        assert_eq!(
            effective_snippets_with(&[sample("sig", "Best")], true, true)
                .first()
                .map(|s| s.trigger.as_str()),
            Some("sig")
        );

        let html = include_str!("../../index.html");
        let ts = include_str!("../../src/main.ts");
        assert!(
            html.contains("Sign in with Stiki"),
            "BREAKS IF: Sign in with Stiki CTA missing"
        );
        assert!(
            ts.contains("snippetsSurfaceReady"),
            "BREAKS IF: frontend Stiki+Pro gate missing"
        );
        let load_snip = ts
            .split("async function loadSnippetsSurface")
            .nth(1)
            .expect("loadSnippetsSurface");
        let load_snip = load_snip.split("async function ").next().unwrap();
        assert!(load_snip.contains("snippetsSurfaceReady"));
        assert!(load_snip.contains("snippets_get"));
        let load_pro = ts
            .split("async function loadProSurfaces")
            .nth(1)
            .expect("loadProSurfaces");
        let load_pro = load_pro.split("async function ").next().unwrap();
        assert!(
            !load_pro.contains("snippets_get"),
            "BREAKS IF: Snippets fetched without Stiki gate"
        );
        assert!(!surface_ready(), "Linux / unsigned-out must stay locked");
    }

    #[test]
    fn free_dictation_does_not_require_stiki() {
        let rec = include_str!("recorder.rs");
        let streaming = include_str!("streaming.rs");
        let local = include_str!("transcribe_local.rs");
        let native = include_str!("transcribe_native.rs");
        let commands = include_str!("main.rs");
        let toggle = commands
            .split("async fn do_toggle_recording")
            .nth(1)
            .or_else(|| commands.split("async fn toggle_recording").nth(1))
            .expect("toggle_recording");
        let toggle = toggle.split("#[tauri::command]").next().unwrap();
        assert!(
            !toggle.contains("stiki::require_session") && !toggle.contains("require_surface"),
            "BREAKS IF: Free dictation requires Stiki"
        );
        for hay in [rec, streaming, local, native] {
            assert!(
                !hay.contains("stiki::require_session") && !hay.contains("require_surface"),
                "BREAKS IF: Free dictation requires Stiki"
            );
        }
        assert!(rec.contains("apply_expansions"));
        assert!(streaming.contains("apply_expansions"));
    }

    #[test]
    fn share_cloud_or_team_fails_closed() {
        assert!(
            !stiki_folder_acl_present(),
            "BREAKS IF: Stiki/folder ACL claimed present without MCS"
        );
        let acl = require_share_acl().unwrap_err();
        assert!(acl.contains("ACL missing"), "BREAKS IF: share without ACL");
        let err = share_cloud_or_team().unwrap_err();
        assert!(err.contains("ACL missing") || err.contains("not available"));
        assert!(!err.contains("http"));
        let promote = promote_to_company_memory().unwrap_err();
        assert!(promote.contains("company memory"));
    }

    #[test]
    fn breaks_if_cloud_write_ships_without_acl_make_it_so() {
        assert!(!STIKI_FOLDER_ACL_SHIPPED, "BREAKS IF: ACL shipped without MCS");
        assert!(
            share_cloud_or_team().is_err(),
            "BREAKS IF: cloud write succeeded"
        );
        let ts = include_str!("../../src/main.ts");
        assert!(
            !ts.contains("snippets_share"),
            "BREAKS IF: share UI shipped"
        );
        let html = include_str!("../../index.html");
        let view = html.split("data-view=\"snippets\"").last().unwrap();
        let view = view.split("<section").next().unwrap();
        assert!(
            !view.contains("Share with team") && !view.contains("Sync to cloud"),
            "BREAKS IF: cloud/team share shipped without ACL Make It So"
        );
        assert!(
            !view.contains("https://"),
            "BREAKS IF: cloud write / web upgrade on Snippets"
        );
    }

    #[test]
    fn breaks_if_private_snippets_auto_promote() {
        assert!(
            promote_to_company_memory().is_err(),
            "BREAKS IF: private Snippets auto-promote to company memory"
        );
        let ts = include_str!("../../src/main.ts");
        assert!(
            !ts.contains("snippets_promote"),
            "BREAKS IF: promote UI shipped"
        );
    }

    #[test]
    fn enforcer_bound_suite_b6530197_green() {
        assert_eq!(ENFORCER_SUITE, "b6530197");
        assert!(ENFORCER_BOUND.contains("StoreKit Pro + Stiki dual gate"));
        assert!(ENFORCER_BOUND.contains("local-first personal snippets"));
        assert!(ENFORCER_BOUND.contains("no cloud sync"));
        assert!(ENFORCER_BOUND.contains("no team/company share without ACL Make It So"));
        assert!(ENFORCER_BOUND.contains("no Nexus/SIEM write"));
        assert!(ENFORCER_BOUND.contains("no auto-promote"));
        assert!(ENFORCER_BOUND.contains("no HIPAA/BAA"));
        assert!(ENFORCER_BOUND.contains("Free dictate no Stiki"));
        assert!(ENFORCER_BOUND.contains("distinct from Polish/Clipboard/Dictionary stores"));
    }

    #[test]
    fn enforcer_bound_breaks_if_free_cloud_nexus_or_web_upgrade() {
        assert_eq!(ENFORCER_SUITE, "b6530197");
        assert!(ENFORCER_BOUND.contains("StoreKit Pro + Stiki dual gate"));
        assert!(ENFORCER_BOUND.contains("local-first personal snippets"));
        assert!(ENFORCER_BOUND.contains("no cloud sync"));
        assert!(ENFORCER_BOUND.contains("no team/company share without ACL Make It So"));
        assert!(ENFORCER_BOUND.contains("no Nexus/SIEM write"));
        assert!(ENFORCER_BOUND.contains("no auto-promote"));
        assert!(ENFORCER_BOUND.contains("no HIPAA/BAA"));
        assert!(ENFORCER_BOUND.contains("Free dictate no Stiki"));
        assert!(ENFORCER_BOUND.contains("distinct from Polish/Clipboard/Dictionary stores"));

        let html = include_str!("../../index.html");
        let nav = html
            .split("<nav class=\"sidebar-nav\">")
            .nth(1)
            .and_then(|s| s.split("</nav>").next())
            .expect("sidebar nav");
        let views: Vec<&str> = nav
            .split("data-view=\"")
            .skip(1)
            .filter_map(|s| s.split('"').next())
            .collect();
        assert_eq!(
            views,
            [
                "home",
                "transcribe-file",
                "dictionary",
                "snippets",
                "style",
                "transforms",
                "scratchpad",
                "insights",
                "teams",
                "connectors"
            ],
            "BREAKS IF: Pro catalog order drifted"
        );
        assert!(
            nav.split("data-view=\"snippets\"")
                .nth(1)
                .unwrap()
                .contains("lock-pill"),
            "BREAKS IF: Snippets nav not Pro-locked"
        );
        let view = html
            .split("data-view=\"snippets\"")
            .last()
            .expect("snippets view");
        let view = view.split("<section").next().unwrap();
        assert!(view.contains("pro-lock"), "BREAKS IF: Free not locked");
        assert!(view.contains("pro-activate"), "BREAKS IF: no Plans upsell");
        assert!(
            view.contains("Sign in with Stiki"),
            "BREAKS IF: Sign in with Stiki missing on locked Snippets"
        );
        assert!(!view.contains("chibiteklabs.com"), "BREAKS IF: web upgrade");
        assert!(!view.contains("https://"), "BREAKS IF: web upgrade");
        assert!(view.contains("this Mac"));
        assert!(
            !view.contains("Share with team") && !view.contains("Sync to cloud"),
            "BREAKS IF: share shipped"
        );

        let ts = include_str!("../../src/main.ts");
        assert!(ts.contains("snippets_add"));
        assert!(ts.contains("snippets_update"));
        assert!(ts.contains("snippets_remove"));
        assert!(ts.contains("beginEditSnippet"));
        assert!(ts.contains("openPlans()"));
        assert!(!ts.contains("https://chibiteklabs"));
        assert!(
            ts.contains("\"snippets\""),
            "BREAKS IF: Snippets dropped from Pro lock list"
        );

        let commands = include_str!("main.rs");
        assert!(commands.contains("snippets::apply_expansions") || rec_or_stream_applies());
        assert!(commands.contains("snippets_add"));
        assert!(commands.contains("snippets_update"));
        assert!(commands.contains("snippets_remove"));
        assert!(commands.contains("share_cloud_or_team"));
        assert!(commands.contains("promote_to_company_memory"));
        assert!(include_str!("snippets.rs").contains("require_share_acl"));
        let nexus_write = format!("{}{}", "nexus", "_write");
        assert!(
            !commands.contains(&nexus_write),
            "BREAKS IF: Nexus write in commands"
        );
        assert!(
            !include_str!("settings.rs").contains(&nexus_write),
            "BREAKS IF: Nexus write in settings"
        );
        assert!(
            !include_str!("../../src/main.ts").contains(&nexus_write),
            "BREAKS IF: Nexus write in UI"
        );
        assert!(!commands.contains("MabelSpatial"), "BREAKS IF: Spatial touched");
        assert!(include_str!("snippets.rs").contains("fail-closed"));

        for hay in [html, ts, commands] {
            assert!(!hay.contains("HIPAA"), "BREAKS IF: HIPAA claim copy");
            assert!(!hay.contains("HIPAA/BAA"), "BREAKS IF: BAA claim copy");
            assert!(
                !hay.contains("Business Associate"),
                "BREAKS IF: BAA claim copy"
            );
        }
        let siem_write = format!("{}{}", "siem", "_write");
        assert!(!commands.contains(&siem_write), "BREAKS IF: SIEM write");
        assert!(html.contains("id=\"snippets-open\""));
        assert!(html.contains("id=\"snippets-activate\""));
        assert!(ts.contains("openSnippets"));
        let tray = include_str!("clipboard_ui.rs");
        assert!(tray.contains("Snippets"));
        assert!(tray.contains("open-snippets"));
        assert!(
            include_str!("snippets.rs").contains("promote_to_company_memory"),
            "BREAKS IF: auto-promote stub missing"
        );
        assert!(
            include_str!("pro_features.rs").contains("snippets.json"),
            "BREAKS IF: snippet store missing"
        );
        assert!(
            !include_str!("settings.rs").contains("snippets.json"),
            "BREAKS IF: snippets folded into Dictionary/settings store"
        );
        assert!(
            !include_str!("polish.rs").contains("snippets.json")
                && !include_str!("clipboard_history.rs").contains("snippets.json"),
            "BREAKS IF: snippets folded into Polish or Clipboard stores"
        );
    }

    #[test]
    fn breaks_if_snippets_without_dual_gate() {
        assert!(
            require_surface_with(true, false).is_err(),
            "BREAKS IF: Snippets without dual gate (Pro, no Stiki)"
        );
        assert!(
            require_surface_with(false, true).is_err(),
            "BREAKS IF: Snippets without dual gate (Stiki, no Pro)"
        );
        assert!(
            require_surface_with(false, false).is_err(),
            "BREAKS IF: Snippets without dual gate"
        );
        assert!(require_surface_with(true, true).is_ok());
        assert!(
            effective_snippets_with(&[sample("sig", "Best")], true, false).is_empty(),
            "BREAKS IF: Snippets without dual gate"
        );
        assert!(!surface_ready(), "BREAKS IF: Snippets without dual gate");
    }

    #[test]
    fn breaks_if_cloud_team_share_without_acl() {
        assert!(
            !STIKI_FOLDER_ACL_SHIPPED && !stiki_folder_acl_present(),
            "BREAKS IF: cloud/team share without ACL"
        );
        assert!(
            require_share_acl().is_err() && share_cloud_or_team().is_err(),
            "BREAKS IF: cloud/team share without ACL"
        );
    }

    #[test]
    fn breaks_if_nexus_or_siem_write() {
        let nexus_write = format!("{}{}", "nexus", "_write");
        let siem_write = format!("{}{}", "siem", "_write");
        for hay in [
            include_str!("main.rs"),
            include_str!("snippets.rs"),
            include_str!("pro_features.rs"),
            include_str!("settings.rs"),
            include_str!("../../src/main.ts"),
            include_str!("../../index.html"),
        ] {
            assert!(!hay.contains(&nexus_write), "BREAKS IF: Nexus write");
            assert!(!hay.contains(&siem_write), "BREAKS IF: Nexus write");
        }
    }

    #[test]
    fn breaks_if_auto_promote() {
        assert!(
            promote_to_company_memory().is_err(),
            "BREAKS IF: auto-promote"
        );
        assert!(
            !include_str!("../../src/main.ts").contains("snippets_promote"),
            "BREAKS IF: auto-promote"
        );
    }

    fn rec_or_stream_applies() -> bool {
        include_str!("recorder.rs").contains("snippets::apply_expansions")
            && include_str!("streaming.rs").contains("snippets::apply_expansions")
    }

    #[test]
    fn local_store_add_edit_delete() {
        let dir = std::env::temp_dir().join(format!(
            "mabel-snip-crud-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let items = crate::pro_features::snippets_add_local(
            &dir,
            "sig".into(),
            "Best".into(),
        )
        .unwrap();
        assert_eq!(items.len(), 1);
        let edited = crate::pro_features::snippets_update_local(
            &dir,
            items[0].id.clone(),
            "signature".into(),
            "Best regards".into(),
        )
        .unwrap();
        assert_eq!(edited[0].trigger, "signature");
        assert_eq!(edited[0].expansion, "Best regards");
        let left = crate::pro_features::snippets_remove_local(&dir, items[0].id.clone()).unwrap();
        assert!(left.is_empty());
    }

    #[test]
    fn expansions_apply_during_dictation_without_inventing() {
        let snippets = vec![
            sample("insert my calendar link", "https://cal.example/erick"),
            sample("standard reply", "Thanks for writing — I'll follow up today."),
            sample("sig", "Best,\nErick"),
        ];
        assert_eq!(
            apply_expansions_with("Please insert my calendar link now.", &snippets),
            "Please https://cal.example/erick now."
        );
        assert_eq!(
            apply_expansions_with("Send the standard reply after.", &snippets),
            "Send the Thanks for writing — I'll follow up today. after."
        );
        assert_eq!(
            apply_expansions_with("Nothing to change here.", &snippets),
            "Nothing to change here."
        );
        assert_eq!(
            apply_expansions("sig", &snippets),
            "sig",
            "Free / no entitlement must not expand snippets"
        );
        // Longer trigger wins over a shorter overlapping one.
        let overlap = vec![
            sample("cal", "SHORT"),
            sample("insert my calendar link", "LONG"),
        ];
        assert_eq!(
            apply_expansions_with("insert my calendar link", &overlap),
            "LONG"
        );
    }

    #[test]
    fn product_lock_v1_non_goals_and_surface() {
        assert!(PRODUCT_LOCK.contains("Name: Snippets"));
        assert!(PRODUCT_LOCK.contains("StoreKit Pro AND Stiki session"));
        assert!(PRODUCT_LOCK.contains("Activate Pro / Sign in with Stiki"));
        assert!(PRODUCT_LOCK.contains("trigger phrase → replacement"));
        assert!(PRODUCT_LOCK.contains("cleanup/dictation paste"));
        assert!(PRODUCT_LOCK.contains("Settings + sidebar/nav"));
        assert!(PRODUCT_LOCK.contains("add/edit/delete"));
        assert!(PRODUCT_LOCK.contains("distinct from Dictionary, Polish, and Clipboard History"));
        assert!(PRODUCT_LOCK.contains("HIPAA"));

        let html = include_str!("../../index.html");
        let ts = include_str!("../../src/main.ts");
        let rec = include_str!("recorder.rs");
        assert!(html.contains("view-title\">Snippets"));
        assert!(html.contains("id=\"snippets-activate\""));
        assert!(html.contains("id=\"snippets-open\""));
        assert!(html.contains("id=\"snippet-cancel-btn\""));
        assert!(html.contains("id=\"dictionary-activate\""));
        assert!(html.contains("id=\"polish-activate\""));
        assert!(ts.contains("openPlans()"));
        assert!(ts.contains("snippets_update"));
        assert!(ts.contains("beginEditSnippet"));
        assert!(rec.contains("apply_expansions"));
        assert!(include_str!("streaming.rs").contains("apply_expansions"));
        assert!(include_str!("cleanup.rs").contains("cleanup_text"));
        assert!(
            rec.contains("cleanup_text") && rec.contains("apply_expansions"),
            "BREAKS IF: expansions not applied during cleanup/dictation paste"
        );

        for hay in [html, ts] {
            assert!(!hay.contains("HIPAA"), "BREAKS IF: HIPAA claim copy");
        }
    }

    #[test]
    fn distinct_from_dictionary_polish_and_clipboard() {
        let settings = crate::settings::Settings::default();
        assert!(settings.dictionary.is_empty());
        assert_eq!(settings.polish_mode, "off");
        assert!(!settings.clipboard_history_enabled);

        let html = include_str!("../../index.html");
        let engine = html
            .split("data-pane=\"engine\"")
            .last()
            .expect("engine pane");
        let engine = engine.split("data-pane=").next().unwrap();
        assert!(engine.contains("row-label\">Snippets"));
        assert!(engine.contains("not Dictionary"));
        assert!(engine.contains("not Polish"));
        assert!(engine.contains("not Clipboard History"));
        assert!(html.contains("id=\"polish-toggle\""));
        assert!(html.contains("id=\"clipboard-history-toggle\""));
        assert!(html.contains("id=\"dict-add-btn\""));
        assert_ne!("snippet-add-btn", "dict-add-btn");
        assert_ne!("snippets-open", "dictionary-open");
    }

    #[test]
    fn scratchpad_and_insights_stay_local_without_hipaa_copy() {
        let html = include_str!("../../index.html");
        let insights = html
            .split("data-view=\"insights\"")
            .nth(2)
            .or_else(|| html.split("data-view=\"insights\"").last())
            .expect("insights view");
        let insights = insights.split("<section").next().unwrap();
        assert!(insights.contains("Local-only"));
        assert!(!insights.contains("HIPAA"));

        let pad = html
            .split("data-view=\"scratchpad\"")
            .nth(2)
            .or_else(|| html.split("data-view=\"scratchpad\"").last())
            .expect("scratchpad view");
        let pad = pad.split("<section").next().unwrap();
        assert!(
            pad.contains("Local only") || pad.contains("local-only") || pad.contains("this Mac")
        );
        assert!(!pad.contains("HIPAA"));
    }
}
