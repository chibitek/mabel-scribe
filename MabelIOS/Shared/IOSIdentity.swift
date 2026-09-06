import SwiftUI

/// Native iPhone / iPad product. New ASC listing.
/// Not Mochii. Not Mac TestFlight (`com.mabel.app` / ASC 6809059582) on a phone.
/// Spatial stays `com.mabel.vision` — do not reuse that listing here.
enum IOSIdentity {
    static let displayName = "Mabel"
    static let bundleID = "com.mabel.ios"
    static let keyboardBundleID = "com.mabel.ios.keyboard"
    static let appGroup = "group.com.mabel.ios"
    static let marketingVersion = "0.1.0"
    static let teamID = "DF9FB764AR" // pragma: allowlist secret
    /// Mac App Store Connect record. iOS is a new listing — do not reuse.
    static let macASCAppID = "6809059582"
    static let macBundleID = "com.mabel.app"
    static let spatialBundleID = "com.mabel.vision"
}

enum IOSPalette {
    static let rose = Color(red: 0.910, green: 0.596, blue: 0.596)
    static let roseDeep = Color(red: 0.780, green: 0.420, blue: 0.430)
    static let cream = Color(red: 0.965, green: 0.929, blue: 0.894)
    static let peach = Color(red: 0.965, green: 0.769, blue: 0.659)
    static let ink = Color(red: 0.165, green: 0.141, blue: 0.125)
    static let mist = Color(red: 0.420, green: 0.380, blue: 0.360)
    static let whisker = Color(red: 0.310, green: 0.270, blue: 0.250)
}

enum IOSPrivacy {
    static let blurb = """
    Free dictation needs no account and no Stiki. Tap to listen, tap to stop. \
    The microphone is off unless you start it. Audio is transcribed on this \
    iPhone with Apple Speech. Mabel does not claim HIPAA or BAA coverage.
    """

    static let micUsage = """
    Mabel uses the microphone only while you are dictating, to turn speech \
    into text on this iPhone. Audio is not uploaded and is not recorded in \
    the background.
    """

    static let speechUsage = """
    Mabel uses on-device Apple Speech so your words become text in the app \
    you are typing into. Recognition does not use a Mabel cloud service.
    """

    static let fullAccessWhy = """
    iOS only lets a custom keyboard use the microphone after Full Access is \
    on. Mabel uses that for dictation you start with a tap — not for silent \
    listening, and not for an account.
    """
}
