use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

use crate::secrets;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Settings {
    pub microphone: String,
    pub engine: String,
    /// On-device backend when `engine == "local"`.
    /// New installs: `parakeet`. Existing 1.2 configs without this field
    /// migrate to `whisper-cpp` so their ggml Q5 path keeps working.
    #[serde(rename = "localEngine", default = "crate::local_engine::default_new_install")]
    pub local_engine: String,
    #[serde(rename = "whisperModel")]
    pub whisper_model: String,
    #[serde(rename = "groqApiKey")]
    pub groq_api_key: String,
    #[serde(rename = "recordingMode")]
    pub recording_mode: String,
    pub hotkey: String,
    #[serde(default = "default_streaming")]
    pub streaming: bool,
    /// Tracks "is a Groq key in the keychain?" without ever actually reading the
    /// keychain, so the UI can show "Saved" status without prompting.
    #[serde(rename = "groqKeyConfigured", default)]
    pub groq_key_configured: bool,
    #[serde(rename = "launchAtLogin", default)]
    pub launch_at_login: bool,
    #[serde(rename = "showInDock", default = "default_true")]
    pub show_in_dock: bool,
    #[serde(rename = "dictationSounds", default = "default_true")]
    pub dictation_sounds: bool,
    #[serde(rename = "pressEnterCommand", default)]
    pub press_enter_command: bool,
    /// "rules" (default) keeps the existing rule-based pass only.
    /// "llm" runs the rule pass first then a local LLM cleanup pass.
    #[serde(rename = "cleanupMode", default = "default_cleanup_mode")]
    pub cleanup_mode: String,
    /// "light" or "standard" — only consulted when cleanup_mode == "llm".
    #[serde(rename = "llmModel", default = "default_llm_model")]
    pub llm_model: String,
    /// Product Polish: "off" | "casual" | "professional" | "polite".
    /// Default off. Live modes are Pro-only and drive the local Gemma path.
    /// Distinct from `clipboardHistoryEnabled` — do not merge those toggles.
    /// Local only — not Nexus / company memory. Coach cannot rewrite
    /// dictation via this setting. Do not merge with a future Nexus polish toggle.
    #[serde(rename = "polishMode", default = "crate::polish::default_mode")]
    pub polish_mode: String,
    /// Desktop companion (animated cat) toggle. Default off so we don't surprise
    /// users on update.
    #[serde(rename = "companionEnabled", default)]
    pub companion_enabled: bool,
    /// "small" | "medium" | "large" — fraction of screen height for the cat.
    #[serde(rename = "companionSize", default = "default_companion_size")]
    pub companion_size: String,
    /// "15min" | "30min" | "1hr" | "2hr" — how often the cat appears.
    #[serde(rename = "companionFrequency", default = "default_companion_frequency")]
    pub companion_frequency: String,
    /// "short" | "medium" | "long" — how long each visit lasts.
    #[serde(rename = "companionVisit", default = "default_companion_visit")]
    pub companion_visit: String,
    /// Last Mabel version the user actually saw the "What's New" popup for.
    /// On launch we compare this to the running version — if they differ, show
    /// the popup with the changelog entries between them, then update this.
    #[serde(rename = "lastSeenVersion", default)]
    pub last_seen_version: String,
    /// "en" (force English decoder — better accuracy on English) or
    /// "multi" (auto-detect). Serde default is "multi" so existing installs
    /// keep their downloaded ggml-{size}.bin; brand-new Settings::default
    /// and first-run persist "en". large-v3 Q5 is multilingual-only on disk.
    #[serde(rename = "whisperLanguage", default = "default_whisper_language")]
    pub whisper_language: String,
    /// Custom dictionary words. Prepended to whisper.cpp's `--prompt` so
    /// proper nouns, acronyms, and jargon get spelled correctly. Stored
    /// locally only.
    #[serde(default)]
    pub dictionary: Vec<String>,
    /// Opt-in Mac clipboard history. Default off. When off, Mabel must not
    /// poll or read pasteboard contents. Turning off wipes the local store.
    /// Distinct from any future Nexus clipboard toggle — do not merge.
    #[serde(rename = "clipboardHistoryEnabled", default)]
    pub clipboard_history_enabled: bool,
}

fn default_streaming() -> bool { false }
fn default_true() -> bool { true }
fn default_cleanup_mode() -> String { "rules".to_string() }
fn default_llm_model() -> String { "standard".to_string() }
fn default_companion_size() -> String { "medium".to_string() }
fn default_companion_frequency() -> String { "30min".to_string() }
fn default_companion_visit() -> String { "medium".to_string() }
fn default_whisper_language() -> String { "multi".to_string() }

