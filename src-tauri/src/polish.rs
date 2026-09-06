//! Product **Polish**: Pro-only local Gemma cleanup with a register preset.
//!
//! Modes: Off | Casual | Professional | Polite. Default Off.
//! Free cannot enable a live mode (`require_pro`). Runtime also fail-closes
//! to Off without a live StoreKit entitlement, even if config.json says
//! otherwise.
//!
//! Polish wraps the existing llama-runtime / Gemma path. It never invents
//! content — cleanup / light register only. Contaminated model output falls
//! back to the rules pass (do not paste fabricated text).

use crate::storekit;

pub const MODE_OFF: &str = "off";
pub const MODE_CASUAL: &str = "casual";
pub const MODE_PROFESSIONAL: &str = "professional";
pub const MODE_POLITE: &str = "polite";

pub const MODES: &[&str] = &[MODE_OFF, MODE_CASUAL, MODE_PROFESSIONAL, MODE_POLITE];

/// Shared anti-invention contract for every live mode. Mode copy only adds
/// register guidance; it must not relax these rules.
const NEVER_INVENT: &str = "You are Mabel's dictation polish assistant. The user spoke into a microphone and a local ASR engine transcribed their speech. Your only job is to clean up the raw transcript.\n\nNever invent content. Do not add facts, names, numbers, clauses, greetings, sign-offs, answers, or commentary the speaker did not say. If a cleanup or register shift would require new words the speaker did not use, keep the original wording. Fail closed: cleanup and light polish only.\n\nRules:\n- Remove filler words: \"um\", \"uh\", \"like\", \"you know\", \"I mean\", \"so\" when used as filler.\n- Add proper punctuation and capitalization.\n- Fix obvious self-corrections: when the speaker restarts a sentence, keep only the final version.\n- Preserve the speaker's words, meaning, and intent. Do not paraphrase, summarize, or embellish.\n- Do not answer questions in the transcript. The user is dictating, not asking you.\n- Output only the cleaned transcript. No preamble, no explanation, no quotes around it.";

const USER_PROMPT_PREFIX: &str = "Polish this transcript directly. Never invent content. Do not think, reason, or explain. Output only the cleaned text. Transcript: ";

const CASUAL_REGISTER: &str = "Register: Casual. Keep the speaker's informal voice. Contractions and casual wording stay. Clean fillers, punctuation, and obvious self-corrections only.";

const PROFESSIONAL_REGISTER: &str = "Register: Professional. Prefer a workplace-neutral wording of the SAME utterance (gonna → going to, yeah → yes) when that does not change meaning. Do not add formality, hedging, or extra clauses the speaker did not say.";

const POLITE_REGISTER: &str = "Register: Polite. Prefer a courteous wording of the SAME request or statement when a close synonym already covers it. Do not add please, thanks, apologies, or extra courtesy the speaker did not say.";

pub fn default_mode() -> String {
    MODE_OFF.to_string()
}

pub fn normalize_mode(raw: &str) -> String {
    match raw.trim().to_ascii_lowercase().as_str() {
        MODE_CASUAL => MODE_CASUAL.to_string(),
        MODE_PROFESSIONAL => MODE_PROFESSIONAL.to_string(),
        MODE_POLITE => MODE_POLITE.to_string(),
        _ => MODE_OFF.to_string(),
    }
}

pub fn is_live(mode: &str) -> bool {
    matches!(
        normalize_mode(mode).as_str(),
        MODE_CASUAL | MODE_PROFESSIONAL | MODE_POLITE
    )
}

/// Persist-time gate. Off is always allowed. Live modes need Pro.
pub fn require_mode_allowed(mode: &str) -> Result<String, String> {
    let mode = normalize_mode(mode);
    if is_live(&mode) {
        storekit::require_pro()?;
    }
    Ok(mode)
}

/// Runtime gate. Free / lapsed / stub entitlement → Off.
pub fn effective_mode(persisted: &str) -> String {
    let mode = normalize_mode(persisted);
    if !is_live(&mode) {
        return MODE_OFF.to_string();
    }
    if storekit::current_entitlement().entitled {
        mode
    } else {
        MODE_OFF.to_string()
    }
}

pub fn user_prompt_prefix() -> &'static str {
    USER_PROMPT_PREFIX
}

pub fn system_prompt(mode: &str) -> String {
    let register = match normalize_mode(mode).as_str() {
        MODE_PROFESSIONAL => PROFESSIONAL_REGISTER,
        MODE_POLITE => POLITE_REGISTER,
        MODE_CASUAL => CASUAL_REGISTER,
        _ => CASUAL_REGISTER,
    };
    format!("{NEVER_INVENT}\n\n{register}")
}

