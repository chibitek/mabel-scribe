# Mabel Spatial

Native **visionOS** dictation sibling to Mac Mabel. SwiftUI + RealityKit. Apple Speech on-device. Version **0.1.0**.

This is **not** a Tauri Mac port, **not** Designed-for-iPhone / Vision compatibility, and **not** Mac TestFlight (`com.mabel.app` / ASC `6809059582`) running on a headset.

| | Mac Mabel Dictation (SoT) | Mabel Spatial |
|---|---|---|
| Folder | `src-tauri/` (unchanged 1.4.0) | `MabelSpatial/` |
| Bundle | `com.mabel.app` | `com.mabel.vision` |
| Display | Mabel / Mabel Dictation | **Mabel Spatial** |
| Team | `DF9FB764AR` | `DF9FB764AR` |
| ASC | `6809059582` | **new visionOS listing** (CIO creates; do not reuse Mac) |
| ASR | Parakeet / WhisperKit / whisper.cpp | Apple Speech / SpeechAnalyzer only |
| Pricing | Pro $5/mo or $48/yr + 30-day trial | Free TestFlight first; Pro later, no website upgrade |

Mac 1.4.0 (`main` ~`ee86d523` / TF 1401) stays Mac source of truth. Do not wire this target into `npm run tauri`.

## Why a sibling folder (not an Xcode multiplatform target)

A shared Xcode multiplatform app (macOS + visionOS, or iOS “Designed for visionOS”) would fight Tauri’s `src-tauri` Mac product, invite a Designed-for-iPhone destination, and risk shipping Mac TF onto the headset. A sibling native visionOS project keeps Mac paths untouched.

## What v0.1.0 does

- Windowed scene labeled **Mabel Spatial**, plus mixed **Immersive Space** (Enter space).
- Companion **orb** with a listening pulse and two eye gleams.
- Floating **transcript panel** (final text + live volatile tail).
- **Eyes + hands start/stop:** look at the orb (system gaze hover) and pinch. Window tap is the same path. No enterprise eye-tracking entitlement; no raw gaze stream.
- **Microphone only while listening.** `AVAudioEngine` tap + session are torn down on stop.
- **Apple Speech** on-device: `SpeechAnalyzer` + `SpeechTranscriber` (visionOS 26+), with `SFSpeechRecognizer` + `requiresOnDeviceRecognition = true` as fallback.
- Privacy: local by default, `PrivacyInfo.xcprivacy` tracking = false, no telemetry. Same MIT gift story as Mac.

StoreKit Pro share with Mac is a later soft follow-up. Do not add IAP until a visionOS ASC record exists.

## Open and run on Apple Silicon (Erick / M5 Max)

Linux CI **cannot** compile visionOS. Run these on the Mac with **Xcode-beta + XROS27** (visionOS 27 SDK). Deployment floor is **visionOS 27.0** so the scheme matches RealityDevice14,1 / visionOS 27 beta.

### Xcode GUI → Vision Pro (RealityDevice14,1 / visionOS 27 beta)

1. Enable **Developer Mode** on the Vision Pro (Settings → Privacy & Security → Developer Mode).
2. Wear the headset, unlock it, and keep it near the Mac.
3. On the Mac: `open MabelSpatial/MabelSpatial.xcodeproj`
4. Signing: Team **Chibitek Labs (`DF9FB764AR`)**, bundle `com.mabel.vision`, Automatic signing.
5. Xcode → Window → **Devices and Simulators**. The Vision Pro should appear as **RealityDevice14,1** on visionOS 27 beta. Pair / trust if asked.
6. Scheme **MabelSpatial**. Destination: the paired **Apple Vision Pro** (device), not an iPhone and not “My Mac”.
7. Product → Run (`⌘R`). First launch: allow **Microphone**. Look at the orb, pinch to listen, pinch again to stop.
8. For the simulator: destination **Apple Vision Pro** (visionOS Simulator). Speech models may be missing in Simulator; the UI and orb still run.

TestFlight comes later on a **new** visionOS ASC listing. Until then, local Xcode install is the path.

### Exact `xcodebuild` destinations (CIO / Erick)

From the **repo root** on the Mac:

```bash
# List destinations this scheme can see (device UDID + simulators).
xcodebuild -project MabelSpatial/MabelSpatial.xcodeproj \
  -scheme MabelSpatial \
  -showdestinations
```

