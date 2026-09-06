//! Product **Dictionary** — Enforcer BOUND (suite b6530197).
//!
//! - Local personal terms / jargon / spelling replacements only
//! - Pro-gated (Free locked + Activate Pro → Plans; no website)
//! - No cloud sync, no team/company share, no Nexus/SIEM write
//! - Private terms must not auto-promote to company memory
//! - No HIPAA/BAA claim copy (until Enforcer+Legal yes)
//! - Scratchpad / Insights stay local-only by default; do not claim HIPAA/BAA
//! - Distinct from Polish modes and Clipboard History
//!
//! BREAKS IF: cloud/team share ships without separate ACL Make It So
//! BREAKS IF: private Dictionary auto-promotes to company memory
//! HELD: cloud sync + team share until MCS with Stiki/folder-style ACL;
//! fail closed if ACL missing.

use crate::storekit;

/// Named Enforcer BOUND. Tests fail if this is violated.
pub const ENFORCER_BOUND: &str = "Pro-gated; local-first; no cloud sync; no team share; no Nexus/SIEM write; no auto-promote; no HIPAA/BAA; Scratchpad/Insights local-only default; fail closed if ACL missing";

/// Held. A later MCS must flip this only with Stiki/folder-style ACL.
pub const STIKI_FOLDER_ACL_SHIPPED: bool = false;

const SHARE_BLOCKED: &str =
    "Cloud and team dictionary share is not available. Dictionary stays on this Mac.";
const ACL_MISSING: &str =
    "Cloud and team dictionary share is held until a Stiki/folder-style ACL Make It So. ACL missing — fail closed.";

pub fn normalize_term(raw: &str) -> Option<String> {
    let term = raw.trim();
    if term.is_empty() {
        None
    } else {
        Some(term.to_string())
    }
}

pub fn normalize_terms(stored: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for raw in stored {
        if let Some(term) = normalize_term(raw) {
            let key = term.to_ascii_lowercase();
            if seen.insert(key) {
                out.push(term);
            }
        }
    }
    out
}

/// Runtime gate: Free / lapsed / stub entitlement → no terms in ASR or cleanup.
pub fn effective_terms(stored: &[String]) -> Vec<String> {
    if storekit::current_entitlement().entitled {
        normalize_terms(stored)
    } else {
        Vec::new()
    }
}

pub fn require_list(stored: &[String]) -> Result<Vec<String>, String> {
    storekit::require_pro()?;
    Ok(normalize_terms(stored))
}

fn parse_candidates(raw: &str) -> Vec<String> {
    raw.split(|c| c == ',' || c == '\n')
        .filter_map(normalize_term)
        .collect()
}

fn apply_add(stored: &mut Vec<String>, raw: &str) -> Result<Vec<String>, String> {
    let candidates = parse_candidates(raw);
    if candidates.is_empty() {
        return Err("Add a word or phrase first".into());
    }
    let mut terms = normalize_terms(stored);
    let mut seen: std::collections::HashSet<String> = terms
        .iter()
        .map(|t| t.to_ascii_lowercase())
        .collect();
    let mut added = 0usize;
    for term in candidates {
        let key = term.to_ascii_lowercase();
        if seen.insert(key) {
            terms.push(term);
            added += 1;
        }
    }
    if added == 0 {
        return Err("That term is already in the dictionary".into());
    }
    *stored = terms;
    Ok(stored.clone())
}

fn apply_update(stored: &mut Vec<String>, from: &str, to: &str) -> Result<Vec<String>, String> {
    let from = normalize_term(from).ok_or_else(|| "Pick a term to edit".to_string())?;
    let to = normalize_term(to).ok_or_else(|| "Replacement cannot be empty".to_string())?;
    let mut terms = normalize_terms(stored);
    let idx = terms
        .iter()
        .position(|t| t.eq_ignore_ascii_case(&from))
        .ok_or_else(|| "Term not found".to_string())?;
    if terms
        .iter()
        .enumerate()
        .any(|(i, t)| i != idx && t.eq_ignore_ascii_case(&to))
    {
        return Err("That term already exists".into());
    }
    terms[idx] = to;
    *stored = terms;
    Ok(stored.clone())
}

fn apply_remove(stored: &mut Vec<String>, term: &str) -> Result<Vec<String>, String> {
    let term = normalize_term(term).ok_or_else(|| "Pick a term to remove".to_string())?;
    let mut terms = normalize_terms(stored);
    let before = terms.len();
    terms.retain(|t| !t.eq_ignore_ascii_case(&term));
    if terms.len() == before {
        return Err("Term not found".into());
    }
    *stored = terms;
    Ok(stored.clone())
}

pub fn add_terms(stored: &mut Vec<String>, raw: &str) -> Result<Vec<String>, String> {
    storekit::require_pro()?;
    apply_add(stored, raw)
}

