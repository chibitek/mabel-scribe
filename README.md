<p align="center">
  <img src="assets/mabel-banner.jpg" alt="Mabel" width="100%" />
</p>

# Mabel

Privacy-first dictation for macOS. Hold a hotkey, speak, release. Mabel transcribes your voice and pastes the text wherever your cursor is. Audio never leaves your Mac unless you explicitly opt in to a cloud engine with your own API key.

Named after a long-haired Persian who would never share your transcripts with anyone.

Open source under the [MIT license](LICENSE). Fork it, build it, ship your own version.

**Public source of truth is this repo (`erickgrau/Mabel`).** `chibitek/mabel-scribe` is a stale public fork (last v1.1.3, May 2026) — do not treat it as current. Pro features (Polish, teams, snippets, and the rest of the StoreKit gate) ship in this tree and unlock at runtime. Signing keys, notarization passwords, updater private keys, and API tokens stay out of git. See [docs/oss-source-of-truth.md](docs/oss-source-of-truth.md).

A native **visionOS** sibling — **Mabel Spatial** (`com.mabel.vision`, v0.1.0) — lives in [`MabelSpatial/`](MabelSpatial/README.md). It is SwiftUI + RealityKit + on-device Apple Speech, not a Tauri port and not Mac TestFlight on a headset. Mac 1.4.0 paths below are unchanged.

---

## What it does

- Press a global hotkey from any app, dictate, and the text appears at your cursor.
- Toggle mode (press to start, press to stop) or push-to-talk (hold while speaking).
- Single-paste dictation: record, transcribe once when you stop, then paste a clean result. Live chunked dictation is paused while the streaming worker is being stabilized.
- Floating overlay shows a live waveform while recording. It floats over fullscreen apps and never steals focus from the app you are typing into.
- Local stats on usage: words per minute, total words dictated, daily streak. Counts only, never content.
- Voice command: end a dictation with "press enter" / "new line" and Mabel submits after pasting.

## Privacy by default

- Audio is recorded to a temp file, transcribed, then deleted.
- Transcripts are never logged to disk.
- No telemetry, no analytics, no remote logging.
- API keys live in the macOS Keychain, never in a config file.
- Insights are local-only counts, never the words.

## Engines

| Engine | Where it runs | What's sent off-device |
|---|---|---|
| **Parakeet (default, new installs)** | FluidAudio CoreML in-process on the Neural Engine | Nothing |
| **WhisperKit large-v3-turbo** | WhisperKit CoreML in-process (optional) | Nothing |
| **whisper.cpp (Developer ID fallback)** | Phase A sidecar + ggml Large v3 Q5 | Nothing |
| **Groq cloud** | Groq's hosted Whisper, opt-in | The audio of each clip |

Local works completely offline once you download the model. Groq is faster and more accurate on long audio but requires a free API key from `console.groq.com`.

## System requirements

- macOS 12 (Monterey) or later
- Apple Silicon (M1 / M2 / M3 / M4). Intel build is not currently distributed.
- ~1 GB free for the recommended Parakeet CoreML models (FluidAudio). WhisperKit large-v3-turbo and whisper.cpp Large v3 Q5 (~1.1 GB) are optional. Parakeet / WhisperKit need macOS 14+; the Developer ID whisper.cpp fallback still runs on the older floor.

## Install (end users)

Download the latest signed and notarized DMG from the [Releases](../../releases) page, mount it, drag Mabel to Applications, open it.

On first launch:

1. macOS asks for **Microphone** access. Click Allow.
2. Mabel auto-downloads Parakeet with a progress bar. One-time setup. You can switch to WhisperKit or whisper.cpp later in Settings → Engine.
3. Mabel triggers macOS's **Accessibility** dialog. Click Open System Settings, flip the Mabel toggle on.
4. Press your hotkey and speak. macOS asks for **Automation (System Events)** the first time text is pasted. Click Allow.

A fourth Keychain prompt appears only if you save a Groq API key.

Default hotkey is `Cmd+D`. Rebind in Settings → General. The Help view inside the app has the full feature reference and troubleshooting.

---

# Build it yourself

If you want to fork Mabel, customize it, and ship your own signed/notarized DMG, the rest of this README walks through every step.

## Prerequisites

