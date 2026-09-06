import SwiftUI

/// Host tab shell. Home is Free without Stiki / without account.
/// Pro tabs need StoreKit Pro AND Stiki (both). StoreKit ≠ Stiki.
/// Cat brand, not Flow.
struct HostRootView: View {
    @Environment(SettingsStore.self) private var settings

    var body: some View {
        TabView {
            HomeTabView()
                .tabItem {
                    Label("Home", systemImage: "house")
                }

            ForEach(EnforcerBound.proHomeTabs, id: \.self) { tab in
                Group {
                    if settings.isTabUnlocked(tab) {
                        ProTabLaterShipPlaceholder(tab: tab)
                    } else {
                        LockedProTabView(tab: tab)
                    }
                }
                .tabItem {
                    Label(tab, systemImage: symbol(for: tab))
                }
            }
        }
        .tint(IOSPalette.roseDeep)
    }

    private func symbol(for tab: String) -> String {
        switch tab {
        case "Dictionary":
            return "book"
        case "Snippets":
            return "text.badge.plus"
        case "Style":
            return "paintbrush"
        default:
            return "note.text"
        }
    }
}
