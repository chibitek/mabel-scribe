//! Product **Scratchpad** — Enforcer BOUND (suite b6530197) CONFIRMED + Sign-on RE-LOCK.
//!
//! GREEN: StoreKit Pro + Stiki dual gate; local-only scratchpad of
//! dictations/notes on this Mac; Scratchpad NOT MCP source/sink v1;
//! no cloud/team/Nexus/Mochii auto-push; no Nexus/Mochii promote; share
//! fail closed; distinct Style (Formal|Casual|Very casual), Transforms
//! (Email|Bullets|Shorter|Clearer), Polish (Off|Casual|Professional|Polite),
//! Dictionary, Snippets, Clipboard; no cloud sync; no team/company share
//! without ACL Make It So; no Nexus/SIEM write; no auto-promote; no HIPAA/BAA;
//! Free dictate no Stiki; sign-out locks Scratchpad.
//!
//! BREAKS IF: Scratchpad without dual gate
//! BREAKS IF: Scratchpad MCP source/sink v1
//! BREAKS IF: cloud/team/Nexus/Mochii promote
//! BREAKS IF: merges Style/Transforms/Polish/Dictionary/Snippets/Clipboard stores
//! BREAKS IF: Free dictate gated
//! BREAKS IF: sign-out leaves Scratchpad unlocked
//! HELD: cloud sync + team share until MCS with Stiki/folder-style ACL;
//! fail-closed if ACL missing.

use std::path::PathBuf;

use crate::connectors;
use crate::pro_features;
use crate::stiki_session;
use crate::storekit;

/// Named Enforcer suite. Tests fail if this is retargeted without a new MCS.
pub const ENFORCER_SUITE: &str = "b6530197";

/// Named Enforcer BOUND. `enforcer_bound_scratchpad_v1_suite_*` tests fold this.
pub const ENFORCER_BOUND: &str = "CONFIRMED Suite b6530197; StoreKit Pro + Stiki dual gate; local-only scratchpad of dictations/notes; Scratchpad NOT MCP source/sink v1; no cloud/team/Nexus/Mochii auto-push; no Nexus/Mochii promote; share fail closed; local-only; sign-out locks Scratchpad; distinct Style/Transforms/Polish/Dictionary/Snippets/Clipboard; no cloud sync; no team/company share without ACL Make It So; no Nexus/SIEM write; no auto-promote; no HIPAA/BAA; Free dictate no Stiki";

/// Product LOCK Scratchpad v1 + Sign-on. Tests fail if the surface drifts.
pub const PRODUCT_LOCK: &str = "Name: Scratchpad; Pro catalog #5 after Dictionary → Snippets → Style → Transforms → Scratchpad; Pro surface requires StoreKit Pro AND Stiki session; Free locked + Activate Pro / Sign in with Stiki; local-only scratchpad of dictations/notes on this Mac; Settings + sidebar/nav + menu bar; add/edit/clear notes; local-first; not Nexus; not Mochii; not MCP source/sink v1; not cloud sync v1; distinct from Style (Formal|Casual|Very casual register), Transforms (Email|Bullet points|Make shorter|Make clearer), Polish (Off|Casual|Professional|Polite Gemma tone rewrite), Dictionary (spelling), Snippets (trigger→expansion), and Clipboard History; non-goals: MCP source/sink, cloud write, Nexus/Mochii promote, HIPAA";

/// Held. A later MCS must flip this only with Stiki/folder-style ACL.
pub const STIKI_FOLDER_ACL_SHIPPED: bool = false;

const SHARE_BLOCKED: &str =
    "Cloud and team scratchpad share is not available. Scratchpad stays on this Mac.";
const ACL_MISSING: &str =
    "Cloud and team scratchpad share is held until a Stiki/folder-style ACL Make It So. ACL missing — fail closed.";

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

/// Ungated local read. Callers must apply the dual gate before show / mutate.
pub fn load_stored(app_dir: &PathBuf) -> String {
    pro_features::scratchpad_read(app_dir)
}