- macOS 12+ on Apple Silicon (build host).
- **Xcode 16+ / Swift 6** (macOS): needed to compile the in-process Parakeet / WhisperKit library (`npm run vendor-asr`). Command Line Tools alone are not enough for that target.
- **Xcode Command Line Tools**: `xcode-select --install`
- **Rust** (1.78+ recommended): `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
- **Node.js 20+** and npm: install via [nodejs.org](https://nodejs.org) or `brew install node`
- **Apple Developer Program membership** ($99/year) — only required if you want to ship a DMG that opens cleanly without Gatekeeper warnings. Not needed for personal/dev builds.

## Clone, install, run in dev

```bash
git clone https://github.com/erickgrau/Mabel.git
cd Mabel
npm install
npm run tauri dev
```

That gives you a hot-reloading dev build. Permissions, hotkey, recording, transcription all work.

## Run tests

```bash
cd src-tauri && cargo test --lib
```

## Build a local production DMG

```bash
npm run tauri build
```

The DMG lands at `src-tauri/target/release/bundle/dmg/Mabel_<version>_aarch64.dmg`. The checked-in config uses placeholder macOS signing values. For distributable builds, create the private signing override documented below. For personal/dev builds, use `npm run tauri dev` unless you have configured local signing.

## Make it your own (forking)

If you fork Mabel and plan to ship your own builds, you must change four identifiers so your build doesn't collide with the upstream signing identity:

1. **Bundle identifier** — `src-tauri/tauri.conf.json` field `identifier`. Change `com.mabel.app` to your own reverse-DNS string (e.g. `com.yourname.mydictate`).
2. **Product name** — `src-tauri/tauri.conf.json` field `productName`.
3. **Signing identity** — keep real signing values out of source. Use a private `src-tauri/tauri.local.conf.json` override with your own Developer ID and `providerShortName` team ID.
4. **Apple Events / TCC reset paths** — anywhere code references `com.mabel.app` directly (e.g. `tccutil reset Microphone com.mabel.app`), update to your bundle ID.

Search the repo for `com.mabel.app` and `Developer ID Application: Your Name (TEAMID)` to find every occurrence that may need to change in your fork.

---

# Sign and notarize a distributable DMG

This is the part that turns "a DMG" into "a DMG users can open without macOS yelling at them." Required if you're distributing to anyone other than yourself.

## One-time Apple Developer setup

### 1. Sign up for the Apple Developer Program

Go to [developer.apple.com/programs](https://developer.apple.com/programs/) and join. $99/year. Approval takes 24-48 hours typically.

### 2. Create a Developer ID Application certificate

This certificate is what makes macOS trust your builds.

a. Open **Keychain Access** → menu → `Certificate Assistant` → `Request a Certificate from a Certificate Authority`.
   - Email: your Apple ID
   - Common Name: your name
   - Choose `Saved to disk` → save the `.certSigningRequest` file.

b. Go to [developer.apple.com/account/resources/certificates/list](https://developer.apple.com/account/resources/certificates/list).

c. Click `+` → choose **Developer ID Application** → Continue → upload the `.certSigningRequest` file → Continue → download the resulting `.cer` file.

d. Double-click the `.cer` file. It installs into your Keychain.

e. Verify it's there:
   ```bash
   security find-identity -v -p codesigning
   ```
   You should see a line like:
   ```
   1) ABCD1234... "Developer ID Application: Your Name (TEAMID)"
   ```

### 3. Create an app-specific password for notarization

Apple's notarization service uses an app-specific password (not your main Apple ID password).

a. Go to [appleid.apple.com/account/manage](https://appleid.apple.com/account/manage) → sign in.
b. Find the **App-Specific Passwords** section → click `+` → label it "Mabel Notarization" (or anything).
c. Apple shows a password like `abcd-efgh-ijkl-mnop`. Copy it. This is shown only once.

### 4. Store notarization credentials in your keychain

```bash
xcrun notarytool store-credentials AC_PASSWORD \
  --apple-id "your-apple-id-email@example.com" \
  --team-id "YOURTEAMID" \
  --password "abcd-efgh-ijkl-mnop"
