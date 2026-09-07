import SwiftUI
#if canImport(UIKit)
import UIKit
#endif

/// Shared cat-energy chrome for the host shell and the keyboard overlay.
/// Mabel cat UI only. Flow / Wispr brand is a hard break.
/// Dictate control is the Mabel cat portrait — not a geometric orb,
/// not a maple leaf.
struct MabelOrb: View {
    var listening: Bool
    var enabled: Bool = true
    var diameter: CGFloat = 148

    @State private var pulse = false

    var body: some View {
        ZStack {
            Circle()
                .fill(listening ? IOSPalette.rose.opacity(0.55) : IOSPalette.peach.opacity(0.35))
                .frame(width: diameter * 1.08, height: diameter * 1.08)
                .scaleEffect(listening && pulse ? 1.08 : 1.0)

            catImage
                .resizable()
                .scaledToFill()
                .frame(width: diameter, height: diameter)
                .clipShape(Circle())
                .opacity(enabled ? 1 : 0.45)
                .overlay {
                    Circle()
                        .strokeBorder(
                            listening ? IOSPalette.roseDeep : IOSPalette.rose.opacity(0.45),
                            lineWidth: listening ? 3 : 1.5
                        )
                }
                .shadow(
                    color: IOSPalette.rose.opacity(listening ? 0.45 : 0.18),
                    radius: listening ? 16 : 8,
                    y: 4
                )
        }
        .frame(width: diameter, height: diameter)
        .accessibilityLabel(listening ? "Mabel, listening. Tap to stop." : "Mabel, idle. Tap to dictate.")
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

    private var catImage: Image {
        #if canImport(UIKit)
        if UIImage(named: "MabelCat") != nil {
            return Image("MabelCat")
        }
        if UIImage(named: "MabelPortrait") != nil {
            return Image("MabelPortrait")
        }
        #endif
        return Image("MabelCat")
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