/// Runtime gate: Free / no Stiki / lapsed → empty surface (notes stay on disk).
pub fn effective_text_with(stored: &str, entitled: bool, signed_in: bool) -> String {
    if entitled && signed_in {
        stored.to_string()
    } else {
        String::new()
    }
}

pub fn effective_text(stored: &str) -> String {
    effective_text_with(
        stored,
        storekit::current_entitlement().entitled,
        stiki_session::is_live(),
    )
}

pub fn require_text(app_dir: &PathBuf) -> Result<String, String> {
    require_surface()?;
    Ok(load_stored(app_dir))
}

/// Save / edit notes. Local file only.
pub fn save(app_dir: &PathBuf, text: String) -> Result<String, String> {
    require_surface()?;
    pro_features::scratchpad_write_local(app_dir, text)
}

/// Clear the pad. Notes stay unused until the user writes again.
pub fn clear(app_dir: &PathBuf) -> Result<String, String> {
    require_surface()?;
    pro_features::scratchpad_write_local(app_dir, String::new())
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

/// BREAKS IF: private Scratchpad auto-promotes to Nexus / Mochii / company memory.
pub fn promote_to_company_memory() -> Result<(), String> {
    Err("Private scratchpad cannot become company memory.".into())
}

/// BREAKS IF: Scratchpad is an MCP source in v1.
pub fn as_mcp_source(text: &str) -> Result<(), String> {
    connectors::scratchpad_as_mcp_source(text)
}

/// BREAKS IF: Scratchpad is an MCP sink in v1.
pub fn as_mcp_sink(text: &str) -> Result<(), String> {
    connectors::scratchpad_as_mcp_sink(text)
}

/// BREAKS IF: Scratchpad exports to Captures / Nexus in v1.
pub fn export_to_captures(text: &str) -> Result<(), String> {
    connectors::export_scratchpad_to_captures(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp() -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "mabel-scratchpad-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn mutations_fail_closed_without_entitlement() {
        let dir = tmp();
        assert!(require_text(&dir).is_err());
        assert!(save(&dir, "hello".into()).is_err());
        assert!(clear(&dir).is_err());
    }

    #[test]
    fn effective_text_empty_without_pro() {
        assert_eq!(effective_text("kept locally"), "");
        assert_eq!(
            effective_text_with("kept locally", true, false),
            "",
            "BREAKS IF: Scratchpad usable without Stiki"
        );
        assert_eq!(effective_text_with("kept locally", false, true), "");
    }

    #[test]
    fn default_is_empty_until_user_writes() {
        let dir = tmp();
        assert!(load_stored(&dir).is_empty());
        let written = pro_features::scratchpad_write_local(&dir, "ship friday".into()).unwrap();
        assert_eq!(written, "ship friday");
        assert_eq!(load_stored(&dir), "ship friday");
        let cleared = pro_features::scratchpad_write_local(&dir, String::new()).unwrap();
        assert!(cleared.is_empty());
        assert!(load_stored(&dir).is_empty());
    }

    #[test]
    fn local_store_persists_notes() {
        let dir = tmp();
        let first = pro_features::scratchpad_write_local(&dir, "first note".into()).unwrap();
        assert_eq!(first, "first note");
        assert_eq!(load_stored(&dir), "first note");
        let second = pro_features::scratchpad_write_local(&dir, "dictation dump".into()).unwrap();
        assert_eq!(second, "dictation dump");
        assert_eq!(load_stored(&dir), "dictation dump");
        assert!(dir.join("scratchpad.txt").exists());
        let left = pro_features::scratchpad_write_local(&dir, String::new()).unwrap();
        assert!(left.is_empty());
        assert!(load_stored(&dir).is_empty());
    }

    #[test]
    fn breaks_if_scratchpad_usable_without_stiki() {
        assert!(
            require_surface_with(true, false).is_err(),
            "BREAKS IF: Scratchpad usable without Stiki"
        );
        assert_eq!(
            effective_text_with("notes", true, false),
            "",
            "BREAKS IF: Scratchpad notes without Stiki"
        );
        assert!(require_surface_with(false, true).is_err());
        assert!(require_surface_with(true, true).is_ok());
        assert_eq!(effective_text_with("notes", true, true), "notes");

        let html = include_str!("../../index.html");
        let ts = include_str!("../../src/main.ts");
        assert!(
            html.contains("Sign in with Stiki"),
            "BREAKS IF: Sign in with Stiki CTA missing"
        );
        assert!(
            ts.contains("scratchpadSurfaceReady"),
            "BREAKS IF: frontend Stiki+Pro gate missing"
        );
        let load_pad = ts
            .split("async function loadScratchpadSurface")
            .nth(1)
            .expect("loadScratchpadSurface");
        let load_pad = load_pad.split("async function ").next().unwrap();
        assert!(load_pad.contains("scratchpadSurfaceReady"));
        assert!(load_pad.contains("scratchpad_get"));
        let load_pro = ts
            .split("async function loadProSurfaces")
            .nth(1)
            .expect("loadProSurfaces");
        let load_pro = load_pro.split("async function ").next().unwrap();
        assert!(
            !load_pro.contains("scratchpad_get"),
            "BREAKS IF: Scratchpad fetched without Stiki gate"
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
        assert!(
            !rec.contains("scratchpad::save") && !streaming.contains("scratchpad::save"),
            "BREAKS IF: dictation auto-writes Scratchpad"
        );
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
        assert!(!ts.contains("scratchpad_share"), "BREAKS IF: share UI shipped");
        let html = include_str!("../../index.html");
        let view = html.split("data-view=\"scratchpad\"").last().unwrap();
        let view = view.split("<section").next().unwrap();
        assert!(
            !view.contains("Share with team") && !view.contains("Sync to cloud"),
            "BREAKS IF: cloud/team share shipped without ACL Make It So"
        );
        assert!(
            !view.contains("https://"),
            "BREAKS IF: cloud write / web upgrade on Scratchpad"
        );
    }

    #[test]
    fn breaks_if_private_scratchpad_auto_promotes() {
        assert!(
            promote_to_company_memory().is_err(),
            "BREAKS IF: private Scratchpad auto-promote to company memory"
        );
        assert!(as_mcp_source("notes").is_err(), "BREAKS IF: Nexus/Mochii promote");
        assert!(as_mcp_sink("notes").is_err(), "BREAKS IF: Nexus/Mochii promote");
        assert!(export_to_captures("notes").is_err(), "BREAKS IF: Nexus/Mochii promote");
        let ts = include_str!("../../src/main.ts");
        assert!(
            !ts.contains("scratchpad_promote"),
            "BREAKS IF: promote UI shipped"
        );
        let html = include_str!("../../index.html");
        assert!(!html.contains("id=\"scratchpad-export-nexus\""));
        assert!(!html.contains("id=\"scratchpad-export-mochii\""));
    }

    #[test]
    fn enforcer_bound_suite_b6530197_green() {
        assert_eq!(ENFORCER_SUITE, "b6530197");
        assert!(ENFORCER_BOUND.contains("CONFIRMED Suite b6530197"));
        assert!(ENFORCER_BOUND.contains("StoreKit Pro + Stiki dual gate"));
        assert!(ENFORCER_BOUND.contains("local-only scratchpad of dictations/notes"));
        assert!(ENFORCER_BOUND.contains("Scratchpad NOT MCP source/sink v1"));
        assert!(ENFORCER_BOUND.contains("no cloud/team/Nexus/Mochii auto-push"));
        assert!(ENFORCER_BOUND.contains("no Nexus/Mochii promote"));
        assert!(ENFORCER_BOUND.contains("share fail closed"));
        assert!(ENFORCER_BOUND.contains("local-only"));
        assert!(ENFORCER_BOUND.contains("sign-out locks Scratchpad"));
        assert!(ENFORCER_BOUND.contains("distinct Style/Transforms/Polish/Dictionary/Snippets/Clipboard"));
        assert!(ENFORCER_BOUND.contains("no cloud sync"));
        assert!(ENFORCER_BOUND.contains("no team/company share without ACL Make It So"));
        assert!(ENFORCER_BOUND.contains("no Nexus/SIEM write"));
        assert!(ENFORCER_BOUND.contains("no auto-promote"));
        assert!(ENFORCER_BOUND.contains("no HIPAA/BAA"));
        assert!(ENFORCER_BOUND.contains("Free dictate no Stiki"));
    }

    #[test]
    fn enforcer_bound_scratchpad_v1_suite_b6530197() {
        assert_eq!(ENFORCER_SUITE, "b6530197");
        assert!(ENFORCER_BOUND.contains("CONFIRMED Suite b6530197"));
        assert!(ENFORCER_BOUND.contains("StoreKit Pro + Stiki dual gate"));
        assert!(ENFORCER_BOUND.contains("local-only"));
        assert!(ENFORCER_BOUND.contains("local-only scratchpad of dictations/notes"));
        assert!(ENFORCER_BOUND.contains("Scratchpad NOT MCP source/sink v1"));
        assert!(ENFORCER_BOUND.contains("no cloud/team/Nexus/Mochii auto-push"));
        assert!(ENFORCER_BOUND.contains("no Nexus/Mochii promote"));
        assert!(ENFORCER_BOUND.contains("share fail closed"));
        assert!(ENFORCER_BOUND.contains("sign-out locks Scratchpad"));
        assert!(ENFORCER_BOUND.contains("no cloud sync"));
        assert!(ENFORCER_BOUND.contains("no Nexus/SIEM write"));
        assert!(ENFORCER_BOUND.contains("no HIPAA/BAA"));
        assert!(ENFORCER_BOUND.contains("Free dictate no Stiki"));

        assert!(
            require_surface_with(true, false).is_err(),
            "BREAKS IF: Scratchpad without dual gate"
        );
        assert!(
            require_surface_with(false, true).is_err(),
            "BREAKS IF: Scratchpad without dual gate"
        );
        assert_eq!(
            effective_text_with("notes", true, false),
            "",
            "BREAKS IF: sign-out leaves Scratchpad unlocked"
        );
        assert!(share_cloud_or_team().is_err(), "BREAKS IF: cloud/team share");
        assert!(promote_to_company_memory().is_err(), "BREAKS IF: auto-promote");
        assert!(as_mcp_source("notes").is_err(), "BREAKS IF: Scratchpad MCP source/sink v1");
        assert!(as_mcp_sink("notes").is_err(), "BREAKS IF: Scratchpad MCP source/sink v1");

        let html = include_str!("../../index.html");
        let ts = include_str!("../../src/main.ts");
        assert!(html.contains("id=\"scratchpad-text\""));
        assert!(html.contains("id=\"scratchpad-clear-btn\""));
        assert!(ts.contains("applyScratchpadGate"));
        assert!(ts.contains("scratchpadSurfaceReady"));
        assert!(ts.contains("stiki_sign_out"));
        let apply = ts
            .split("function applyScratchpadGate")
            .nth(1)
            .expect("applyScratchpadGate");
        let apply = apply.split("function ").next().unwrap();
        assert!(
            apply.contains("scratchpadSurfaceReady") && apply.contains("stikiLive"),
            "BREAKS IF: sign-out leaves Scratchpad unlocked"
        );
        let sign_out = ts
            .split("stiki-signout")
            .nth(1)
            .expect("stiki-signout");
        let sign_out = sign_out.split("document.querySelectorAll").next().unwrap();
        assert!(
            sign_out.contains("applyStikiFromConnectors") || sign_out.contains("applyProLocks"),
            "BREAKS IF: sign-out leaves Scratchpad unlocked"
        );
        assert!(
            include_str!("pro_features.rs").contains("scratchpad.txt")
                && !include_str!("settings.rs").contains("scratchpad.txt"),
            "BREAKS IF: Scratchpad folded into Dictionary/settings store"
        );
    }

    #[test]
    fn enforcer_bound_breaks_if_free_cloud_nexus_or_web_upgrade() {
        assert_eq!(ENFORCER_SUITE, "b6530197");
        assert!(ENFORCER_BOUND.contains("CONFIRMED Suite b6530197"));
        assert!(ENFORCER_BOUND.contains("StoreKit Pro + Stiki dual gate"));
        assert!(ENFORCER_BOUND.contains("local-only scratchpad of dictations/notes"));
        assert!(ENFORCER_BOUND.contains("Scratchpad NOT MCP source/sink v1"));
        assert!(ENFORCER_BOUND.contains("no cloud/team/Nexus/Mochii auto-push"));
        assert!(ENFORCER_BOUND.contains("no Nexus/Mochii promote"));
        assert!(ENFORCER_BOUND.contains("share fail closed"));
        assert!(ENFORCER_BOUND.contains("sign-out locks Scratchpad"));
        assert!(ENFORCER_BOUND.contains("no HIPAA/BAA"));
        assert!(ENFORCER_BOUND.contains("Free dictate no Stiki"));

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
        let pad_idx = views
            .iter()
            .position(|v| *v == "scratchpad")
            .expect("scratchpad nav");
        let xf_idx = views.iter().position(|v| *v == "transforms").unwrap();
        let style_idx = views.iter().position(|v| *v == "style").unwrap();
        let dict_idx = views.iter().position(|v| *v == "dictionary").unwrap();
        let snip_idx = views.iter().position(|v| *v == "snippets").unwrap();
        let insights_idx = views.iter().position(|v| *v == "insights").unwrap();
        assert_eq!(pad_idx, xf_idx + 1, "BREAKS IF: Scratchpad is not catalog #5");
        assert!(dict_idx < snip_idx && snip_idx < style_idx && style_idx < xf_idx && xf_idx < pad_idx);
        assert_eq!(insights_idx, pad_idx + 1);
        assert!(
            nav.split("data-view=\"scratchpad\"")
                .nth(1)
                .unwrap()
                .contains("lock-pill"),
            "BREAKS IF: Scratchpad nav not Pro-locked"
        );
        let view = html
            .split("data-view=\"scratchpad\"")
            .last()
            .expect("scratchpad view");
        let view = view.split("<section").next().unwrap();
        assert!(view.contains("pro-lock"), "BREAKS IF: Free not locked");
        assert!(view.contains("pro-activate"), "BREAKS IF: no Plans upsell");
        assert!(
            view.contains("Sign in with Stiki"),
            "BREAKS IF: Sign in with Stiki missing on locked Scratchpad"
        );
        assert!(
            view.contains("this Mac") || view.contains("local-only") || view.contains("Local only"),
            "BREAKS IF: Scratchpad not local-only"
        );
        assert!(
            !view.contains("data-style-mode")
                && !view.contains("data-xf-action")
                && !view.contains("id=\"polish-mode-select\""),
            "BREAKS IF: Scratchpad offers Style/Transforms/Polish controls"
        );
        assert!(!view.contains("chibiteklabs.com"), "BREAKS IF: web upgrade");
        assert!(!view.contains("https://"), "BREAKS IF: web upgrade");
        assert!(
            !view.contains("Share with team") && !view.contains("Sync to cloud"),
            "BREAKS IF: share shipped"
        );

        let ts = include_str!("../../src/main.ts");
        assert!(ts.contains("scratchpad_save"));
        assert!(ts.contains("saveScratchpad"));
        assert!(ts.contains("openPlans()"));
        assert!(!ts.contains("https://chibiteklabs"));
        assert!(
            ts.contains("\"scratchpad\""),
            "BREAKS IF: Scratchpad dropped from Pro lock list"
        );

        let commands = include_str!("main.rs");
        assert!(commands.contains("scratchpad_get"));
        assert!(commands.contains("scratchpad_save"));
        assert!(commands.contains("scratchpad_clear"));
        assert!(commands.contains("share_cloud_or_team"));
        assert!(commands.contains("promote_to_company_memory"));
        assert!(include_str!("scratchpad.rs").contains("require_share_acl"));
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
        assert!(include_str!("scratchpad.rs").contains("fail-closed"));

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
        assert!(html.contains("id=\"scratchpad-open\""));
        assert!(html.contains("id=\"scratchpad-activate\""));
        assert!(ts.contains("openScratchpad"));
        let tray = include_str!("clipboard_ui.rs");
        assert!(tray.contains("Scratchpad"));
        assert!(tray.contains("open-scratchpad"));
        assert!(
            include_str!("scratchpad.rs").contains("promote_to_company_memory"),
            "BREAKS IF: auto-promote stub missing"
        );
        assert!(
            include_str!("pro_features.rs").contains("scratchpad.txt"),
            "BREAKS IF: scratchpad store missing"
        );
        assert!(
            !include_str!("settings.rs").contains("scratchpad.txt"),
            "BREAKS IF: scratchpad folded into Dictionary/settings store"
        );
        assert!(
            !include_str!("polish.rs").contains("scratchpad.txt")
                && !include_str!("style.rs").contains("scratchpad.txt")
                && !include_str!("transforms.rs").contains("scratchpad.txt")
                && !include_str!("snippets.rs").contains("scratchpad.txt")
                && !include_str!("clipboard_history.rs").contains("scratchpad.txt")
                && !include_str!("dictionary.rs")
                    .split("#[cfg(test)]")
                    .next()
                    .unwrap()
                    .contains("scratchpad.txt"),
            "BREAKS IF: scratchpad folded into Style, Transforms, Polish, Dictionary, Snippets, or Clipboard stores"
        );
    }

    #[test]
    fn breaks_if_scratchpad_without_dual_gate() {
        assert!(
            require_surface_with(true, false).is_err(),
            "BREAKS IF: Scratchpad without dual gate (Pro, no Stiki)"
        );
        assert!(
            require_surface_with(false, true).is_err(),
            "BREAKS IF: Scratchpad without dual gate (Stiki, no Pro)"
        );
        assert!(
            require_surface_with(false, false).is_err(),
            "BREAKS IF: Scratchpad without dual gate"
        );
        assert!(require_surface_with(true, true).is_ok());
        assert_eq!(
            effective_text_with("notes", true, false),
            "",
            "BREAKS IF: Scratchpad without dual gate"
        );
        assert!(!surface_ready(), "BREAKS IF: Scratchpad without dual gate");
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
        let nexus = format!("{}{}", "nexus", "_write");
        let siem = format!("{}{}", "siem", "_write");
        let pad_prod = include_str!("scratchpad.rs")
            .split("#[cfg(test)]")
            .next()
            .expect("scratchpad prod");
        for hay in [
            include_str!("main.rs"),
            pad_prod,
            include_str!("pro_features.rs"),
            include_str!("settings.rs"),
            include_str!("../../src/main.ts"),
            include_str!("../../index.html"),
        ] {
            assert!(!hay.contains(&nexus), "BREAKS IF: Nexus write");
            assert!(!hay.contains(&siem), "BREAKS IF: Nexus write");
        }
    }

    #[test]
    fn breaks_if_auto_promote() {
        assert!(
            promote_to_company_memory().is_err(),
            "BREAKS IF: auto-promote"
        );
        assert!(
            !include_str!("../../src/main.ts").contains("scratchpad_promote"),
            "BREAKS IF: auto-promote"
        );
    }

    #[test]
    fn breaks_if_mcp_source_or_sink() {
        assert!(
            as_mcp_source("notes").is_err() && as_mcp_sink("notes").is_err(),
            "BREAKS IF: Scratchpad MCP source/sink v1"
        );
        assert!(export_to_captures("notes").is_err());
        let html = include_str!("../../index.html");
        assert!(!html.contains("id=\"scratchpad-export-nexus\""));
        assert!(!html.contains("id=\"scratchpad-export-mochii\""));
        let pad = include_str!("pro_features.rs");
        let read = pad.split("pub fn scratchpad_read").nth(1).unwrap();
        let read = read.split("pub fn scratchpad_write_local").next().unwrap();
        assert!(!read.contains("connectors"), "BREAKS IF: Scratchpad auto-MCP'd");
        let write = pad.split("pub fn scratchpad_write_local").nth(1).unwrap();
        let write = write.split("#[cfg(test)]").next().unwrap();
        assert!(!write.contains("connectors"), "BREAKS IF: Scratchpad auto-MCP'd");
    }

    #[test]
    fn product_lock_v1_non_goals_and_surface() {
        assert!(PRODUCT_LOCK.contains("Name: Scratchpad"));
        assert!(PRODUCT_LOCK.contains("Dictionary → Snippets → Style → Transforms → Scratchpad"));
        assert!(PRODUCT_LOCK.contains("StoreKit Pro AND Stiki session"));
        assert!(PRODUCT_LOCK.contains("Activate Pro / Sign in with Stiki"));
        assert!(PRODUCT_LOCK.contains("local-only scratchpad of dictations/notes"));
        assert!(PRODUCT_LOCK.contains("Settings + sidebar/nav + menu bar"));
        assert!(PRODUCT_LOCK.contains("add/edit/clear notes"));
        assert!(PRODUCT_LOCK.contains("not MCP source/sink v1"));
        assert!(PRODUCT_LOCK.contains("Formal|Casual|Very casual"));
        assert!(PRODUCT_LOCK.contains("Email|Bullet points|Make shorter|Make clearer"));
        assert!(PRODUCT_LOCK.contains("Off|Casual|Professional|Polite"));
        assert!(PRODUCT_LOCK.contains("HIPAA"));

        let html = include_str!("../../index.html");
        let ts = include_str!("../../src/main.ts");
        assert!(html.contains("view-title\">Scratchpad"));
        assert!(html.contains("id=\"scratchpad-activate\""));
        assert!(html.contains("id=\"scratchpad-open\""));
        assert!(html.contains("id=\"scratchpad-clear-btn\""));
        assert!(html.contains("id=\"dictionary-activate\""));
        assert!(html.contains("id=\"snippets-activate\""));
        assert!(html.contains("id=\"style-activate\""));
        assert!(html.contains("id=\"transforms-activate\""));
        assert!(html.contains("id=\"polish-activate\""));
        assert!(ts.contains("openPlans()"));
        assert!(ts.contains("scratchpad_save"));
        assert!(ts.contains("saveScratchpad"));

        for hay in [html, ts] {
            assert!(!hay.contains("HIPAA"), "BREAKS IF: HIPAA claim copy");
        }
    }

    #[test]
    fn distinct_from_style_transforms_polish_dictionary_snippets_and_clipboard() {
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
        assert!(engine.contains("row-label\">Scratchpad"));
        assert!(engine.contains("not Style"));
        assert!(engine.contains("not Transforms"));
        assert!(engine.contains("not Polish"));
        assert!(engine.contains("not Dictionary"));
        assert!(engine.contains("not Snippets"));
        assert!(engine.contains("not Clipboard"));
        assert!(html.contains("id=\"polish-toggle\""));
        assert!(html.contains("id=\"polish-mode-select\""));
        assert!(html.contains("id=\"clipboard-history-toggle\""));
        assert!(html.contains("id=\"dict-add-btn\""));
        assert!(html.contains("id=\"snippet-add-btn\""));
        assert!(html.contains("id=\"style-clear-btn\""));
        assert!(html.contains("id=\"xf-clear-btn\""));
        assert_ne!("scratchpad-clear-btn", "style-clear-btn");
        assert_ne!("scratchpad-clear-btn", "xf-clear-btn");
        assert_ne!("scratchpad-open", "style-open");
        assert_ne!("scratchpad-open", "transforms-open");
        assert_ne!("scratchpad-open", "dictionary-open");
        assert_ne!("scratchpad-open", "snippets-open");
        let pad_view = html.split("data-view=\"scratchpad\"").last().unwrap();
        let pad_view = pad_view.split("<section").next().unwrap();
        assert!(
            !pad_view.contains("data-style-mode")
                && !pad_view.contains("data-xf-action")
                && !pad_view.contains("id=\"polish-mode-select\""),
            "BREAKS IF: Scratchpad offers Style register, Transforms actions, or Polish modes"
        );
        assert!(pad_view.contains("Style") && pad_view.contains("Transforms") && pad_view.contains("Polish"));
        assert!(!include_str!("style.rs").contains("scratchpad.txt"));
        assert!(!include_str!("transforms.rs").contains("scratchpad.txt"));
        assert!(!include_str!("polish.rs").contains("scratchpad.txt"));
    }

    #[test]
    fn insights_stay_local_without_hipaa_copy() {
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
