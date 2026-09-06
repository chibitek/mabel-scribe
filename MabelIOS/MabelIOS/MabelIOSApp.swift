import SwiftUI

/// Native iOS host for the Mabel keyboard. Not Tauri, not Mac Catalyst,
/// not Designed-for-visionOS, not Mochii.
@main
struct MabelIOSApp: App {
    @State private var session = SpeechSession()
    @State private var settings = SettingsStore()

    var body: some Scene {
        WindowGroup {
            HostRootView()
                .environment(session)
                .environment(settings)
        }
    }
}
