import SwiftUI

/// Dictation-first keyboard overlay with Mabel cat chrome.
/// Globe / delete / space / return stay available so this is a real input method.
struct KeyboardRootView: View {
    @Bindable var session: SpeechSession
    @Bindable var chrome: KeyboardChrome

    var body: some View {
        VStack(spacing: 10) {
            Text(session.statusMessage)
                .font(.caption.weight(.semibold))
                .foregroundStyle(session.phase == .denied ? IOSPalette.roseDeep : IOSPalette.ink)
                .multilineTextAlignment(.center)
                .lineLimit(2)
                .padding(.horizontal, 12)
                .padding(.top, 8)

            HStack(alignment: .center, spacing: 14) {
                Button {
                    session.toggleListening(context: .keyboard(hasFullAccess: chrome.hasFullAccess)) { text in
                        let piece = text.trimmingCharacters(in: .whitespacesAndNewlines)
                        guard piece.isEmpty == false else { return }
                        chrome.insert(piece)
                        chrome.insert(" ")
                    }
                } label: {
                    MabelOrb(
                        listening: session.isListening,
                        enabled: session.lastGate.allowsAudio || session.isListening,
                        diameter: 92
                    )
                }
                .buttonStyle(.plain)
                .accessibilityHint("Starts or stops dictation. Does nothing if permissions are off.")

                VStack(alignment: .leading, spacing: 6) {
                    Text(session.isListening ? "Listening" : "Mabel")
                        .font(.system(.headline, design: .rounded))
                        .foregroundStyle(IOSPalette.ink)
                    Text(previewText)
                        .font(.footnote)
                        .foregroundStyle(IOSPalette.mist)
                        .lineLimit(3)
                }
                .frame(maxWidth: .infinity, alignment: .leading)
            }
            .padding(.horizontal, 16)

            HStack(spacing: 8) {
                if chrome.needsInputModeSwitch {
                    key("🌐") { chrome.advanceToNextInputMode() }
                }
                key(".") { chrome.insert(".") }
                key("⌫") { chrome.deleteBackward() }
                Button("space") { chrome.insert(" ") }
                    .font(.subheadline.weight(.semibold))
                    .foregroundStyle(IOSPalette.ink)
                    .frame(maxWidth: .infinity, minHeight: 40)
                    .background(Color.white.opacity(0.78), in: RoundedRectangle(cornerRadius: 10, style: .continuous))
                key("⏎") { chrome.insert("\n") }
            }
            .padding(.horizontal, 12)
            .padding(.bottom, 10)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .background(IOSPalette.cream)
    }

    private var previewText: String {
        if session.displayTranscript.isEmpty == false {
            return session.displayTranscript
        }
        if session.lastGate.allowsAudio {
            return "Tap the orb. Speak. Tap again — text goes into this field."
        }
        return session.lastGate.userMessage
    }

    private func key(_ label: String, action: @escaping () -> Void) -> some View {
        Button(label, action: action)
            .font(.title3)
            .frame(width: 44, height: 40)
            .background(Color.white.opacity(0.78), in: RoundedRectangle(cornerRadius: 10, style: .continuous))
            .foregroundStyle(IOSPalette.ink)
    }
}
