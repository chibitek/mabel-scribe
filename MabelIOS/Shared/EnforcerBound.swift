import Foundation

/// Enforcer BOUND addendum (fold hard). Product RE-LOCK for Mabel iOS ship #2.
/// Suite b6530197 Local-only privacy mode + Keyboard locks + Polish.
///
/// GREEN (prior iOS): Mabel cat UI only (no Flow brand). Keyboard is not a
/// silent spy (explicit start; fail closed without mic permission). Ship
/// order is Keyboard → Polish → Dictionary → Scratchpad → Languages.
///
/// GREEN (b6530197): Local-only mode ships; real HIPAA BAA parked (no Make It So);
/// no HIPAA / BAA / Wispr BAA / compliant claim in UI or App Store;
/// improve-models OFF default (no silent training upload);
/// cloud storage + dictation cloud OFF/unavailable v1;
/// no silent cloud of audio/transcripts/Scratchpad/history;
/// local-only / local-first copy OK in Settings.
///
/// BREAKS IF (prior iOS): Flow brand; keyboard spy / ambient always-on
/// listen; Free dictate requires Stiki; keyboard audio without permission.
///
/// BREAKS IF (b6530197): HIPAA/BAA/Wispr BAA claim ships;
/// improve-models default ON or silent upload;
/// dictation cloud or cloud storage ON/available as sync v1;
/// silent cloud of local content.
///
/// GREEN (b6530197 Home IA): Free Home + dictate available without Stiki /
/// without account. Pro tabs / Pro surfaces require StoreKit Pro AND Stiki
/// session (both). StoreKit ≠ Stiki; neither alone unlocks Pro tabs.
///
/// BREAKS IF (b6530197 Home IA): Free Home or Free dictate gated on
/// Stiki/account; Pro tabs unlock without StoreKit Pro + Stiki; StoreKit
/// alone or Stiki alone treated as full Pro unlock for those tabs.
///
/// GREEN (Polish): Off|Casual|Professional|Polite; default OFF; StoreKit Pro AND Stiki dual gate;
/// Free locked Activate Pro / Sign in with Stiki; local-first rules fail closed;
/// never invent; sign-out locks Polish; distinct Style/Dictionary/Snippets/Scratchpad/Clipboard;
/// Free dictate ungated; no cloud/team/Nexus/SIEM; no HIPAA/BAA.
///
/// BREAKS IF (Polish): default ON; cloud rewrite; invent / expand meaning;
/// Nexus/SIEM write; HIPAA/BAA; Free dictate gated; StoreKit alone or Stiki
/// alone unlocks live Polish; merged with Style Formal|Casual|Very casual.
///
/// GREEN (CoS maple): maple = Mabel brand / Mabel cat icon, not a maple leaf.
/// Dictate control is the Mabel cat (replace orb). Sticky listen + full
/// keyboard edit. Soft nits later.
///
/// BREAKS IF (CoS maple): maple leaf icon; geometric orb as the dictate icon.
///
/// GREEN (history-survive-updates): MUST migrate local App Group / container only.
/// MUST NOT invent cloud sync, Nexus/Mochii write, clipboard spy/coach, or HIPAA claim.
/// iOS 0.1.0 has no history file.
/// BREAKS IF: cloud sync; Nexus/Mochii write; clipboard spy/coach; HIPAA claim;
/// invented iOS history; mock restore.
///
/// GREEN (Erick 2026-09-07 Flow→Mabel iOS UX): sticky listen after explicit
/// Start; clear on-state "Mabel is on" (not Flow brand); swipe bottom-edge
/// continue/confirm does not start the mic; full keyboard Start/mic chrome;
/// keyboard tone picker is existing Polish Off|Casual|Professional|Polite
/// only (Style Formal|Casual|Very casual stays the Style catalog — later
/// iOS body, distinct store); Pro+Stiki dual gate; default Off; privacy
/// card lists local defaults; no HIPAA/BAA.
/// BREAKS IF (Erick 2026-09-07 Flow→Mabel iOS UX): Flow brand / "Flow is on";
/// ambient / always-on listen; swipe starts the mic; second tone catalog;
/// Formal/Casual invented separate from Polish/Style; dual gate weakened;
/// Polish default ON; HIPAA/BAA claim; cloud / Nexus / Mochii write.
enum EnforcerBound {
    static let displayBrand = "Mabel"
    static let forbiddenBrands = ["Flow", "Wispr"]
    static let shipOrder = ["Keyboard", "Polish", "Dictionary", "Scratchpad", "Languages"]
    static let thisTip = "Polish"
    /// Host Settings IA. Free dictate does not use these panes.
    static let settingsPanes = ["Account", "General", "Keyboard", "Polish", "Notifications", "Data & privacy"]
    static let polishModes = ["off", "casual", "professional", "polite"]
    static let polishDefaultOff = true
    static let polishRequiresDualGate = true
    static let polishSignOutLocks = true
    static let polishDistinctFromStyle = true
    static let polishLockedMessage = "Needs Activate Pro and Sign in with Stiki. Both. Home dictate stays free."
    /// Host Home IA. Free: Home only. Pro tabs need StoreKit Pro AND Stiki.
    static let homeTabs = ["Home", "Dictionary", "Snippets", "Style", "Scratchpad"]
    static let freeHomeTabs = ["Home"]
    static let proHomeTabs = ["Dictionary", "Snippets", "Style", "Scratchpad"]
    /// GREEN: Free Home + dictate available without Stiki / without account.
    static let freeHomeAndDictateRequireAccount = false
    /// GREEN: Pro tabs require StoreKit Pro AND Stiki session (both).
    static let proUnlockRequiresStoreKitAndStiki = true
    /// StoreKit ≠ Stiki. Neither alone unlocks Pro tabs.
    static let storeKitEqualsStiki = false
    /// Suite b6530197 — Local-only privacy mode.
    static let privacySuite = "b6530197"
    /// Surface copy. Use this only — never HIPAA/BAA.
    static let privacySurfaceName = "Local-only privacy mode"
    /// GREEN: local-only mode ships.
    static let localOnlyModeShips = true
    /// iOS cloud storage OFF/unavailable v1.
    static let cloudStorageAvailableV1 = false
    /// iOS dictation cloud OFF/unavailable v1. Local engines only.
    static let dictationCloudAvailableV1 = false
    /// BREAKS IF: dictation/cloud sync available v1.
    static let cloudSyncAvailableV1 = false
    /// Improve-models must boot OFF. User may toggle locally; never silent training upload.
    static let improveModelsDefaultOn = false
    static let silentTrainingUploadAllowed = false
    /// No silent cloud of audio / transcripts / Scratchpad / history.
    static let silentCloudAllowed = false
    /// Real HIPAA BAA parked. No Make It So. Not a claim in UI or App Store.
    static let hipaaBAAFollowUpParked = true
    static let hipaaMakeItSo = false
    /// BREAKS IF: HIPAA/BAA/Wispr BAA claim ships.
    static let wisprBAAClaimAllowed = false
    /// GREEN: Local-only / local-first copy OK in Settings.
    static let localOnlyLocalFirstCopyOK = true
    /// CoS Product LOCK (TF 0.1.0/2): maple = Mabel cat icon, not a maple leaf.
    static let mapleMeansMabelCatIcon = true
    static let mapleLeafIconAllowed = false
    static let dictateIconIsMabelCat = true
    static let geometricOrbDictateIconAllowed = false
    /// Enforcer BOUND addendum (CoS): MUST migrate local App Group /
    /// container only. MUST NOT invent cloud sync, Nexus/Mochii write,
    /// clipboard spy/coach, or HIPAA claim.
    static let historySurviveLocalContainerOnly = true
    static let historySurviveCloudSyncAllowed = false
    static let historySurviveNexusWriteAllowed = false
    static let historySurviveMochiiWriteAllowed = false
    static let historySurviveClipboardSpyAllowed = false
    static let historySurviveCoachAllowed = false
    static let historySurviveHipaaClaimAllowed = false
    /// Erick 2026-09-07 Flow→Mabel iOS UX. On-state is Mabel, not Flow.
    static let keyboardOnStateCopy = "Mabel is on"
    static let keyboardFlowIsOnCopyAllowed = false
    static let keyboardSwipeConfirmDoesNotStartMic = true
    static let keyboardStartMicChrome = true
    static let keyboardTonePickerIsExistingPolish = true
    static let keyboardSecondToneCatalogAllowed = false
    static let keyboardInventedFormalCasualCatalogAllowed = false
    static let privacyCardListsLocalDefaults = true

    static var shipOrderLine: String {
        shipOrder.joined(separator: " → ")
    }

    static func isForbiddenBrand(_ name: String) -> Bool {
        let folded = name.trimmingCharacters(in: .whitespacesAndNewlines)
        return forbiddenBrands.contains { $0.caseInsensitiveCompare(folded) == .orderedSame }
    }

    /// StoreKit ≠ Stiki. Neither flag alone is a Pro unlock.
    static func isProUnlocked(stikiSignedIn: Bool, storeKitEntitled: Bool) -> Bool {
        stikiSignedIn && storeKitEntitled
    }

    /// Free Home is never gated on Stiki/account. Pro tabs need both.
    static func isHomeTabUnlocked(_ tab: String, stikiSignedIn: Bool, storeKitEntitled: Bool) -> Bool {
        if freeHomeTabs.contains(tab) {
            return true
        }
        if proHomeTabs.contains(tab) {
            return isProUnlocked(stikiSignedIn: stikiSignedIn, storeKitEntitled: storeKitEntitled)
        }
        return false
    }
}
