import SwiftUI
import UIKit

/// Home tab — Free. Cat brand, not Flow. Dictate needs no account.
struct HomeTabView: View {
    @Environment(SpeechSession.self) private var session
    @Environment(SettingsStore.self) private var settings
    @Environment(HistoryStore.self) private var history
    @Environment(\.scenePhase) private var scenePhase
    @State private var showKeyboardSetup = true

    var body: some View {
        NavigationStack {
            ScrollView {
                VStack(alignment: .leading, spacing: 22) {
                    hero
                    masterCard
                    tryInAnyApp
                    if showKeyboardSetup {
                        setupCard
                    }
                    statsCarousel
                    activityFeed
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
                ToolbarItem(placement: .topBarLeading) {
                    Menu {
                        NavigationLink("Account") {
                            AccountSettingsPane()
                        }
                        NavigationLink("Settings") {
                            SettingsRootView()
                        }
                    } label: {
                        Image(systemName: "line.3.horizontal")
                            .foregroundStyle(IOSPalette.roseDeep)
                    }
                    .accessibilityLabel("Account and Settings")
                }
            }
        }
        .tint(IOSPalette.roseDeep)
        .onAppear {
            session.refreshGate(context: .host)
            history.reload()
        }
        .onChange(of: scenePhase) { _, phase in
            if phase != .active && session.isListening {
                session.stopListening()
            }
            if phase == .active {
                session.refreshGate(context: .host)
                history.reload()
            }
        }
        .onChange(of: session.isListening) { _, listening in
            if listening == false {
                history.reload()
            }
        }
    }

    private var hero: some View {
        VStack(spacing: 14) {
            Image("MabelPortrait")
                .resizable()
                .scaledToFill()
                .frame(width: 108, height: 108)
                .clipShape(RoundedRectangle(cornerRadius: 32, style: .continuous))
                .shadow(color: IOSPalette.rose.opacity(0.35), radius: 14, y: 6)
                .accessibilityLabel("Mabel, a long-haired cat")

            Text("Dictate in any app")
                .font(.system(size: 26, weight: .semibold, design: .rounded))
                .foregroundStyle(IOSPalette.ink)
                .frame(maxWidth: .infinity, alignment: .center)

            Text("Keyboard overlay · tap to talk · no account")
                .font(.callout)
                .foregroundStyle(IOSPalette.mist)
                .frame(maxWidth: .infinity, alignment: .center)
        }
        .frame(maxWidth: .infinity)
        .padding(.top, 4)
    }

    private var masterCard: some View {
        HStack {
            VStack(alignment: .leading, spacing: 4) {
                Text("Master")
                    .font(.headline.weight(.semibold))
                    .foregroundStyle(IOSPalette.ink)
                Text(settings.masterOn
                     ? "Dictation and keyboard ready. Does not listen until you tap the orb."
                     : "Off. Dictate stays silent until you turn this on.")
                    .font(.footnote)
                    .foregroundStyle(IOSPalette.mist)
            }
            Spacer()
            Toggle("Master On", isOn: Binding(
                get: { settings.masterOn },
                set: { settings.masterOn = $0; settings.persist() }
            ))
            .labelsHidden()
            .tint(IOSPalette.roseDeep)
        }
        .padding(16)
        .background(Color.white.opacity(0.72), in: RoundedRectangle(cornerRadius: 20, style: .continuous))
    }

    private var tryInAnyApp: some View {
        Button("Try in any app") {
            showKeyboardSetup = true
        }
        .buttonStyle(.borderedProminent)
        .tint(IOSPalette.roseDeep)
        .frame(maxWidth: .infinity)
        .accessibilityHint("Opens keyboard setup so you can enable Mabel in any app.")
    }

    private var statsCarousel: some View {
        ScrollView(.horizontal, showsIndicators: false) {
            HStack(spacing: 12) {
                statCard(title: "Words today", value: "\(history.wordsToday)")
                statCard(title: "Streak", value: "\(history.streak)")
                statCard(title: "Sessions", value: "\(history.sessions)")
            }
        }
        .accessibilityElement(children: .contain)
        .accessibilityLabel("Stats carousel")
    }

    private func statCard(title: String, value: String) -> some View {
        VStack(alignment: .leading, spacing: 6) {
            Text(value)
                .font(.system(size: 28, weight: .semibold, design: .rounded))
                .foregroundStyle(IOSPalette.ink)
            Text(title)
                .font(.footnote)
                .foregroundStyle(IOSPalette.mist)
        }
        .padding(16)
        .frame(minWidth: 132, alignment: .leading)
        .background(Color.white.opacity(0.72), in: RoundedRectangle(cornerRadius: 18, style: .continuous))
    }

    private var activityFeed: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text("Activity")
                .font(.headline.weight(.semibold))
                .foregroundStyle(IOSPalette.ink)
            if history.recentDays.isEmpty {
                Text("No activity yet. Counts stay on this iPhone — \(EnforcerBound.privacySurfaceName).")
                    .font(.footnote)
                    .foregroundStyle(IOSPalette.mist)
            } else {
                ForEach(history.recentDays.prefix(14)) { day in
                    HStack {
                        Text(day.date)
                            .font(.footnote.monospaced())
                            .foregroundStyle(IOSPalette.ink)
                        Spacer()
                        Text("\(day.words) words · \(day.dictations) sessions")
                            .font(.footnote)
                            .foregroundStyle(IOSPalette.mist)
                    }
                }
            }
        }
        .padding(16)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(Color.white.opacity(0.72), in: RoundedRectangle(cornerRadius: 20, style: .continuous))
        .accessibilityLabel("Dated activity feed")
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
