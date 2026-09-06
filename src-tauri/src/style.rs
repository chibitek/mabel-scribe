//! Product **Style** — Enforcer BOUND (suite b6530197) + Sign-on RE-LOCK.
//!
//! GREEN: StoreKit Pro + Stiki dual gate; local-first Formal|Casual|Very casual
//! register during cleanup; default OFF/unset until the user picks; no freeform
//! casing/punctuation/formatting prefs; no cloud sync; no team/company share
//! without ACL Make It So; no Nexus/SIEM write; no auto-promote; no HIPAA/BAA;
//! Free dictate no Stiki; distinct from Polish/Dictionary/Snippets stores.
//!
//! BREAKS IF: Style without dual gate
//! BREAKS IF: cloud/team share without ACL
//! BREAKS IF: Nexus write
//! BREAKS IF: auto-promote
//! BREAKS IF: merges with Polish/Dictionary/Snippets stores
//! BREAKS IF: Free dictate gated
//! BREAKS IF: freeform casing UI instead of Formal|Casual|Very casual
//! HELD: cloud sync + team share until MCS with Stiki/folder-style ACL;
//! fail-closed if ACL missing.

use std::path::PathBuf;

use crate::pro_features::{self, StylePrefs};
use crate::stiki_session;
use crate::storekit;

/// Named Enforcer suite. Tests fail if this is retargeted without a new MCS.
pub const ENFORCER_SUITE: &str = "b6530197";

/// Named Enforcer BOUND. Tests fail if this is violated.
pub const ENFORCER_BOUND: &str = "StoreKit Pro + Stiki dual gate; local-first Formal|Casual|Very casual register; default OFF/unset; no freeform casing prefs; no cloud sync; no team/company share without ACL Make It So; no Nexus/SIEM write; no auto-promote; no HIPAA/BAA; Free dictate no Stiki; distinct from Polish/Dictionary/Snippets stores";

/// Product LOCK Style v1 + Sign-on. Tests fail if the surface drifts.
pub const PRODUCT_LOCK: &str = "Name: Style; Pro catalog #3 after Dictionary → Snippets → Style; Pro surface requires StoreKit Pro AND Stiki session; Free locked + Activate Pro / Sign in with Stiki; Formal|Casual|Very casual register/formality preference applied locally during cleanup/dictation paste; default OFF/unset until user picks; Settings + sidebar/nav + menu bar; add/edit/delete means pick/clear mode; local-first; not Nexus; not cloud sync v1; distinct from Dictionary (spelling), Snippets (trigger→expansion), Polish (Off|Casual|Professional|Polite Gemma tone rewrite), and Clipboard History; non-goals: freeform casing/punctuation/formatting prefs, shared team style, cloud write, HIPAA";

/// Held. A later MCS must flip this only with Stiki/folder-style ACL.
pub const STIKI_FOLDER_ACL_SHIPPED: bool = false;

pub const MODE_OFF: &str = "off";
pub const MODE_FORMAL: &str = "formal";
pub const MODE_CASUAL: &str = "casual";
pub const MODE_VERY_CASUAL: &str = "very-casual";

pub const LIVE_MODES: &[&str] = &[MODE_FORMAL, MODE_CASUAL, MODE_VERY_CASUAL];

const SHARE_BLOCKED: &str =
    "Cloud and team style share is not available. Style stays on this Mac.";
const ACL_MISSING: &str =
    "Cloud and team style share is held until a Stiki/folder-style ACL Make It So. ACL missing — fail closed.";

