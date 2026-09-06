import Foundation
import Observation

/// Host Settings prefs. App Group so the keyboard can read extension
/// prefs later. Free dictate never reads Stiki or Pro from this store.
@MainActor
@Observable
final class SettingsStore {
    private let defaults: UserDefaults

    var stikiSignedIn = false
    var storeKitEntitled = false

    var languagesNote = "English (on-device)"
    var idleSeconds = 0
    var actionButtonDictate = false
    var autoOpenNote = false
    var lowData = false
    var haptics = true

    var qwertyLayout = true
    var autocorrect = true
    var keyboardMicPref = true
    var keyboardSounds = false

    var pushNotifications = false
    var liveActivities = false

    var improveModels = EnforcerBound.improveModelsDefaultOn
    /// v1 unavailable. Always false — cloud ON is a hard break.
    private(set) var cloudStorage = EnforcerBound.cloudStorageAvailableV1
    /// v1 unavailable. Local Apple Speech only — dictation cloud ON is a hard break.
    private(set) var dictationCloud = EnforcerBound.dictationCloudAvailableV1
    var autoDelete = false
    /// Ready switch only. Does not start the microphone.
    var masterOn = true

    /// Pro surfaces require Stiki AND StoreKit. Not used by Free dictate.
    var isProUnlocked: Bool { stikiSignedIn && storeKitEntitled }

    func isTabUnlocked(_ tab: String) -> Bool {
        if EnforcerBound.freeHomeTabs.contains(tab) {
            return true
        }
        if EnforcerBound.proHomeTabs.contains(tab) {
            return isProUnlocked
        }
        return false
    }

    init(defaults: UserDefaults? = nil) {
        self.defaults = defaults ?? UserDefaults(suiteName: IOSIdentity.appGroup) ?? .standard
        improveModels = self.defaults.bool(forKey: Key.improveModels)
        autoDelete = self.defaults.bool(forKey: Key.autoDelete)
        haptics = self.defaults.object(forKey: Key.haptics) as? Bool ?? true
        lowData = self.defaults.bool(forKey: Key.lowData)
        autoOpenNote = self.defaults.bool(forKey: Key.autoOpenNote)
        actionButtonDictate = self.defaults.bool(forKey: Key.actionButton)
        idleSeconds = self.defaults.integer(forKey: Key.idleSeconds)
        qwertyLayout = self.defaults.object(forKey: Key.qwerty) as? Bool ?? true
        autocorrect = self.defaults.object(forKey: Key.autocorrect) as? Bool ?? true
        keyboardMicPref = self.defaults.object(forKey: Key.keyboardMic) as? Bool ?? true
        keyboardSounds = self.defaults.bool(forKey: Key.keyboardSounds)
        pushNotifications = self.defaults.bool(forKey: Key.push)
        liveActivities = self.defaults.bool(forKey: Key.liveActivities)
        masterOn = self.defaults.object(forKey: Key.masterOn) as? Bool ?? true
        // Ignore any stored true. Cloud ON v1 / silent cloud is a hard break.
        cloudStorage = EnforcerBound.cloudStorageAvailableV1
        dictationCloud = EnforcerBound.dictationCloudAvailableV1
    }

    /// Cloud storage cannot be enabled in v1. Local-first toggles only.
    func requestCloudStorage(_ wanted: Bool) {
        _ = wanted
        cloudStorage = EnforcerBound.cloudStorageAvailableV1
        defaults.set(EnforcerBound.cloudStorageAvailableV1, forKey: Key.cloudStorage)
    }

    /// Dictation cloud cannot be enabled in v1. Local engines only.
    func requestDictationCloud(_ wanted: Bool) {
        _ = wanted
        dictationCloud = EnforcerBound.dictationCloudAvailableV1
        defaults.set(EnforcerBound.dictationCloudAvailableV1, forKey: Key.dictationCloud)
    }

    func persist() {
        defaults.set(improveModels, forKey: Key.improveModels)
        defaults.set(autoDelete, forKey: Key.autoDelete)
        defaults.set(haptics, forKey: Key.haptics)
        defaults.set(lowData, forKey: Key.lowData)
        defaults.set(autoOpenNote, forKey: Key.autoOpenNote)
        defaults.set(actionButtonDictate, forKey: Key.actionButton)
        defaults.set(idleSeconds, forKey: Key.idleSeconds)
        defaults.set(qwertyLayout, forKey: Key.qwerty)
        defaults.set(autocorrect, forKey: Key.autocorrect)
        defaults.set(keyboardMicPref, forKey: Key.keyboardMic)
        defaults.set(keyboardSounds, forKey: Key.keyboardSounds)
        defaults.set(pushNotifications, forKey: Key.push)
        defaults.set(liveActivities, forKey: Key.liveActivities)
        defaults.set(masterOn, forKey: Key.masterOn)
        defaults.set(EnforcerBound.cloudStorageAvailableV1, forKey: Key.cloudStorage)
        cloudStorage = EnforcerBound.cloudStorageAvailableV1
        defaults.set(EnforcerBound.dictationCloudAvailableV1, forKey: Key.dictationCloud)
        dictationCloud = EnforcerBound.dictationCloudAvailableV1
    }

    private enum Key {
        static let improveModels = "settings.improveModels"
        static let autoDelete = "settings.autoDelete"
        static let cloudStorage = "settings.cloudStorage"
        static let dictationCloud = "settings.dictationCloud"
        static let haptics = "settings.haptics"
        static let lowData = "settings.lowData"
        static let autoOpenNote = "settings.autoOpenNote"
        static let actionButton = "settings.actionButton"
        static let idleSeconds = "settings.idleSeconds"
        static let qwerty = "settings.qwerty"
        static let autocorrect = "settings.autocorrect"
        static let keyboardMic = "settings.keyboardMic"
        static let keyboardSounds = "settings.keyboardSounds"
        static let push = "settings.push"
        static let liveActivities = "settings.liveActivities"
        static let masterOn = "settings.masterOn"
    }
}
