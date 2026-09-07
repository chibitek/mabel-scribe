import SwiftUI

/// Settings pane bodies. Scaffold only — no StoreKit import, no Stiki SDK.
/// Dual gate (Stiki + Pro) is displayed; Free dictate does not read it.

struct AccountSettingsPane: View {
    @Environment(SettingsStore.self) private var settings

    var body: some View {
        Form {
            Section("Stiki") {
                Text("Sign in is optional. Free keyboard dictate does not need an account.")
                    .font(.footnote)
                    .foregroundStyle(IOSPalette.mist)
                stikiButton("Continue with Apple")
                stikiButton("Continue with Google")
                stikiButton("Continue with Microsoft")
                Text(settings.stikiSignedIn ? "Signed in (scaffold)" : "Not signed in")
                    .font(.footnote)
                    .foregroundStyle(IOSPalette.mist)
            }

            Section("Pro") {
                Text("Pro needs Stiki and a purchase. Both. Not required for Free dictate.")
                    .font(.footnote)
                    .foregroundStyle(IOSPalette.mist)
                proButton("Subscribe")
                proButton("Restore")
                proButton("Manage")
                proButton("Have a code?")
                Text(settings.isProUnlocked ? "Pro unlocked" : "Pro locked · needs Stiki + purchase")
                    .font(.footnote)
                    .foregroundStyle(IOSPalette.mist)
            }

            Section {
                Button("Sign out") {
                    settings.signOutStiki()
                }
                .disabled(settings.stikiSignedIn == false)
            } footer: {
                Text("Sign out locks Polish. Free dictate stays available.")
                    .foregroundStyle(IOSPalette.mist)
            }
        }
        .scrollContentBackground(.hidden)
        .background(IOSPalette.cream.ignoresSafeArea())
        .navigationTitle("Account")
        .tint(IOSPalette.roseDeep)
    }

    private func stikiButton(_ title: String) -> some View {
        Button(title) {
            settings.stikiSignedIn = false
            settings.persist()
        }
        .disabled(true)
    }

    private func proButton(_ title: String) -> some View {
        Button(title) {
            // Purchase / restore / redeem when ASC products exist.
            // Dual gate stays closed until Stiki AND entitlement are both live.
            settings.storeKitEntitled = false
            settings.persist()
        }
        .disabled(true)
    }
}

struct GeneralSettingsPane: View {
    var body: some View {
        SettingsStoreForm { store in
            Section("Languages") {
                Text(store.languagesNote)
                Text("Languages ship later. This tip is Polish.")
                    .font(.footnote)
                    .foregroundStyle(IOSPalette.mist)
            }
            Section("Idle") {
                Stepper(idleLabel(store.idleSeconds), value: Binding(
                    get: { store.idleSeconds },
                    set: { store.idleSeconds = $0; store.persist() }
                ), in: 0...30)
                Text("Idle only stops a session you already started. It never starts the microphone.")
                    .font(.footnote)
                    .foregroundStyle(IOSPalette.mist)
            }
            Section("Phone") {
                Toggle("Action Button starts dictate", isOn: bind(store, \.actionButtonDictate))
                Toggle("Auto-open note", isOn: bind(store, \.autoOpenNote))
                Toggle("Low data", isOn: bind(store, \.lowData))
                Toggle("Haptics", isOn: bind(store, \.haptics))
            }
        }
        .navigationTitle("General")
    }

    private func idleLabel(_ seconds: Int) -> String {
        seconds == 0 ? "Idle stop off" : "Stop after \(seconds)s of silence"
    }
}

struct KeyboardSettingsPane: View {
    var body: some View {
        SettingsStoreForm { store in
            Section("Layout") {
                Toggle("QWERTY layout", isOn: bind(store, \.qwertyLayout))
                Toggle("Autocorrect", isOn: bind(store, \.autocorrect))
            }
            Section("Microphone") {
                Toggle("Allow keyboard mic", isOn: bind(store, \.keyboardMicPref))
                Text("Off means the keyboard stays silent even if iOS permission is on. System denial still fail-closes.")
                    .font(.footnote)
                    .foregroundStyle(IOSPalette.mist)
            }
            Section("Sounds") {
                Toggle("Key sounds", isOn: bind(store, \.keyboardSounds))
            }
        }
        .navigationTitle("Keyboard")
    }
}

