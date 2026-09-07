import SwiftUI

/// Full keyboard Start/mic chrome. Mabel cat UI only — not Flow brand.
/// Dictation starts only on an explicit Start tap. Swipe confirm never
/// starts the mic. Tone chips are existing Polish only (default Off;
/// live modes need the dual gate). Not a second Formal/Casual catalog.
/// Style Formal|Casual|Very casual stays the Style catalog (later ship).
struct KeyboardRootView: View {
    @Bindable var session: SpeechSession
    @Bindable var chrome: KeyboardChrome
    @State private var shifted = true
    @State private var digits = false
    @State private var swipeTravel: CGFloat = 0

    var body: some View {
        VStack(spacing: 6) {
            onStateBar
            previewLine
            polishPicker
            if showQwerty {
                qwertyBoard
            } else {
                compactBar
            }
            swipeConfirmEdge
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .top)
        .background(IOSPalette.cream)
    }

    private var showQwerty: Bool {
        chrome.settings.qwertyLayout
    }

    private var polishMode: String {
        chrome.settings.effectivePolishMode
    }

    private var previewText: String {
        if session.displayTranscript.isEmpty == false {
            return session.displayTranscript
        }
        if session.isListening {
            return "Speak — swipe up to confirm, tap Start to stop"
        }
        if session.lastGate.allowsAudio {
            return "Tap Start. Speak. Swipe up to confirm — then edit with the full keyboard."
        }
        return session.lastGate.userMessage
    }

    private var onStateBar: some View {
        HStack(spacing: 8) {
            Circle()
                .fill(session.isListening ? IOSPalette.roseDeep : IOSPalette.mist.opacity(0.35))
                .frame(width: 8, height: 8)
            Text(session.isListening ? EnforcerBound.keyboardOnStateCopy : session.statusMessage)
                .font(.caption.weight(.semibold))
                .foregroundStyle(session.phase == .denied ? IOSPalette.roseDeep : IOSPalette.ink)
                .lineLimit(2)
            Spacer(minLength: 0)
            if session.isListening {
                Text("Listening")
                    .font(.caption2.weight(.bold))
                    .foregroundStyle(IOSPalette.roseDeep)
            }
        }
        .padding(.horizontal, 12)
        .padding(.top, 8)
        .padding(.bottom, 2)
        .accessibilityIdentifier("keyboard-on-state")
        .accessibilityLabel(session.isListening ? "Mabel is on. Listening." : session.statusMessage)
    }

    private var previewLine: some View {
        Text(previewText)
            .font(.caption)
            .foregroundStyle(IOSPalette.mist)
            .lineLimit(2)
            .frame(maxWidth: .infinity, alignment: .leading)
            .padding(.horizontal, 12)
    }

