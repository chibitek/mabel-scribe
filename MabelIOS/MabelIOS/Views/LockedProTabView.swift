import SwiftUI

/// Locked Pro tab shell. Dual gate: Stiki AND StoreKit.
/// Free Home + dictate never go through this view.
struct LockedProTabView: View {
    let tab: String
    @Environment(SettingsStore.self) private var settings

    var body: some View {
        NavigationStack {
            VStack(spacing: 18) {
                Image(systemName: "lock.circle")
                    .font(.system(size: 44))
                    .foregroundStyle(IOSPalette.roseDeep)
                Text(tab)
                    .font(.system(size: 26, weight: .semibold, design: .rounded))
                    .foregroundStyle(IOSPalette.ink)
                Text("Needs Activate Pro and Sign in with Stiki. Both. Home dictate stays free.")
                    .font(.footnote)
                    .foregroundStyle(IOSPalette.mist)
                    .multilineTextAlignment(.center)
                    .padding(.horizontal, 24)

                NavigationLink("Activate Pro") {
                    AccountSettingsPane()
                }
                .buttonStyle(.borderedProminent)
                .tint(IOSPalette.roseDeep)

                NavigationLink("Sign in with Stiki") {
                    AccountSettingsPane()
                }
                .buttonStyle(.bordered)
            }
            .frame(maxWidth: .infinity, maxHeight: .infinity)
            .background(IOSPalette.cream.ignoresSafeArea())
            .navigationTitle(tab)
            .navigationBarTitleDisplayMode(.inline)
        }
        .tint(IOSPalette.roseDeep)
    }
}

/// Unlocked Pro tab with no feature body this tip (Keyboard ship #1).
struct ProTabLaterShipPlaceholder: View {
    let tab: String

    var body: some View {
        NavigationStack {
            VStack(spacing: 12) {
                Text(tab)
                    .font(.system(size: 26, weight: .semibold, design: .rounded))
                    .foregroundStyle(IOSPalette.ink)
                Text("Unlocked. Feature body ships later. This tip is Polish. Dictionary, Scratchpad, and Languages stay later.")
                    .font(.footnote)
                    .foregroundStyle(IOSPalette.mist)
                    .multilineTextAlignment(.center)
                    .padding(.horizontal, 24)
            }
            .frame(maxWidth: .infinity, maxHeight: .infinity)
            .background(IOSPalette.cream.ignoresSafeArea())
            .navigationTitle(tab)
        }
    }
}