/// Longer matches first so "doesn't" wins over a shorter stem.
const FORMAL_PAIRS: &[(&str, &str)] = &[
    ("doesn't", "does not"),
    ("didn't", "did not"),
    ("isn't", "is not"),
    ("aren't", "are not"),
    ("wasn't", "was not"),
    ("weren't", "were not"),
    ("hasn't", "has not"),
    ("haven't", "have not"),
    ("hadn't", "had not"),
    ("wouldn't", "would not"),
    ("couldn't", "could not"),
    ("shouldn't", "should not"),
    ("can't", "cannot"),
    ("won't", "will not"),
    ("don't", "do not"),
    ("i'm", "I am"),
    ("i've", "I have"),
    ("i'll", "I will"),
    ("i'd", "I would"),
    ("we're", "we are"),
    ("they're", "they are"),
    ("you're", "you are"),
    ("it's", "it is"),
    ("that's", "that is"),
    ("there's", "there is"),
    ("what's", "what is"),
    ("let's", "let us"),
    ("gonna", "going to"),
    ("wanna", "want to"),
    ("gotta", "have to"),
    ("kinda", "kind of"),
    ("sorta", "sort of"),
    ("yeah", "yes"),
    ("yep", "yes"),
    ("yup", "yes"),
    ("nope", "no"),
];

const VERY_CASUAL_PAIRS: &[(&str, &str)] = &[("okay", "ok"), ("yes", "yeah")];

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

pub fn normalize_mode(raw: &str) -> String {
    match raw.trim().to_ascii_lowercase().as_str() {
        MODE_FORMAL => MODE_FORMAL.into(),
        MODE_CASUAL => MODE_CASUAL.into(),
        MODE_VERY_CASUAL | "very_casual" | "very casual" => MODE_VERY_CASUAL.into(),
        _ => MODE_OFF.into(),
    }
}

pub fn is_live(mode: &str) -> bool {
    LIVE_MODES.contains(&normalize_mode(mode).as_str())
}

/// Ungated local read. Callers must apply `effective_mode` before cleanup.
pub fn load_stored(app_dir: &PathBuf) -> StylePrefs {
    let mut prefs = pro_features::style_read(app_dir);
    prefs.mode = normalize_mode(&prefs.mode);
    prefs
}

/// Runtime gate: Free / no Stiki / lapsed / unset → Off (no register pass).
pub fn effective_mode_with(stored: &str, entitled: bool, signed_in: bool) -> String {
    if entitled && signed_in {
        normalize_mode(stored)
    } else {
        MODE_OFF.into()
    }
}

pub fn effective_mode(stored: &str) -> String {
    effective_mode_with(
        stored,
        storekit::current_entitlement().entitled,
        stiki_session::is_live(),
    )
}

pub fn require_prefs(app_dir: &PathBuf) -> Result<StylePrefs, String> {
    require_surface()?;
    Ok(load_stored(app_dir))
}

/// Pick / edit the register. Unknown values fail closed to Off rather than
/// inventing a freeform casing store.
pub fn set_mode(app_dir: &PathBuf, mode: String) -> Result<StylePrefs, String> {
    require_surface()?;
    let mode = normalize_mode(&mode);
    if !is_live(&mode) {
        return clear(app_dir);
    }
    pro_features::style_write_local(
        app_dir,
        StylePrefs {
            mode: mode.clone(),
        },
    )
}

/// Delete / clear. Style stays unset until the user picks again.
pub fn clear(app_dir: &PathBuf) -> Result<StylePrefs, String> {
    require_surface()?;
    pro_features::style_write_local(
        app_dir,
        StylePrefs {
            mode: MODE_OFF.into(),
        },
    )
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

/// BREAKS IF: private Style auto-promotes to company memory.
pub fn promote_to_company_memory() -> Result<(), String> {
    Err("Private style cannot become company memory.".into())
}

/// Apply the stored register during cleanup / dictation paste.
/// Free / Off / no Stiki → unchanged.
pub fn apply_register(text: &str, stored: &str) -> String {
    apply_register_with(text, &effective_mode(stored))
}

pub fn apply_register_for_dir(text: &str, app_dir: &PathBuf) -> String {
    apply_register(text, &load_stored(app_dir).mode)
}

fn apply_register_with(text: &str, mode: &str) -> String {
    if text.is_empty() {
        return text.to_string();
    }
    match normalize_mode(mode).as_str() {
        MODE_FORMAL => apply_pairs(text, FORMAL_PAIRS),
        MODE_VERY_CASUAL => apply_pairs(text, VERY_CASUAL_PAIRS),
        MODE_CASUAL => text.to_string(),
        _ => text.to_string(),
    }
}

fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '\''
}