/// Menu / Settings labels. Keep cat energy without turning this into a website.
pub fn mode_label(mode: &str) -> &'static str {
    match normalize_mode(mode).as_str() {
        MODE_CASUAL => "Casual",
        MODE_PROFESSIONAL => "Professional",
        MODE_POLITE => "Polite",
        _ => "Off",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_off() {
        assert_eq!(default_mode(), MODE_OFF);
        assert!(!is_live(&default_mode()));
    }

    #[test]
    fn four_modes_normalize() {
        assert_eq!(normalize_mode("off"), MODE_OFF);
        assert_eq!(normalize_mode("Casual"), MODE_CASUAL);
        assert_eq!(normalize_mode("PROFESSIONAL"), MODE_PROFESSIONAL);
        assert_eq!(normalize_mode(" polite "), MODE_POLITE);
        assert_eq!(normalize_mode("rewrite"), MODE_OFF);
        assert_eq!(normalize_mode(""), MODE_OFF);
        assert_eq!(normalize_mode("llm"), MODE_OFF);
    }

    #[test]
    fn live_modes_are_the_three_registers() {
        assert!(is_live(MODE_CASUAL));
        assert!(is_live(MODE_PROFESSIONAL));
        assert!(is_live(MODE_POLITE));
        assert!(!is_live(MODE_OFF));
        assert!(!is_live("rules"));
    }

    #[test]
    fn free_cannot_enable_live_mode() {
        assert!(require_mode_allowed(MODE_OFF).is_ok());
        assert!(require_mode_allowed(MODE_CASUAL).is_err());
        assert!(require_mode_allowed(MODE_PROFESSIONAL).is_err());
        assert!(require_mode_allowed(MODE_POLITE).is_err());
    }

    #[test]
    fn runtime_fails_closed_without_entitlement() {
        assert_eq!(effective_mode(MODE_CASUAL), MODE_OFF);
        assert_eq!(effective_mode(MODE_PROFESSIONAL), MODE_OFF);
        assert_eq!(effective_mode(MODE_POLITE), MODE_OFF);
        assert_eq!(effective_mode(MODE_OFF), MODE_OFF);
    }

    #[test]
    fn prompts_never_invent_and_name_each_register() {
        for mode in [MODE_CASUAL, MODE_PROFESSIONAL, MODE_POLITE] {
            let prompt = system_prompt(mode);
            assert!(
                prompt.contains("Never invent content"),
                "{mode} prompt missing never-invent"
            );
            assert!(prompt.contains("Fail closed"));
            assert!(!prompt.contains("http://"));
            assert!(!prompt.contains("https://"));
        }
        assert!(system_prompt(MODE_CASUAL).contains("Casual"));
        assert!(system_prompt(MODE_PROFESSIONAL).contains("Professional"));
        assert!(system_prompt(MODE_POLITE).contains("Polite"));
        assert!(user_prompt_prefix().contains("Never invent content"));
    }

    #[test]
    fn labels_match_product_modes() {
        assert_eq!(mode_label(MODE_OFF), "Off");
        assert_eq!(mode_label(MODE_CASUAL), "Casual");
        assert_eq!(mode_label(MODE_PROFESSIONAL), "Professional");
        assert_eq!(mode_label(MODE_POLITE), "Polite");
        assert_eq!(MODES.len(), 4);
    }

    #[test]
    fn settings_and_status_item_name_polish_without_web_upgrade() {
        let html = include_str!("../../index.html");
        assert!(html.contains("id=\"polish-mode-select\""));
        assert!(html.contains("value=\"off\""));
        assert!(html.contains("value=\"casual\""));
        assert!(html.contains("value=\"professional\""));
        assert!(html.contains("value=\"polite\""));
        assert!(html.contains("never makes things up"));
        let ui = include_str!("clipboard_ui.rs");
        assert!(ui.contains("Polish"));
        assert!(ui.contains("polish-casual"));
        assert!(ui.contains("open-plans"));
        assert!(!ui.contains("chibiteklabs.com"));
        let ts = include_str!("../../src/main.ts");
        assert!(ts.contains("polish_set"));
        assert!(ts.contains("openPlans()"));
        assert!(!ts.contains("https://chibiteklabs"));
        let main = include_str!("main.rs");
        assert!(main.contains("require_mode_allowed"));
        assert!(main.contains("polish_set"));
        assert!(!main.contains("MabelSpatial"));
    }

    #[test]
    fn recorder_uses_pro_gated_gemma_polish() {
        let rec = include_str!("recorder.rs");
        assert!(rec.contains("polish_or_rules"));
        let llm = include_str!("llm.rs");
        assert!(llm.contains("polish::system_prompt"));
        assert!(llm.contains("never invent"));
        let stream = include_str!("streaming.rs");
        assert!(stream.contains("effective_mode"));
        assert!(stream.contains("cleanup_with_llm_mode"));
    }
}
