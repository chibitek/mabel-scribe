import Foundation

/// Enforcer BOUND addendum (fold hard). Product RE-LOCK for Mabel iOS ship #1.
/// Suite b6530197 Local-only privacy mode + prior iOS locks.
///
/// GREEN (prior iOS): Mabel cat UI only (no Flow brand). Keyboard is not a
/// silent spy (explicit start; fail closed without mic permission). Ship
/// order is Keyboard → Polish → Dictionary → Scratchpad → Languages.
///
/// GREEN (b6530197): no HIPAA/BAA claim UI; improve-models OFF default;
/// iOS cloud storage / dictation cloud OFF/unavailable v1; no silent cloud;
/// local-only mode ships; real BAA parked.
///
/// BREAKS IF (prior iOS): Flow brand; keyboard spy / ambient always-on
/// listen; Free dictate requires Stiki; keyboard audio without permission.
///
/// BREAKS IF (b6530197): HIPAA/BAA claim; improve-models default ON;
/// dictation/cloud sync available v1; or silent cloud.
enum EnforcerBound {
    static let displayBrand = "Mabel"
    static let forbiddenBrands = ["Flow", "Wispr"]
    static let shipOrder = ["Keyboard", "Polish", "Dictionary", "Scratchpad", "Languages"]
    static let thisTip = "Keyboard"
    /// Host Settings IA. Scaffold only. Free dictate does not use these panes.
    static let settingsPanes = ["Account", "General", "Keyboard", "Notifications", "Data & privacy"]
    /// Host Home IA. Free: Home only. Pro tabs need Stiki AND StoreKit.
    static let homeTabs = ["Home", "Dictionary", "Snippets", "Style", "Scratchpad"]
    static let freeHomeTabs = ["Home"]
    static let proHomeTabs = ["Dictionary", "Snippets", "Style", "Scratchpad"]
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
    /// Improve-models must boot OFF. User may toggle locally; never silent upload.
    static let improveModelsDefaultOn = false
    static let silentCloudAllowed = false
    /// Real BAA parked. Not a claim. Do not surface in UI / ASC / marketing.
    static let hipaaBAAFollowUpParked = true

    static var shipOrderLine: String {
        shipOrder.joined(separator: " → ")
    }

    static func isForbiddenBrand(_ name: String) -> Bool {
        let folded = name.trimmingCharacters(in: .whitespacesAndNewlines)
        return forbiddenBrands.contains { $0.caseInsensitiveCompare(folded) == .orderedSame }
    }
}
