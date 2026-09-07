import Foundation

/// Snapshot of system microphone permission. Never start audio on denied.
enum MicrophoneAuth: Equatable {
    case granted
    case denied
    case undetermined
}

/// Snapshot of Speech Recognition permission. On-device path only.
enum SpeechAuth: Equatable {
    case authorized
    case denied
    case restricted
    case undetermined
}

/// Fail-closed gate for Free keyboard dictation.
/// Ready is the only state that may power the microphone.
/// No account, StoreKit, or Stiki check belongs here — those are later Pro.
enum DictatePermissionGate: Equatable {
    case ready
    case needsFullAccess
    case needsMicrophone
    case needsSpeechRecognition
    case undeterminedOpenHost

    var allowsAudio: Bool { self == .ready }

    var userMessage: String {
        switch self {
        case .ready:
            return "Tap Start to dictate"
        case .needsFullAccess:
            return "Turn on Full Access for Mabel in Settings → General → Keyboard"
        case .needsMicrophone:
            return "Microphone is off. Open the Mabel app and allow it, or this keyboard stays silent."
        case .needsSpeechRecognition:
            return "Speech Recognition is off. Open the Mabel app and allow it."
        case .undeterminedOpenHost:
            return "Open the Mabel app to allow Microphone and Speech. This keyboard will not listen first."
        }
    }

    /// Keyboard: Full Access + granted mic + authorized speech.
    /// Undetermined or denied → no audio (fail closed, no prompt from the extension).
    static func evaluateKeyboard(
        hasFullAccess: Bool,
        microphone: MicrophoneAuth,
        speech: SpeechAuth
    ) -> DictatePermissionGate {
        if hasFullAccess == false {
            return .needsFullAccess
        }
        if microphone == .denied {
            return .needsMicrophone
        }
        if speech == .denied || speech == .restricted {
            return .needsSpeechRecognition
        }
        if microphone == .undetermined || speech == .undetermined {
            return .undeterminedOpenHost
        }
        if microphone == .granted && speech == .authorized {
            return .ready
        }
        return .needsMicrophone
    }

    /// Host app: no Full Access requirement. Undetermined may be prompted
    /// by the host; denied still fails closed and never starts the engine.
    static func evaluateHost(
        microphone: MicrophoneAuth,
        speech: SpeechAuth
    ) -> DictatePermissionGate {
        if microphone == .denied {
            return .needsMicrophone
        }
        if speech == .denied || speech == .restricted {
            return .needsSpeechRecognition
        }
        if microphone == .undetermined || speech == .undetermined {
            return .undeterminedOpenHost
        }
        if microphone == .granted && speech == .authorized {
            return .ready
        }
        return .needsMicrophone
    }
}
