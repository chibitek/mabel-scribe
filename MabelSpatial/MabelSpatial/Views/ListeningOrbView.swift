import SwiftUI

/// Windowed companion orb. Same eyes + hands start/stop as the RealityKit orb:
/// look to hover, pinch / tap to toggle. Pulses only while listening.
struct ListeningOrbView: View {
    @Environment(SpatialSession.self) private var session
    @State private var pulse = false

    var body: some View {
        ZStack {
            Circle()
                .fill(
                    RadialGradient(
                        colors: session.isListening
                            ? [Color.white.opacity(0.95), SpatialPalette.rose, SpatialPalette.roseDeep]
                            : [SpatialPalette.cream, SpatialPalette.rose.opacity(0.92), SpatialPalette.roseDeep.opacity(0.75)],
                        center: .init(x: 0.38, y: 0.32),
                        startRadius: 8,
                        endRadius: 110
                    )
                )
                .frame(width: 168, height: 168)
                .scaleEffect(session.isListening && pulse ? 1.08 : 1.0)
                .shadow(
                    color: SpatialPalette.rose.opacity(session.isListening ? 0.65 : 0.28),
                    radius: session.isListening ? 28 : 14,
                    y: 6
                )

            HStack(spacing: 22) {
                Capsule()
                    .fill(.white.opacity(0.92))
                    .frame(width: 14, height: session.isListening ? 22 : 16)
                Capsule()
                    .fill(.white.opacity(0.92))
                    .frame(width: 14, height: session.isListening ? 22 : 16)
            }
            .offset(y: -6)
        }
        .hoverEffect()
        .accessibilityLabel(session.isListening ? "Mabel orb, listening. Pinch to stop." : "Mabel orb, idle. Pinch to listen.")
        .accessibilityAddTraits(.isButton)
        .onTapGesture {
            session.toggleListening()
        }
        .onChange(of: session.isListening) { _, listening in
            if listening {
                withAnimation(.easeInOut(duration: 0.8).repeatForever(autoreverses: true)) {
                    pulse = true
                }
            } else {
                withAnimation(.easeOut(duration: 0.25)) {
                    pulse = false
                }
            }
        }
    }
}