    /// Existing Polish Off|Casual|Professional|Polite. Not Style Formal|Casual|Very casual.
    /// setPolishMode keeps the Pro+Stiki dual gate and default Off.
    private var polishPicker: some View {
        HStack(spacing: 6) {
            Text("Polish")
                .font(.caption2.weight(.bold))
                .foregroundStyle(IOSPalette.mist)
            ForEach(Polish.modes, id: \.self) { mode in
                Button {
                    pickPolish(mode)
                } label: {
                    Text(Polish.modeLabel(mode))
                        .font(.caption2.weight(.semibold))
                        .padding(.horizontal, 8)
                        .padding(.vertical, 5)
                        .background(
                            polishMode == mode
                                ? IOSPalette.rose.opacity(0.55)
                                : Color.white.opacity(0.88),
                            in: Capsule()
                        )
                        .foregroundStyle(IOSPalette.ink)
                }
                .buttonStyle(.plain)
                .accessibilityLabel("Polish \(Polish.modeLabel(mode))")
            }
        }
        .padding(.horizontal, 12)
        .accessibilityIdentifier("keyboard-polish-picker")
        .accessibilityElement(children: .contain)
        .accessibilityHint("Existing Polish modes. Default Off. Live modes need Pro and Stiki. Not a second tone catalog.")
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

            startMicChrome
        }
    }

    private var compactBar: some View {
        startMicChrome
    }

    /// Full keyboard Start/mic chrome. Start is explicit — never ambient.
    private var startMicChrome: some View {
        HStack(spacing: 5) {
            if chrome.needsInputModeSwitch {
                modifierKey("🌐") { chrome.advanceToNextInputMode() }
            }
            modifierKey(digits ? "ABC" : "123") { digits.toggle() }
            startMicKey
            Button("space") { chrome.insert(" ") }
                .font(.subheadline.weight(.semibold))
                .foregroundStyle(IOSPalette.ink)
                .frame(maxWidth: .infinity, minHeight: 40)
                .background(Color.white.opacity(0.88), in: RoundedRectangle(cornerRadius: 6, style: .continuous))
                .buttonStyle(.plain)
            modifierKey("⏎") { chrome.insert("\n") }
        }
        .padding(.horizontal, 4)
        .accessibilityIdentifier("keyboard-start-chrome")
    }

    private var startMicKey: some View {
        Button {
            toggleDictate()
        } label: {
            HStack(spacing: 6) {
                MabelOrb(
                    listening: session.isListening,
                    enabled: session.lastGate.allowsAudio || session.isListening,
                    diameter: 28
                )
                Text(session.isListening ? "Stop" : "Start")
                    .font(.subheadline.weight(.semibold))
            }
            .frame(minWidth: 88, minHeight: 40)
            .padding(.horizontal, 8)
            .background(
                session.isListening
                    ? IOSPalette.rose.opacity(0.7)
                    : IOSPalette.peach.opacity(0.7),
                in: RoundedRectangle(cornerRadius: 8, style: .continuous)
            )
            .foregroundStyle(IOSPalette.ink)
        }
        .buttonStyle(.plain)
        .accessibilityIdentifier("keyboard-start")
        .accessibilityHint("Starts or stops dictation. Does nothing if permissions are off.")
    }

    /// Bottom-edge swipe continue/confirm. Does not start the microphone.
    private var swipeConfirmEdge: some View {
        VStack(spacing: 4) {
            Capsule()
                .fill(IOSPalette.roseDeep.opacity(session.isListening ? 0.7 : 0.35))
                .frame(width: 44, height: 5)
                .offset(y: min(0, swipeTravel * 0.2))
            Text(session.isListening ? "Swipe up to confirm · keep listening" : "Swipe up to insert")
                .font(.caption2.weight(.semibold))
                .foregroundStyle(IOSPalette.mist)
        }
        .frame(maxWidth: .infinity)
        .padding(.top, 2)
        .padding(.bottom, 8)
        .contentShape(Rectangle())
        .gesture(
            DragGesture(minimumDistance: 12)
                .onChanged { value in
                    swipeTravel = value.translation.height
                }
                .onEnded { value in
                    swipeTravel = 0
                    if value.translation.height < -28 {
                        confirmFromSwipe()
                    }
                }
        )
        .accessibilityIdentifier("keyboard-swipe-confirm")
        .accessibilityLabel("Swipe up to confirm")
        .accessibilityHint("Confirms the current words and keeps listening. Does not start the microphone.")
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

    private func toggleDictate() {
        if session.isListening {
            session.stopListening()
            return
        }
        if chrome.settings.keyboardMicPref == false {
            session.statusMessage = "Keyboard mic is off in Mabel Settings"
            return
        }
        session.toggleListening(context: .keyboard(hasFullAccess: chrome.hasFullAccess)) { text in
            let piece = text.trimmingCharacters(in: .whitespacesAndNewlines)
            guard piece.isEmpty == false else { return }
            chrome.insert(piece)
            chrome.insert(" ")
        }
    }

    private func pickPolish(_ mode: String) {
        if case .failure(let error) = chrome.settings.setPolishMode(mode) {
            session.statusMessage = error.message
        }
    }

    private func confirmFromSwipe() {
        let piece = session.confirmUtterance()
        guard piece.isEmpty == false else { return }
        chrome.insert(piece)
        chrome.insert(" ")
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
