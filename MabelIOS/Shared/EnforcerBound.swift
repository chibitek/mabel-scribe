import Foundation

/// Enforcer BOUND addendum (fold hard). Product RE-LOCK for Mabel iOS ship #1.
/// Suite b6530197 Data & privacy + prior iOS locks.
///
/// GREEN: Mabel cat UI only (no Flow brand). Keyboard is not a silent spy
/// (explicit start; fail closed without mic permission). Ship order is
/// Keyboard → Polish → Dictionary → Scratchpad → Languages.
/// Local-only privacy mode (v1 NOW): cloud OFF/unavailable; dictation cloud
/// OFF/unavailable (on-device engines only); improve-models OFF default;
/// no silent cloud. HIPAA/BAA follow-up is parked — no claim in UI / ASC /
/// marketing / Settings. Local-first toggles OK.
///
/// BREAKS IF: Flow brand; keyboard spy / ambient always-on listen;
/// Free dictate requires Stiki; keyboard audio without permission;
/// cloud ON v1; improve-models default ON / silent upload; BAA/HIPAA claim
/// (including “HIPAA compliant”); silent cloud; dictation cloud ON iOS v1.
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
    /// Suite b6530197 — Data & privacy. Cloud cannot be ON in v1.
    static let cloudStorageAvailableV1 = false
    /// Improve-models must boot OFF. User may toggle locally; never silent upload.
    static let improveModelsDefaultOn = false
    static let silentCloudAllowed = false
    /// v1 NOW surface name. Use this copy — never “HIPAA compliant.”
    static let privacySurfaceName = "Local-only privacy mode"
    /// iOS dictation stays on-device. No cloud engine in v1.
    static let dictationCloudAvailableV1 = false
    /// Real HIPAA BAA work is parked. Not a claim. Do not surface in UI / ASC / marketing.
    static let hipaaBAAFollowUpParked = true

    static var shipOrderLine: String {
        shipOrder.joined(separator: " → ")
    }

    static func isForbiddenBrand(_ name: String) -> Bool {
        let folded = name.trimmingCharacters(in: .whitespacesAndNewlines)
        return forbiddenBrands.contains { $0.caseInsensitiveCompare(folded) == .orderedSame }
    }
}
