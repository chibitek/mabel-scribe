# Mabel iOS

Native **iOS** keyboard-overlay dictation. SwiftUI host + custom keyboard extension. Apple Speech on-device. Version **0.1.0**.

This is **not** Mochii, **not** a Tauri Mac port, **not** Mac TestFlight (`com.mabel.app` / ASC `6809059582`) on a phone, and **not** Mabel Spatial (`com.mabel.vision`).

| | Mac Mabel Dictation (SoT) | Mabel Spatial | Mabel iOS |
|---|---|---|---|
| Folder | `src-tauri/` (unchanged 1.4.0) | `MabelSpatial/` | `MabelIOS/` |
| Bundle | `com.mabel.app` | `com.mabel.vision` | **`com.mabel.ios`** |
| Keyboard | — | — | `com.mabel.ios.keyboard` |
| Display | Mabel / Mabel Dictation | Mabel Spatial | **Mabel** |
| Team | `DF9FB764AR` | `DF9FB764AR` | `DF9FB764AR` |
| ASC | `6809059582` | new visionOS listing | **new iOS listing** (CIO creates; do not reuse Mac) |
| ASR | Parakeet / WhisperKit / whisper.cpp | Apple Speech | Apple Speech on-device |
| Ship #1 | Overlay hotkey | Orb + immersive | **Keyboard overlay in any app** |

Mac 1.4.0 stays Mac source of truth. Spatial stays a visionOS sibling. Do not wire this target into `npm run tauri`.

## Why a sibling folder (not `erickgrau/mabel-ios`, not multiplatform)

A shared Xcode multiplatform app (macOS + iOS, or “Designed for iPhone” on Vision) would fight Tauri’s `src-tauri` Mac product, invite a Designed-for-iPhone destination onto Spatial, and risk shipping Mac TestFlight onto a phone. A new `mabel-ios` repo would split Product history for no compile win — this tree is already native Swift, not Tauri.

`MabelIOS/` follows `MabelSpatial/`: one repo, separate bundle, separate ASC later.

## Product lock + Enforcer BOUND (fold hard)

Ship order (do not skip or reorder): **Keyboard → Polish → Dictionary → Scratchpad → Languages**. This tip is Keyboard only.

- **Mabel cat UI only.** No Flow brand. No Wispr clone.
- **Free dictate:** no account, no Stiki, no StoreKit. Tap the orb → speak → tap to stop → text inserts.
- **Keyboard is not a silent spy.** Explicit orb start. Fail closed without Microphone (and Speech / Full Access). Lifecycle hooks never start the mic. `viewWillDisappear` tears it down. No ambient / always-on listen.
- **Data & privacy (Suite b6530197):** cloud storage OFF/unavailable v1; improve-models OFF default; no silent cloud; no HIPAA/BAA/Wispr BAA; local-first toggles OK.
- Host **Settings** scaffold: Account (Stiki + Pro, dual gate), General, Keyboard, Notifications, Data & privacy. Free dictate does not use Account.
- Host **Home IA:** tabs Home | Dictionary | Snippets | Style | Scratchpad. Free: Home + dictate. Pro tabs locked until Stiki AND purchase.
- Later ships (Polish, Dictionary, Scratchpad, Languages) plus Connectors / Notetaker stay out of this tip.

**BREAKS IF:** Flow brand; keyboard spy / ambient always-on listen; Free dictate requires Stiki; keyboard audio without permission; cloud ON v1; improve-models default ON / silent upload; BAA/HIPAA claim; silent cloud.

## What v0.1.0 does

- Host app shell with Mabel cat chrome (cream / rose / portrait). Bottom tabs Home | Dictionary | Snippets | Style | Scratchpad. Hamburger opens Account / Settings. Setup steps enable the keyboard.
- Host playground: same on-device dictate path, used to grant permissions.
- Custom keyboard (`UIInputViewController`) with the orb, live preview, globe / delete / space / return.
- On-device `SFSpeechRecognizer` (`requiresOnDeviceRecognition = true`). No Apple network-speech fallback. No whisper.cpp / Parakeet / WhisperKit.
- Privacy: `PrivacyInfo.xcprivacy` tracking = false. Mic powered only while listening.

Simulator often has no on-device speech model. The UI, gate, and keyboard still run; dictate on a physical iPhone.