/// What we actually serialize to disk. Excludes the Groq API key, which lives
/// in the OS keychain. Keeps the same JSON shape the UI expects, minus the key.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct DiskSettings {
    microphone: String,
    engine: String,
    #[serde(rename = "localEngine", default)]
    local_engine: Option<String>,
    #[serde(rename = "whisperModel")]
    whisper_model: String,
    #[serde(rename = "recordingMode")]
    recording_mode: String,
    hotkey: String,
    #[serde(default = "default_streaming")]
    streaming: bool,
    #[serde(rename = "groqKeyConfigured", default)]
    groq_key_configured: bool,
    #[serde(rename = "launchAtLogin", default)]
    launch_at_login: bool,
    #[serde(rename = "showInDock", default = "default_true")]
    show_in_dock: bool,
    #[serde(rename = "dictationSounds", default = "default_true")]
    dictation_sounds: bool,
    #[serde(rename = "pressEnterCommand", default)]
    press_enter_command: bool,
    #[serde(rename = "cleanupMode", default = "default_cleanup_mode")]
    cleanup_mode: String,
    #[serde(rename = "llmModel", default = "default_llm_model")]
    llm_model: String,
    #[serde(rename = "polishMode", default = "crate::polish::default_mode")]
    polish_mode: String,
    #[serde(rename = "companionEnabled", default)]
    companion_enabled: bool,
    #[serde(rename = "companionSize", default = "default_companion_size")]
    companion_size: String,
    #[serde(rename = "companionFrequency", default = "default_companion_frequency")]
    companion_frequency: String,
    #[serde(rename = "companionVisit", default = "default_companion_visit")]
    companion_visit: String,
    #[serde(rename = "lastSeenVersion", default)]
    last_seen_version: String,
    #[serde(rename = "whisperLanguage", default = "default_whisper_language")]
    whisper_language: String,
    #[serde(default)]
    dictionary: Vec<String>,
    #[serde(rename = "clipboardHistoryEnabled", default)]
    clipboard_history_enabled: bool,
}

