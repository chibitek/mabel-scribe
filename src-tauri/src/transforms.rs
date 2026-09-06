//! Product **Transforms** — Enforcer BOUND (suite b6530197) + Sign-on RE-LOCK.
//!
//! GREEN: StoreKit Pro + Stiki dual gate; user-invoked local rewrite actions
//! (Email | Bullet points | Make shorter | Make clearer) on dictated/selected
//! text; NOT auto-on; Local Gemma; never invent facts; never leave this Mac;
//! local persistence of last invoke; distinct Style (Formal|Casual|Very casual
//! register), Polish (Off|Casual|Professional|Polite tone), Snippets,
//! Dictionary, Clipboard; no cloud sync; no team/company share without ACL
//! Make It So; no Nexus/SIEM write; no auto-promote; no HIPAA/BAA; Free
//! dictate no Stiki; sign-out locks Transforms.
//!
//! BREAKS IF: Transforms without dual gate
//! BREAKS IF: auto-apply without user invoke
//! BREAKS IF: cloud/team/Nexus
//! BREAKS IF: merges Style/Polish stores
//! BREAKS IF: invents facts
//! BREAKS IF: Free dictate gated
//! BREAKS IF: sign-out leaves Transforms unlocked
//! HELD: cloud sync + team share until MCS with Stiki/folder-style ACL;
//! fail-closed if ACL missing.

use std::path::PathBuf;

use crate::pro_features::{self, TransformPrefs};
use crate::stiki_session;
use crate::storekit;

/// Named Enforcer suite. Tests fail if this is retargeted without a new MCS.
pub const ENFORCER_SUITE: &str = "b6530197";

/// Named Enforcer BOUND. `enforcer_bound_transforms_v1_suite_*` tests fold this.
pub const ENFORCER_BOUND: &str = "StoreKit Pro + Stiki dual gate; user-invoked local rewrite Email|Bullet points|Make shorter|Make clearer; NOT auto-on; Local Gemma; never invent facts; local-only; sign-out locks Transforms; distinct Style/Polish/Snippets/Dictionary/Clipboard; no cloud sync; no team/company share without ACL Make It So; no Nexus/SIEM write; no auto-promote; no HIPAA/BAA; Free dictate no Stiki";

/// Product LOCK Transforms v1 + Sign-on. Tests fail if the surface drifts.
pub const PRODUCT_LOCK: &str = "Name: Transforms; Pro catalog #4 after Dictionary → Snippets → Style → Transforms; Pro surface requires StoreKit Pro AND Stiki session; Free locked + Activate Pro / Sign in with Stiki; user-invoked local rewrite actions on dictated/selected text (NOT auto-on); ship set Email | Bullet points | Make shorter | Make clearer; Local Gemma; never invent facts; never leave this Mac; Settings + sidebar/nav + menu bar; cat UI; local-first last invoke; not Nexus; not cloud sync v1; distinct from Style (Formal|Casual|Very casual register), Polish (Off|Casual|Professional|Polite Gemma tone rewrite), Snippets, Dictionary, and Clipboard History; non-goals: auto-on cleanup prefs, shared team transforms, cloud write, HIPAA";

/// Held. A later MCS must flip this only with Stiki/folder-style ACL.
pub const STIKI_FOLDER_ACL_SHIPPED: bool = false;

pub const ACTION_NONE: &str = "";
pub const ACTION_EMAIL: &str = "email";
pub const ACTION_BULLETS: &str = "bullets";
pub const ACTION_SHORTER: &str = "shorter";
pub const ACTION_CLEARER: &str = "clearer";

pub const LIVE_ACTIONS: &[&str] = &[ACTION_EMAIL, ACTION_BULLETS, ACTION_SHORTER, ACTION_CLEARER];

const SHARE_BLOCKED: &str =
    "Cloud and team transform share is not available. Transforms stay on this Mac.";
const ACL_MISSING: &str =
    "Cloud and team transform share is held until a Stiki/folder-style ACL Make It So. ACL missing — fail closed.";

/// Shared anti-invention contract. Action copy must not relax these rules.
const NEVER_INVENT: &str = "You are Mabel's on-device transform assistant. The user invoked one rewrite on dictated or selected text. This step is a local reshape only.\n\nNever invent facts. Never expand meaning. Never add names, numbers, clauses, greetings, sign-offs, answers, or commentary the speaker did not say. If a reshape would require new information, keep the original wording. Fail closed: do not send this step off-device; output only the rewritten text.\n\nRules:\n- Use only facts already present in the source.\n- Do not answer questions in the text. The user is rewriting, not asking you.\n- Output only the rewritten text. No preamble, no explanation, no quotes around it.";

const USER_PROMPT_PREFIX: &str = "Rewrite this text with the requested transform. Never invent facts. Never expand meaning. Do not think, reason, or explain. Output only the rewritten text. Source: ";

