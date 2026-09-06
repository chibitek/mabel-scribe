import AVFoundation
import Foundation
import Speech

enum SpeechEngineEvent: Sendable {
    case volatile(String)
    case final(String)
    case status(String)
    case failed(String)
}

enum SpeechEngineError: LocalizedError {
    case permissionDenied
    case fullAccessRequired
    case engineUnavailable
    case localeUnsupported

    var errorDescription: String? {
        switch self {
        case .permissionDenied:
            return "Microphone or Speech Recognition denied."
        case .fullAccessRequired:
            return "Keyboard Full Access is required to use the microphone."
        case .engineUnavailable:
            return "On-device Apple Speech is unavailable here."
        case .localeUnsupported:
            return "No on-device speech model for this locale."
        }
    }
}

enum SpeechPermissionContext: Equatable {
    /// Host app may show system permission sheets.
    case host
    /// Keyboard must already have Full Access + granted permissions.
    /// Never prompt. Never start audio when the gate is closed.
    case keyboard(hasFullAccess: Bool)
}

/// Apple Speech only. No whisper.cpp, Parakeet, WhisperKit, Tauri, or Stiki.
///
/// On-device `SFSpeechRecognizer` with `requiresOnDeviceRecognition = true`.
/// No network speech fallback — if the device cannot dictate locally, fail.
/// AVAudioEngine is created only after the permission gate passes, and is
/// torn down in `stop()` so the microphone is never ambient.
final class OnDeviceSpeechEngine: @unchecked Sendable {
    private var engine: AVAudioEngine?
    private var recognizer: SFSpeechRecognizer?
    private var recognitionRequest: SFSpeechAudioBufferRecognitionRequest?
    private var recognitionTask: SFSpeechRecognitionTask?
    private var sink: (@Sendable (SpeechEngineEvent) -> Void)?

    func start(
        context: SpeechPermissionContext,
        handler: @escaping @Sendable (SpeechEngineEvent) -> Void
    ) async throws {
        sink = handler
        try await assertPermissions(context: context)

        let recognizer = SFSpeechRecognizer(locale: preferredLocale())
        guard let recognizer, recognizer.isAvailable else {
            throw SpeechEngineError.engineUnavailable
        }
        guard recognizer.supportsOnDeviceRecognition else {
            throw SpeechEngineError.engineUnavailable
        }

        let request = SFSpeechAudioBufferRecognitionRequest()
        request.shouldReportPartialResults = true
        request.requiresOnDeviceRecognition = EnforcerBound.dictationCloudAvailableV1 == false
        if #available(iOS 16.0, *) {
            request.addsPunctuation = true
        }

        self.recognizer = recognizer
        recognitionRequest = request

        recognitionTask = recognizer.recognitionTask(with: request) { [weak self] result, error in
            if let result {
                let text = result.bestTranscription.formattedString
                if result.isFinal {
                    self?.sink?(.final(text))
                } else {
                    self?.sink?(.volatile(text))
                }
            }
            if let error {
                self?.sink?(.failed(error.localizedDescription))
            }
        }

        try startMicTap()
    }

    func stop() async {
        if let request = recognitionRequest {
            request.endAudio()
        }
        recognitionTask?.cancel()
        recognitionTask = nil
        recognitionRequest = nil
        recognizer = nil

        if let engine {
            engine.inputNode.removeTap(onBus: 0)
            if engine.isRunning {
                engine.stop()
            }
        }
        engine = nil
        deactivateAudioSession()
        sink = nil
    }

    // MARK: - Permissions (fail closed before any audio session)

    func currentMicrophoneAuth() -> MicrophoneAuth {
        switch AVAudioApplication.shared.recordPermission {
        case .granted:
            return .granted
        case .denied:
            return .denied
        case .undetermined:
            return .undetermined
        @unknown default:
            return .denied
        }
    }

    func currentSpeechAuth() -> SpeechAuth {
        switch SFSpeechRecognizer.authorizationStatus() {
        case .authorized:
            return .authorized
        case .denied:
            return .denied
        case .restricted:
            return .restricted
        case .notDetermined:
            return .undetermined
        @unknown default:
            return .denied
        }
    }

    private func assertPermissions(context: SpeechPermissionContext) async throws {
        switch context {
        case .host:
            try await requestHostPermissions()
        case .keyboard(let hasFullAccess):
            let gate = DictatePermissionGate.evaluateKeyboard(
                hasFullAccess: hasFullAccess,
                microphone: currentMicrophoneAuth(),
                speech: currentSpeechAuth()
            )
            if gate.allowsAudio == false {
                if gate == .needsFullAccess {
                    throw SpeechEngineError.fullAccessRequired
                }
                throw SpeechEngineError.permissionDenied
            }
        }
    }

    /// Host-only: show system sheets if needed. Does not start the microphone.
    func promptHostPermissions() async throws {
        try await requestHostPermissions()
    }

    private func requestHostPermissions() async throws {
        let micBefore = currentMicrophoneAuth()
        if micBefore == .denied {
            throw SpeechEngineError.permissionDenied
        }
        if micBefore == .undetermined {
            let granted = await withCheckedContinuation { continuation in
                AVAudioApplication.requestRecordPermission { allowed in
                    continuation.resume(returning: allowed)
                }
            }
            if granted == false {
                throw SpeechEngineError.permissionDenied
            }
        }

        let speechBefore = currentSpeechAuth()
        if speechBefore == .denied || speechBefore == .restricted {
            throw SpeechEngineError.permissionDenied
        }
        if speechBefore == .undetermined {
            let status = await withCheckedContinuation { continuation in
                SFSpeechRecognizer.requestAuthorization { value in
                    continuation.resume(returning: value)
                }
            }
            guard status == .authorized else {
                throw SpeechEngineError.permissionDenied
            }
        }

        let gate = DictatePermissionGate.evaluateHost(
            microphone: currentMicrophoneAuth(),
            speech: currentSpeechAuth()
        )
        if gate.allowsAudio == false {
            throw SpeechEngineError.permissionDenied
        }
    }

    // MARK: - Microphone (only after the gate, only while listening)

    private func startMicTap() throws {
        let session = AVAudioSession.sharedInstance()
        try session.setCategory(.record, mode: .measurement, options: [])
        try session.setActive(true)

        let engine = AVAudioEngine()
        self.engine = engine
        let input = engine.inputNode
        let format = input.outputFormat(forBus: 0)
        input.installTap(onBus: 0, bufferSize: 1024, format: format) { [weak self] buffer, _ in
            self?.recognitionRequest?.append(buffer)
        }
        engine.prepare()
        try engine.start()
    }

    private func deactivateAudioSession() {
        do {
            try AVAudioSession.sharedInstance().setActive(false, options: .notifyOthersOnDeactivation)
        } catch {
            // Session may already be idle after the host app or another keyboard dismissed.
        }
    }

    private func preferredLocale() -> Locale {
        let current = Locale.current
        if current.language.languageCode != nil {
            return current
        }
        return Locale(identifier: "en-US")
    }
}
