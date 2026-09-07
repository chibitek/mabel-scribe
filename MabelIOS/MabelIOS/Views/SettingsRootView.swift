import SwiftUI

/// Host Settings IA. Mabel cat chrome — not Flow.
/// Polish is ship #2. Keyboard overlay stays ship #1.
struct SettingsRootView: View {
    @Environment(SettingsStore.self) private var settings

    var body: some View {
        List {
            ForEach(EnforcerBound.settingsPanes, id: \.self) { pane in
                NavigationLink(pane) {
                    settingsPane(pane)
                }
            }
        }
        .scrollContentBackground(.hidden)
        .background(IOSPalette.cream.ignoresSafeArea())
        .navigationTitle("Settings")
        .navigationBarTitleDisplayMode(.inline)
        .tint(IOSPalette.roseDeep)
    }

    @ViewBuilder
    private func settingsPane(_ pane: String) -> some View {
        switch pane {
        case "Account":
            AccountSettingsPane()
        case "General":
            GeneralSettingsPane()
        case "Keyboard":
            KeyboardSettingsPane()
        case "Polish":
            PolishSettingsPane()
        case "Notifications":
            NotificationsSettingsPane()
        default:
            PrivacySettingsPane()
        }
    }
}
