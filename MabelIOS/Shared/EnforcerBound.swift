import Foundation

/// Enforcer BOUND addendum (fold hard). Product RE-LOCK for Mabel iOS ship #1.
///
/// GREEN: Mabel cat UI only (no Flow brand). Keyboard is not a silent spy
/// (explicit start; fail closed without mic permission). Ship order is
/// Keyboard → Polish → Dictionary → Scratchpad → Languages.
///
/// BREAKS IF: Flow brand; keyboard spy / ambient always-on listen;
/// Free dictate requires Stiki; keyboard audio without permission.
enum EnforcerBound {
    static let displayBrand = "Mabel"
    static let forbiddenBrands = ["Flow", "Wispr"]
    static let shipOrder = ["Keyboard", "Polish", "Dictionary", "Scratchpad", "Languages"]
    static let thisTip = "Keyboard"
    /// Host Settings IA. Scaffold only. Free dictate does not use these panes.
    static let settingsPanes = ["Account", "General", "Keyboard", "Notifications", "Data & privacy"]

    static var shipOrderLine: String {
        shipOrder.joined(separator: " → ")
    }

    static func isForbiddenBrand(_ name: String) -> Bool {
        let folded = name.trimmingCharacters(in: .whitespacesAndNewlines)
        return forbiddenBrands.contains { $0.caseInsensitiveCompare(folded) == .orderedSame }
    }
}
