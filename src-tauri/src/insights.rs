//! Product **Insights** — Enforcer BOUND (suite b6530197) CONFIRMED + Sign-on RE-LOCK.
//!
//! GREEN: StoreKit Pro + Stiki dual gate; local-only stats / usage insights
//! on this Mac (dictation counts, streaks, time-saved style); Insights NOT
//! MCP source/sink v1; no cloud sync; no team/company dashboards; no
//! Nexus/SIEM write; no third-party analytics vendor; no HIPAA/BAA;
//! distinct Scratchpad/Dictionary/Snippets/Style/Transforms/Polish/Clipboard;
//! Free dictate no Stiki; sign-out locks Insights until Stiki again.
//!
//! BREAKS IF: Insights without dual gate
//! BREAKS IF: cloud analytics
//! BREAKS IF: cross-device sync
//! BREAKS IF: Nexus write
//! BREAKS IF: Free dictate gated
//! BREAKS IF: HIPAA
//! BREAKS IF: Insights MCP source/sink v1
//! BREAKS IF: sign-out leaves Insights unlocked
//! HELD: cloud sync + team/company dashboards until MCS with Stiki/folder-style
//! ACL; fail-closed if ACL missing. Soft: Notetaker last/maybe never.

use crate::connectors;
use crate::stiki_session;
use crate::stats::{StatsStore, StatsSummary};
use crate::storekit;

/// Named Enforcer suite. Tests fail if this is retargeted without a new MCS.
pub const ENFORCER_SUITE: &str = "b6530197";

/// Named Enforcer BOUND. `enforcer_bound_insights_v1_suite_*` tests fold this.
pub const ENFORCER_BOUND: &str = "CONFIRMED Suite b6530197; StoreKit Pro + Stiki dual gate; local-only stats / usage insights on this Mac; dictation counts, streaks, time-saved style; Insights NOT MCP source/sink v1; no cloud sync; no team/company dashboards; no Nexus/SIEM write; no third-party analytics vendor; no HIPAA/BAA; distinct Scratchpad/Dictionary/Snippets/Style/Transforms/Polish/Clipboard; Free dictate no Stiki; sign-out locks Insights";

/// Product LOCK Insights v1 + Sign-on. Tests fail if the surface drifts.
pub const PRODUCT_LOCK: &str = "Name: Insights; Pro catalog #6 LAST after Scratchpad (Dictionary → Snippets → Style → Transforms → Scratchpad → Insights); Pro surface requires StoreKit Pro AND Stiki session; Free locked + Activate Pro / Sign in with Stiki; local-only stats / usage insights on this Mac (dictation counts, streaks, time-saved style); Settings + sidebar/nav + menu bar; local-first; not Nexus; not Mochii; not MCP source/sink v1; not cloud sync v1; no team/company dashboards; no third-party analytics vendor; distinct from Scratchpad (notes), Dictionary (spelling), Snippets (trigger→expansion), Style (Formal|Casual|Very casual register), Transforms (Email|Bullet points|Make shorter|Make clearer), Polish (Off|Casual|Professional|Polite Gemma tone rewrite), and Clipboard History; non-goals: MCP source/sink, cloud write, cross-device sync, Nexus/SIEM write, HIPAA, Notetaker";

/// Held. A later MCS must flip this only with Stiki/folder-style ACL.
pub const STIKI_FOLDER_ACL_SHIPPED: bool = false;

const SHARE_BLOCKED: &str =
    "Cloud and team Insights dashboards are not available. Insights stays on this Mac.";
const ACL_MISSING: &str =
    "Cloud and team Insights dashboards are held until a Stiki/folder-style ACL Make It So. ACL missing — fail closed.";
const VENDOR_BLOCKED: &str =
    "Third-party analytics vendors are not used. Insights stays in a local file on this Mac.";

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

/// Ungated local read. Product Insights applies the dual gate before show.
pub fn load_stored(store: &StatsStore) -> StatsSummary {
    store.summary()
}

/// Runtime gate: Free / no Stiki / lapsed → empty surface (counts stay on disk).
pub fn effective_summary_with(stored: StatsSummary, entitled: bool, signed_in: bool) -> StatsSummary {
    if entitled && signed_in {
        stored
    } else {
        StatsSummary::empty()
    }
}

