import Foundation
import Observation

/// Local-only dictation counts for iOS host + keyboard.
///
/// Durable path is the App Group container
/// `group.com.mabel.ios` / Library/Application Support / `com.mabel.ios` /
/// `stats.json`. That container survives TestFlight overlays (0.1.0/2+).
/// First install is empty — there is no legacy iOS migrate on this tip.
///
/// Never reset on version bump. Never write empty over a present file.
/// Copy-not-claimed: this is the current store, not a Mac import.
/// Local-first: no iCloud, CloudKit, Nexus, Mochii, or mock restore.
/// Free Home reads this store with no Stiki / StoreKit gate.
@MainActor
@Observable
final class HistoryStore {
    static let fileName = "stats.json"
    static let markerName = ".history-store-v1"

    var snapshot: HistorySnapshot

    private let fileURL: URL

    init(fileURL: URL? = nil) {
        self.fileURL = fileURL ?? Self.defaultFileURL()
        self.snapshot = Self.readExisting(from: self.fileURL)
    }

    var wordsToday: UInt64 { snapshot.daily[Self.todayKey()]?.words ?? 0 }
    var sessions: UInt64 { snapshot.totalDictations }
    var streak: UInt64 { Self.currentStreak(snapshot) }

    var recentDays: [HistoryDay] {
        snapshot.daily.keys.sorted(by: >).compactMap { key in
            guard let day = snapshot.daily[key], day.dictations > 0 else { return nil }
            return HistoryDay(date: key, dictations: day.dictations, words: day.words)
        }
    }

    func reload() {
        snapshot = Self.readExisting(from: fileURL)
    }

    func record(text: String, seconds: Double = 0) {
        let words = Self.wordCount(text)
        guard words > 0 else { return }
        Self.persistTake(words: words, seconds: seconds, fileURL: fileURL)
        reload()
    }

    /// Keyboard + host (separate processes) write the same App Group file.
    static func persistTake(text: String, seconds: Double = 0) {
        let words = wordCount(text)
        guard words > 0 else { return }
        persistTake(words: words, seconds: seconds, fileURL: defaultFileURL())
    }

    /// App Group first. App-container Application Support only if the group
    /// is unavailable (should not happen on the signed TF host/keyboard).
    static func defaultFileURL() -> URL {
        if let group = FileManager.default.containerURL(
            forSecurityApplicationGroupIdentifier: IOSIdentity.appGroup
        ) {
            return group
                .appendingPathComponent("Library", isDirectory: true)
                .appendingPathComponent("Application Support", isDirectory: true)
                .appendingPathComponent(IOSIdentity.bundleID, isDirectory: true)
                .appendingPathComponent(fileName)
        }
        let support = FileManager.default.urls(
            for: .applicationSupportDirectory,
            in: .userDomainMask
        ).first ?? FileManager.default.temporaryDirectory
        return support
            .appendingPathComponent(IOSIdentity.bundleID, isDirectory: true)
            .appendingPathComponent(fileName)
    }

    static func wordCount(_ text: String) -> UInt64 {
        let parts = text.split { $0.isWhitespace || $0.isNewline }.filter { !$0.isEmpty }
        return UInt64(parts.count)
    }

    // MARK: - File (fail closed: never overwrite unreadable bytes)

    static func persistTake(words: UInt64, seconds: Double, fileURL: URL) {
        var snap = readExisting(from: fileURL)
        let key = todayKey()
        var day = snap.daily[key] ?? DailyStat()
        day.dictations += 1
        day.words += words
        day.seconds += seconds
        snap.daily[key] = day
        snap.totalDictations += 1
        snap.totalWords += words
        snap.totalSeconds += seconds
        writeIfEncodable(snap, to: fileURL)
    }

    static func readExisting(from fileURL: URL) -> HistorySnapshot {
        let fm = FileManager.default
        guard fm.fileExists(atPath: fileURL.path) else {
            return .empty
        }
        do {
            let data = try Data(contentsOf: fileURL)
            if data.isEmpty {
                return .empty
            }
            return try JSONDecoder().decode(HistorySnapshot.self, from: data)
        } catch {
            // Present but unreadable: leave the file untouched so an update
            // cannot look like a wipe. In-memory view stays empty.
            return .empty
        }
    }

    private static func writeIfEncodable(_ snap: HistorySnapshot, to fileURL: URL) {
        do {
            let parent = fileURL.deletingLastPathComponent()
            try FileManager.default.createDirectory(at: parent, withIntermediateDirectories: true)
            let data = try JSONEncoder().encode(snap)
            try data.write(to: fileURL, options: .atomic)
            let marker = parent.appendingPathComponent(markerName)
            if FileManager.default.fileExists(atPath: marker.path) == false {
                try Data("app-group-local\n".utf8).write(to: marker, options: .atomic)
            }
        } catch {
            // Fail closed: keep the previous file.
        }
    }

    static func todayKey() -> String {
        let f = DateFormatter()
        f.calendar = Calendar(identifier: .gregorian)
        f.locale = Locale(identifier: "en_US_POSIX")
        f.timeZone = TimeZone.current
        f.dateFormat = "yyyy-MM-dd"
        return f.string(from: Date())
    }

    static func currentStreak(_ snap: HistorySnapshot) -> UInt64 {
        var date = Date()
        var streak: UInt64 = 0
        let cal = Calendar(identifier: .gregorian)
        let key = { (d: Date) -> String in
            let f = DateFormatter()
            f.calendar = cal
            f.locale = Locale(identifier: "en_US_POSIX")
            f.timeZone = TimeZone.current
            f.dateFormat = "yyyy-MM-dd"
            return f.string(from: d)
        }
        if (snap.daily[key(date)]?.dictations ?? 0) == 0 {
            date = cal.date(byAdding: .day, value: -1, to: date) ?? date
        }
        while (snap.daily[key(date)]?.dictations ?? 0) > 0 {
            streak += 1
            guard let prev = cal.date(byAdding: .day, value: -1, to: date) else { break }
            date = prev
        }
        return streak
    }
}

struct HistoryDay: Equatable, Identifiable {
    var id: String { date }
    let date: String
    let dictations: UInt64
    let words: UInt64
}

struct DailyStat: Codable, Equatable {
    var dictations: UInt64 = 0
    var words: UInt64 = 0
    var seconds: Double = 0
}

struct HistorySnapshot: Codable, Equatable {
    var daily: [String: DailyStat] = [:]
    var totalDictations: UInt64 = 0
    var totalWords: UInt64 = 0
    var totalSeconds: Double = 0

    static let empty = HistorySnapshot()

    enum CodingKeys: String, CodingKey {
        case daily
        case totalDictations = "total_dictations"
        case totalWords = "total_words"
        case totalSeconds = "total_seconds"
    }
}
