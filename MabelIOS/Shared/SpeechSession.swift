import SwiftUI

enum ListenPhase: Equatable {
    case idle
    case preparing
    case listening
    case denied
}

/// Shared host + keyboard dictation state.
/// Mic session lives only while `listening`. No account / Stiki gate.
@MainActor
@Observable
final class SpeechSession {
    var phase: ListenPhase = .idle
    var finalTranscript = ""
    var volatileTail = ""
    var statusMessage = "Tap the orb to dictate"
    var lastError: String?
    var lastGate: DictatePermissionGate = .undeterminedOpenHost

    var isListening: Bool { phase == .listening || phase == .preparing }

    var displayTranscript: String {
        let head = finalTranscript.trimmingCharacters(in: .whitespacesAndNewlines)
        let tail = volatileTail.trimmingCharacters(in: .whitespacesAndNewlines)
        if head.isEmpty { return tail }
        if tail.isEmpty { return head }
        if tail.hasPrefix(head) { return tail }
        return head + " " + tail
    }

    private let speech = OnDeviceSpeechEngine()
    private var onStopped: ((String) -> Void)?

    func refreshGate(context: SpeechPermissionContext) {
        switch context {
        case .host:
            lastGate = DictatePermissionGate.evaluateHost(
                microphone: speech.currentMicrophoneAuth(),
                speech: speech.currentSpeechAuth()
            )
        case .keyboard(let hasFullAccess):
            lastGate = DictatePermissionGate.evaluateKeyboard(
                hasFullAccess: hasFullAccess,
                microphone: speech.currentMicrophoneAuth(),
                speech: speech.currentSpeechAuth()
            )
        }
        if phase != .listening && phase != .preparing {
            statusMessage = lastGate.userMessage
            switch lastGate {
            case .ready, .undeterminedOpenHost:
                phase = .idle
            case .needsFullAccess, .needsMicrophone, .needsSpeechRecognition:
                phase = .denied
            }
        }
    }

    func requestHostPermissions() {
        Task { [weak self] in
            guard let self else { return }
            do {
                try await self.speech.promptHostPermissions()
                self.refreshGate(context: .host)
                self.phase = self.lastGate.allowsAudio ? .idle : .denied
                self.statusMessage = self.lastGate.userMessage
                self.lastError = self.lastGate.allowsAudio ? nil : self.lastGate.userMessage
            } catch {
                self.refreshGate(context: .host)
                self.phase = .denied
                self.statusMessage = self.lastGate.userMessage
                self.lastError = error.localizedDescription
            }
        }
    }

    func toggleListening(context: SpeechPermissionContext, onStopped: ((String) -> Void)? = nil) {
        if isListening {
            stopListening()
        } else {
            startListening(context: context, onStopped: onStopped)
        }
    }

    func startListening(context: SpeechPermissionContext, onStopped: ((String) -> Void)? = nil) {
        guard phase != .listening, phase != .preparing else { return }
        self.onStopped = onStopped
        refreshGate(context: context)

        // Fail closed: never create an audio session unless the gate is ready
        // or the host is allowed to prompt from an undetermined state.
        let mayPrompt = (context == .host && lastGate == .undeterminedOpenHost)
        if lastGate.allowsAudio == false && mayPrompt == false {
            phase = .denied
            statusMessage = lastGate.userMessage
            lastError = lastGate.userMessage
            return
        }

        lastError = nil
        finalTranscript = ""
        volatileTail = ""
        phase = .preparing
        statusMessage = "Preparing on-device speech…"

        Task { [weak self] in
            guard let self else { return }
            do {
                try await self.speech.start(context: context) { [weak self] event in
                    Task { @MainActor in
                        self?.apply(event)
                    }
                }
                self.phase = .listening
                self.statusMessage = "Listening · tap the orb to stop · mic is on"
            } catch SpeechEngineError.permissionDenied, SpeechEngineError.fullAccessRequired {
                self.refreshGate(context: context)
                self.phase = .denied
                self.statusMessage = self.lastGate.userMessage
                self.lastError = self.lastGate.userMessage
            } catch {
                self.phase = .idle
                self.statusMessage = self.lastGate.userMessage
                self.lastError = error.localizedDescription
            }
        }
    }

    func stopListening() {
        Task { [weak self] in
            guard let self else { return }
            await self.speech.stop()
            if self.volatileTail.isEmpty == false {
                self.appendFinal(self.volatileTail)
                self.volatileTail = ""
            }
            let committed = Polish.applyFromAppGroup(self.displayTranscript)
            self.phase = .idle
            self.statusMessage = "Mic off · tap the orb to dictate"
            if committed.isEmpty == false {
                self.onStopped?(committed)
            }
            self.onStopped = nil
        }
    }

    func clearTranscript() {
        finalTranscript = ""
        volatileTail = ""
        lastError = nil
    }

    private func apply(_ event: SpeechEngineEvent) {
        switch event {
        case .volatile(let text):
            volatileTail = text
        case .final(let text):
            appendFinal(text)
            volatileTail = ""
        case .status(let message):
            statusMessage = message
        case .failed(let message):
            lastError = message
            phase = .idle
            statusMessage = "Mic off · tap the orb to dictate"
            Task { await speech.stop() }
        }
    }

    private func appendFinal(_ text: String) {
        let piece = text.trimmingCharacters(in: .whitespacesAndNewlines)
        guard piece.isEmpty == false else { return }
        if finalTranscript.isEmpty {
            finalTranscript = piece
        } else if piece.hasPrefix(finalTranscript) {
            finalTranscript = piece
        } else {
            finalTranscript += " " + piece
        }
    }
}