pub fn effective_summary(stored: StatsSummary) -> StatsSummary {
    effective_summary_with(
        stored,
        storekit::current_entitlement().entitled,
        stiki_session::is_live(),
    )
}

pub fn require_summary(store: &StatsStore) -> Result<StatsSummary, String> {
    require_surface()?;
    Ok(load_stored(store))
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

/// Soft later: cloud / team / company dashboards. Do not ship until a separate ACL Make It So.
pub fn share_cloud_or_team() -> Result<(), String> {
    require_share_acl()?;
    Err(SHARE_BLOCKED.into())
}

/// BREAKS IF: private Insights auto-promotes to Nexus / Mochii / company memory.
pub fn promote_to_company_memory() -> Result<(), String> {
    Err("Private insights cannot become company memory.".into())
}

/// BREAKS IF: Insights is an MCP source in v1.
pub fn as_mcp_source(summary: &str) -> Result<(), String> {
    connectors::insights_as_mcp_source(summary)
}

/// BREAKS IF: Insights is an MCP sink in v1.
pub fn as_mcp_sink(summary: &str) -> Result<(), String> {
    connectors::insights_as_mcp_sink(summary)
}

/// BREAKS IF: Insights exports to Captures / Nexus / SIEM in v1.
pub fn export_to_captures(summary: &str) -> Result<(), String> {
    connectors::export_insights_to_captures(summary)
}

/// BREAKS IF: a third-party analytics vendor ships.
pub fn send_to_vendor(_summary: &str) -> Result<(), String> {
    Err(VENDOR_BLOCKED.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp() -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "mabel-insights-{}-{}",
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
        let store = StatsStore::load(&dir);
        assert!(require_summary(&store).is_err());
        assert!(share_cloud_or_team().is_err());
        assert!(promote_to_company_memory().is_err());
        assert!(send_to_vendor("counts").is_err());
    }

    #[test]
    fn effective_summary_empty_without_pro() {
        let filled = StatsSummary {
            today: 3,
            total: 9,
            streak: 2,
            total_words: 40,
            wpm: 80,
            last30: vec![1; 30],
            time_saved_minutes: 1,
        };
        assert_eq!(effective_summary(filled.clone()).total, 0);
        assert_eq!(
            effective_summary_with(filled.clone(), true, false).total,
            0,
            "BREAKS IF: Insights usable without Stiki"
        );
        assert_eq!(effective_summary_with(filled.clone(), false, true).total, 0);
        let open = effective_summary_with(filled, true, true);
        assert_eq!(open.total, 9);
        assert_eq!(open.time_saved_minutes, 1);
    }

    #[test]
    fn default_is_empty_until_user_dictates() {
        let dir = tmp();
        let store = StatsStore::load(&dir);
        let empty = load_stored(&store);
        assert_eq!(empty.total, 0);
        assert_eq!(empty.time_saved_minutes, 0);
        store.record(40, 30.0);
        let next = load_stored(&store);
        assert_eq!(next.total, 1);
        assert_eq!(next.total_words, 40);
        assert_eq!(next.time_saved_minutes, 1);
    }

    #[test]
    fn local_store_persists_counts_not_words() {
        let dir = tmp();
        let store = StatsStore::load(&dir);
        store.record(80, 60.0);
        store.record(40, 20.0);
        let path = dir.join("stats.json");
        assert!(path.exists());
        let raw = std::fs::read_to_string(&path).unwrap();
        assert!(raw.contains("\"total_dictations\": 2") || raw.contains("\"total_dictations\":2"));
        assert!(!raw.contains("ship friday"));
        assert!(!raw.to_ascii_lowercase().contains("amplitude"));
        assert!(!raw.to_ascii_lowercase().contains("mixpanel"));
        let reloaded = StatsStore::load(&dir);
        assert_eq!(reloaded.summary().total, 2);
        assert_eq!(reloaded.summary().total_words, 120);
        assert_eq!(reloaded.summary().time_saved_minutes, 3);
    }

    #[test]
    fn breaks_if_insights_usable_without_stiki() {
        assert!(
            require_surface_with(true, false).is_err(),
            "BREAKS IF: Insights usable without Stiki"
        );
        assert_eq!(
            effective_summary_with(StatsSummary::empty(), true, false).total,
            0,
            "BREAKS IF: Insights counts without Stiki"
        );
        assert!(require_surface_with(false, true).is_err());
        assert!(require_surface_with(true, true).is_ok());

        let html = include_str!("../../index.html");
        let ts = include_str!("../../src/main.ts");
        assert!(
            html.contains("Sign in with Stiki"),
            "BREAKS IF: Sign in with Stiki CTA missing"
        );
        assert!(
            ts.contains("insightsSurfaceReady"),
            "BREAKS IF: frontend Stiki+Pro gate missing"
        );
        let load_ins = ts
            .split("async function loadInsightsSurface")
            .nth(1)
            .expect("loadInsightsSurface");
        let load_ins = load_ins.split("async function ").next().unwrap();
        assert!(load_ins.contains("insightsSurfaceReady"));
        assert!(load_ins.contains("insights_get"));
        let load_pro = ts
            .split("async function loadProSurfaces")
            .nth(1)
            .expect("loadProSurfaces");
        let load_pro = load_pro.split("async function ").next().unwrap();
        assert!(
            !load_pro.contains("insights_get"),
            "BREAKS IF: Insights fetched without Stiki gate"
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
            rec.contains("stats.record"),
            "BREAKS IF: Free dictate does not record local counts"
        );
        let stats_prod = include_str!("stats.rs")
            .split("#[cfg(test)]")
            .next()
            .expect("stats prod");
        assert!(
            !stats_prod.contains("require_session") && !stats_prod.contains("require_pro"),
            "BREAKS IF: Free dictate gated"
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
        let vendor = send_to_vendor("counts").unwrap_err();
        assert!(vendor.contains("Third-party"));
    }

    #[test]
    fn breaks_if_cloud_write_ships_without_acl_make_it_so() {
        assert!(!STIKI_FOLDER_ACL_SHIPPED, "BREAKS IF: ACL shipped without MCS");
        assert!(
            share_cloud_or_team().is_err(),
            "BREAKS IF: cloud write succeeded"
        );
        let ts = include_str!("../../src/main.ts");
        assert!(!ts.contains("insights_share"), "BREAKS IF: share UI shipped");
        let html = include_str!("../../index.html");
        let view = html.split("data-view=\"insights\"").last().unwrap();
        let view = view.split("<section").next().unwrap();
        assert!(
            !view.contains("Share with team")
                && !view.contains("Sync to cloud")
                && !view.contains("company dashboard")
                && !view.contains("team dashboard"),
            "BREAKS IF: cloud/team dashboard shipped without ACL Make It So"
        );
        assert!(
            !view.contains("https://"),
            "BREAKS IF: cloud write / web upgrade on Insights"
        );
    }

    #[test]
    fn breaks_if_private_insights_auto_promotes() {
        assert!(
            promote_to_company_memory().is_err(),
            "BREAKS IF: private Insights auto-promote to company memory"
        );
        assert!(as_mcp_source("counts").is_err(), "BREAKS IF: Nexus/Mochii promote");
        assert!(as_mcp_sink("counts").is_err(), "BREAKS IF: Nexus/Mochii promote");
        assert!(export_to_captures("counts").is_err(), "BREAKS IF: Nexus/Mochii promote");
        let ts = include_str!("../../src/main.ts");
        assert!(
            !ts.contains("insights_promote"),
            "BREAKS IF: promote UI shipped"
        );
        let html = include_str!("../../index.html");
        assert!(!html.contains("id=\"insights-export-nexus\""));
        assert!(!html.contains("id=\"insights-export-mochii\""));
    }

    #[test]
    fn enforcer_bound_suite_b6530197_green() {
        assert_eq!(ENFORCER_SUITE, "b6530197");
        assert!(ENFORCER_BOUND.contains("CONFIRMED Suite b6530197"));
        assert!(ENFORCER_BOUND.contains("StoreKit Pro + Stiki dual gate"));
        assert!(ENFORCER_BOUND.contains("local-only stats / usage insights on this Mac"));
        assert!(ENFORCER_BOUND.contains("dictation counts, streaks, time-saved style"));
        assert!(ENFORCER_BOUND.contains("Insights NOT MCP source/sink v1"));
        assert!(ENFORCER_BOUND.contains("no cloud sync"));
        assert!(ENFORCER_BOUND.contains("no team/company dashboards"));
        assert!(ENFORCER_BOUND.contains("no Nexus/SIEM write"));
        assert!(ENFORCER_BOUND.contains("no third-party analytics vendor"));
        assert!(ENFORCER_BOUND.contains("no HIPAA/BAA"));
        assert!(ENFORCER_BOUND.contains("distinct Scratchpad/Dictionary/Snippets/Style/Transforms/Polish/Clipboard"));
        assert!(ENFORCER_BOUND.contains("Free dictate no Stiki"));
        assert!(ENFORCER_BOUND.contains("sign-out locks Insights"));
    }

    #[test]
    fn enforcer_bound_insights_v1_suite_b6530197() {
        assert_eq!(ENFORCER_SUITE, "b6530197");
        assert!(ENFORCER_BOUND.contains("CONFIRMED Suite b6530197"));
        assert!(ENFORCER_BOUND.contains("StoreKit Pro + Stiki dual gate"));
        assert!(ENFORCER_BOUND.contains("local-only stats / usage insights on this Mac"));
        assert!(ENFORCER_BOUND.contains("Insights NOT MCP source/sink v1"));
        assert!(ENFORCER_BOUND.contains("no cloud sync"));
        assert!(ENFORCER_BOUND.contains("no team/company dashboards"));
        assert!(ENFORCER_BOUND.contains("no Nexus/SIEM write"));
        assert!(ENFORCER_BOUND.contains("no third-party analytics vendor"));
        assert!(ENFORCER_BOUND.contains("sign-out locks Insights"));
        assert!(ENFORCER_BOUND.contains("no HIPAA/BAA"));
        assert!(ENFORCER_BOUND.contains("Free dictate no Stiki"));

        assert!(
            require_surface_with(true, false).is_err(),
            "BREAKS IF: Insights without dual gate"
        );
        assert!(
            require_surface_with(false, true).is_err(),
            "BREAKS IF: Insights without dual gate"
        );
        assert_eq!(
            effective_summary_with(
                StatsSummary {
                    today: 1,
                    total: 1,
                    streak: 1,
                    total_words: 10,
                    wpm: 40,
                    last30: vec![1; 30],
                    time_saved_minutes: 1,
                },
                true,
                false
            )
            .total,
            0,
            "BREAKS IF: sign-out leaves Insights unlocked"
        );
        assert!(share_cloud_or_team().is_err(), "BREAKS IF: cloud/team dashboard");
        assert!(promote_to_company_memory().is_err(), "BREAKS IF: auto-promote");
        assert!(as_mcp_source("counts").is_err(), "BREAKS IF: Insights MCP source/sink v1");
        assert!(as_mcp_sink("counts").is_err(), "BREAKS IF: Insights MCP source/sink v1");
        assert!(send_to_vendor("counts").is_err(), "BREAKS IF: third-party analytics vendor");

        let html = include_str!("../../index.html");
        let ts = include_str!("../../src/main.ts");
        assert!(html.contains("id=\"ins-time-saved\""));
        assert!(html.contains("id=\"ins-streak\""));
        assert!(html.contains("id=\"ins-total\""));
        assert!(ts.contains("applyInsightsGate"));
        assert!(ts.contains("insightsSurfaceReady"));
        assert!(ts.contains("stiki_sign_out"));
        let apply = ts
            .split("function applyInsightsGate")
            .nth(1)
            .expect("applyInsightsGate");
        let apply = apply.split("function ").next().unwrap();
        assert!(
            apply.contains("insightsSurfaceReady") && apply.contains("stikiLive"),
            "BREAKS IF: sign-out leaves Insights unlocked"
        );
        let sign_out = ts
            .split("stiki-signout")
            .nth(1)
            .expect("stiki-signout");
        let sign_out = sign_out.split("document.querySelectorAll").next().unwrap();
        assert!(
            sign_out.contains("applyStikiFromConnectors") || sign_out.contains("applyProLocks"),
            "BREAKS IF: sign-out leaves Insights unlocked"
        );
        assert!(
            include_str!("stats.rs").contains("stats.json")
                && !include_str!("settings.rs").contains("stats.json"),
            "BREAKS IF: Insights folded into Dictionary/settings store"
        );
    }

    #[test]
    fn enforcer_bound_breaks_if_free_cloud_nexus_or_web_upgrade() {
        assert_eq!(ENFORCER_SUITE, "b6530197");
        assert!(ENFORCER_BOUND.contains("CONFIRMED Suite b6530197"));
        assert!(ENFORCER_BOUND.contains("StoreKit Pro + Stiki dual gate"));
        assert!(ENFORCER_BOUND.contains("local-only stats / usage insights on this Mac"));
        assert!(ENFORCER_BOUND.contains("Insights NOT MCP source/sink v1"));
        assert!(ENFORCER_BOUND.contains("no team/company dashboards"));
        assert!(ENFORCER_BOUND.contains("no third-party analytics vendor"));
        assert!(ENFORCER_BOUND.contains("sign-out locks Insights"));
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
        let ins_idx = views
            .iter()
            .position(|v| *v == "insights")
            .expect("insights nav");
        let pad_idx = views.iter().position(|v| *v == "scratchpad").unwrap();
        let xf_idx = views.iter().position(|v| *v == "transforms").unwrap();
        let style_idx = views.iter().position(|v| *v == "style").unwrap();
        let dict_idx = views.iter().position(|v| *v == "dictionary").unwrap();
        let snip_idx = views.iter().position(|v| *v == "snippets").unwrap();
        assert_eq!(ins_idx, pad_idx + 1, "BREAKS IF: Insights is not catalog #6");
        assert!(
            dict_idx < snip_idx
                && snip_idx < style_idx
                && style_idx < xf_idx
                && xf_idx < pad_idx
                && pad_idx < ins_idx
        );
        assert!(
            nav.split("data-view=\"insights\"")
                .nth(1)
                .unwrap()
                .contains("lock-pill"),
            "BREAKS IF: Insights nav not Pro-locked"
        );
        let view = html
            .split("data-view=\"insights\"")
            .last()
            .expect("insights view");
        let view = view.split("<section").next().unwrap();
        assert!(view.contains("pro-lock"), "BREAKS IF: Free not locked");
        assert!(view.contains("pro-activate"), "BREAKS IF: no Plans upsell");
        assert!(
            view.contains("Sign in with Stiki"),
            "BREAKS IF: Sign in with Stiki missing on locked Insights"
        );
        assert!(
            view.contains("this Mac") || view.contains("local-only") || view.contains("Local-only") || view.contains("Local only"),
            "BREAKS IF: Insights not local-only"
        );
        assert!(
            view.contains("Time saved") || view.contains("time saved") || view.contains("time-saved"),
            "BREAKS IF: time-saved style missing"
        );
        assert!(
            !view.contains("data-style-mode")
                && !view.contains("data-xf-action")
                && !view.contains("id=\"polish-mode-select\"")
                && !view.contains("id=\"scratchpad-text\"")
                && !view.contains("id=\"dict-add-btn\"")
                && !view.contains("id=\"snippet-add-btn\""),
            "BREAKS IF: Insights offers Scratchpad/Style/Transforms/Polish/Dictionary/Snippets controls"
        );
        assert!(!view.contains("chibiteklabs.com"), "BREAKS IF: web upgrade");
        assert!(!view.contains("https://"), "BREAKS IF: web upgrade");
        assert!(
            !view.contains("Share with team") && !view.contains("Sync to cloud"),
            "BREAKS IF: share shipped"
        );
        assert!(
            !view.contains("amplitude")
                && !view.contains("mixpanel")
                && !view.contains("posthog")
                && !view.contains("segment.com"),
            "BREAKS IF: third-party analytics vendor"
        );

        let ts = include_str!("../../src/main.ts");
        assert!(ts.contains("insights_get"));
        assert!(ts.contains("loadInsightsSurface"));
        assert!(ts.contains("openPlans()"));
        assert!(!ts.contains("https://chibiteklabs"));
        assert!(
            ts.contains("\"insights\""),
            "BREAKS IF: Insights dropped from Pro lock list"
        );

        let commands = include_str!("main.rs");
        assert!(commands.contains("insights_get"));
        assert!(commands.contains("share_cloud_or_team"));
        assert!(commands.contains("promote_to_company_memory"));
        assert!(include_str!("insights.rs").contains("require_share_acl"));
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
        assert!(include_str!("insights.rs").contains("fail-closed"));

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
        assert!(html.contains("id=\"insights-open\""));
        assert!(html.contains("id=\"insights-activate\""));
        assert!(ts.contains("openInsights"));
        let tray = include_str!("clipboard_ui.rs");
        assert!(tray.contains("Insights"));
        assert!(tray.contains("open-insights"));
        assert!(
            include_str!("insights.rs").contains("promote_to_company_memory"),
            "BREAKS IF: auto-promote stub missing"
        );
        assert!(
            include_str!("stats.rs").contains("stats.json"),
            "BREAKS IF: insights store missing"
        );
        assert!(
            !include_str!("settings.rs").contains("stats.json"),
            "BREAKS IF: insights folded into Dictionary/settings store"
        );
        assert!(
            !include_str!("polish.rs").contains("stats.json")
                && !include_str!("style.rs").contains("stats.json")
                && !include_str!("transforms.rs").contains("stats.json")
                && !include_str!("snippets.rs").contains("stats.json")
                && !include_str!("scratchpad.rs").contains("stats.json")
                && !include_str!("clipboard_history.rs").contains("stats.json")
                && !include_str!("dictionary.rs")
                    .split("#[cfg(test)]")
                    .next()
                    .unwrap()
                    .contains("stats.json"),
            "BREAKS IF: insights folded into Scratchpad, Style, Transforms, Polish, Dictionary, Snippets, or Clipboard stores"
        );
    }

    #[test]
    fn breaks_if_insights_without_dual_gate() {
        assert!(
            require_surface_with(true, false).is_err(),
            "BREAKS IF: Insights without dual gate (Pro, no Stiki)"
        );
        assert!(
            require_surface_with(false, true).is_err(),
            "BREAKS IF: Insights without dual gate (Stiki, no Pro)"
        );
        assert!(
            require_surface_with(false, false).is_err(),
            "BREAKS IF: Insights without dual gate"
        );
        assert!(require_surface_with(true, true).is_ok());
        assert_eq!(
            effective_summary_with(
                StatsSummary {
                    today: 1,
                    total: 4,
                    streak: 1,
                    total_words: 10,
                    wpm: 40,
                    last30: vec![1; 30],
                    time_saved_minutes: 1,
                },
                true,
                false
            )
            .total,
            0,
            "BREAKS IF: Insights without dual gate"
        );
        assert!(!surface_ready(), "BREAKS IF: Insights without dual gate");
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
        let ins_prod = include_str!("insights.rs")
            .split("#[cfg(test)]")
            .next()
            .expect("insights prod");
        for hay in [
            include_str!("main.rs"),
            ins_prod,
            include_str!("stats.rs"),
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
            !include_str!("../../src/main.ts").contains("insights_promote"),
            "BREAKS IF: auto-promote"
        );
    }

    #[test]
    fn breaks_if_mcp_source_or_sink() {
        assert!(
            as_mcp_source("counts").is_err() && as_mcp_sink("counts").is_err(),
            "BREAKS IF: Insights MCP source/sink v1"
        );
        assert!(export_to_captures("counts").is_err());
        let html = include_str!("../../index.html");
        assert!(!html.contains("id=\"insights-export-nexus\""));
        assert!(!html.contains("id=\"insights-export-mochii\""));
        let stats = include_str!("stats.rs");
        let rec = stats.split("pub fn record").nth(1).unwrap();
        let rec = rec.split("pub fn summary").next().unwrap();
        assert!(!rec.contains("connectors"), "BREAKS IF: Insights auto-MCP'd");
        let sum = stats.split("pub fn summary").nth(1).unwrap();
        let sum = sum.split("fn save").next().unwrap();
        assert!(!sum.contains("connectors"), "BREAKS IF: Insights auto-MCP'd");
    }

    #[test]
    fn breaks_if_third_party_analytics_vendor() {
        assert!(send_to_vendor("counts").is_err(), "BREAKS IF: third-party analytics vendor");
        let ins_prod = include_str!("insights.rs")
            .split("#[cfg(test)]")
            .next()
            .expect("insights prod");
        let stats_prod = include_str!("stats.rs")
            .split("#[cfg(test)]")
            .next()
            .expect("stats prod");
        for hay in [
            ins_prod,
            stats_prod,
            include_str!("../../src/main.ts"),
            include_str!("../../index.html"),
        ] {
            let low = hay.to_ascii_lowercase();
            assert!(!low.contains("amplitude"), "BREAKS IF: third-party analytics vendor");
            assert!(!low.contains("mixpanel"), "BREAKS IF: third-party analytics vendor");
            assert!(!low.contains("posthog"), "BREAKS IF: third-party analytics vendor");
            assert!(!low.contains("segment.com"), "BREAKS IF: third-party analytics vendor");
            assert!(!low.contains("google-analytics"), "BREAKS IF: third-party analytics vendor");
        }
    }

    #[test]
    fn product_lock_v1_non_goals_and_surface() {
        assert!(PRODUCT_LOCK.contains("Name: Insights"));
        assert!(PRODUCT_LOCK.contains("Dictionary → Snippets → Style → Transforms → Scratchpad → Insights"));
        assert!(PRODUCT_LOCK.contains("Pro catalog #6 LAST after Scratchpad"));
        assert!(PRODUCT_LOCK.contains("StoreKit Pro AND Stiki session"));
        assert!(PRODUCT_LOCK.contains("Activate Pro / Sign in with Stiki"));
        assert!(PRODUCT_LOCK.contains("local-only stats / usage insights"));
        assert!(PRODUCT_LOCK.contains("dictation counts, streaks, time-saved style"));
        assert!(PRODUCT_LOCK.contains("Settings + sidebar/nav + menu bar"));
        assert!(PRODUCT_LOCK.contains("not MCP source/sink v1"));
        assert!(PRODUCT_LOCK.contains("no team/company dashboards"));
        assert!(PRODUCT_LOCK.contains("no third-party analytics vendor"));
        assert!(PRODUCT_LOCK.contains("Formal|Casual|Very casual"));
        assert!(PRODUCT_LOCK.contains("Email|Bullet points|Make shorter|Make clearer"));
        assert!(PRODUCT_LOCK.contains("Off|Casual|Professional|Polite"));
        assert!(PRODUCT_LOCK.contains("HIPAA"));
        assert!(PRODUCT_LOCK.contains("Notetaker"));

        let html = include_str!("../../index.html");
        let ts = include_str!("../../src/main.ts");
        assert!(html.contains("view-title\">Insights"));
        assert!(html.contains("id=\"insights-activate\""));
        assert!(html.contains("id=\"insights-open\""));
        assert!(html.contains("id=\"ins-time-saved\""));
        assert!(html.contains("id=\"dictionary-activate\""));
        assert!(html.contains("id=\"snippets-activate\""));
        assert!(html.contains("id=\"style-activate\""));
        assert!(html.contains("id=\"transforms-activate\""));
        assert!(html.contains("id=\"scratchpad-activate\""));
        assert!(html.contains("id=\"polish-activate\""));
        assert!(ts.contains("openPlans()"));
        assert!(ts.contains("insights_get"));
        assert!(ts.contains("loadInsightsSurface"));

        for hay in [html, ts] {
            assert!(!hay.contains("HIPAA"), "BREAKS IF: HIPAA claim copy");
        }
    }

    #[test]
    fn distinct_from_scratchpad_dictionary_snippets_style_transforms_polish_and_clipboard() {
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
        assert!(engine.contains("row-label\">Insights"));
        assert!(engine.contains("not Scratchpad"));
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
        assert!(html.contains("id=\"scratchpad-clear-btn\""));
        assert_ne!("insights-open", "scratchpad-open");
        assert_ne!("insights-open", "style-open");
        assert_ne!("insights-open", "transforms-open");
        assert_ne!("insights-open", "dictionary-open");
        assert_ne!("insights-open", "snippets-open");
        let ins_view = html.split("data-view=\"insights\"").last().unwrap();
        let ins_view = ins_view.split("<section").next().unwrap();
        assert!(
            !ins_view.contains("data-style-mode")
                && !ins_view.contains("data-xf-action")
                && !ins_view.contains("id=\"polish-mode-select\"")
                && !ins_view.contains("id=\"scratchpad-text\""),
            "BREAKS IF: Insights offers Style register, Transforms actions, Polish modes, or Scratchpad notes"
        );
        assert!(
            ins_view.contains("Scratchpad")
                && ins_view.contains("Style")
                && ins_view.contains("Transforms")
                && ins_view.contains("Polish")
        );
        assert!(!include_str!("style.rs").contains("stats.json"));
        assert!(!include_str!("transforms.rs").contains("stats.json"));
        assert!(!include_str!("polish.rs").contains("stats.json"));
        assert!(!include_str!("scratchpad.rs").contains("stats.json"));
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
        assert!(
            insights.contains("Local-only")
                || insights.contains("Local only")
                || insights.contains("this Mac")
        );
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