## Open and run on Apple Silicon (Erick)

Linux CI **cannot** compile iOS. Run these on the Mac with **Xcode 16+**.

### Xcode GUI → Simulator or iPhone

1. `open MabelIOS/MabelIOS.xcodeproj`
2. Signing: Team **Chibitek Labs (`DF9FB764AR`)**, host `com.mabel.ios`, keyboard `com.mabel.ios.keyboard`, Automatic signing. Xcode will create App Group `group.com.mabel.ios` if needed.
3. Scheme **MabelIOS**. Destination: **iPhone 16** (simulator) or a paired iPhone. Not “My Mac”, not Vision Pro.
4. Product → Run (`⌘R`). First launch: **Allow Microphone** and **Speech Recognition** from the host (Allow permissions, or tap the playground orb).
5. Enable the keyboard:
   1. iPhone Settings → General → Keyboard → Keyboards → Add New Keyboard… → **Mabel**
   2. Tap **Mabel** → turn on **Allow Full Access**
6. Open Notes (or any app). Switch to the Mabel keyboard. Tap the orb to dictate. Tap again to stop. Text inserts into the field.
7. Deny Microphone in Settings and reopen the keyboard — the orb must stay silent (fail closed).

TestFlight comes later on a **new** iOS ASC listing. Until then, local Xcode install is the path.

### Exact `xcodebuild` destinations (CIO / Erick)

From the **repo root** on the Mac:

```bash
xcodebuild -project MabelIOS/MabelIOS.xcodeproj \
  -scheme MabelIOS \
  -showdestinations
```

**iOS Simulator:**

```bash
xcodebuild -project MabelIOS/MabelIOS.xcodeproj \
  -scheme MabelIOS \
  -destination 'platform=iOS Simulator,name=iPhone 16,OS=latest' \
  -configuration Debug \
  build
```

**Generic device** (compile-check for `iphoneos`, no UDID):

```bash
xcodebuild -project MabelIOS/MabelIOS.xcodeproj \
  -scheme MabelIOS \
  -destination 'generic/platform=iOS' \
  -configuration Debug \
  build
```

**Physical iPhone.** After pairing, copy the UDID from `-showdestinations` or `xcrun xctrace list devices`:

```bash
xcodebuild -project MabelIOS/MabelIOS.xcodeproj \
  -scheme MabelIOS \
  -destination 'platform=iOS,id=DEVICE_UDID' \
  -allowProvisioningUpdates \
  -configuration Debug \
  build
```

A wrapper that prints these commands and **exits 2 on Linux** lives at [`scripts/xcodebuild-ios.sh`](scripts/xcodebuild-ios.sh).

Linux structure check (safe here, does not invoke Xcode):

```bash
bash MabelIOS/scripts/validate-ios-keyboard-scaffold.sh
```

## Identity locks

- Bundle ID: `com.mabel.ios` (keyboard `com.mabel.ios.keyboard`)
- Display name: **Mabel** (Product Name `Mabel`; Xcode scheme `MabelIOS`)
- Team: `DF9FB764AR`
- Marketing version: `0.1.0` / build `1` (iOS is versioned separately from Mac 1.4.0)
- Deployment: `IPHONEOS_DEPLOYMENT_TARGET = 17.0`
- Entitlements: App Group `group.com.mabel.ios` only. No JIT / unsigned exec / StoreKit.
- Pro later: StoreKit **and** Stiki together. Not required for Free keyboard dictate.

## Layout

```
MabelIOS/
  README.md
  scripts/validate-ios-keyboard-scaffold.sh
  scripts/xcodebuild-ios.sh
  MabelIOS.xcodeproj/
  Shared/                          Gate + EnforcerBound + SettingsStore + speech
  MabelIOS/                        Host app shell + Settings IA (cat UI)
  MabelKeyboard/                   Custom keyboard extension
```

## Compile notes

Do not “fix” a compile error by importing FluidAudio, WhisperKit, whisper.cpp, Tauri, StoreKit, or Stiki.

Keyboard memory is tight. Keep Apple Speech streaming; do not add a local Whisper model in the appex.

If App Group provisioning fails on first Run, let Xcode register `group.com.mabel.ios` under team `DF9FB764AR` and Run again.
