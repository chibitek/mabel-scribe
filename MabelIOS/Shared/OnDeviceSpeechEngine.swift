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
///
/// Sticky ASR: Apple Speech finalizes after a short pause. That is not a
/// user stop. Keep the mic up and start a new recognition task until the
/// user taps stop (or Settings idle-stop, default off). Not hold-to-talk.
/// Not ambient / always-on — start is still an explicit tap after the gate.
final class OnDeviceSpeechEngine: @unchecked Sendable {
    private var engine: AVAudioEngine?
    private var recognizer: SFSpeechRecognizer?
    private var recognitionRequest: SFSpeechAudioBufferRecognitionRequest?
    private var recognitionTask: SFSpeechRecognitionTask?
    private var sink: (@Sendable (SpeechEngineEvent) -> Void)?
    private let workQueue = DispatchQueue(label: "com.mabel.ios.speech")
    private let lock = NSLock()
    private var wantsListen = false
    private var taskGeneration = 0
    private var restartWork: DispatchWorkItem?

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

        self.recognizer = recognizer
        lock.lock()
        wantsListen = true
        taskGeneration += 1
        lock.unlock()

        try startMicTap()
        try beginRecognition()
    }

    func stop() async {
        lock.lock()
        wantsListen = false
        taskGeneration += 1
        restartWork?.cancel()
        restartWork = nil
        lock.unlock()
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

    /// New recognition task on the existing mic tap after swipe-confirm.
    /// Does not start listening. No-op unless the user already tapped Start.
    func rollRecognitionKeepingMic() {
        lock.lock()
        let listening = wantsListen
        lock.unlock()
        guard listening else { return }
        scheduleStickyRestart(delay: 0)
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

    /// New recognition task on the existing mic tap. Does not create AVAudioEngine.
    private func beginRecognition() throws {
        guard let recognizer else {
            throw SpeechEngineError.engineUnavailable
        }

        let request = SFSpeechAudioBufferRecognitionRequest()
        request.shouldReportPartialResults = true
        request.requiresOnDeviceRecognition = EnforcerBound.dictationCloudAvailableV1 == false
        request.taskHint = .dictation
        if #available(iOS 16.0, *) {
            request.addsPunctuation = true
        }

        lock.lock()
        taskGeneration += 1
        let generation = taskGeneration
        lock.unlock()

        recognitionRequest = request
        recognitionTask = recognizer.recognitionTask(with: request) { [weak self] result, error in
            guard let self else { return }
            self.lock.lock()
            let current = self.taskGeneration
            self.lock.unlock()
            guard current == generation else { return }

            if let result {
                let text = result.bestTranscription.formattedString
                if result.isFinal {
                    self.sink?(.final(text))
                } else {
                    self.sink?(.volatile(text))
                }
            }

            if let error {
                if self.shouldRestart(after: error) {
                    self.scheduleStickyRestart(delay: 0.18)
                    return
                }
                if self.isListeningWanted {
                    self.sink?(.failed(error.localizedDescription))
                }
                return
            }

            if result?.isFinal == true {
                // Pause between utterances — restart now so the next words are not clipped.
                self.scheduleStickyRestart(delay: 0)
            }
        }
    }

    private var isListeningWanted: Bool {
        lock.lock()
        defer { lock.unlock() }
        return wantsListen
    }

    /// End-of-speech, silence, cancel-on-restart, and retry are not user stop.
    private func shouldRestart(after error: Error) -> Bool {
        guard isListeningWanted else { return false }
        return Self.isRecoverableEndOfSpeech(error)
    }

    static func isRecoverableEndOfSpeech(_ error: Error) -> Bool {
        let ns = error as NSError
        if ns.domain == "kAFAssistantErrorDomain" {
            // 203 retry, 209/216 timeout, 1101/1110 no speech, 1700 canceled.
            return [203, 209, 216, 301, 1101, 1110, 1700].contains(ns.code)
        }
        if ns.domain == NSURLErrorDomain {
            return true
        }
        let text = ns.localizedDescription.lowercased()
        if text.contains("no speech")
            || text.contains("retry")
            || text.contains("canceled")
            || text.contains("cancelled")
            || text.contains("timeout")
            || text.contains("try again")
        {
            return true
        }
        return false
    }

    private func scheduleStickyRestart(delay: TimeInterval) {
        workQueue.async { [weak self] in
            guard let self else { return }
            guard self.isListeningWanted else { return }
            self.restartWork?.cancel()
            let work = DispatchWorkItem { [weak self] in
                self?.restartRecognitionIfListening()
            }
            self.restartWork = work
            if delay <= 0 {
                self.workQueue.async(execute: work)
            } else {
                self.workQueue.asyncAfter(deadline: .now() + delay, execute: work)
            }
        }
    }

    private func restartRecognitionIfListening() {
        guard isListeningWanted else { return }
        recognitionTask = nil
        recognitionRequest = nil
        do {
            try beginRecognition()
        } catch {
            if isListeningWanted {
                sink?(.failed(error.localizedDescription))
            }
        }
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