**visionOS Simulator** (compile + run in sim):

```bash
xcodebuild -project MabelSpatial/MabelSpatial.xcodeproj \
  -scheme MabelSpatial \
  -destination 'platform=visionOS Simulator,name=Apple Vision Pro,OS=latest' \
  -configuration Debug \
  build
```

**Generic device** (compile-check for `xros`, no UDID):

```bash
xcodebuild -project MabelSpatial/MabelSpatial.xcodeproj \
  -scheme MabelSpatial \
  -destination 'generic/platform=visionOS' \
  -configuration Debug \
  build
```

**Physical Vision Pro** (RealityDevice14,1). After pairing, copy the UDID from `-showdestinations` or `xcrun xctrace list devices`:

```bash
xcodebuild -project MabelSpatial/MabelSpatial.xcodeproj \
  -scheme MabelSpatial \
  -destination 'platform=visionOS,id=DEVICE_UDID' \
  -allowProvisioningUpdates \
  -configuration Debug \
  build
```

Then install/run from Xcode (`⌘R` on that destination), or:

```bash
xcodebuild -project MabelSpatial/MabelSpatial.xcodeproj \
  -scheme MabelSpatial \
  -destination 'platform=visionOS,id=DEVICE_UDID' \
  -allowProvisioningUpdates \
  -configuration Debug \
  build

xcrun devicectl device install app --device DEVICE_UDID \
  path/to/Build/Products/Debug-xros/MabelSpatial.app
```

A wrapper that prints these commands and **exits 2 on Linux** lives at [`scripts/xcodebuild-visionos.sh`](scripts/xcodebuild-visionos.sh).

Linux structure check (safe here, does not invoke Xcode):

```bash
bash MabelSpatial/scripts/validate-spatial-scaffold.sh
```

## Identity locks

- Bundle ID: `com.mabel.vision`
- Display name: **Mabel Spatial** (Product Name `MabelSpatial`; home screen uses the display name)
- Team: `DF9FB764AR`
- Marketing version: `0.1.0` / build `1` (spatial is versioned separately from Mac 1.4.0)
- Deployment: `XROS_DEPLOYMENT_TARGET = 27.0` (Xcode-beta / XROS27 / RealityDevice14,1)
- Entitlements: sandbox + `device.audio-input` only. No JIT / unsigned exec / disable-library-validation.
- First-run Speech locale assets use Apple `AssetInventory` (system). If that install fails on-device, CIO may add `com.apple.security.network.client` for that download only — never for analytics.

## Layout

```
MabelSpatial/
  README.md
  scripts/validate-spatial-scaffold.sh
  scripts/xcodebuild-visionos.sh
  MabelSpatial.xcodeproj/
  MabelSpatial/
    MabelSpatialApp.swift          Window + mixed ImmersiveSpace
    Info.plist / entitlements / PrivacyInfo.xcprivacy
    Models/                        Session + identity
    Speech/OnDeviceSpeechEngine.swift
    Reality/                       Orb entity + immersive view
    Views/                         Window orb + transcript panel
    Assets.xcassets/               visionOS layered App Icon
```

## Compile notes (Xcode-beta / XROS27)

`XROS_DEPLOYMENT_TARGET` is **27.0**. SpeechAnalyzer exists since visionOS 26; this target floors at 27 so Erick’s Xcode-beta + RealityDevice14,1 build does not fight a 26.0 destination.

XROS27 Speech surface used by `OnDeviceSpeechEngine.swift` (Apple Speech headers / WWDC25 session 277):

- `SpeechDetector(detectionOptions: SpeechDetector.DetectionOptions(sensitivityLevel:), reportResults:)` — **not** `SpeechDetector(sensitivityLevel:)`
- `SpeechTranscriber.Result.isFinal` — **not** `isVolatile`. Partial/live tail is `isFinal == false` when `reportingOptions` includes `.volatileResults`
- `analyzer.start(inputSequence:)` + `finalizeAndFinishThroughEndOfInput()`
- `SpeechAnalyzer.bestAvailableAudioFormat(compatibleWith:)`
- `AssetInventory.assetInstallationRequest(supporting:)`

Do not “fix” a compile error by importing FluidAudio, WhisperKit, whisper.cpp, or any Tauri crate.