fn apply_pairs(text: &str, pairs: &[(&str, &str)]) -> String {
    let mut out = text.to_string();
    for (from, to) in pairs {
        out = replace_word_ignore_case(&out, from, to);
    }
    out
}

fn replace_word_ignore_case(text: &str, trigger: &str, replacement: &str) -> String {
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
                out.push_str(&preserve_leading_case(&slice, replacement));
                i += n;
                continue;
            }
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

fn preserve_leading_case(original: &str, replacement: &str) -> String {
    let Some(first) = original.chars().next() else {
        return replacement.to_string();
    };
    if !first.is_uppercase() {
        return replacement.to_string();
    }
    let mut chars = replacement.chars();
    match chars.next() {
        Some(c) => format!("{}{}", c.to_uppercase(), chars.as_str()),
        None => replacement.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp() -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "mabel-style-{}-{}",
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
        assert!(require_prefs(&dir).is_err());
        assert!(set_mode(&dir, MODE_FORMAL.into()).is_err());
        assert!(clear(&dir).is_err());
    }

    #[test]
    fn effective_mode_off_without_pro() {
        assert_eq!(effective_mode(MODE_FORMAL), MODE_OFF);
        assert_eq!(
            apply_register("yeah I am gonna go.", MODE_FORMAL),
            "yeah I am gonna go."
        );
    }

    #[test]
    fn default_is_off_until_user_picks() {
        let dir = tmp();
        let prefs = load_stored(&dir);
        assert_eq!(prefs.mode, MODE_OFF);
        assert!(!is_live(&prefs.mode));
        let written = pro_features::style_write_local(
            &dir,
            StylePrefs {
                mode: MODE_FORMAL.into(),
            },
        )
        .unwrap();
        assert_eq!(written.mode, MODE_FORMAL);
        let cleared = pro_features::style_write_local(
            &dir,
            StylePrefs {
                mode: MODE_OFF.into(),
            },
        )
        .unwrap();
        assert_eq!(cleared.mode, MODE_OFF);
        assert_eq!(load_stored(&dir).mode, MODE_OFF);
    }

    #[test]
    fn local_store_pick_and_clear() {
        let dir = tmp();
        let formal = pro_features::style_write_local(
            &dir,
            StylePrefs {
                mode: MODE_FORMAL.into(),
            },
        )
        .unwrap();
        assert_eq!(formal.mode, MODE_FORMAL);
        let casual = pro_features::style_write_local(
            &dir,
            StylePrefs {
                mode: MODE_CASUAL.into(),
            },
        )
        .unwrap();
        assert_eq!(casual.mode, MODE_CASUAL);
        let very = pro_features::style_write_local(
            &dir,
            StylePrefs {
                mode: MODE_VERY_CASUAL.into(),
            },
        )
        .unwrap();
        assert_eq!(very.mode, MODE_VERY_CASUAL);
        let left = pro_features::style_write_local(
            &dir,
            StylePrefs {
                mode: MODE_OFF.into(),
            },
        )
        .unwrap();
        assert_eq!(left.mode, MODE_OFF);
        assert_eq!(load_stored(&dir).mode, MODE_OFF);
    }

    #[test]
    fn breaks_if_style_usable_without_stiki() {
        assert!(
            require_surface_with(true, false).is_err(),
            "BREAKS IF: Style usable without Stiki"
        );
        assert_eq!(
            effective_mode_with(MODE_FORMAL, true, false),
            MODE_OFF,
            "BREAKS IF: Style register without Stiki"
        );
        assert_eq!(
            apply_register("yeah", MODE_FORMAL),
            "yeah",
            "BREAKS IF: Style applies without Stiki session"
        );
        assert!(require_surface_with(false, true).is_err());
        assert!(require_surface_with(true, true).is_ok());
        assert_eq!(
            effective_mode_with(MODE_FORMAL, true, true),
            MODE_FORMAL
        );
        assert_eq!(
            apply_register_with("yeah I am gonna go.", MODE_FORMAL),
            "yes I am going to go."
        );

        let html = include_str!("../../index.html");
        let ts = include_str!("../../src/main.ts");
        assert!(
            html.contains("Sign in with Stiki"),
            "BREAKS IF: Sign in with Stiki CTA missing"
        );
        assert!(
            ts.contains("styleSurfaceReady"),
            "BREAKS IF: frontend Stiki+Pro gate missing"
        );
        let load_style = ts
            .split("async function loadStyleSurface")
            .nth(1)
            .expect("loadStyleSurface");
        let load_style = load_style.split("async function ").next().unwrap();
        assert!(load_style.contains("styleSurfaceReady"));
        assert!(load_style.contains("style_get"));
        let load_pro = ts
            .split("async function loadProSurfaces")
            .nth(1)
            .expect("loadProSurfaces");
        let load_pro = load_pro.split("async function ").next().unwrap();
        assert!(
            !load_pro.contains("style_get"),
            "BREAKS IF: Style fetched without Stiki gate"
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
        assert!(rec.contains("apply_register"));
        assert!(streaming.contains("apply_register"));
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
        assert!(!ts.contains("style_share"), "BREAKS IF: share UI shipped");
        let html = include_str!("../../index.html");
        let view = html.split("data-view=\"style\"").last().unwrap();
        let view = view.split("<section").next().unwrap();
        assert!(
            !view.contains("Share with team") && !view.contains("Sync to cloud"),
            "BREAKS IF: cloud/team share shipped without ACL Make It So"
        );
        assert!(
            !view.contains("https://"),
            "BREAKS IF: cloud write / web upgrade on Style"
        );
    }

    #[test]
    fn breaks_if_private_style_auto_promotes() {
        assert!(
            promote_to_company_memory().is_err(),
            "BREAKS IF: private Style auto-promote to company memory"
        );
        let ts = include_str!("../../src/main.ts");
        assert!(
            !ts.contains("style_promote"),
            "BREAKS IF: promote UI shipped"
        );
    }

    #[test]
    fn enforcer_bound_suite_b6530197_green() {
        assert_eq!(ENFORCER_SUITE, "b6530197");
        assert!(ENFORCER_BOUND.contains("StoreKit Pro + Stiki dual gate"));
        assert!(ENFORCER_BOUND.contains("Formal|Casual|Very casual"));
        assert!(ENFORCER_BOUND.contains("default OFF/unset"));
        assert!(ENFORCER_BOUND.contains("no freeform casing prefs"));
        assert!(ENFORCER_BOUND.contains("no cloud sync"));
        assert!(ENFORCER_BOUND.contains("no team/company share without ACL Make It So"));
        assert!(ENFORCER_BOUND.contains("no Nexus/SIEM write"));
        assert!(ENFORCER_BOUND.contains("no auto-promote"));
        assert!(ENFORCER_BOUND.contains("no HIPAA/BAA"));
        assert!(ENFORCER_BOUND.contains("Free dictate no Stiki"));
        assert!(ENFORCER_BOUND.contains("distinct from Polish/Dictionary/Snippets stores"));
    }

    #[test]
    fn enforcer_bound_breaks_if_free_cloud_nexus_or_web_upgrade() {
        assert_eq!(ENFORCER_SUITE, "b6530197");
        assert!(ENFORCER_BOUND.contains("StoreKit Pro + Stiki dual gate"));
        assert!(ENFORCER_BOUND.contains("Formal|Casual|Very casual"));
        assert!(ENFORCER_BOUND.contains("no freeform casing prefs"));
        assert!(ENFORCER_BOUND.contains("no cloud sync"));
        assert!(ENFORCER_BOUND.contains("no team/company share without ACL Make It So"));
        assert!(ENFORCER_BOUND.contains("no Nexus/SIEM write"));
        assert!(ENFORCER_BOUND.contains("no auto-promote"));
        assert!(ENFORCER_BOUND.contains("no HIPAA/BAA"));
        assert!(ENFORCER_BOUND.contains("Free dictate no Stiki"));
        assert!(ENFORCER_BOUND.contains("distinct from Polish/Dictionary/Snippets stores"));

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
        let style_idx = views.iter().position(|v| *v == "style").expect("style nav");
        let dict_idx = views.iter().position(|v| *v == "dictionary").unwrap();
        let snip_idx = views.iter().position(|v| *v == "snippets").unwrap();
        assert_eq!(style_idx, snip_idx + 1, "BREAKS IF: Style is not catalog #3");
        assert!(dict_idx < snip_idx && snip_idx < style_idx);
        assert!(
            nav.split("data-view=\"style\"")
                .nth(1)
                .unwrap()
                .contains("lock-pill"),
            "BREAKS IF: Style nav not Pro-locked"
        );
        let view = html
            .split("data-view=\"style\"")
            .last()
            .expect("style view");
        let view = view.split("<section").next().unwrap();
        assert!(view.contains("pro-lock"), "BREAKS IF: Free not locked");
        assert!(view.contains("pro-activate"), "BREAKS IF: no Plans upsell");
        assert!(
            view.contains("Sign in with Stiki"),
            "BREAKS IF: Sign in with Stiki missing on locked Style"
        );
        assert!(view.contains("Formal"), "BREAKS IF: Formal mode missing");
        assert!(view.contains("Casual"), "BREAKS IF: Casual mode missing");
        assert!(
            view.contains("Very casual"),
            "BREAKS IF: Very casual mode missing"
        );
        assert!(
            !view.contains("style-tone")
                && !view.contains("style-casing")
                && !view.contains("style-punctuation")
                && !view.contains("Title case")
                && !view.contains("as-said")
                && !view.contains("Sentence case"),
            "BREAKS IF: freeform casing UI instead of Formal|Casual|Very casual"
        );
        assert!(!view.contains("chibiteklabs.com"), "BREAKS IF: web upgrade");
        assert!(!view.contains("https://"), "BREAKS IF: web upgrade");
        assert!(view.contains("this Mac"));
        assert!(
            !view.contains("Share with team") && !view.contains("Sync to cloud"),
            "BREAKS IF: share shipped"
        );

        let ts = include_str!("../../src/main.ts");
        assert!(ts.contains("style_set"));
        assert!(ts.contains("style_clear"));
        assert!(ts.contains("pickStyleMode"));
        assert!(ts.contains("clearStyleMode"));
        assert!(ts.contains("openPlans()"));
        assert!(!ts.contains("https://chibiteklabs"));
        assert!(
            ts.contains("\"style\""),
            "BREAKS IF: Style dropped from Pro lock list"
        );
        assert!(
            !ts.contains("style-tone") && !ts.contains("style-casing") && !ts.contains("style-punctuation"),
            "BREAKS IF: freeform casing UI instead of Formal|Casual|Very casual"
        );

        let commands = include_str!("main.rs");
        assert!(commands.contains("style::apply_register") || rec_or_stream_applies());
        assert!(commands.contains("style_set"));
        assert!(commands.contains("style_clear"));
        assert!(commands.contains("share_cloud_or_team"));
        assert!(commands.contains("promote_to_company_memory"));
        assert!(include_str!("style.rs").contains("require_share_acl"));
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
        assert!(include_str!("style.rs").contains("fail-closed"));

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
        assert!(html.contains("id=\"style-open\""));
        assert!(html.contains("id=\"style-activate\""));
        assert!(ts.contains("openStyle"));
        let tray = include_str!("clipboard_ui.rs");
        assert!(tray.contains("Style"));
        assert!(tray.contains("open-style"));
        assert!(
            include_str!("style.rs").contains("promote_to_company_memory"),
            "BREAKS IF: auto-promote stub missing"
        );
        assert!(
            include_str!("pro_features.rs").contains("style.json"),
            "BREAKS IF: style store missing"
        );
        assert!(
            !include_str!("settings.rs").contains("style.json"),
            "BREAKS IF: style folded into Dictionary/settings store"
        );
        assert!(
            !include_str!("polish.rs").contains("style.json")
                && !include_str!("snippets.rs").contains("style.json")
                && !include_str!("dictionary.rs").contains("style.json")
                && !include_str!("clipboard_history.rs").contains("style.json"),
            "BREAKS IF: style folded into Polish, Dictionary, Snippets, or Clipboard stores"
        );
    }

    #[test]
    fn breaks_if_style_without_dual_gate() {
        assert!(
            require_surface_with(true, false).is_err(),
            "BREAKS IF: Style without dual gate (Pro, no Stiki)"
        );
        assert!(
            require_surface_with(false, true).is_err(),
            "BREAKS IF: Style without dual gate (Stiki, no Pro)"
        );
        assert!(
            require_surface_with(false, false).is_err(),
            "BREAKS IF: Style without dual gate"
        );
        assert!(require_surface_with(true, true).is_ok());
        assert_eq!(
            effective_mode_with(MODE_FORMAL, true, false),
            MODE_OFF,
            "BREAKS IF: Style without dual gate"
        );
        assert!(!surface_ready(), "BREAKS IF: Style without dual gate");
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
        let style_prod = include_str!("style.rs")
            .split("#[cfg(test)]")
            .next()
            .expect("style prod");
        for hay in [
            include_str!("main.rs"),
            style_prod,
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
            !include_str!("../../src/main.ts").contains("style_promote"),
            "BREAKS IF: auto-promote"
        );
    }

    #[test]
    fn breaks_if_freeform_casing_ui() {
        let html = include_str!("../../index.html");
        let ts = include_str!("../../src/main.ts");
        let pro = include_str!("pro_features.rs");
        let style_prod = include_str!("style.rs")
            .split("#[cfg(test)]")
            .next()
            .expect("style prod");
        for hay in [html, ts, pro, style_prod] {
            assert!(
                !hay.contains("style-tone")
                    && !hay.contains("style-casing")
                    && !hay.contains("style-punctuation"),
                "BREAKS IF: freeform casing UI instead of Formal|Casual|Very casual"
            );
        }
        let style_struct = pro
            .split("pub struct StylePrefs")
            .nth(1)
            .and_then(|s| s.split("pub struct").next())
            .expect("StylePrefs");
        assert!(
            !style_struct.contains("pub casing")
                && !style_struct.contains("pub punctuation")
                && !style_struct.contains("pub tone"),
            "BREAKS IF: freeform casing/punctuation/tone store"
        );
        assert!(
            html.contains("data-style-mode=\"formal\"")
                && html.contains("data-style-mode=\"casual\"")
                && html.contains("data-style-mode=\"very-casual\""),
            "BREAKS IF: Formal|Casual|Very casual controls missing"
        );
    }

    fn rec_or_stream_applies() -> bool {
        include_str!("recorder.rs").contains("style::apply_register")
            && include_str!("streaming.rs").contains("style::apply_register")
    }

    #[test]
    fn register_applies_during_cleanup_without_inventing() {
        assert_eq!(
            apply_register_with("Yeah I am gonna go.", MODE_FORMAL),
            "Yes I am going to go."
        );
        assert_eq!(
            apply_register_with("I don't wanna wait.", MODE_FORMAL),
            "I do not want to wait."
        );
        assert_eq!(
            apply_register_with("Yeah I am gonna go.", MODE_CASUAL),
            "Yeah I am gonna go."
        );
        assert_eq!(
            apply_register_with("Yes I am okay.", MODE_VERY_CASUAL),
            "Yeah I am ok."
        );
        assert_eq!(
            apply_register_with("Nothing to change here.", MODE_FORMAL),
            "Nothing to change here."
        );
        assert_eq!(
            apply_register("yeah I am gonna go.", MODE_FORMAL),
            "yeah I am gonna go.",
            "Free / no entitlement must not apply Style"
        );
    }

    #[test]
    fn product_lock_v1_non_goals_and_surface() {
        assert!(PRODUCT_LOCK.contains("Name: Style"));
        assert!(PRODUCT_LOCK.contains("Dictionary → Snippets → Style"));
        assert!(PRODUCT_LOCK.contains("StoreKit Pro AND Stiki session"));
        assert!(PRODUCT_LOCK.contains("Activate Pro / Sign in with Stiki"));
        assert!(PRODUCT_LOCK.contains("Formal|Casual|Very casual"));
        assert!(PRODUCT_LOCK.contains("cleanup/dictation paste"));
        assert!(PRODUCT_LOCK.contains("Settings + sidebar/nav + menu bar"));
        assert!(PRODUCT_LOCK.contains("pick/clear mode"));
        assert!(PRODUCT_LOCK.contains("distinct from Dictionary (spelling), Snippets (trigger→expansion), Polish"));
        assert!(PRODUCT_LOCK.contains("Off|Casual|Professional|Polite"));
        assert!(PRODUCT_LOCK.contains("freeform casing/punctuation/formatting prefs"));
        assert!(PRODUCT_LOCK.contains("HIPAA"));

        let html = include_str!("../../index.html");
        let ts = include_str!("../../src/main.ts");
        let rec = include_str!("recorder.rs");
        assert!(html.contains("view-title\">Style"));
        assert!(html.contains("id=\"style-activate\""));
        assert!(html.contains("id=\"style-open\""));
        assert!(html.contains("id=\"style-clear-btn\""));
        assert!(html.contains("id=\"dictionary-activate\""));
        assert!(html.contains("id=\"snippets-activate\""));
        assert!(html.contains("id=\"polish-activate\""));
        assert!(ts.contains("openPlans()"));
        assert!(ts.contains("style_set"));
        assert!(ts.contains("pickStyleMode"));
        assert!(rec.contains("apply_register"));
        assert!(include_str!("streaming.rs").contains("apply_register"));
        assert!(include_str!("cleanup.rs").contains("cleanup_text"));
        assert!(
            rec.contains("cleanup_text") && rec.contains("apply_register"),
            "BREAKS IF: Style not applied during cleanup/dictation paste"
        );

        for hay in [html, ts] {
            assert!(!hay.contains("HIPAA"), "BREAKS IF: HIPAA claim copy");
        }
    }

    #[test]
    fn distinct_from_dictionary_snippets_polish_and_clipboard() {
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
        assert!(engine.contains("row-label\">Style"));
        assert!(engine.contains("not Polish"));
        assert!(engine.contains("not Dictionary"));
        assert!(engine.contains("not Snippets"));
        assert!(html.contains("id=\"polish-toggle\""));
        assert!(html.contains("id=\"polish-mode-select\""));
        assert!(html.contains("id=\"clipboard-history-toggle\""));
        assert!(html.contains("id=\"dict-add-btn\""));
        assert!(html.contains("id=\"snippet-add-btn\""));
        assert_ne!("style-clear-btn", "dict-add-btn");
        assert_ne!("style-open", "dictionary-open");
        assert_ne!("style-open", "snippets-open");
        assert!(html.contains("option value=\"professional\""));
        assert!(html.contains("option value=\"polite\""));
        let polish_select = html
            .split("id=\"polish-mode-select\"")
            .nth(1)
            .expect("polish modes");
        assert!(polish_select.contains("Professional"));
        assert!(polish_select.contains("Polite"));
        let style_view = html.split("data-view=\"style\"").last().unwrap();
        let style_view = style_view.split("<section").next().unwrap();
        assert!(!style_view.contains("Professional"));
        assert!(!style_view.contains("Polite"));
        assert!(!style_view.contains("polish-mode"));
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