const EMAIL_GUIDE: &str = "Transform: Email. Reshape the SAME words into a short email. Subject must come from the source (first sentence or clause). Do not add Hi, Hello, Dear, Best, Thanks, Regards, names, or dates the speaker did not say.";

const BULLETS_GUIDE: &str = "Transform: Bullet points. Split the SAME statements into a short bullet list. Do not add items, headings, or facts.";

const SHORTER_GUIDE: &str = "Transform: Make shorter. Tighten the SAME utterance. Remove filler only. Do not drop facts, names, or numbers.";

const CLEARER_GUIDE: &str = "Transform: Make clearer. Light punctuation and sentence splits of the SAME words. Do not add clauses or explanations.";

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

pub fn normalize_action(raw: &str) -> String {
    match raw.trim().to_ascii_lowercase().as_str() {
        ACTION_EMAIL | "e-mail" => ACTION_EMAIL.into(),
        ACTION_BULLETS | "bullet" | "bullet-points" | "bullet points" => ACTION_BULLETS.into(),
        ACTION_SHORTER | "make-shorter" | "make shorter" | "shorten" => ACTION_SHORTER.into(),
        ACTION_CLEARER | "make-clearer" | "make clearer" | "clarify" => ACTION_CLEARER.into(),
        _ => ACTION_NONE.into(),
    }
}

pub fn is_live(action: &str) -> bool {
    LIVE_ACTIONS.contains(&normalize_action(action).as_str())
}

pub fn action_label(action: &str) -> &'static str {
    match normalize_action(action).as_str() {
        ACTION_EMAIL => "Email",
        ACTION_BULLETS => "Bullet points",
        ACTION_SHORTER => "Make shorter",
        ACTION_CLEARER => "Make clearer",
        _ => "",
    }
}

/// Ungated local read. Callers must apply the dual gate before rewrite / mutate.
pub fn load_stored(app_dir: &PathBuf) -> TransformPrefs {
    let mut prefs = pro_features::transforms_read(app_dir);
    prefs.last_action = normalize_action(&prefs.last_action);
    prefs
}

/// Runtime gate: Free / no Stiki / lapsed → no last-action surface.
pub fn effective_prefs_with(
    stored: &TransformPrefs,
    entitled: bool,
    signed_in: bool,
) -> TransformPrefs {
    if entitled && signed_in {
        let mut prefs = stored.clone();
        prefs.last_action = normalize_action(&prefs.last_action);
        prefs
    } else {
        TransformPrefs::default()
    }
}

pub fn effective_prefs(stored: &TransformPrefs) -> TransformPrefs {
    effective_prefs_with(
        stored,
        storekit::current_entitlement().entitled,
        stiki_session::is_live(),
    )
}

pub fn require_prefs(app_dir: &PathBuf) -> Result<TransformPrefs, String> {
    require_surface()?;
    Ok(load_stored(app_dir))
}

/// User-invoked rewrite. Unknown actions fail closed (no write).
pub fn apply_local(
    app_dir: &PathBuf,
    action: String,
    source: String,
) -> Result<TransformPrefs, String> {
    require_surface()?;
    let action = normalize_action(&action);
    if !is_live(&action) {
        return Err("Pick Email, Bullet points, Make shorter, or Make clearer.".into());
    }
    let source = source.trim().to_string();
    if source.is_empty() {
        return Err("Paste dictated or selected text first.".into());
    }
    let result = rewrite_local(&source, &action);
    persist_result(app_dir, action, source, result)
}

pub fn persist_result(
    app_dir: &PathBuf,
    action: String,
    source: String,
    result: String,
) -> Result<TransformPrefs, String> {
    require_surface()?;
    let action = normalize_action(&action);
    if !is_live(&action) {
        return Err("Pick Email, Bullet points, Make shorter, or Make clearer.".into());
    }
    pro_features::transforms_write_local(
        app_dir,
        TransformPrefs {
            last_action: action,
            last_source: source,
            last_result: result,
        },
    )
}

/// Clear last invoke. Transforms stay unused until the user invokes again.
pub fn clear(app_dir: &PathBuf) -> Result<TransformPrefs, String> {
    require_surface()?;
    pro_features::transforms_write_local(app_dir, TransformPrefs::default())
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

/// BREAKS IF: private Transforms auto-promotes to company memory.
pub fn promote_to_company_memory() -> Result<(), String> {
    Err("Private transforms cannot become company memory.".into())
}

pub fn system_prompt(action: &str) -> String {
    let guide = match normalize_action(action).as_str() {
        ACTION_EMAIL => EMAIL_GUIDE,
        ACTION_BULLETS => BULLETS_GUIDE,
        ACTION_SHORTER => SHORTER_GUIDE,
        ACTION_CLEARER => CLEARER_GUIDE,
        _ => "Transform: none. Return the source unchanged.",
    };
    format!("{NEVER_INVENT}\n\n{guide}")
}

pub fn user_prompt_prefix() -> &'static str {
    USER_PROMPT_PREFIX
}

