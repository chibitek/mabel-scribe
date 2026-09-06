import SwiftUI

enum ListenPhase: Equatable {
    case idle
    case preparing
    case listening
    case denied
}

/// Shared window + immersive state. Mic session lives only while `listening`.
@MainActor
@Observable
final class SpatialSession {
    var phase: ListenPhase = .idle
    var finalTranscript = ""
    var volatileTail = ""
    var statusMessage = "Look at the orb · pinch to listen"
    var lastError: String?
    var immersiveOpen = false

    var isListening: Bool { phase == .listening }

    var displayTranscript: String {
        let head = finalTranscript.trimmingCharacters(in: .whitespacesAndNewlines)
        let tail = volatileTail.trimmingCharacters(in: .whitespacesAndNewlines)
        if head.isEmpty { return tail }
        if tail.isEmpty { return head }
        return head + " " + tail
    }

    private let speech = OnDeviceSpeechEngine()

    func toggleListening() {
        if isListening || phase == .preparing {
            stopListening()
        } else {
            startListening()
        }
    }

    func startListening() {
        guard phase != .listening, phase != .preparing else { return }
        lastError = nil
        phase = .preparing
        statusMessage = "Preparing on-device speech…"

        Task { [weak self] in
            guard let self else { return }
            do {
                try await self.speech.start { [weak self] event in
                    Task { @MainActor in
                        self?.apply(event)
                    }
                }
                self.phase = .listening
                self.statusMessage = "Listening · pinch the orb to stop · mic is on"
            } catch SpeechEngineError.permissionDenied {
                self.phase = .denied
                self.statusMessage = "Microphone access needed to dictate"
                self.lastError = "Grant Microphone in Settings → Mabel Spatial"
            } catch {
                self.phase = .idle
                self.statusMessage = "Look at the orb · pinch to listen"
                self.lastError = error.localizedDescription
            }
        }
    }

    func stopListening() {
        Task { [weak self] in
            guard let self else { return }
            await self.speech.stop()
            self.phase = .idle
            self.statusMessage = "Mic off · look at the orb · pinch to listen"
            if self.volatileTail.isEmpty == false {
                self.appendFinal(self.volatileTail)
                self.volatileTail = ""
            }
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
            statusMessage = "Mic off · look at the orb · pinch to listen"
        }
    }

    private func appendFinal(_ text: String) {
        let piece = text.trimmingCharacters(in: .whitespacesAndNewlines)
        guard piece.isEmpty == false else { return }
        if finalTranscript.isEmpty {
            finalTranscript = piece
        } else {
            finalTranscript += " " + piece
        }
    }
}
