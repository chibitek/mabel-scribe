import SwiftUI

/// Native visionOS entry. Not Tauri, not Designed-for-iPhone, not Mac Catalyst.
@main
struct MabelSpatialApp: App {
    @State private var session = SpatialSession()

    var body: some Scene {
        WindowGroup {
            WindowRootView()
                .environment(session)
        }
        .windowStyle(.plain)
        .windowResizability(.contentSize)
        .defaultSize(width: 520, height: 740)

        ImmersiveSpace(id: SpatialSceneID.immersive) {
            ImmersiveSpaceView()
                .environment(session)
        }
        .immersionStyle(selection: .constant(.mixed), in: .mixed)
    }
}