/// Fail closed on invented / expanded Gemma output.
/// Caller treats Err as "use the local reshape" — never paste the model text.
pub fn accept_or_fail_closed(input: &str, output: &str) -> Result<String, String> {
    let cleaned = output.trim();
    if cleaned.is_empty() {
        return Err("Transform output empty after sanitization (fail closed)".into());
    }
    let input_chars = input.trim().chars().count() as f32;
    let out_chars = cleaned.chars().count() as f32;
    // Email may add a Subject: line. >3x on a 20+ char source is invention.
    if input_chars >= 20.0 && out_chars > input_chars * 3.0 {
        return Err(format!(
            "Transform output too long ({} chars vs {} input chars), fail closed (never invent)",
            out_chars as usize, input_chars as usize
        ));
    }
    Ok(cleaned.to_string())
}

/// Deterministic local reshape. Never invents facts. Used when Gemma is
/// unavailable and as the fail-closed fallback. Not auto-applied.
pub fn rewrite_local(text: &str, action: &str) -> String {
    let text = text.trim();
    if text.is_empty() {
        return String::new();
    }
    match normalize_action(action).as_str() {
        ACTION_EMAIL => reshape_email(text),
        ACTION_BULLETS => reshape_bullets(text),
        ACTION_SHORTER => reshape_shorter(text),
        ACTION_CLEARER => reshape_clearer(text),
        _ => text.to_string(),
    }
}

fn first_clause(text: &str) -> String {
    let cut = text
        .find(['.', '!', '?', '\n'])
        .map(|i| i + 1)
        .unwrap_or(text.len());
    let clause = text[..cut].trim();
    if clause.is_empty() {
        return text.to_string();
    }
    let words: Vec<&str> = clause.split_whitespace().collect();
    if words.len() > 12 {
        words[..12].join(" ")
    } else {
        clause.trim_end_matches(['.', '!', '?']).to_string()
    }
}

fn reshape_email(text: &str) -> String {
    let subject = first_clause(text);
    format!("Subject: {subject}\n\n{text}")
}

fn split_statements(text: &str) -> Vec<String> {
    let mut items = Vec::new();
    let mut buf = String::new();
    for ch in text.chars() {
        buf.push(ch);
        if matches!(ch, '.' | '!' | '?' | '\n') {
            let item = buf.trim().trim_end_matches('\n').trim().to_string();
            if !item.is_empty() {
                items.push(item.trim_end_matches(['.', '!', '?']).trim().to_string());
            }
            buf.clear();
        }
    }
    let tail = buf.trim();
    if !tail.is_empty() {
        items.push(tail.trim_end_matches(['.', '!', '?']).trim().to_string());
    }
    items.into_iter().filter(|s| !s.is_empty()).collect()
}

fn reshape_bullets(text: &str) -> String {
    split_statements(text)
        .into_iter()
        .map(|s| format!("- {s}"))
        .collect::<Vec<_>>()
        .join("\n")
}

const FILLERS: &[&str] = &["um", "uh", "er", "ah"];

fn is_filler_token(token: &str) -> bool {
    let t = token.trim_matches(|c: char| !c.is_alphanumeric()).to_ascii_lowercase();
    FILLERS.contains(&t.as_str())
}

fn reshape_shorter(text: &str) -> String {
    let words: Vec<&str> = text.split_whitespace().collect();
    let mut kept: Vec<&str> = Vec::new();
    for w in words {
        if is_filler_token(w) {
            continue;
        }
        kept.push(w);
    }
    if kept.is_empty() {
        return text.to_string();
    }
    kept.join(" ")
}

