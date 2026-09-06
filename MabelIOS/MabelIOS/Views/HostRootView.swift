import SwiftUI
import UIKit

/// Host shell: cat UI, permission gates, keyboard install path, Free dictate playground.
struct HostRootView: View {
    @Environment(SpeechSession.self) private var session
    @Environment(\.scenePhase) private var scenePhase

    var body: some View {
        NavigationStack {
            ScrollView {
                VStack(alignment: .leading, spacing: 22) {
                    hero
                    WhiskerDivider()
                    setupCard
                    HostDictatePlayground()
                    privacyCard
                    versionLine
                }
                .padding(22)
            }
            .background(IOSPalette.cream.ignoresSafeArea())
            .navigationTitle(IOSIdentity.displayName)
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .topBarTrailing) {
                    NavigationLink {
                        SettingsRootView()
                    } label: {
                        Image(systemName: "gearshape")
                            .foregroundStyle(IOSPalette.roseDeep)
                    }
                    .accessibilityLabel("Settings")
                }
            }
        }
        .tint(IOSPalette.roseDeep)
        .onAppear {
            session.refreshGate(context: .host)
        }
        .onChange(of: scenePhase) { _, phase in
            if phase != .active && session.isListening {
                session.stopListening()
            }
            if phase == .active {
                session.refreshGate(context: .host)
            }
        }
    }

    private var hero: some View {
        VStack(spacing: 14) {
            Image("MabelPortrait")
                .resizable()
                .scaledToFill()
                .frame(width: 132, height: 132)
                .clipShape(RoundedRectangle(cornerRadius: 36, style: .continuous))
                .shadow(color: IOSPalette.rose.opacity(0.35), radius: 16, y: 8)
                .accessibilityLabel("Mabel, a long-haired cat")

            Text("Dictate in any app")
                .font(.system(size: 28, weight: .semibold, design: .rounded))
                .foregroundStyle(IOSPalette.ink)
                .frame(maxWidth: .infinity, alignment: .center)

            Text("Keyboard overlay · tap to talk · no account")
                .font(.callout)
                .foregroundStyle(IOSPalette.mist)
                .frame(maxWidth: .infinity, alignment: .center)
        }
        .frame(maxWidth: .infinity)
        .padding(.top, 8)
    }

    private var setupCard: some View {
        VStack(alignment: .leading, spacing: 14) {
            Text("Make the keyboard live")
                .font(.headline.weight(.semibold))
                .foregroundStyle(IOSPalette.ink)

            setupRow(
                number: "1",
                title: "Allow Microphone + Speech",
                detail: session.lastGate == .ready
                    ? "Granted. Free dictate does not need an account."
                    : "Mabel asks only when you tap Allow. Denied means the keyboard stays silent."
            )
            HStack(spacing: 10) {
                Button("Allow permissions") {
                    session.requestHostPermissions()
                }
                .buttonStyle(.borderedProminent)
                .tint(IOSPalette.roseDeep)

                Button("iOS Settings") {
                    openSystemSettings()
                }
                .buttonStyle(.bordered)
            }

            setupRow(
                number: "2",
                title: "Add the Mabel keyboard",
                detail: "Settings → General → Keyboard → Keyboards → Add New Keyboard… → Mabel"
            )

            setupRow(
                number: "3",
                title: "Turn on Allow Full Access",
                detail: IOSPrivacy.fullAccessWhy
            )
        }
        .padding(16)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(Color.white.opacity(0.72), in: RoundedRectangle(cornerRadius: 20, style: .continuous))
    }

    private func setupRow(number: String, title: String, detail: String) -> some View {
        HStack(alignment: .top, spacing: 12) {
            Text(number)
                .font(.caption.weight(.bold))
                .foregroundStyle(.white)
                .frame(width: 22, height: 22)
                .background(IOSPalette.roseDeep, in: Circle())
            VStack(alignment: .leading, spacing: 4) {
                Text(title)
                    .font(.subheadline.weight(.semibold))
                    .foregroundStyle(IOSPalette.ink)
                Text(detail)
                    .font(.footnote)
                    .foregroundStyle(IOSPalette.mist)
            }
        }
    }

    private var privacyCard: some View {
        Text(IOSPrivacy.blurb)
            .font(.footnote)
            .foregroundStyle(IOSPalette.mist)
            .padding(14)
            .frame(maxWidth: .infinity, alignment: .leading)
            .background(Color.white.opacity(0.62), in: RoundedRectangle(cornerRadius: 16, style: .continuous))
    }

    private var versionLine: some View {
        Text("v\(IOSIdentity.marketingVersion) · \(IOSIdentity.bundleID) · \(EnforcerBound.thisTip) · \(EnforcerBound.shipOrderLine)")
            .font(.caption2.monospaced())
            .foregroundStyle(IOSPalette.mist.opacity(0.85))
            .frame(maxWidth: .infinity, alignment: .center)
            .padding(.bottom, 12)
    }

    private func openSystemSettings() {
        guard let url = URL(string: UIApplication.openSettingsURLString) else { return }
        UIApplication.shared.open(url)
    }
}
