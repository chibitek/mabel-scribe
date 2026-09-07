import SwiftUI

/// In-app Free dictate path. Same gate as the keyboard, minus Full Access.
/// Used to grant permissions and prove the orb start/stop without Stiki.
struct HostDictatePlayground: View {
    @Environment(SpeechSession.self) private var session
    @Environment(SettingsStore.self) private var settings
    @State private var scratch = ""

    var body: some View {
        VStack(alignment: .leading, spacing: 14) {
            Text("Try it here first")
                .font(.headline.weight(.semibold))
                .foregroundStyle(IOSPalette.ink)
            Text("Tap Mabel. Speak. Tap again to stop. Text lands in the field below — not in iCloud, not in Stiki.")
                .font(.footnote)
                .foregroundStyle(IOSPalette.mist)

            HStack(alignment: .center, spacing: 18) {
                Button {
                    guard settings.masterOn else { return }
                    session.toggleListening(context: .host) { text in
                        if scratch.isEmpty {
                            scratch = text
                        } else {
                            scratch += " " + text
                        }
                    }
                } label: {
                    MabelOrb(
                        listening: session.isListening,
                        enabled: settings.masterOn,
                        diameter: 108
                    )
                }
                .buttonStyle(.plain)

                VStack(alignment: .leading, spacing: 6) {
                    Text(session.isListening ? EnforcerBound.keyboardOnStateCopy : session.statusMessage)
                        .font(.subheadline.weight(.medium))
                        .foregroundStyle(session.phase == .denied ? IOSPalette.roseDeep : IOSPalette.ink)
                        .accessibilityLabel(session.isListening ? "Mabel is on. Listening." : session.statusMessage)
                    if session.displayTranscript.isEmpty == false && session.isListening {
                        Text(session.displayTranscript)
                            .font(.footnote)
                            .foregroundStyle(IOSPalette.mist)
                            .lineLimit(4)
                    }
                }
                .frame(maxWidth: .infinity, alignment: .leading)
            }

            TextField("Transcript lands here", text: $scratch, axis: .vertical)
                .textFieldStyle(.roundedBorder)
                .lineLimit(3...8)

            HStack {
                Button("Clear") {
                    scratch = ""
                    session.clearTranscript()
                }
                .disabled(scratch.isEmpty && session.displayTranscript.isEmpty)
                Spacer()
            }
        }
        .padding(16)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(Color.white.opacity(0.72), in: RoundedRectangle(cornerRadius: 20, style: .continuous))
    }
}
