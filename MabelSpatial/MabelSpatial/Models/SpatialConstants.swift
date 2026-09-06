import SwiftUI

enum SpatialSceneID {
    static let immersive = "MabelSpatialImmersive"
}

enum SpatialIdentity {
    static let displayName = "Mabel Spatial"
    static let bundleID = "com.mabel.vision"
    static let marketingVersion = "0.1.0"
    static let teamID = "DF9FB764AR" // pragma: allowlist secret
    /// Mac App Store Connect record. Spatial is a new listing — do not reuse.
    static let macASCAppID = "6809059582"
    static let macBundleID = "com.mabel.app"
}

enum SpatialPalette {
    static let rose = Color(red: 0.910, green: 0.596, blue: 0.596)
    static let roseDeep = Color(red: 0.780, green: 0.420, blue: 0.430)
    static let cream = Color(red: 0.965, green: 0.929, blue: 0.894)
    static let ink = Color(red: 0.165, green: 0.141, blue: 0.125)
    static let mist = Color(red: 0.420, green: 0.380, blue: 0.360)
}

enum SpatialPrivacy {
    static let blurb = """
    Audio is transcribed on this Vision Pro with Apple Speech. \
    The microphone is powered only while you are listening. \
    Transcripts stay on-device. No telemetry. Same open-source gift as Mac Mabel.
    """

    static let micUsage = """
    Mabel Spatial uses the microphone only while you are listening, \
    to transcribe dictation on this Vision Pro. Audio is not uploaded.
    """

    static let speechUsage = """
    Mabel Spatial uses on-device Apple Speech so your words become text \
    on this Vision Pro. Recognition does not use a Mabel cloud service.
    """
}