struct PolishSettingsPane: View {
    var body: some View {
        SettingsStoreForm { store in
            Section("Polish") {
                Text("Off, Casual, Professional, or Polite. After dictate, a local tone rewrite on this iPhone — autocorrect and light reword only. Never invents facts. Not Style (Formal, Casual, Very casual). Not Dictionary, Snippets, Scratchpad, or Clipboard. Coach cannot rewrite. Not Nexus.")
                    .font(.footnote)
                    .foregroundStyle(IOSPalette.mist)
                    .accessibilityIdentifier("polish-hint")

                if store.isProUnlocked {
                    Toggle("Polish", isOn: Binding(
                        get: { Polish.isLive(store.effectivePolishMode) },
                        set: { on in
                            if on {
                                let next = store.polishMode == Polish.off ? Polish.casual : store.polishMode
                                store.setPolishMode(next)
                            } else {
                                store.setPolishMode(Polish.off)
                            }
                        }
                    ))
                    .accessibilityIdentifier("polish-toggle")
                    .accessibilityLabel("Polish")

                    Picker("Polish mode", selection: Binding(
                        get: { store.effectivePolishMode },
                        set: { store.setPolishMode($0) }
                    )) {
                        Text("Off").tag(Polish.off)
                        Text("Casual").tag(Polish.casual)
                        Text("Professional").tag(Polish.professional)
                        Text("Polite").tag(Polish.polite)
                    }
                    .accessibilityIdentifier("polish-mode-select")
                    .accessibilityLabel("Polish mode")
                } else {
                    Text(EnforcerBound.polishLockedMessage)
                        .font(.footnote)
                        .foregroundStyle(IOSPalette.mist)
                    NavigationLink("Activate Pro") {
                        AccountSettingsPane()
                    }
                    .accessibilityIdentifier("polish-activate")
                    NavigationLink("Sign in with Stiki") {
                        AccountSettingsPane()
                    }
                }
            } footer: {
                Text("Default Off. Sign out locks Polish. Free dictate stays ungated. Local-first — no cloud, no team, no Nexus.")
                    .foregroundStyle(IOSPalette.mist)
            }
        }
        .navigationTitle("Polish")
    }
}

struct NotificationsSettingsPane: View {
    var body: some View {
        SettingsStoreForm { store in
            Section {
                Toggle("Push notifications", isOn: bind(store, \.pushNotifications))
                Toggle("Live Activities", isOn: bind(store, \.liveActivities))
            } footer: {
                Text("Off by default. Nothing is registered until you turn a switch on in a later ship.")
                    .foregroundStyle(IOSPalette.mist)
            }
        }
        .navigationTitle("Notifications")
    }
}

struct PrivacySettingsPane: View {
    var body: some View {
        SettingsStoreForm { store in
            Section(EnforcerBound.privacySurfaceName) {
                Text(IOSPrivacy.localOnlyMode)
                    .font(.footnote)
                    .foregroundStyle(IOSPalette.mist)
            }
            Section {
                Toggle("Improve models", isOn: bind(store, \.improveModels))
                Toggle("Dictation cloud", isOn: Binding(
                    get: { EnforcerBound.dictationCloudAvailableV1 },
                    set: { store.requestDictationCloud($0) }
                ))
                .disabled(EnforcerBound.dictationCloudAvailableV1 == false)
                Toggle("Cloud storage", isOn: Binding(
                    get: { EnforcerBound.cloudStorageAvailableV1 },
                    set: { store.requestCloudStorage($0) }
                ))
                .disabled(EnforcerBound.cloudStorageAvailableV1 == false)
                Toggle("Auto-delete transcripts", isOn: bind(store, \.autoDelete))
            } footer: {
                Text("Improve models is off unless you turn it on. Dictation cloud and cloud storage are unavailable in v1 — local engines only. Scratchpad stays local-first.")
                    .foregroundStyle(IOSPalette.mist)
            }
        }
        .navigationTitle("Data & privacy")
    }
}

private struct SettingsStoreForm<Content: View>: View {
    @Environment(SettingsStore.self) private var settings
    @ViewBuilder var content: (SettingsStore) -> Content

    var body: some View {
        Form {
            content(settings)
        }
        .scrollContentBackground(.hidden)
        .background(IOSPalette.cream.ignoresSafeArea())
        .tint(IOSPalette.roseDeep)
    }
}

private func bind(_ store: SettingsStore, _ keyPath: ReferenceWritableKeyPath<SettingsStore, Bool>) -> Binding<Bool> {
    Binding(
        get: { store[keyPath: keyPath] },
        set: { store[keyPath: keyPath] = $0; store.persist() }
    )
}