```

You'll see `Credentials saved to Keychain.` The profile name `AC_PASSWORD` is what you'll reference later.

## Configure local signing

The checked-in `src-tauri/tauri.conf.json` uses placeholder signing values so the repository is safe to publish. Do not commit your real Apple Developer identity or team ID. `scripts/release-macos.sh` reads the DMG identity from `MABEL_SIGNING_IDENTITY` only (see `.env.example`).

Create a private `src-tauri/tauri.local.conf.json` file for local or CI signing:

```json
{
  "bundle": {
    "macOS": {
      "signingIdentity": "Developer ID Application: Your Name (TEAMID)",
      "providerShortName": "TEAMID"
    }
  }
}
```

`src-tauri/tauri.local.conf.json` is ignored by Git. In CI, generate this file from private secrets immediately before the build, or pass an equivalent Tauri config override.

The `entitlements.plist` file is already in `src-tauri/`. It declares:
- Audio input (mic capture)
- Network client (Groq API, model download)
- Apple Events (paste via System Events)
- JIT and unsigned executable memory (Developer ID DMG: WebKit + whisper.cpp sidecar fallback)
- Disable library validation (Developer ID DMG only: whisper-cpp + ggml dylibs)

The last two hardened-runtime entitlements, plus JIT, are security-sensitive. They stay on the **Developer ID DMG** so the Phase A sidecar still loads. The default Parakeet / WhisperKit path does not need them.

Mac App Store / TestFlight is a **second flavor**, not this DMG path. `npm run build:dmg` / `scripts/release-macos.sh` stay on Developer ID + `entitlements.plist`. `npm run build:mas` uses the sandbox draft and **refuses unless** `MABEL_MAS_EXPERIMENT=1`. **1.4.0 wires StoreKit 2 Pro IAP** but is **not claimed MAS-ready** until ASC products exist and a Mac binary completes purchase. Local purchase/trial prove: `bash scripts/prove-storekit-mac.sh` (must Run `Mabel.app` from the **Mabel-StoreKit** Xcode scheme — not `/Applications/Mabel 2.app`). See [docs/mas-and-testflight.md](docs/mas-and-testflight.md) and [docs/app-store-iap.md](docs/app-store-iap.md).

## Build with signing + notarization

```bash
APPLE_ID="your-apple-id@example.com" \
APPLE_PASSWORD="abcd-efgh-ijkl-mnop" \
APPLE_TEAM_ID="YOURTEAMID" \
npm run tauri build -- --config src-tauri/tauri.local.conf.json
```

The build:
1. Compiles release Rust binary
2. Bundles into `Mabel.app`
3. Signs the binary, sidecar, and `.app` with your Developer ID
4. Submits the `.app` to Apple's notary service (waits ~3-5 minutes)
5. Staples the notarization ticket to the `.app`
6. Builds and signs a `.dmg`

## Notarize and staple the DMG

Tauri's bundler notarizes the `.app` but **not** the DMG itself. Apple recommends notarizing the DMG too. Run these after `npm run tauri build`:

```bash
DMG="src-tauri/target/release/bundle/dmg/Mabel_$(node -p 'require("./package.json").version')_aarch64.dmg"

xcrun notarytool submit "$DMG" --keychain-profile AC_PASSWORD --wait
xcrun stapler staple "$DMG"
spctl -a -t open --context context:primary-signature -vv "$DMG"
```

The last command should print `accepted / source=Notarized Developer ID`. If so, the DMG is fully notarized and stapled.

## Optional: include a FIRST LAUNCH.txt in the DMG

Mabel's distributed DMG includes a `FIRST LAUNCH.txt` next to `Mabel.app` so first-time users see permission setup notes. To include your own:

```bash
WORK="/tmp/dmg-work" && rm -rf "$WORK" && mkdir -p "$WORK"
hdiutil convert "$DMG" -format UDRW -o "$WORK/rw.dmg" -quiet
MOUNT=$(hdiutil attach -nobrowse -noverify -noautoopen "$WORK/rw.dmg" | tail -1 | awk '{ for(i=3;i<=NF;i++) printf "%s ", $i; print "" }' | sed 's/ *$//')
cp /path/to/your/FIRST\ LAUNCH.txt "$MOUNT/"
hdiutil detach "$MOUNT" -quiet
hdiutil convert "$WORK/rw.dmg" -format UDZO -imagekey zlib-level=9 -o "$WORK/final.dmg" -quiet
mv "$WORK/final.dmg" "$DMG"
codesign --sign "Developer ID Application: Your Name (YOURTEAMID)" --timestamp "$DMG"
xcrun notarytool submit "$DMG" --keychain-profile AC_PASSWORD --wait
xcrun stapler staple "$DMG"
```

The final DMG is signed, notarized, stapled, and ready to share.

---

# How it's built

Mabel is a [Tauri 2](https://tauri.app) desktop app. The UI is vanilla TypeScript with no framework. The backend is Rust.

## Stack

| Layer | Tech |
|---|---|
| Shell | Tauri 2 (`macos-private-api` feature) |
| Frontend | TypeScript 5, Vite 6, vanilla DOM, custom CSS |
| Backend | Rust 2021 edition |
| Audio capture | `cpal` 0.15 |
| WAV encoding | `hound` 3.5 |
| Local transcription | FluidAudio Parakeet + WhisperKit (in-process CoreML); whisper.cpp sidecar fallback on Developer ID |
| Cloud transcription | `reqwest` 0.12 against Groq's `whisper-large-v3` |
| Async runtime | `tokio` 1 (full features) |
| Floating overlay | `tauri-nspanel` v2 (NSPanel-backed window) |
| AppKit interop | `objc2` 0.6 + `objc2-app-kit` 0.3 |
| Secrets | `keyring` 3 (macOS Keychain via `apple-native`) |
| Clipboard | `arboard` 3 |
| Global hotkey | `tauri-plugin-global-shortcut` 2 |
| Autostart | `tauri-plugin-autostart` 2 |
| Shell open | `tauri-plugin-shell` 2 |
| Date / time | `chrono` 0.4 |

## Architecture

```
src/                       Frontend (TypeScript + HTML + CSS)
  main.ts                  Settings, hotkey rebind, stats wiring,
                           first-run model download, Accessibility request
  overlay.html             Floating dictation panel
  style.css                All styles

