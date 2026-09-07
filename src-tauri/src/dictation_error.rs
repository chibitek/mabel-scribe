//! User-visible dictation failures. The MAS/TF empty-record path used to
//! fail open (overlay closes, no text, no reason). Every empty-capture,
//! denied-mic, missing-model, and paste failure must go through here.

use tauri::{AppHandle, Emitter};

pub const TITLE_MIC: &str = "Mic access needed";
pub const TITLE_MODEL: &str = "Model not ready";
pub const TITLE_CAPTURE: &str = "No audio captured";
pub const TITLE_EMPTY: &str = "Nothing recognized";
pub const TITLE_CLOUD: &str = "Cloud engine failed";
pub const TITLE_PASTE: &str = "Paste blocked";
pub const TITLE_GENERIC: &str = "Dictation failed";

/// Too short for a CoreAudio IOProc to deliver a buffer even with a live mic.
pub const SHORT_RECORDING_MS: u64 = 250;

#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
pub struct UserError {
    pub title: String,
    pub message: String,
}

impl UserError {
    pub fn new(title: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            message: message.into(),
        }
    }

    pub fn generic(message: impl Into<String>) -> Self {
        Self::new(TITLE_GENERIC, message)
    }
}

impl From<UserError> for String {
    fn from(value: UserError) -> Self {
        value.message
    }
}

pub fn emit(app: &AppHandle, err: &UserError) {
    eprintln!("[Mabel] {}: {}", err.title, err.message);
    let _ = app.emit("transcription-error", err.clone());
}

pub fn mic_denied() -> UserError {
    UserError::new(
        TITLE_MIC,
        "Mabel does not have Microphone access, so nothing was recorded. \
         Grant access in System Settings → Privacy & Security → Microphone, then try again.",
    )
}

pub fn model_missing(engine_label: &str) -> UserError {
    UserError::new(
        TITLE_MODEL,
        format!(
            "{engine_label} is not downloaded yet. Open Mabel → Settings → Engine and download \
             the model before dictating. Stop will not produce text until the model is ready."
        ),
    )
}

pub fn whisper_cpp_excluded() -> UserError {
    UserError::new(
        TITLE_MODEL,
        "whisper.cpp is not in this Mac App Store / TestFlight build. \
         Switch the local engine to Parakeet or WhisperKit in Settings → Engine.",
    )
}

pub fn cloud_key_missing() -> UserError {
    UserError::new(
        TITLE_CLOUD,
        "Cloud transcription is selected but no Groq API key is saved. \
         Add a key in Settings → Engine, or switch to Parakeet.",
    )
}

/// Keychain / Groq auth failure. Must not be classified as "Nothing recognized".
pub fn cloud_unavailable(detail: &str) -> UserError {
    let lower = detail.to_lowercase();
    if lower.contains("keychain") {
        return UserError::new(
            TITLE_CLOUD,
            format!(
                "Cloud transcription could not read the Groq API key ({detail}). \
                 Unlock the macOS login keychain, re-save the key in Settings → Engine, \
                 or switch to Parakeet."
            ),
        );
    }
    UserError::new(
        TITLE_CLOUD,
        format!(
            "Cloud transcription failed ({detail}). \
             Check the Groq API key in Settings → Engine, or switch to Parakeet."
        ),
    )
}

pub fn capture_empty(elapsed_ms: u64) -> UserError {
    if elapsed_ms < SHORT_RECORDING_MS {
        return UserError::new(
            TITLE_CAPTURE,
            "Recording was too short to capture audio. Press the hotkey, speak, then stop.",
        );
    }
    UserError::new(
        TITLE_CAPTURE,
        "No audio reached Mabel. Grant Microphone access in System Settings → Privacy & Security \
         → Microphone, or pick another input in Settings → General.",
    )
}

pub fn capture_silent(elapsed_ms: u64, rms: f32) -> UserError {
    let _ = rms;
    if elapsed_ms < SHORT_RECORDING_MS {
        return capture_empty(elapsed_ms);
    }
    UserError::new(
        TITLE_CAPTURE,
        "The microphone produced only silence. This is what a Mac App Store / TestFlight build \
         does when Microphone TCC is denied or never prompted. Grant Microphone access for Mabel \
         in System Settings → Privacy & Security → Microphone, then dictate again.",
    )
}

pub fn nothing_recognized() -> UserError {
    UserError::new(
        TITLE_EMPTY,
        "Audio was captured but the speech model returned no text. Speak closer to the microphone, \
         or open Settings → Engine and confirm the model finished downloading.",
    )
}

pub fn paste_failed(detail: &str) -> UserError {
    UserError::new(
        TITLE_PASTE,
        format!(
            "Transcription succeeded but paste was blocked ({detail}). Grant Accessibility and \
             Automation (System Events) for Mabel in System Settings → Privacy & Security."
        ),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_recording_uses_too_short_copy() {
        let err = capture_empty(80);
        assert_eq!(err.title, TITLE_CAPTURE);
        assert!(err.message.contains("too short"), "{}", err.message);
    }

    #[test]
    fn empty_capture_mentions_microphone_settings() {
        let err = capture_empty(1_000);
        assert!(err.message.contains("Microphone"), "{}", err.message);
        assert!(!err.message.contains("too short"));
    }

    #[test]
    fn silent_capture_calls_out_tcc() {
        let err = capture_silent(2_000, 0.0);
        assert_eq!(err.title, TITLE_CAPTURE);
        assert!(
            err.message.contains("TCC") || err.message.contains("silence"),
            "{}",
            err.message
        );
        assert!(err.message.contains("Microphone"), "{}", err.message);
    }

    #[test]
    fn mic_denied_is_actionable() {
        let err = mic_denied();
        assert_eq!(err.title, TITLE_MIC);
        assert!(err.message.contains("Privacy & Security"));
    }

    #[test]
    fn model_missing_names_engine() {
        let err = model_missing("Parakeet");
        assert_eq!(err.title, TITLE_MODEL);
        assert!(err.message.contains("Parakeet"));
        assert!(err.message.contains("Settings"));
    }

    #[test]
    fn empty_transcript_does_not_look_like_success() {
        let err = nothing_recognized();
        assert_eq!(err.title, TITLE_EMPTY);
        assert!(err.message.contains("no text"));
    }

    #[test]
    fn paste_failed_mentions_ax_and_automation() {
        let err = paste_failed("AppleScript paste failed");
        assert_eq!(err.title, TITLE_PASTE);
        assert!(err.message.contains("Accessibility"));
        assert!(err.message.contains("System Events"));
    }

    #[test]
    fn cloud_keychain_fail_is_not_nothing_recognized() {
        let err = cloud_unavailable("Keychain read error: default keychain could not be found");
        assert_eq!(err.title, TITLE_CLOUD);
        assert_ne!(err.title, TITLE_EMPTY);
        assert!(
            err.message.contains("keychain") || err.message.contains("Keychain"),
            "{}",
            err.message
        );
        assert!(err.message.contains("Parakeet"), "{}", err.message);
    }

    #[test]
    fn cloud_key_missing_is_not_model_or_empty() {
        let err = cloud_key_missing();
        assert_eq!(err.title, TITLE_CLOUD);
        assert_ne!(err.title, TITLE_EMPTY);
        assert_ne!(err.title, TITLE_MODEL);
    }
}