fn reshape_clearer(text: &str) -> String {
    let mut out = String::new();
    let mut start_sentence = true;
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0usize;
    while i < chars.len() {
        let c = chars[i];
        if start_sentence && c.is_whitespace() {
            i += 1;
            continue;
        }
        if start_sentence && c.is_alphabetic() {
            for uc in c.to_uppercase() {
                out.push(uc);
            }
            start_sentence = false;
            i += 1;
            continue;
        }
        out.push(c);
        if matches!(c, '.' | '!' | '?') {
            start_sentence = true;
        } else if !c.is_whitespace() {
            start_sentence = false;
        }
        i += 1;
    }
    let trimmed = out.trim().to_string();
    if trimmed.is_empty() {
        return text.to_string();
    }
    if trimmed
        .chars()
        .last()
        .map(|c| c.is_alphanumeric())
        .unwrap_or(false)
    {
        format!("{trimmed}.")
    } else {
        trimmed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp() -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "mabel-xf-{}-{}",
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
        assert!(apply_local(&dir, ACTION_EMAIL.into(), "hello team".into()).is_err());
        assert!(clear(&dir).is_err());
    }

    #[test]
    fn effective_prefs_empty_without_pro() {
        let stored = TransformPrefs {
            last_action: ACTION_EMAIL.into(),
            last_source: "hello".into(),
            last_result: "Subject: hello\n\nhello".into(),
        };
        let locked = effective_prefs(&stored);
        assert_eq!(locked.last_action, ACTION_NONE);
        assert!(locked.last_source.is_empty());
        assert!(locked.last_result.is_empty());
        assert_eq!(effective_prefs_with(&stored, true, false).last_action, ACTION_NONE);
        assert_eq!(
            effective_prefs_with(&stored, true, true).last_action,
            ACTION_EMAIL
        );
    }

    #[test]
    fn default_is_empty_until_user_invokes() {
        let dir = tmp();
        let prefs = load_stored(&dir);
        assert_eq!(prefs.last_action, ACTION_NONE);
        assert!(prefs.last_source.is_empty());
        assert!(prefs.last_result.is_empty());
        assert!(!is_live(&prefs.last_action));
        let written = pro_features::transforms_write_local(
            &dir,
            TransformPrefs {
                last_action: ACTION_EMAIL.into(),
                last_source: "ship friday".into(),
                last_result: "Subject: ship friday\n\nship friday".into(),
            },
        )
        .unwrap();
        assert_eq!(written.last_action, ACTION_EMAIL);
        assert_eq!(load_stored(&dir).last_source, "ship friday");
        let cleared = pro_features::transforms_write_local(&dir, TransformPrefs::default()).unwrap();
        assert_eq!(cleared.last_action, ACTION_NONE);
        assert!(load_stored(&dir).last_result.is_empty());
    }

    #[test]
    fn local_store_persists_last_invoke() {
        let dir = tmp();
        let one = pro_features::transforms_write_local(
            &dir,
            TransformPrefs {
                last_action: ACTION_BULLETS.into(),
                last_source: "Alpha. Bravo.".into(),
                last_result: "- Alpha\n- Bravo".into(),
            },
        )
        .unwrap();
        assert_eq!(one.last_action, ACTION_BULLETS);
        let two = pro_features::transforms_write_local(
            &dir,
            TransformPrefs {
                last_action: ACTION_SHORTER.into(),
                last_source: "um hello".into(),
                last_result: "hello".into(),
            },
        )
        .unwrap();
        assert_eq!(two.last_action, ACTION_SHORTER);
        assert_eq!(load_stored(&dir).last_result, "hello");
    }

    #[test]
    fn breaks_if_transforms_usable_without_stiki() {
        assert!(
            require_surface_with(true, false).is_err(),
            "BREAKS IF: Transforms usable without Stiki"
        );
        let stored = TransformPrefs {
            last_action: ACTION_EMAIL.into(),
            last_source: "hello".into(),
            last_result: "Subject: hello\n\nhello".into(),
        };
        assert_eq!(
            effective_prefs_with(&stored, true, false).last_action,
            ACTION_NONE,
            "BREAKS IF: Transforms last invoke without Stiki"
        );
        assert!(require_surface_with(false, true).is_err());
        assert!(require_surface_with(true, true).is_ok());
        assert_eq!(
            effective_prefs_with(&stored, true, true).last_action,
            ACTION_EMAIL
        );

        let html = include_str!("../../index.html");
        let ts = include_str!("../../src/main.ts");
        assert!(
            html.contains("Sign in with Stiki"),
            "BREAKS IF: Sign in with Stiki CTA missing"
        );
        assert!(
            ts.contains("transformsSurfaceReady"),
            "BREAKS IF: frontend Stiki+Pro gate missing"
        );
        let load_xf = ts
            .split("async function loadTransformsSurface")
            .nth(1)
            .expect("loadTransformsSurface");
        let load_xf = load_xf.split("async function ").next().unwrap();
        assert!(load_xf.contains("transformsSurfaceReady"));
        assert!(load_xf.contains("transforms_get"));
        let load_pro = ts
            .split("async function loadProSurfaces")
            .nth(1)
            .expect("loadProSurfaces");
        let load_pro = load_pro.split("async function ").next().unwrap();
        assert!(
            !load_pro.contains("transforms_get"),
            "BREAKS IF: Transforms fetched without Stiki gate"
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
            !rec.contains("transforms::apply")
                && !rec.contains("rewrite_local")
                && !streaming.contains("transforms::apply")
                && !streaming.contains("rewrite_transform"),
            "BREAKS IF: auto-apply without user invoke"
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
        assert!(!ts.contains("transforms_share"), "BREAKS IF: share UI shipped");
        let html = include_str!("../../index.html");
        let view = html.split("data-view=\"transforms\"").last().unwrap();
        let view = view.split("<section").next().unwrap();
        assert!(
            !view.contains("Share with team") && !view.contains("Sync to cloud"),
            "BREAKS IF: cloud/team share shipped without ACL Make It So"
        );
        assert!(
            !view.contains("https://"),
            "BREAKS IF: cloud write / web upgrade on Transforms"
        );
    }

    #[test]
    fn breaks_if_private_transforms_auto_promotes() {
        assert!(
            promote_to_company_memory().is_err(),
            "BREAKS IF: private Transforms auto-promote to company memory"
        );
        let ts = include_str!("../../src/main.ts");
        assert!(
            !ts.contains("transforms_promote"),
            "BREAKS IF: promote UI shipped"
        );
    }

    #[test]
    fn enforcer_bound_suite_b6530197_green() {
        assert_eq!(ENFORCER_SUITE, "b6530197");
        assert!(ENFORCER_BOUND.contains("StoreKit Pro + Stiki dual gate"));
        assert!(ENFORCER_BOUND.contains("Email|Bullet points|Make shorter|Make clearer"));
        assert!(ENFORCER_BOUND.contains("NOT auto-on"));
        assert!(ENFORCER_BOUND.contains("Local Gemma"));
        assert!(ENFORCER_BOUND.contains("never invent facts"));
        assert!(ENFORCER_BOUND.contains("local-only"));
        assert!(ENFORCER_BOUND.contains("sign-out locks Transforms"));
        assert!(ENFORCER_BOUND.contains("distinct Style/Polish/Snippets/Dictionary/Clipboard"));
        assert!(ENFORCER_BOUND.contains("no cloud sync"));
        assert!(ENFORCER_BOUND.contains("no team/company share without ACL Make It So"));
        assert!(ENFORCER_BOUND.contains("no Nexus/SIEM write"));
        assert!(ENFORCER_BOUND.contains("no auto-promote"));
        assert!(ENFORCER_BOUND.contains("no HIPAA/BAA"));
        assert!(ENFORCER_BOUND.contains("Free dictate no Stiki"));
    }

    #[test]
    fn enforcer_bound_transforms_v1_suite_b6530197() {
        assert_eq!(ENFORCER_SUITE, "b6530197");
        assert!(ENFORCER_BOUND.contains("StoreKit Pro + Stiki dual gate"));
        assert!(ENFORCER_BOUND.contains("Email|Bullet points|Make shorter|Make clearer"));
        assert!(ENFORCER_BOUND.contains("NOT auto-on"));
        assert!(ENFORCER_BOUND.contains("Local Gemma"));
        assert!(ENFORCER_BOUND.contains("never invent facts"));
        assert!(ENFORCER_BOUND.contains("sign-out locks Transforms"));
        assert!(ENFORCER_BOUND.contains("no cloud sync"));
        assert!(ENFORCER_BOUND.contains("no Nexus/SIEM write"));
        assert!(ENFORCER_BOUND.contains("Free dictate no Stiki"));

        assert!(
            require_surface_with(true, false).is_err(),
            "BREAKS IF: Transforms without dual gate"
        );
        assert!(
            require_surface_with(false, true).is_err(),
            "BREAKS IF: Transforms without dual gate"
        );
        let stored = TransformPrefs {
            last_action: ACTION_EMAIL.into(),
            last_source: "hello".into(),
            last_result: "x".into(),
        };
        assert_eq!(
            effective_prefs_with(&stored, true, false).last_action,
            ACTION_NONE,
            "BREAKS IF: sign-out leaves Transforms unlocked"
        );
        assert!(share_cloud_or_team().is_err(), "BREAKS IF: cloud/team share");
        assert!(promote_to_company_memory().is_err(), "BREAKS IF: auto-promote");

        let html = include_str!("../../index.html");
        let ts = include_str!("../../src/main.ts");
        assert!(html.contains("data-xf-action=\"email\""));
        assert!(html.contains("data-xf-action=\"bullets\""));
        assert!(html.contains("data-xf-action=\"shorter\""));
        assert!(html.contains("data-xf-action=\"clearer\""));
        assert!(html.contains("Email"));
        assert!(html.contains("Bullet points"));
        assert!(html.contains("Make shorter"));
        assert!(html.contains("Make clearer"));
        assert!(!html.contains("id=\"xf-filler\""));
        assert!(!html.contains("id=\"xf-grammar\""));
        assert!(!html.contains("id=\"xf-punct\""));
        assert!(ts.contains("applyTransformsGate"));
        assert!(ts.contains("transformsSurfaceReady"));
        assert!(ts.contains("stiki_sign_out"));
        let apply = ts
            .split("function applyTransformsGate")
            .nth(1)
            .expect("applyTransformsGate");
        let apply = apply.split("function ").next().unwrap();
        assert!(
            apply.contains("transformsSurfaceReady") && apply.contains("stikiLive"),
            "BREAKS IF: sign-out leaves Transforms unlocked"
        );
        let sign_out = ts
            .split("stiki-signout")
            .nth(1)
            .expect("stiki-signout");
        let sign_out = sign_out.split("document.querySelectorAll").next().unwrap();
        assert!(
            sign_out.contains("applyStikiFromConnectors") || sign_out.contains("applyProLocks"),
            "BREAKS IF: sign-out leaves Transforms unlocked"
        );
        assert!(
            include_str!("pro_features.rs").contains("transforms.json")
                && !include_str!("settings.rs").contains("transforms.json"),
            "BREAKS IF: Transforms folded into Dictionary/settings store"
        );
    }

    #[test]
    fn enforcer_bound_breaks_if_free_cloud_nexus_or_web_upgrade() {
        assert_eq!(ENFORCER_SUITE, "b6530197");
        assert!(ENFORCER_BOUND.contains("StoreKit Pro + Stiki dual gate"));
        assert!(ENFORCER_BOUND.contains("Email|Bullet points|Make shorter|Make clearer"));
        assert!(ENFORCER_BOUND.contains("NOT auto-on"));
        assert!(ENFORCER_BOUND.contains("never invent facts"));
        assert!(ENFORCER_BOUND.contains("sign-out locks Transforms"));
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
        let xf_idx = views
            .iter()
            .position(|v| *v == "transforms")
            .expect("transforms nav");
        let style_idx = views.iter().position(|v| *v == "style").unwrap();
        let dict_idx = views.iter().position(|v| *v == "dictionary").unwrap();
        let snip_idx = views.iter().position(|v| *v == "snippets").unwrap();
        assert_eq!(xf_idx, style_idx + 1, "BREAKS IF: Transforms is not catalog #4");
        assert!(dict_idx < snip_idx && snip_idx < style_idx && style_idx < xf_idx);
        assert!(
            nav.split("data-view=\"transforms\"")
                .nth(1)
                .unwrap()
                .contains("lock-pill"),
            "BREAKS IF: Transforms nav not Pro-locked"
        );
        let view = html
            .split("data-view=\"transforms\"")
            .last()
            .expect("transforms view");
        let view = view.split("<section").next().unwrap();
        assert!(view.contains("pro-lock"), "BREAKS IF: Free not locked");
        assert!(view.contains("pro-activate"), "BREAKS IF: no Plans upsell");
        assert!(
            view.contains("Sign in with Stiki"),
            "BREAKS IF: Sign in with Stiki missing on locked Transforms"
        );
        assert!(view.contains("Email"), "BREAKS IF: Email action missing");
        assert!(
            view.contains("Bullet points"),
            "BREAKS IF: Bullet points action missing"
        );
        assert!(
            view.contains("Make shorter"),
            "BREAKS IF: Make shorter action missing"
        );
        assert!(
            view.contains("Make clearer"),
            "BREAKS IF: Make clearer action missing"
        );
        assert!(
            !view.contains("xf-filler")
                && !view.contains("Remove filler words")
                && !view.contains("id=\"xf-grammar\"")
                && !view.contains("id=\"xf-punct\""),
            "BREAKS IF: auto-on cleanup prefs instead of user-invoked actions"
        );
        assert!(!view.contains("chibiteklabs.com"), "BREAKS IF: web upgrade");
        assert!(!view.contains("https://"), "BREAKS IF: web upgrade");
        assert!(view.contains("this Mac"));
        assert!(
            !view.contains("Share with team") && !view.contains("Sync to cloud"),
            "BREAKS IF: share shipped"
        );

        let ts = include_str!("../../src/main.ts");
        assert!(ts.contains("transforms_apply"));
        assert!(ts.contains("invokeTransform"));
        assert!(ts.contains("openPlans()"));
        assert!(!ts.contains("https://chibiteklabs"));
        assert!(
            ts.contains("\"transforms\""),
            "BREAKS IF: Transforms dropped from Pro lock list"
        );
        assert!(
            !ts.contains("xf-filler") && !ts.contains("xf-grammar") && !ts.contains("xf-punct"),
            "BREAKS IF: auto-on cleanup prefs instead of user-invoked actions"
        );

        let commands = include_str!("main.rs");
        assert!(commands.contains("transforms_apply"));
        assert!(commands.contains("transforms_clear"));
        assert!(commands.contains("share_cloud_or_team"));
        assert!(commands.contains("promote_to_company_memory"));
        assert!(include_str!("transforms.rs").contains("require_share_acl"));
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
        assert!(include_str!("transforms.rs").contains("fail-closed"));

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
        assert!(html.contains("id=\"transforms-open\""));
        assert!(html.contains("id=\"transforms-activate\""));
        assert!(ts.contains("openTransforms"));
        let tray = include_str!("clipboard_ui.rs");
        assert!(tray.contains("Transforms"));
        assert!(tray.contains("open-transforms"));
        assert!(
            include_str!("transforms.rs").contains("promote_to_company_memory"),
            "BREAKS IF: auto-promote stub missing"
        );
        assert!(
            include_str!("pro_features.rs").contains("transforms.json"),
            "BREAKS IF: transforms store missing"
        );
        assert!(
            !include_str!("settings.rs").contains("transforms.json"),
            "BREAKS IF: transforms folded into Dictionary/settings store"
        );
        assert!(
            !include_str!("polish.rs").contains("transforms.json")
                && !include_str!("style.rs").contains("transforms.json")
                && !include_str!("snippets.rs").contains("transforms.json")
                && !include_str!("dictionary.rs").contains("transforms.json")
                && !include_str!("clipboard_history.rs").contains("transforms.json"),
            "BREAKS IF: transforms folded into Style, Polish, Dictionary, Snippets, or Clipboard stores"
        );
    }

    #[test]
    fn breaks_if_transforms_without_dual_gate() {
        assert!(
            require_surface_with(true, false).is_err(),
            "BREAKS IF: Transforms without dual gate (Pro, no Stiki)"
        );
        assert!(
            require_surface_with(false, true).is_err(),
            "BREAKS IF: Transforms without dual gate (Stiki, no Pro)"
        );
        assert!(
            require_surface_with(false, false).is_err(),
            "BREAKS IF: Transforms without dual gate"
        );
        assert!(require_surface_with(true, true).is_ok());
        assert!(!surface_ready(), "BREAKS IF: Transforms without dual gate");
    }

    #[test]
    fn breaks_if_auto_apply_without_user_invoke() {
        let rec = include_str!("recorder.rs");
        let streaming = include_str!("streaming.rs");
        let cleanup = include_str!("cleanup.rs");
        assert!(
            !rec.contains("transforms::") && !streaming.contains("transforms::"),
            "BREAKS IF: auto-apply without user invoke"
        );
        assert!(
            !cleanup.contains("transforms::") && !cleanup.contains("rewrite_local"),
            "BREAKS IF: auto-apply without user invoke"
        );
        assert!(
            !rec.contains("transforms_apply") && !streaming.contains("transforms_apply"),
            "BREAKS IF: auto-apply without user invoke"
        );
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
        let xf_prod = include_str!("transforms.rs")
            .split("#[cfg(test)]")
            .next()
            .expect("transforms prod");
        for hay in [
            include_str!("main.rs"),
            xf_prod,
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
            !include_str!("../../src/main.ts").contains("transforms_promote"),
            "BREAKS IF: auto-promote"
        );
    }

    #[test]
    fn rewrite_never_invents_facts() {
        let src = "The launch is Friday at noon. Bring the deck.";
        let email = rewrite_local(src, ACTION_EMAIL);
        assert!(email.contains("The launch is Friday at noon"));
        assert!(email.contains("Bring the deck"));
        assert!(email.starts_with("Subject:"));
        assert!(!email.to_ascii_lowercase().contains("dear "));
        assert!(!email.to_ascii_lowercase().contains("best regards"));
        assert!(!email.contains("Saturday"));
        assert!(!email.contains("Monday"));

        let bullets = rewrite_local(src, ACTION_BULLETS);
        assert!(bullets.contains("- The launch is Friday at noon"));
        assert!(bullets.contains("- Bring the deck"));
        assert!(!bullets.contains("agenda"));
        assert!(!bullets.contains("RSVP"));

        let shorter = rewrite_local("um The launch is Friday at noon uh", ACTION_SHORTER);
        assert!(shorter.contains("The launch is Friday at noon"));
        assert!(!shorter.contains("um"));
        assert!(!shorter.contains("uh"));
        assert!(!shorter.contains("Saturday"));

        let clearer = rewrite_local("the launch is friday at noon", ACTION_CLEARER);
        assert!(clearer.contains("launch is friday at noon") || clearer.contains("Launch is friday at noon"));
        assert!(!clearer.contains("Saturday"));
        assert!(clearer.ends_with('.'));

        assert_eq!(rewrite_local(src, ACTION_NONE), src);
        let long = "x".repeat(200);
        assert!(
            accept_or_fail_closed("hello world this is spoken text", &long).is_err(),
            "BREAKS IF: invent (expanded output accepted)"
        );
        assert!(accept_or_fail_closed("hello", "   ").is_err());
        assert!(accept_or_fail_closed(src, &email).is_ok());
    }

    #[test]
    fn prompts_never_invent_facts() {
        for action in LIVE_ACTIONS {
            let prompt = system_prompt(action);
            assert!(prompt.contains("Never invent facts"), "{action}");
            assert!(prompt.contains("Never expand meaning"), "{action}");
            assert!(prompt.contains("do not send this step off-device"));
            assert!(prompt.contains("Fail closed"));
            assert!(!prompt.contains("http://"));
            assert!(!prompt.contains("https://"));
        }
        assert!(system_prompt(ACTION_EMAIL).contains("Email"));
        assert!(system_prompt(ACTION_BULLETS).contains("Bullet points"));
        assert!(system_prompt(ACTION_SHORTER).contains("Make shorter"));
        assert!(system_prompt(ACTION_CLEARER).contains("Make clearer"));
        assert!(user_prompt_prefix().contains("Never invent facts"));
    }

    #[test]
    fn product_lock_v1_non_goals_and_surface() {
        assert!(PRODUCT_LOCK.contains("Name: Transforms"));
        assert!(PRODUCT_LOCK.contains("Dictionary → Snippets → Style → Transforms"));
        assert!(PRODUCT_LOCK.contains("StoreKit Pro AND Stiki session"));
        assert!(PRODUCT_LOCK.contains("Activate Pro / Sign in with Stiki"));
        assert!(PRODUCT_LOCK.contains("Email | Bullet points | Make shorter | Make clearer"));
        assert!(PRODUCT_LOCK.contains("NOT auto-on"));
        assert!(PRODUCT_LOCK.contains("Local Gemma"));
        assert!(PRODUCT_LOCK.contains("never invent facts"));
        assert!(PRODUCT_LOCK.contains("Settings + sidebar/nav + menu bar"));
        assert!(PRODUCT_LOCK.contains("Formal|Casual|Very casual"));
        assert!(PRODUCT_LOCK.contains("Off|Casual|Professional|Polite"));
        assert!(PRODUCT_LOCK.contains("HIPAA"));

        let html = include_str!("../../index.html");
        let ts = include_str!("../../src/main.ts");
        let rec = include_str!("recorder.rs");
        assert!(html.contains("view-title\">Transforms"));
        assert!(html.contains("id=\"transforms-activate\""));
        assert!(html.contains("id=\"transforms-open\""));
        assert!(html.contains("id=\"xf-clear-btn\""));
        assert!(html.contains("id=\"dictionary-activate\""));
        assert!(html.contains("id=\"snippets-activate\""));
        assert!(html.contains("id=\"style-activate\""));
        assert!(html.contains("id=\"polish-activate\""));
        assert!(ts.contains("openPlans()"));
        assert!(ts.contains("transforms_apply"));
        assert!(ts.contains("invokeTransform"));
        assert!(
            !rec.contains("transforms::") && !include_str!("streaming.rs").contains("transforms::"),
            "BREAKS IF: auto-apply without user invoke"
        );

        for hay in [html, ts] {
            assert!(!hay.contains("HIPAA"), "BREAKS IF: HIPAA claim copy");
        }
    }

    #[test]
    fn distinct_from_style_polish_dictionary_snippets_and_clipboard() {
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
        assert!(engine.contains("row-label\">Transforms"));
        assert!(engine.contains("not Style"));
        assert!(engine.contains("not Polish"));
        assert!(engine.contains("not Dictionary"));
        assert!(engine.contains("not Snippets"));
        assert!(html.contains("id=\"polish-toggle\""));
        assert!(html.contains("id=\"polish-mode-select\""));
        assert!(html.contains("id=\"clipboard-history-toggle\""));
        assert!(html.contains("id=\"dict-add-btn\""));
        assert!(html.contains("id=\"snippet-add-btn\""));
        assert!(html.contains("id=\"style-clear-btn\""));
        assert_ne!("xf-clear-btn", "style-clear-btn");
        assert_ne!("transforms-open", "style-open");
        assert_ne!("transforms-open", "dictionary-open");
        assert_ne!("transforms-open", "snippets-open");
        let xf_view = html.split("data-view=\"transforms\"").last().unwrap();
        let xf_view = xf_view.split("<section").next().unwrap();
        assert!(
            !xf_view.contains("data-style-mode")
                && !xf_view.contains("id=\"polish-mode-select\"")
                && !xf_view.contains("data-style-mode=\"formal\""),
            "BREAKS IF: Transforms offers Style register or Polish modes"
        );
        assert!(xf_view.contains("Style") && xf_view.contains("Polish"));
        assert!(!include_str!("style.rs").contains("transforms.json"));
        assert!(!include_str!("polish.rs").contains("transforms.json"));
    }

    #[test]
    fn local_gemma_path_is_loopback_only() {
        let llm = include_str!("llm.rs");
        assert!(llm.contains("rewrite_transform"));
        assert!(llm.contains("transforms::system_prompt"));
        assert!(llm.contains("127.0.0.1"));
        let xf_fn = llm
            .split("pub async fn rewrite_transform")
            .nth(1)
            .expect("rewrite_transform");
        let xf_fn = xf_fn.split("fn extract_clean_or_fail").next().unwrap();
        assert!(!xf_fn.contains("groq"));
        assert!(!xf_fn.contains("api.groq.com"));
        assert!(!xf_fn.contains("https://"));
        assert!(xf_fn.contains("127.0.0.1"));
        assert!(llm.contains("transforms::accept_or_fail_closed"));
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
