import Foundation

/// Typed gate failure. `Result<String, String>` + String-as-Error trips
/// Xcode 26.6 SwiftUI Section type-check on the Polish settings pane.
struct PolishGateError: Error, Equatable {
    let message: String
}

/// Product **Polish** — Enforcer BOUND (fold hard). iOS ship #2 after Keyboard.
///
/// Modes: Off | Casual | Professional | Polite (tone rewrite).
/// Default OFF. Live modes need StoreKit Pro AND Stiki (dual gate).
/// Free locked + Activate Pro / Sign in with Stiki. Free dictate stays ungated.
/// Sign-out locks Polish (effective mode → Off).
/// Local-first rules rewrite on this iPhone. Fail closed: never invent,
/// never expand meaning, never leave the device. Not Gemma-in-appex
/// (keyboard memory). Not Style (Formal|Casual|Very casual). Not Dictionary,
/// Snippets, Scratchpad, or Clipboard.
///
/// BREAKS IF: default ON; cloud; invent; Nexus/SIEM write; HIPAA/BAA;
/// Free dictate gated; StoreKit alone or Stiki alone unlocks live Polish;
/// merged with Style/Dictionary/Snippets/Scratchpad/Clipboard stores.
enum Polish {
    static let productName = "Polish"
    static let off = "off"
    static let casual = "casual"
    static let professional = "professional"
    static let polite = "polite"
    static let modes = [off, casual, professional, polite]
    static let defaultMode = off

    static let enforcerBound =
        "default OFF; local rules fail closed; never invent; not Nexus write; StoreKit Pro + Stiki dual gate; sign-out locks Polish; distinct Style/Dictionary/Snippets/Scratchpad/Clipboard; no cloud/team/Nexus/SIEM; no HIPAA/BAA; Free dictate no Stiki; clipboardHistoryEnabled != Polish"

    static let productLock =
        "Name: Polish; iOS ship #2 after Keyboard (Keyboard → Polish → Dictionary → Scratchpad → Languages); Pro surface requires StoreKit Pro AND Stiki session; Free locked + Activate Pro / Sign in with Stiki; modes Off|Casual|Professional|Polite tone rewrite after ASR; default OFF; Settings; local-first rules on this iPhone; fail closed never invent; not Nexus; not cloud sync v1; no HIPAA/BAA; sign-out locks Polish; Free dictate no Stiki; distinct from Style (Formal|Casual|Very casual register), Dictionary (spelling), Snippets (trigger→expansion), Scratchpad (notes), and Clipboard History; non-goals: Gemma-in-appex, cloud write, Nexus/SIEM, team share, no HIPAA"

    /// Shared anti-invention contract. Mode copy only adds register guidance.
    static let neverInvent =
        "You are Mabel's on-device dictation polish assistant. The user spoke into a microphone and a local ASR engine transcribed their speech. This step is autocorrect and light reword only. Never invent facts. Never expand meaning. Never add names, numbers, clauses, greetings, sign-offs, answers, or commentary the speaker did not say. If a cleanup or register shift would require new information, keep the original wording. Fail closed: do not send this step off-device; output only the cleaned transcript. Coach cannot rewrite. Not Nexus. Not company memory."

    static let userPromptPrefix =
        "Autocorrect and lightly reword this transcript. Never invent facts. Never expand meaning. Do not think, reason, or explain. Output only the cleaned text. Transcript: "

    static let casualRegister =
        "Register: Casual. Keep the speaker's informal voice. Contractions and casual wording stay. Autocorrect and light cleanup only."

    static let professionalRegister =
        "Register: Professional. Prefer a workplace-neutral wording of the SAME utterance (gonna → going to, yeah → yes) when that does not change or expand meaning. Do not add formality, hedging, or extra clauses the speaker did not say."

    static let politeRegister =
        "Register: Polite. Prefer a courteous wording of the SAME request or statement when a close synonym already covers it. Do not add please, thanks, apologies, or extra courtesy the speaker did not say."

    enum DefaultsKey {
        static let mode = "settings.polishMode"
        static let stikiSignedIn = "settings.stikiSignedIn"
        static let storeKitEntitled = "settings.storeKitEntitled"
    }

    static func normalizeMode(_ raw: String) -> String {
        switch raw.trimmingCharacters(in: .whitespacesAndNewlines).lowercased() {
        case casual: return casual
        case professional: return professional
        case polite: return polite
        default: return off
        }
    }

    static func isLive(_ mode: String) -> Bool {
        switch normalizeMode(mode) {
        case casual, professional, polite:
            return true
        default:
            return false
        }
    }

    static func modeLabel(_ mode: String) -> String {
        switch normalizeMode(mode) {
        case casual: return "Casual"
        case professional: return "Professional"
        case polite: return "Polite"
        default: return "Off"
        }
    }

    static func systemPrompt(_ mode: String) -> String {
        let register: String
        switch normalizeMode(mode) {
        case professional: register = professionalRegister
        case polite: register = politeRegister
        default: register = casualRegister
        }
        return neverInvent + "\n\n" + register
    }

    /// Persist-time gate. Off is always allowed. Live modes need both flags.
    static func requireModeAllowed(
        _ raw: String,
        stikiSignedIn: Bool,
        storeKitEntitled: Bool
    ) -> Result<String, PolishGateError> {
        let mode = normalizeMode(raw)
        if isLive(mode) {
            guard EnforcerBound.isProUnlocked(stikiSignedIn: stikiSignedIn, storeKitEntitled: storeKitEntitled) else {
                return .failure(PolishGateError(message: EnforcerBound.polishLockedMessage))
            }
        }
        return .success(mode)
    }

