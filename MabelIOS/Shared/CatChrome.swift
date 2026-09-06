import SwiftUI

/// Shared cat-energy chrome for the host shell and the keyboard overlay.
/// Rose / cream / whisker — not a dark Wispr or Flow clone.
struct MabelOrb: View {
    var listening: Bool
    var enabled: Bool = true
    var diameter: CGFloat = 148

    @State private var pulse = false

    var body: some View {
        ZStack {
            Circle()
                .fill(
                    RadialGradient(
                        colors: listening
                            ? [Color.white.opacity(0.95), IOSPalette.rose, IOSPalette.roseDeep]
                            : [IOSPalette.cream, IOSPalette.rose.opacity(0.92), IOSPalette.roseDeep.opacity(0.75)],
                        center: .init(x: 0.38, y: 0.32),
                        startRadius: 6,
                        endRadius: diameter * 0.65
                    )
                )
                .frame(width: diameter, height: diameter)
                .scaleEffect(listening && pulse ? 1.07 : 1.0)
                .opacity(enabled ? 1 : 0.45)
                .shadow(
                    color: IOSPalette.rose.opacity(listening ? 0.55 : 0.22),
                    radius: listening ? 22 : 12,
                    y: 5
                )

            // Ears
            HStack(spacing: diameter * 0.38) {
                ear
                ear
            }
            .offset(y: -diameter * 0.42)

            HStack(spacing: diameter * 0.14) {
                Capsule()
                    .fill(.white.opacity(0.92))
                    .frame(width: diameter * 0.08, height: listening ? diameter * 0.14 : diameter * 0.10)
                Capsule()
                    .fill(.white.opacity(0.92))
                    .frame(width: diameter * 0.08, height: listening ? diameter * 0.14 : diameter * 0.10)
            }
            .offset(y: -diameter * 0.04)
        }
        .accessibilityLabel(listening ? "Mabel orb, listening. Tap to stop." : "Mabel orb, idle. Tap to dictate.")
        .accessibilityAddTraits(.isButton)
        .onChange(of: listening) { _, isOn in
            if isOn {
                withAnimation(.easeInOut(duration: 0.8).repeatForever(autoreverses: true)) {
                    pulse = true
                }
            } else {
                withAnimation(.easeOut(duration: 0.22)) {
                    pulse = false
                }
            }
        }
    }

    private var ear: some View {
        Triangle()
            .fill(IOSPalette.roseDeep.opacity(0.9))
            .frame(width: diameter * 0.18, height: diameter * 0.16)
    }
}

private struct Triangle: Shape {
    func path(in rect: CGRect) -> Path {
        var path = Path()
        path.move(to: CGPoint(x: rect.midX, y: rect.minY))
        path.addLine(to: CGPoint(x: rect.maxX, y: rect.maxY))
        path.addLine(to: CGPoint(x: rect.minX, y: rect.maxY))
        path.closeSubpath()
        return path
    }
}

struct WhiskerDivider: View {
    var body: some View {
        HStack(spacing: 10) {
            Capsule().fill(IOSPalette.whisker.opacity(0.25)).frame(height: 1)
            Circle().fill(IOSPalette.rose).frame(width: 6, height: 6)
            Capsule().fill(IOSPalette.whisker.opacity(0.25)).frame(height: 1)
        }
    }
}
