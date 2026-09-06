/// Local on-device engine ids persisted as `localEngine`.
///
/// New installs default to Parakeet. Existing configs that predate this
/// field stay on whisper-cpp so 1.2.0 users keep their ggml Q5 path.

pub const PARAKEET: &str = "parakeet";
pub const WHISPERKIT: &str = "whisperkit";
pub const WHISPER_CPP: &str = "whisper-cpp";

pub const ALLOWED: &[&str] = &[PARAKEET, WHISPERKIT, WHISPER_CPP];

/// Default for `Settings::default()` / brand-new installs.
pub fn default_new_install() -> String {
    PARAKEET.to_string()
}

/// Missing `localEngine` on disk means a pre-Phase-B config.
pub fn migrate_missing_field() -> String {
    WHISPER_CPP.to_string()
}

pub fn validate(id: &str) -> Result<&str, String> {
    if ALLOWED.contains(&id) {
        Ok(id)
    } else {
        Err(format!("Invalid local engine: {}", id))
    }
}

pub fn is_native_coreml(id: &str) -> bool {
    id == PARAKEET || id == WHISPERKIT
}

/// whisper.cpp sidecar is compiled into the Developer ID / default feature set.
/// The MAS flavor builds `--no-default-features` so this is false there.
pub fn whisper_cpp_sidecar_compiled() -> bool {
    cfg!(feature = "whisper-cpp-sidecar")
}