    /// Runtime gate. Free / lapsed / signed-out Stiki → Off.
    static func effectiveMode(
        _ persisted: String,
        stikiSignedIn: Bool,
        storeKitEntitled: Bool
    ) -> String {
        let mode = normalizeMode(persisted)
        guard isLive(mode) else { return off }
        if EnforcerBound.isProUnlocked(stikiSignedIn: stikiSignedIn, storeKitEntitled: storeKitEntitled) {
            return mode
        }
        return off
    }

    static func applyFromAppGroup(_ text: String) -> String {
        let defaults = UserDefaults(suiteName: IOSIdentity.appGroup) ?? .standard
        let persisted = defaults.string(forKey: DefaultsKey.mode) ?? defaultMode
        let stiki = defaults.bool(forKey: DefaultsKey.stikiSignedIn)
        let storeKit = defaults.bool(forKey: DefaultsKey.storeKitEntitled)
        return apply(
            text,
            persistedMode: persisted,
            stikiSignedIn: stiki,
            storeKitEntitled: storeKit
        )
    }

    static func apply(
        _ text: String,
        persistedMode: String,
        stikiSignedIn: Bool,
        storeKitEntitled: Bool
    ) -> String {
        let piece = text.trimmingCharacters(in: .whitespacesAndNewlines)
        guard piece.isEmpty == false else { return text }
        let mode = effectiveMode(
            persistedMode,
            stikiSignedIn: stikiSignedIn,
            storeKitEntitled: storeKitEntitled
        )
        guard isLive(mode) else { return text }
        let rewritten = rewrite(piece, mode: mode)
        return acceptOrFailClosed(input: piece, output: rewritten) ?? text
    }

    /// Fail closed on invented / expanded output. Caller keeps the original.
    static func acceptOrFailClosed(input: String, output: String) -> String? {
        let cleaned = output.trimmingCharacters(in: .whitespacesAndNewlines)
        if cleaned.isEmpty { return nil }
        let inputChars = Double(input.trimmingCharacters(in: .whitespacesAndNewlines).count)
        let outChars = Double(cleaned.count)
        if inputChars >= 20 && outChars > inputChars * 2.5 {
            return nil
        }
        return cleaned
    }

    static func rewrite(_ text: String, mode: String) -> String {
        var s = stripSelfCorrections(text)
        s = stripFillers(s)
        s = applyRegister(s, mode: normalizeMode(mode))
        return punctuate(s)
    }

    private static let alwaysFillers: Set<String> = ["um", "uh", "er", "ah", "umm", "uhh"]

    private static let professionalMap: [String: String] = [
        "gonna": "going to",
        "wanna": "want to",
        "gotta": "have to",
        "kinda": "kind of",
        "yeah": "yes",
        "yup": "yes",
        "yep": "yes",
        "nope": "no",
        "dunno": "don't know",
    ]

    private static let politeMap: [String: String] = [
        "gonna": "going to",
        "wanna": "want to",
        "yeah": "yes",
        "yup": "yes",
        "yep": "yes",
        "nope": "no",
        "hey": "hello",
    ]

    private static func stripSelfCorrections(_ text: String) -> String {
        var s = text
        for marker in [" no wait ", " no, wait "] {
            if let range = s.range(of: marker, options: .caseInsensitive) {
                s = String(s[range.upperBound...])
            }
        }
        return s
    }

    private static func stripFillers(_ text: String) -> String {
        var s = text
        for phrase in ["you know", "i mean"] {
            while let range = s.range(of: phrase, options: .caseInsensitive) {
                s.replaceSubrange(range, with: " ")
            }
        }
        let tokens = s.split(whereSeparator: { $0.isWhitespace }).map(String.init)
        var kept: [String] = []
        for (index, token) in tokens.enumerated() {
            let core = token.trimmingCharacters(in: .punctuationCharacters).lowercased()
            if alwaysFillers.contains(core) { continue }
            if core == "like" {
                let prevEndsComma = index > 0 && tokens[index - 1].hasSuffix(",")
                if index == 0 || prevEndsComma || index == tokens.count - 1 {
                    continue
                }
            }
            if core == "so" && index == 0 { continue }
            kept.append(token)
        }
        return kept.joined(separator: " ")
    }

    private static func applyRegister(_ text: String, mode: String) -> String {
        let map: [String: String]
        switch mode {
        case professional: map = professionalMap
        case polite: map = politeMap
        default: return text
        }
        let tokens = text.split(whereSeparator: { $0.isWhitespace }).map(String.init)
        let rewritten = tokens.map { token -> String in
            let trimmed = token.trimmingCharacters(in: .punctuationCharacters)
            guard let replacement = map[trimmed.lowercased()] else { return token }
            guard let range = token.range(of: trimmed, options: .caseInsensitive) else { return token }
            let leading = String(token[..<range.lowerBound])
            let trailing = String(token[range.upperBound...])
            return leading + matchCase(replacement, like: trimmed) + trailing
        }
        return rewritten.joined(separator: " ")
    }

    private static func matchCase(_ replacement: String, like sample: String) -> String {
        if sample == sample.uppercased() && sample.count > 1 {
            return replacement.uppercased()
        }
        if let first = sample.first, first.isUppercase {
            return replacement.prefix(1).uppercased() + replacement.dropFirst()
        }
        return replacement
    }

    private static func punctuate(_ text: String) -> String {
        var s = text.replacingOccurrences(of: "  +", with: " ", options: .regularExpression)
            .trimmingCharacters(in: .whitespacesAndNewlines)
        guard s.isEmpty == false else { return s }
        if let first = s.first, first.isLetter {
            s.replaceSubrange(s.startIndex...s.startIndex, with: String(first).uppercased())
        }
        if let last = s.last, last.isLetter || last.isNumber {
            s += "."
        }
        return s
    }
}
