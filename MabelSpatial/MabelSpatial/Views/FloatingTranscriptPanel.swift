import SwiftUI

/// Floating transcript card. Final words stay; the volatile tail is lighter.
struct FloatingTranscriptPanel: View {
    @Environment(SpatialSession.self) private var session

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            HStack {
                Text("Transcript")
                    .font(.headline)
                Spacer()
                Circle()
                    .fill(session.isListening ? Color.green : SpatialPalette.mist.opacity(0.45))
                    .frame(width: 9, height: 9)
                    .accessibilityLabel(session.isListening ? "Microphone on" : "Microphone off")
            }

            ScrollView {
                VStack(alignment: .leading, spacing: 4) {
                    if session.displayTranscript.isEmpty {
                        Text("Pinch the orb and speak. Words stay on this headset.")
                            .foregroundStyle(SpatialPalette.mist)
                    } else {
                        HStack(alignment: .firstTextBaseline, spacing: 4) {
                            Text(session.finalTranscript)
                                .foregroundStyle(SpatialPalette.ink)
                            Text(session.volatileTail)
                                .foregroundStyle(SpatialPalette.mist)
                        }
                    }
                }
                .font(.body)
                .frame(maxWidth: .infinity, alignment: .leading)
            }
            .frame(minHeight: 96, maxHeight: 180)

            if let error = session.lastError {
                Text(error)
                    .font(.caption)
                    .foregroundStyle(.red)
            }
        }
        .padding(16)
        .frame(maxWidth: 440, alignment: .leading)
        .glassBackgroundEffect()
        .accessibilityElement(children: .combine)
        .accessibilityLabel(transcriptAccessibility)
    }

    private var transcriptAccessibility: String {
        if session.displayTranscript.isEmpty {
            return "Transcript empty"
        }
        return "Transcript: \(session.displayTranscript)"
    }
}