/// Parakeet TDT version FluidAudio should load for a language setting.
/// English-only → v2 (higher English recall). Multilingual → v3.
pub fn parakeet_version(language: &str) -> &'static str {
    match language {
        "en" => "v2",
        _ => "v3",
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct LocalEngineInfo {
    pub id: String,
    pub label: String,
    pub available: bool,
    pub mas_clean: bool,
}

pub fn catalog() -> Vec<LocalEngineInfo> {
    vec![
        LocalEngineInfo {
            id: PARAKEET.to_string(),
            label: "Parakeet (default, on-device)".to_string(),
            available: cfg!(target_os = "macos"),
            mas_clean: true,
        },
        LocalEngineInfo {
            id: WHISPERKIT.to_string(),
            label: "WhisperKit large-v3-turbo".to_string(),
            available: cfg!(target_os = "macos"),
            mas_clean: true,
        },
        LocalEngineInfo {
            id: WHISPER_CPP.to_string(),
            label: "whisper.cpp Large v3 Q5 (Developer ID fallback)".to_string(),
            available: whisper_cpp_sidecar_compiled(),
            mas_clean: false,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_install_defaults_to_parakeet() {
        assert_eq!(default_new_install(), "parakeet");
        assert!(is_native_coreml(&default_new_install()));
    }

    #[test]
    fn existing_configs_keep_whisper_cpp() {
        assert_eq!(migrate_missing_field(), "whisper-cpp");
        assert!(!is_native_coreml(&migrate_missing_field()));
    }

    #[test]
    fn validate_accepts_known_ids() {
        assert_eq!(validate("parakeet").unwrap(), "parakeet");
        assert_eq!(validate("whisperkit").unwrap(), "whisperkit");
        assert_eq!(validate("whisper-cpp").unwrap(), "whisper-cpp");
        assert!(validate("whisper").is_err());
        assert!(validate("../evil").is_err());
    }

    #[test]
    fn parakeet_version_follows_language() {
        assert_eq!(parakeet_version("en"), "v2");
        assert_eq!(parakeet_version("multi"), "v3");
        assert_eq!(parakeet_version("auto"), "v3");
    }

    #[test]
    fn catalog_marks_only_coreml_engines_mas_clean() {
        let cat = catalog();
        let parakeet = cat.iter().find(|e| e.id == PARAKEET).unwrap();
        let whisperkit = cat.iter().find(|e| e.id == WHISPERKIT).unwrap();
        let cpp = cat.iter().find(|e| e.id == WHISPER_CPP).unwrap();
        assert!(parakeet.mas_clean);
        assert!(whisperkit.mas_clean);
        assert!(!cpp.mas_clean);
        assert_eq!(cpp.available, cfg!(feature = "whisper-cpp-sidecar"));
    }

    #[test]
    fn mas_entitlements_omit_forbidden_keys() {
        let plist = include_str!("../entitlements.mas.plist");
        assert!(
            plist.contains("<key>com.apple.security.app-sandbox</key>"),
            "MAS flavor must enable App Sandbox"
        );
        assert!(
            !plist.contains("<key>com.apple.security.cs.disable-library-validation</key>"),
            "MAS entitlements must not disable library validation"
        );
        assert!(
            !plist.contains("<key>com.apple.security.cs.allow-unsigned-executable-memory</key>"),
            "MAS entitlements must not allow unsigned executable memory"
        );
    }

    #[test]
    fn dmg_entitlements_keep_sidecar_exceptions() {
        let plist = include_str!("../entitlements.plist");
        assert!(plist.contains("<key>com.apple.security.cs.disable-library-validation</key>"));
        assert!(
            plist.contains("<key>com.apple.security.cs.allow-unsigned-executable-memory</key>")
        );
        assert!(
            !plist.contains("<key>com.apple.security.app-sandbox</key>"),
            "Developer ID DMG stays unsandboxed"
        );
    }

    #[test]
    fn mas_overlay_drops_whisper_cpp_sidecar() {
        let overlay = include_str!("../tauri.mas.conf.json");
        let v: serde_json::Value = serde_json::from_str(overlay).unwrap();
        let external = v["bundle"]["externalBin"].as_array().unwrap();
        assert!(external.is_empty());
        let frameworks = v["bundle"]["macOS"]["frameworks"].as_array().unwrap();
        assert!(
            frameworks
                .iter()
                .all(|f| !f.as_str().unwrap_or("").contains("ggml")
                    && !f.as_str().unwrap_or("").contains("libwhisper"))
        );
        assert_eq!(v["bundle"]["macOS"]["entitlements"], "entitlements.mas.plist");
    }

    #[test]
    fn info_plist_declares_microphone_and_apple_events_usage() {
        let plist = include_str!("../Info.plist");
        assert!(
            plist.contains("<key>NSMicrophoneUsageDescription</key>"),
            "Without NSMicrophoneUsageDescription, TCC will not prompt and HAL goes silent"
        );
        assert!(plist.contains("<key>NSAppleEventsUsageDescription</key>"));
    }

    #[test]
    fn mas_entitlements_keep_mic_and_system_events_paste() {
        let plist = include_str!("../entitlements.mas.plist");
        assert!(
            plist.contains("<key>com.apple.security.device.audio-input</key>"),
            "MAS flavor must request the microphone entitlement; without it TCC denies silently"
        );
        assert!(
            plist.contains("<key>com.apple.security.temporary-exception.apple-events</key>"),
            "MAS sandbox blocks System Events paste without a temporary Apple Events exception"
        );
        assert!(
            plist.contains("com.apple.systemevents"),
            "Apple Events exception must name System Events"
        );
    }

    #[test]
    fn mabel_asr_swift6_fixes_match_cio_report() {
        let swift = include_str!("../../native/MabelASR/Sources/MabelASR/MabelASR.swift");
        assert!(
            swift.contains("typealias mabel_asr_progress_cb"),
            "C callback must be a Swift typealias — mixed-language header is not visible"
        );
        assert!(
            swift.contains("LastErrorBox"),
            "lastErrorC must live in a locked Sendable box"
        );
        assert!(
            swift.contains("await manager.isAvailable") || swift.contains("await mgr.isAvailable"),
            "AsrManager.isAvailable is actor-isolated"
        );
        assert!(
            !swift.contains("$0.text?"),
            "WhisperKit text is String, not String?"
        );
        assert!(swift.contains("[TranscriptionResult]"));
        assert!(
            swift.contains("decoderState: &state"),
            "FluidAudio 0.15.6 AsrManager.transcribe takes decoderState, not source:"
        );
        assert!(
            swift.contains("actor ParakeetWarmSession"),
            "Parakeet must keep one AsrManager warm across takes"
        );
        assert!(
            swift.contains("parakeetWarm.transcribe"),
            "each take must reuse the warm Parakeet session"
        );
        assert!(
            swift.contains("WhisperKitWarm"),
            "WhisperKit must keep one kit warm across takes"
        );
        assert!(
            !swift.contains("await manager.cleanup()"),
            "do not teardown / unload AsrManager between takes"
        );
        assert!(
            swift.contains("parakeetLanguage(from: languageHint)"),
            "C language string must map to FluidAudio Language?, not String?"
        );
        assert!(swift.contains("Language(rawValue:"));
        assert!(swift.contains("-> Language?"));
        assert!(!swift.contains("source: .system"));
        let manifest = include_str!("../../native/MabelASR/Package.swift");
        assert!(
            !manifest.contains("publicHeadersPath"),
            "pure Swift target so mabel_asr_progress_cb is defined in Swift"
        );
        let docs = include_str!("../../docs/app-store-iap.md");
        assert!(docs.contains("npm run vendor-asr"));
        assert!(docs.contains("--product MabelASR"));
    }
}