impl From<&Settings> for DiskSettings {
    fn from(s: &Settings) -> Self {
        Self {
            microphone: s.microphone.clone(),
            engine: s.engine.clone(),
            local_engine: Some(s.local_engine.clone()),
            whisper_model: s.whisper_model.clone(),
            recording_mode: s.recording_mode.clone(),
            hotkey: s.hotkey.clone(),
            streaming: s.streaming,
            groq_key_configured: s.groq_key_configured,
            launch_at_login: s.launch_at_login,
            show_in_dock: s.show_in_dock,
            dictation_sounds: s.dictation_sounds,
            press_enter_command: s.press_enter_command,
            cleanup_mode: s.cleanup_mode.clone(),
            llm_model: s.llm_model.clone(),
            polish_mode: s.polish_mode.clone(),
            companion_enabled: s.companion_enabled,
            companion_size: s.companion_size.clone(),
            companion_frequency: s.companion_frequency.clone(),
            companion_visit: s.companion_visit.clone(),
            last_seen_version: s.last_seen_version.clone(),
            whisper_language: s.whisper_language.clone(),
            dictionary: s.dictionary.clone(),
            clipboard_history_enabled: s.clipboard_history_enabled,
        }
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            microphone: "default".to_string(),
            engine: "local".to_string(),
            local_engine: crate::local_engine::default_new_install(),
            whisper_model: crate::transcribe_local::recommended_model_size().to_string(),
            groq_api_key: String::new(),
            recording_mode: "toggle".to_string(),
            hotkey: "CmdOrCtrl+D".to_string(),
            streaming: false,
            groq_key_configured: false,
            launch_at_login: false,
            show_in_dock: true,
            dictation_sounds: true,
            press_enter_command: false,
            cleanup_mode: default_cleanup_mode(),
            llm_model: default_llm_model(),
            polish_mode: crate::polish::default_mode(),
            companion_enabled: false,
            companion_size: default_companion_size(),
            companion_frequency: default_companion_frequency(),
            companion_visit: default_companion_visit(),
            last_seen_version: String::new(),
            whisper_language: "en".to_string(),
            dictionary: Vec::new(),
            clipboard_history_enabled: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SettingsLoad {
    pub settings: Settings,
    pub error: Option<String>,
}

impl Settings {
    pub fn config_path(app_dir: &PathBuf) -> PathBuf {
        app_dir.join("config.json")
    }

    pub fn load(app_dir: &PathBuf) -> Self {
        Self::load_with_status(app_dir).settings
    }

    /// Load settings. A present-but-unparseable `config.json` is never
    /// overwritten with defaults — that used to look like an upgrade wipe.
    pub fn load_with_status(app_dir: &PathBuf) -> SettingsLoad {
        let path = Self::config_path(app_dir);
        let (mut settings, mut needs_migration, error) = match fs::read_to_string(&path) {
            Ok(contents) => {
                // Detect plaintext groqApiKey in old config.json and stage it
                // for migration into the keychain.
                let legacy_key = serde_json::from_str::<serde_json::Value>(&contents)
                    .ok()
                    .and_then(|v| {
                        v.get("groqApiKey")
                            .and_then(|x| x.as_str())
                            .filter(|s| !s.is_empty())
                            .map(|s| s.to_string())
                    });

                match serde_json::from_str::<DiskSettings>(&contents) {
                    Ok(d) => {
                        let parsed = Settings {
                            microphone: d.microphone,
                            engine: d.engine,
                            local_engine: d
                                .local_engine
                                .unwrap_or_else(crate::local_engine::migrate_missing_field),
                            whisper_model: d.whisper_model,
                            groq_api_key: String::new(),
                            recording_mode: d.recording_mode,
                            hotkey: d.hotkey,
                            streaming: d.streaming,
                            groq_key_configured: d.groq_key_configured,
                            launch_at_login: d.launch_at_login,
                            show_in_dock: d.show_in_dock,
                            dictation_sounds: d.dictation_sounds,
                            press_enter_command: d.press_enter_command,
                            cleanup_mode: d.cleanup_mode,
                            llm_model: d.llm_model,
                            polish_mode: crate::polish::normalize_mode(&d.polish_mode),
                            companion_enabled: d.companion_enabled,
                            companion_size: d.companion_size,
                            companion_frequency: d.companion_frequency,
                            companion_visit: d.companion_visit,
                            last_seen_version: d.last_seen_version,
                            whisper_language: d.whisper_language,
                            dictionary: d.dictionary,
                            clipboard_history_enabled: d.clipboard_history_enabled,
                        };
                        if let Some(key) = legacy_key {
                            let _ = secrets::set_groq_key(&key);
                            (parsed, true, None)
                        } else {
                            (parsed, false, None)
                        }
                    }
                    Err(e) => {
                        let msg = format!(
                            "Could not read settings in {}. The file was left untouched so nothing was wiped. ({e})",
                            path.display()
                        );
                        (Self::default(), false, Some(msg))
                    }
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => (Self::default(), false, None),
            Err(e) => {
                let msg = format!(
                    "Could not read settings in {}. The file was left untouched so nothing was wiped. ({e})",
                    path.display()
                );
                (Self::default(), false, Some(msg))
            }
        };

        // Backward-compat migration: older builds stored whisperLanguage as
        // "auto". Internally we now persist "multi" for that behavior.
        if settings.whisper_language == "auto" {
            settings.whisper_language = "multi".to_string();
            needs_migration = true;
        }

        if crate::local_engine::validate(&settings.local_engine).is_err() {
            settings.local_engine = crate::local_engine::default_new_install();
            needs_migration = true;
        }

        if settings.local_engine == crate::local_engine::WHISPER_CPP
            && !crate::local_engine::whisper_cpp_sidecar_compiled()
        {
            settings.local_engine = crate::local_engine::default_new_install();
            needs_migration = true;
        }

        // Live streaming is temporarily disabled while the VAD worker shutdown
        // path is stabilized. Force older persisted configs back to the reliable
        // full-utterance path at backend load time, not only from the UI.
        if settings.streaming {
            settings.streaming = false;
            needs_migration = true;
        }

        // Don't proactively read the Groq key from the keychain on startup.
        // Unsigned dev builds get a new binary signature on every rebuild, which
        // causes macOS to prompt for keychain access repeatedly and can hang the
        // launch. The key is fetched on-demand at cloud transcription time, and
        // the UI shows a "key is set" status without exposing the value.
        // Never persist defaults over an unreadable file.
        if error.is_none() && needs_migration {
            let _ = settings.save(app_dir);
        }

        SettingsLoad { settings, error }
    }


    pub fn save(&self, app_dir: &PathBuf) -> Result<(), String> {
        let path = Self::config_path(app_dir);
        fs::create_dir_all(app_dir).map_err(|e| e.to_string())?;
        // If a non-empty key came in, flip the configured flag so the UI can
        // show "Saved" without ever reading the keychain back.
        let mut to_disk = self.clone();
        if !to_disk.groq_api_key.is_empty() {
            to_disk.groq_key_configured = true;
        }
        let disk: DiskSettings = (&to_disk).into();
        let json = serde_json::to_string_pretty(&disk).map_err(|e| e.to_string())?;
        fs::write(&path, json).map_err(|e| e.to_string())?;
        // Only touch the keychain if the caller actually provided a non-empty
        // key. Settings saves from the UI ship an empty groq_api_key on every
        // change (mic, mode, hotkey, etc) and we don't want each one to trigger
        // a keychain prompt on unsigned dev builds.
        if !self.groq_api_key.is_empty() {
            secrets::set_groq_key(&self.groq_api_key)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_settings() {
        let settings = Settings::default();
        assert_eq!(settings.microphone, "default");
        assert_eq!(settings.engine, "local");
        assert_eq!(settings.local_engine, "parakeet");
        assert_eq!(settings.whisper_model, "large-v3");
        assert_eq!(settings.whisper_language, "en");
        assert_eq!(settings.groq_api_key, "");
        assert_eq!(settings.recording_mode, "toggle");
        assert_eq!(settings.hotkey, "CmdOrCtrl+D");
        assert!(!settings.streaming);
        assert!(
            !settings.clipboard_history_enabled,
            "clipboard history is opt-in and must default off"
        );
        assert_eq!(
            settings.polish_mode, "off",
            "BREAKS IF: default ON"
        );
        assert_ne!(
            settings.polish_mode.as_str(),
            if settings.clipboard_history_enabled { "on" } else { "clipboard" },
            "clipboardHistoryEnabled is not Polish"
        );
    }

    #[test]
    fn missing_polish_mode_field_defaults_off() {
        let json = r#"{
            "microphone": "default",
            "engine": "local",
            "whisperModel": "large-v3",
            "recordingMode": "toggle",
            "hotkey": "CmdOrCtrl+D"
        }"#;
        let parsed: DiskSettings = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.polish_mode, "off");
    }

    #[test]
    fn missing_clipboard_history_field_defaults_off() {
        let json = r#"{
            "microphone": "default",
            "engine": "local",
            "whisperModel": "large-v3",
            "recordingMode": "toggle",
            "hotkey": "CmdOrCtrl+D"
        }"#;
        let parsed: DiskSettings = serde_json::from_str(json).unwrap();
        assert!(!parsed.clipboard_history_enabled);
    }

    #[test]
    fn test_disk_settings_excludes_groq_key() {
        let settings = Settings {
            groq_api_key: "secret-key".to_string(),
            ..Settings::default()
        };
        let disk: DiskSettings = (&settings).into();
        let json = serde_json::to_string(&disk).unwrap();
        assert!(!json.contains("groqApiKey"));
        assert!(!json.contains("secret-key"));
    }

    #[test]
    fn test_disk_round_trip_preserves_non_secret_fields() {
        let mut settings = Settings::default();
        settings.engine = "cloud".to_string();
        settings.whisper_model = "medium".to_string();
        settings.recording_mode = "push-to-talk".to_string();

        let disk: DiskSettings = (&settings).into();
        let json = serde_json::to_string(&disk).unwrap();
        let parsed: DiskSettings = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, disk);
    }

    #[test]
    fn missing_local_engine_field_migrates_to_whisper_cpp() {
        let json = r#"{
            "microphone": "default",
            "engine": "local",
            "whisperModel": "large-v3",
            "recordingMode": "toggle",
            "hotkey": "CmdOrCtrl+D"
        }"#;
        let parsed: DiskSettings = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.local_engine, None);
        let migrated = parsed
            .local_engine
            .clone()
            .unwrap_or_else(crate::local_engine::migrate_missing_field);
        assert_eq!(migrated, "whisper-cpp");
    }

    #[test]
    fn unreadable_config_is_not_overwritten() {
        let dir = std::env::temp_dir().join(format!(
            "mabel-settings-corrupt-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let path = Settings::config_path(&dir);
        fs::write(&path, "{not-json").unwrap();
        let loaded = Settings::load_with_status(&dir);
        assert!(loaded.error.is_some(), "{:?}", loaded.error);
        assert!(
            loaded.error.as_deref().unwrap().contains("left untouched"),
            "{:?}",
            loaded.error
        );
        assert_eq!(fs::read_to_string(&path).unwrap(), "{not-json");
        let _ = fs::remove_dir_all(&dir);
    }
}
