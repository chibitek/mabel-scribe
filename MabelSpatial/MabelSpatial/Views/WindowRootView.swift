import SwiftUI

/// Windowed companion: labeled Mabel Spatial, orb, transcript, enter-space.
struct WindowRootView: View {
    @Environment(SpatialSession.self) private var session
    @Environment(\.openImmersiveSpace) private var openImmersiveSpace
    @Environment(\.dismissImmersiveSpace) private var dismissImmersiveSpace

    var body: some View {
        VStack(spacing: 22) {
            header
            ListeningOrbView()
            statusLine
            FloatingTranscriptPanel()
            controls
            privacyCard
            versionLine
        }
        .padding(28)
        .frame(minWidth: 480, minHeight: 700)
        .background(SpatialPalette.cream.opacity(0.22))
    }

    private var header: some View {
        VStack(spacing: 6) {
            Text(SpatialIdentity.displayName)
                .font(.system(size: 34, weight: .semibold, design: .rounded))
                .foregroundStyle(SpatialPalette.ink)
            Text("Privacy-first dictation for Apple Vision Pro")
                .font(.callout)
                .foregroundStyle(SpatialPalette.mist)
        }
        .accessibilityElement(children: .combine)
        .accessibilityAddTraits(.isHeader)
    }

    private var statusLine: some View {
        Text(session.statusMessage)
            .font(.subheadline.weight(.medium))
            .foregroundStyle(session.phase == .denied ? .red : SpatialPalette.ink)
            .multilineTextAlignment(.center)
            .frame(maxWidth: 420)
    }

    private var controls: some View {
        HStack(spacing: 14) {
            Button(session.isListening ? "Stop listening" : "Start listening") {
                session.toggleListening()
            }
            .buttonStyle(.borderedProminent)
            .tint(SpatialPalette.roseDeep)

            Button("Clear") {
                session.clearTranscript()
            }
            .buttonStyle(.bordered)
            .disabled(session.displayTranscript.isEmpty)

            Button(session.immersiveOpen ? "Leave space" : "Enter space") {
                Task { await toggleImmersive() }
            }
            .buttonStyle(.bordered)
        }
    }

    private var privacyCard: some View {
        Text(SpatialPrivacy.blurb)
            .font(.footnote)
            .foregroundStyle(SpatialPalette.mist)
            .multilineTextAlignment(.leading)
            .padding(14)
            .frame(maxWidth: 440, alignment: .leading)
            .background(.thinMaterial, in: RoundedRectangle(cornerRadius: 16, style: .continuous))
    }

    private var versionLine: some View {
        Text("v\(SpatialIdentity.marketingVersion) · \(SpatialIdentity.bundleID) · native visionOS")
            .font(.caption2.monospaced())
            .foregroundStyle(SpatialPalette.mist.opacity(0.85))
    }

    private func toggleImmersive() async {
        if session.immersiveOpen {
            await dismissImmersiveSpace()
            session.immersiveOpen = false
        } else {
            switch await openImmersiveSpace(id: SpatialSceneID.immersive) {
            case .opened:
                session.immersiveOpen = true
            case .userCancelled, .error:
                session.immersiveOpen = false
            @unknown default:
                session.immersiveOpen = false
            }
        }
    }
}
