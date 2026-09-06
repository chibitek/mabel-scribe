import RealityKit
import SwiftUI

/// Mixed immersive space: companion orb + floating transcript attachment.
/// Eyes + hands: look at the orb (HoverEffect) and pinch (SpatialTapGesture).
struct ImmersiveSpaceView: View {
    @Environment(SpatialSession.self) private var session

    var body: some View {
        RealityView { content, attachments in
            let orb = ListeningOrbEntity()
            orb.position = SIMD3<Float>(0, 1.42, -0.85)
            content.add(orb)

            if let panel = attachments.entity(for: "transcript") {
                panel.position = SIMD3<Float>(0.32, 1.44, -0.82)
                content.add(panel)
            }

            if let label = attachments.entity(for: "title") {
                label.position = SIMD3<Float>(0, 1.62, -0.88)
                content.add(label)
            }
        } update: { content, _ in
            for entity in content.entities {
                if let orb = entity as? ListeningOrbEntity {
                    orb.setListening(session.isListening)
                }
                entity.children.forEach { child in
                    if let orb = child as? ListeningOrbEntity {
                        orb.setListening(session.isListening)
                    }
                }
            }
        } attachments: {
            Attachment(id: "transcript") {
                FloatingTranscriptPanel()
                    .environment(session)
                    .frame(width: 380)
            }
            Attachment(id: "title") {
                Text(SpatialIdentity.displayName)
                    .font(.largeTitle.weight(.semibold))
                    .foregroundStyle(SpatialPalette.cream)
                    .padding(.horizontal, 20)
                    .padding(.vertical, 10)
                    .glassBackgroundEffect()
                    .accessibilityAddTraits(.isHeader)
            }
        }
        .gesture(
            SpatialTapGesture()
                .targetedToAnyEntity()
                .onEnded { value in
                    if value.entity.components.has(ListenOrbMarker.self) {
                        session.toggleListening()
                    }
                }
        )
    }
}