src-tauri/
  build.rs                 Embeds git hash + version at compile time
  entitlements.plist       Hardened-runtime entitlements for Developer ID DMG
  entitlements.mas.plist   MAS/TestFlight sandbox draft (see docs/mas-and-testflight.md)
  Mabel.storekit           Local StoreKit 2 config (monthly + yearly + 30-day trial)
  src/
    main.rs                Tauri commands, plugin registration, app setup
    lib.rs                 Module roots + version constants
    storekit.rs            StoreKit 2 entitlement (fail-closed)
    teams.rs               On-device org / seats / invites (Pro-gated)
    pro_features.rs        Snippets / style / transforms / scratchpad stores
    settings.rs            Persisted user prefs (config.json)
    audio.rs               cpal recorder, ring buffer, RMS metering
    recorder.rs            Recording state machine, orchestration
    streaming.rs           VAD-driven chunking for live dictation
    local_engine.rs        Parakeet / WhisperKit / whisper.cpp choice
    transcribe_native.rs   In-process CoreML FFI (macOS)
    transcribe_local.rs    whisper.cpp sidecar invocation (DMG fallback)
    transcribe_groq.rs     Groq HTTP client
    cleanup.rs             Whisper output post-processing
    polish.rs              Pro-only Polish (toggle + modes, local Gemma, never invent)
    paste.rs               Clipboard + osascript paste, Return keystroke
    overlay_macos.rs       NSPanel conversion for the overlay window
    system_ui.rs           Dock visibility, sounds, Accessibility request
    secrets.rs             Keychain read/write via the keyring crate
    stats.rs               Local-only daily counts, WPM, streak
    downloader.rs          Whisper / LLM model fetcher with progress events
    llm.rs                 Local Gemma cleanup via bundled llama-server
    llama-runtime/         Vendored llama-server + dylibs (see scripts/vendor-llama-server.sh)

native/MabelStoreKit/      StoreKit 2 Swift dylib (MAS/TF Pro)
tools/MabelStoreKitProve/  Xcode scheme that launches Mabel.app under Mabel.storekit
scripts/prove-storekit-mac.sh  CIO Mac prove: 1.4.0 + dylib + Run Mabel-StoreKit

MabelSpatial/              Native visionOS app (SwiftUI + RealityKit)
                           Apple Speech on-device. Separate ASC later.
                           See MabelSpatial/README.md. Not in npm/tauri paths.
```

## Audio path

cpal captures audio into an in-memory buffer. Current builds write the buffered audio to a temporary WAV file when you stop, transcribe once, delete the temp file, and paste once. The VAD-driven streaming worker remains in the codebase but is disabled while its shutdown behavior is being stabilized.

## Overlay

The overlay window is converted from a standard NSWindow to a non-activating NSPanel via `tauri-nspanel`. This is the same primitive Spotlight uses. It floats over fullscreen apps, joins all Spaces, and never takes key/main status, so the app you are dictating into keeps focus.

## Build-time version stamping

`src-tauri/build.rs` embeds the short git hash as an env var at compile time. Reachable from Rust as `mabel_lib::MABEL_GIT_HASH`. Surfaced in the UI footer and About pane via the `get_version` command.

## Project layout note

`tauri-plugin-single-instance` is intentionally not registered while a startup race is being investigated. Avoid double-launching during development.

---

## Contributing

Forks are welcome. PRs are welcome. The codebase is small and the patterns are obvious — read [main.rs](src-tauri/src/main.rs) and [main.ts](src/main.ts) and you'll have the model in 20 minutes.

If you ship a fork as your own product, please change the bundle identifier and signing identity (see [Make it your own](#make-it-your-own-forking)) so it doesn't collide with the upstream.

## License

MIT. See [LICENSE](LICENSE).
