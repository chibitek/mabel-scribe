import SwiftUI

/// Dictation-first keyboard overlay with Mabel cat chrome plus a full
/// QWERTY so the user can edit the transcript. Globe / delete / space /
/// return stay available so this is a real input method.
struct KeyboardRootView: View {
    @Bindable var session: SpeechSession
    @Bindable var chrome: KeyboardChrome
    @State private var shifted = true
    @State private var digits = false

    var body: some View {
        VStack(spacing: 8) {
            Text(session.statusMessage)
                .font(.caption.weight(.semibold))
                .foregroundStyle(session.phase == .denied ? IOSPalette.roseDeep : IOSPalette.ink)
                .multilineTextAlignment(.center)
                .lineLimit(2)
                .padding(.horizontal, 12)
                .padding(.top, 6)

            HStack(alignment: .center, spacing: 12) {
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
                        diameter: 56
                    )
                }
                .buttonStyle(.plain)
                .accessibilityHint("Starts or stops dictation. Does nothing if permissions are off.")

                VStack(alignment: .leading, spacing: 4) {
                    Text(session.isListening ? "Listening" : "Mabel")
                        .font(.system(.subheadline, design: .rounded).weight(.semibold))
                        .foregroundStyle(IOSPalette.ink)
                    Text(previewText)
                        .font(.caption)
                        .foregroundStyle(IOSPalette.mist)
                        .lineLimit(2)
                }
                .frame(maxWidth: .infinity, alignment: .leading)
            }
            .padding(.horizontal, 12)

            if showQwerty {
                qwertyBoard
            } else {
                compactBar
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .top)
        .background(IOSPalette.cream)
    }

    private var showQwerty: Bool {
        let defaults = UserDefaults(suiteName: IOSIdentity.appGroup) ?? .standard
        return defaults.object(forKey: "settings.qwerty") as? Bool ?? true
    }

    private var previewText: String {
        if session.displayTranscript.isEmpty == false {
            return session.displayTranscript
        }
        if session.lastGate.allowsAudio {
            return "Tap Mabel. Speak. Tap again — then edit with the full keyboard."
        }
        return session.lastGate.userMessage
    }

    private var qwertyBoard: some View {
        VStack(spacing: 6) {
            keyRow(digits ? KeyboardLayout.numbers : KeyboardLayout.top)
            keyRow(digits ? KeyboardLayout.symbols : KeyboardLayout.middle)
            HStack(spacing: 5) {
                modifierKey(digits ? "ABC" : "⇧") {
                    if digits {
                        digits = false
                    } else {
                        shifted.toggle()
                    }
                }
                keyRow(digits ? KeyboardLayout.punct : KeyboardLayout.bottom, padded: false)
                modifierKey("⌫") { chrome.deleteBackward() }
            }
            .padding(.horizontal, 4)

            HStack(spacing: 5) {
                if chrome.needsInputModeSwitch {
                    modifierKey("🌐") { chrome.advanceToNextInputMode() }
                }
                modifierKey(digits ? "ABC" : "123") { digits.toggle() }
                Button("space") { chrome.insert(" ") }
                    .font(.subheadline.weight(.semibold))
                    .foregroundStyle(IOSPalette.ink)
                    .frame(maxWidth: .infinity, minHeight: 40)
                    .background(Color.white.opacity(0.88), in: RoundedRectangle(cornerRadius: 6, style: .continuous))
                    .buttonStyle(.plain)
                modifierKey("⏎") { chrome.insert("\n") }
            }
            .padding(.horizontal, 4)
            .padding(.bottom, 8)
        }
    }

    private var compactBar: some View {
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

    private func keyRow(_ keys: [String], padded: Bool = true) -> some View {
        HStack(spacing: 5) {
            ForEach(keys, id: \.self) { glyph in
                Button {
                    insertGlyph(glyph)
                } label: {
                    Text(displayGlyph(glyph))
                        .font(.body.weight(.medium))
                        .frame(maxWidth: .infinity, minHeight: 40)
                        .background(Color.white.opacity(0.88), in: RoundedRectangle(cornerRadius: 6, style: .continuous))
                        .foregroundStyle(IOSPalette.ink)
                }
                .buttonStyle(.plain)
            }
        }
        .padding(.horizontal, padded ? 4 : 0)
    }

    private func key(_ label: String, action: @escaping () -> Void) -> some View {
        Button(label, action: action)
            .font(.title3)
            .frame(width: 44, height: 40)
            .background(Color.white.opacity(0.78), in: RoundedRectangle(cornerRadius: 10, style: .continuous))
            .foregroundStyle(IOSPalette.ink)
    }

    private func modifierKey(_ label: String, action: @escaping () -> Void) -> some View {
        Button(label, action: action)
            .font(.subheadline.weight(.semibold))
            .frame(width: 44, height: 40)
            .background(IOSPalette.peach.opacity(0.55), in: RoundedRectangle(cornerRadius: 6, style: .continuous))
            .foregroundStyle(IOSPalette.ink)
            .buttonStyle(.plain)
    }

    private func displayGlyph(_ glyph: String) -> String {
        if digits { return glyph }
        return shifted ? glyph : glyph.lowercased()
    }

    private func insertGlyph(_ glyph: String) {
        chrome.insert(displayGlyph(glyph))
        if digits == false && shifted {
            shifted = false
        }
    }
}

private enum KeyboardLayout {
    static let top = ["Q", "W", "E", "R", "T", "Y", "U", "I", "O", "P"]
    static let middle = ["A", "S", "D", "F", "G", "H", "J", "K", "L"]
    static let bottom = ["Z", "X", "C", "V", "B", "N", "M"]
    static let numbers = ["1", "2", "3", "4", "5", "6", "7", "8", "9", "0"]
    static let symbols = ["-", "/", ":", ";", "(", ")", "$", "&", "@", "\""]
    static let punct = [".", ",", "?", "!", "'"]
}