pub fn update_term(stored: &mut Vec<String>, from: &str, to: &str) -> Result<Vec<String>, String> {
    storekit::require_pro()?;
    apply_update(stored, from, to)
}

pub fn remove_term(stored: &mut Vec<String>, term: &str) -> Result<Vec<String>, String> {
    storekit::require_pro()?;
    apply_remove(stored, term)
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

/// BREAKS IF: private Dictionary auto-promotes to company memory.
pub fn promote_to_company_memory() -> Result<(), String> {
    Err("Private dictionary terms cannot become company memory.".into())
}

/// Spelling hint for the local Gemma cleanup pass. Empty when Free or empty list.
pub fn cleanup_spelling_hint(stored: &[String]) -> String {
    let terms = effective_terms(stored);
    if terms.is_empty() {
        return String::new();
    }
    format!(
        " If the speaker used any of these terms, keep this spelling: {}. Do not add a term that was not spoken.",
        terms.join(", ")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mutations_fail_closed_without_entitlement() {
        let mut terms = vec!["Mabel".to_string()];
        assert!(add_terms(&mut terms, "Chibitek").is_err());
        assert!(update_term(&mut terms, "Mabel", "Mochi").is_err());
        assert!(remove_term(&mut terms, "Mabel").is_err());
        assert_eq!(terms, vec!["Mabel".to_string()]);
        assert!(require_list(&terms).is_err());
    }

    #[test]
    fn effective_terms_empty_without_pro() {
        assert!(effective_terms(&["Mabel".into(), "Chibitek".into()]).is_empty());
        assert!(cleanup_spelling_hint(&["Mabel".into()]).is_empty());
    }

    #[test]
    fn normalize_drops_blanks_and_duplicates() {
        let terms = normalize_terms(&[
            "  ".into(),
            "Mabel".into(),
            "mabel".into(),
            "Chibitek".into(),
            "".into(),
        ]);
        assert_eq!(terms, vec!["Mabel".to_string(), "Chibitek".to_string()]);
    }

    #[test]
    fn pro_store_can_add_edit_delete_local_terms() {
        let mut terms = Vec::new();
        assert_eq!(
            apply_add(&mut terms, "Chibitek, GGUF\nMabel").unwrap(),
            vec!["Chibitek", "GGUF", "Mabel"]
        );
        assert_eq!(
            apply_update(&mut terms, "GGUF", "ggml").unwrap(),
            vec!["Chibitek", "ggml", "Mabel"]
        );
        assert_eq!(
            apply_remove(&mut terms, "ggml").unwrap(),
            vec!["Chibitek", "Mabel"]
        );
        assert!(apply_add(&mut terms, "chibitek").is_err());
        assert!(apply_update(&mut terms, "missing", "x").is_err());
        assert!(apply_remove(&mut terms, "nope").is_err());
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
    fn breaks_if_share_ships_without_acl_make_it_so() {
        assert!(!STIKI_FOLDER_ACL_SHIPPED, "BREAKS IF: ACL shipped without MCS");
        assert!(share_cloud_or_team().is_err(), "BREAKS IF: share succeeded");
        let ts = include_str!("../../src/main.ts");
        assert!(
            !ts.contains("dictionary_share"),
            "BREAKS IF: share UI shipped"
        );
        let html = include_str!("../../index.html");
        let dict_view = html.split("data-view=\"dictionary\"").last().unwrap();
        let dict_view = dict_view.split("<section").next().unwrap();
        assert!(
            !dict_view.contains("Share with team") && !dict_view.contains("Sync to cloud"),
            "BREAKS IF: cloud/team share shipped without ACL Make It So"
        );
    }

    #[test]
    fn breaks_if_private_dictionary_auto_promotes() {
        assert!(
            promote_to_company_memory().is_err(),
            "BREAKS IF: private Dictionary auto-promotes to company memory"
        );
        let ts = include_str!("../../src/main.ts");
        assert!(
            !ts.contains("dictionary_promote"),
            "BREAKS IF: promote UI shipped"
        );
    }

    #[test]
    fn enforcer_bound_breaks_if_free_cloud_nexus_or_web_upgrade() {
        assert!(ENFORCER_BOUND.contains("Pro-gated"));
        assert!(ENFORCER_BOUND.contains("local-first"));
        assert!(ENFORCER_BOUND.contains("no cloud sync"));
        assert!(ENFORCER_BOUND.contains("no team share"));
        assert!(ENFORCER_BOUND.contains("no Nexus/SIEM write"));
        assert!(ENFORCER_BOUND.contains("no auto-promote"));
        assert!(ENFORCER_BOUND.contains("no HIPAA/BAA"));
        assert!(ENFORCER_BOUND.contains("Scratchpad/Insights local-only default"));
        assert!(ENFORCER_BOUND.contains("fail closed if ACL missing"));

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
                "teams"
            ],
            "BREAKS IF: Pro catalog order drifted"
        );
        assert!(nav.contains("data-view=\"dictionary\""));
        assert!(
            nav.split("data-view=\"dictionary\"")
                .nth(1)
                .unwrap()
                .contains("lock-pill"),
            "BREAKS IF: Dictionary nav not Pro-locked"
        );
        let dict_view = html
            .split("data-view=\"dictionary\"")
            .last()
            .expect("dictionary view");
        let dict_view = dict_view.split("<section").next().unwrap();
        assert!(dict_view.contains("pro-lock"), "BREAKS IF: Free not locked");
        assert!(dict_view.contains("pro-activate"), "BREAKS IF: no Plans upsell");
        assert!(
            !dict_view.contains("chibiteklabs.com"),
            "BREAKS IF: web upgrade"
        );
        assert!(
            !dict_view.contains("https://"),
            "BREAKS IF: web upgrade"
        );
        assert!(dict_view.contains("this Mac"));
        assert!(
            !dict_view.contains("Share with team") && !dict_view.contains("Sync to cloud"),
            "BREAKS IF: share shipped"
        );

        let ts = include_str!("../../src/main.ts");
        assert!(ts.contains("dictionary_add"));
        assert!(ts.contains("dictionary_update"));
        assert!(ts.contains("dictionary_remove"));
        assert!(ts.contains("openPlans()"));
        assert!(!ts.contains("https://chibiteklabs"));
        assert!(
            ts.contains("\"dictionary\""),
            "BREAKS IF: Dictionary dropped from Pro lock list"
        );

        let commands = include_str!("main.rs");
        assert!(commands.contains("dictionary::effective_terms"));
        assert!(commands.contains("dictionary_add"));
        assert!(commands.contains("share_cloud_or_team"));
        assert!(commands.contains("promote_to_company_memory"));
        assert!(commands.contains("require_share_acl") || include_str!("dictionary.rs").contains("require_share_acl"));
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
        assert!(include_str!("dictionary.rs").contains("fail-closed"));

        let whisper = include_str!("transcribe_local.rs");
        assert!(whisper.contains("build_prompt"));
        let native = include_str!("transcribe_native.rs");
        assert!(
            native.contains("dictionary::effective_terms"),
            "BREAKS IF: ASR hook skipped"
        );
        let llm = include_str!("llm.rs");
        assert!(
            llm.contains("cleanup_spelling_hint"),
            "BREAKS IF: cleanup path skipped"
        );
        let polish_fn = llm.split("pub async fn polish_or_rules").nth(1).unwrap();
        let polish_fn = polish_fn.split("pub async fn ensure_and_cleanup").next().unwrap();
        assert!(!polish_fn.contains("groq"), "BREAKS IF: cloud cleanup");

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
        assert!(
            include_str!("dictionary.rs").contains("promote_to_company_memory"),
            "BREAKS IF: auto-promote stub missing"
        );
        assert!(html.contains("id=\"dictionary-open\""));
        assert!(html.contains("id=\"dictionary-activate\""));
        assert!(ts.contains("openDictionary"));
        let tray = include_str!("clipboard_ui.rs");
        assert!(tray.contains("Dictionary"));
        assert!(tray.contains("open-dictionary"));
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
        assert!(
            insights.contains("Local-only"),
            "BREAKS IF: Insights not local-only default"
        );
        assert!(
            !insights.contains("HIPAA")
                && !insights.contains("HIPAA/BAA")
                && !insights.contains("Business Associate"),
            "BREAKS IF: HIPAA/BAA copy on Insights"
        );

        let pad = html
            .split("data-view=\"scratchpad\"")
            .nth(2)
            .or_else(|| html.split("data-view=\"scratchpad\"").last())
            .expect("scratchpad view");
        let pad = pad.split("<section").next().unwrap();
        assert!(
            pad.contains("Local only") || pad.contains("local-only") || pad.contains("this Mac"),
            "BREAKS IF: Scratchpad not local-only default"
        );
        assert!(
            !pad.contains("HIPAA")
                && !pad.contains("HIPAA/BAA")
                && !pad.contains("Business Associate"),
            "BREAKS IF: HIPAA/BAA copy on Scratchpad"
        );

        let stats = include_str!("stats.rs");
        assert!(stats.contains("Local-only"));
        assert!(stats.contains("ever leaves the device"));
        assert!(!stats.contains("HIPAA"));
        let pro = include_str!("pro_features.rs");
        assert!(pro.contains("scratchpad.txt"));
        assert!(!pro.contains("HIPAA"));
    }

    #[test]
    fn polish_and_clipboard_stay_independent() {
        let settings = crate::settings::Settings::default();
        assert_eq!(settings.polish_mode, "off");
        assert!(!settings.clipboard_history_enabled);
        assert!(settings.dictionary.is_empty());
        let html = include_str!("../../index.html");
        assert!(html.contains("id=\"polish-toggle\""));
        assert!(html.contains("id=\"clipboard-history-toggle\""));
        assert_ne!("polish-toggle", "dict-add-btn");
    }
}
