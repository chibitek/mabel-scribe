import RealityKit
import UIKit

/// Marker so SpatialTapGesture can target the companion orb.
struct ListenOrbMarker: Component {}

/// Soft rose companion orb. Pulses while listening. Gaze highlight via HoverEffect.
@MainActor
final class ListeningOrbEntity: Entity {
    private let sphere = ModelEntity()
    private var pulseController: AnimationPlaybackController?

    required init() {
        super.init()
        name = "MabelListeningOrb"
        components.set(ListenOrbMarker())

        let mesh = MeshResource.generateSphere(radius: 0.075)
        sphere.model = ModelComponent(mesh: mesh, materials: [Self.idleMaterial()])
        sphere.name = "MabelOrbSphere"

        var collision = CollisionComponent(shapes: [.generateSphere(radius: 0.09)])
        collision.mode = .trigger
        sphere.components.set(collision)
        sphere.components.set(InputTargetComponent(allowedInputTypes: [.indirect, .direct]))
        sphere.components.set(HoverEffectComponent())
        sphere.components.set(ListenOrbMarker())

        addChild(sphere)
        addEyeHighlights()
    }

    func setListening(_ listening: Bool) {
        sphere.model?.materials = [listening ? Self.listeningMaterial() : Self.idleMaterial()]
        if listening {
            startPulse()
        } else {
            stopPulse()
        }
    }

    private func addEyeHighlights() {
        let eyeMesh = MeshResource.generateSphere(radius: 0.009)
        let shine = UnlitMaterial(color: UIColor(white: 1.0, alpha: 0.92))

        let left = ModelEntity(mesh: eyeMesh, materials: [shine])
        left.position = [-0.022, 0.018, 0.062]
        let right = ModelEntity(mesh: eyeMesh, materials: [shine])
        right.position = [0.022, 0.018, 0.062]
        sphere.addChild(left)
        sphere.addChild(right)
    }

    private func startPulse() {
        stopPulse()
        let animation = FromToByAnimation<Transform>(
            name: "mabel-orb-pulse",
            from: Transform(scale: SIMD3<Float>(repeating: 1.0)),
            to: Transform(scale: SIMD3<Float>(repeating: 1.14)),
            duration: 0.8,
            timing: .easeInOut,
            bindTarget: .transform,
            repeatMode: .autoReverse
        )
        guard let resource = try? AnimationResource.generate(with: animation) else { return }
        pulseController = sphere.playAnimation(resource.repeat())
    }

    private func stopPulse() {
        pulseController?.stop()
        pulseController = nil
        sphere.scale = SIMD3<Float>(repeating: 1.0)
    }

    private static func idleMaterial() -> RealityKit.Material {
        var material = PhysicallyBasedMaterial()
        material.baseColor = .init(tint: UIColor(red: 0.91, green: 0.60, blue: 0.60, alpha: 1))
        material.roughness = .init(floatLiteral: 0.38)
        material.metallic = .init(floatLiteral: 0.02)
        material.emissiveColor = .init(color: UIColor(red: 0.91, green: 0.55, blue: 0.55, alpha: 1))
        material.emissiveIntensity = 0.35
        return material
    }

    private static func listeningMaterial() -> RealityKit.Material {
        var material = PhysicallyBasedMaterial()
        material.baseColor = .init(tint: UIColor(red: 0.96, green: 0.72, blue: 0.70, alpha: 1))
        material.roughness = .init(floatLiteral: 0.22)
        material.metallic = .init(floatLiteral: 0.04)
        material.emissiveColor = .init(color: UIColor(red: 1.0, green: 0.78, blue: 0.74, alpha: 1))
        material.emissiveIntensity = 1.15
        return material
    }
}
