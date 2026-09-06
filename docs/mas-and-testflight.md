# Dual flavor: Developer ID DMG vs Mac App Store / TestFlight

**Mabel Spatial** (`com.mabel.vision`, [`MabelSpatial/`](../MabelSpatial/README.md)) is a separate native visionOS listing. Do not upload it to Mac ASC `6809059582` / `com.mabel.app`.

**This tree is not Mac App Store ready.** Do not upload a binary to MAS or Mac TestFlight. Do not ship 1.1.7 or 1.2.0 sidecar builds to the store. 1.3.0 lands the Phase B engines; it does **not** claim a passing MAS build.

The outside-store channel stays the **GitHub notarized Developer ID DMG** (`scripts/release-macos.sh`). MAS / TestFlight is a second flavor: App Sandbox, Apple Distribution, a Mac App Store provisioning profile, and engines that do not need `disable-library-validation`.

## Engine status (1.3.0 / Phase B)

| Engine | How it runs | DMG | MAS flavor | Needs `disable-library-validation` / unsigned exec memory |
|---|---|---|---|---|
| **Parakeet (default, new installs)** | In-process FluidAudio CoreML / ANE | yes | yes (intended default) | **No** |
| **WhisperKit large-v3-turbo** | In-process WhisperKit CoreML / ANE | yes | yes (optional) | **No** |
| **whisper.cpp large-v3 Q5** | Sidecar + ggml dylibs (Phase A) | yes (fallback) | **excluded** | **Yes** — DMG only |

Parakeet / WhisperKit are **structurally MAS-clean for the transcription path**: they do not require the two entitlements Apple rejects. That is not the same as “the app is MAS-ready.”

Still open before any store upload:

- **App Store Connect IAP** on app `6809059582`: subscription group + `com.mabel.app.pro.monthly` / `com.mabel.app.pro.yearly` with 30-day (1-month) free intro. CIO ASC offer IDs (FREE_TRIAL 1×ONE_MONTH, NEW+EXISTING): monthly `01ff2bea-692e-4aa8-a03c-eff20209605f`, yearly `b9c12df1-5153-459a-b50d-9e320b14885d`. Custom/OTU codes are ASC 409 until Mac app + IAPs are Approved (listing is PREPARE_FOR_SUBMISSION / READY_TO_SUBMIT). Eng does not invent prices. In-app redeem is Plans → Have a code? (`offerCodeRedemption`, macOS 15+). See [app-store-iap.md](app-store-iap.md). No free MAS ship until a Mac binary completes purchase / restore / manage.
- Enable **In-App Purchase** on App ID `com.mabel.app` and regenerate the MAS provisioning profile. This is not a plist key; do not add Apple Pay merchant entitlements. Developer ID `entitlements.plist` stays unchanged.
- Sandboxed WebKit may still need `allow-jit` (omitted today; add only with an App Review justification).
- `llama-server` AI cleanup is a sidecar on the DMG. Keep it off or replace it before MAS.
- `NemoTextProcessing.framework` (FluidAudio) must be re-signed with the same team as the app.
- No Apple Distribution + MAS provisioning run has been done in this repo. Notarized DMG is still Erick-on-M5.

## P0: TF 1.3.x/1303 → 1.4.0/1401 history + StoreKit hang

Erick smoked **1.4.0 (1401)** on ASC `6809059582` from tip-era `main` `ee86d523` (StoreKit #11), not a later Polish tip. Prior install was TF **1.3.x / 1303**. Same sandbox container.

- Insights / usage (`stats.json`) and `config.json` must still be there after the 1.4.0 open. Do not load-or-default over those files. Schema migrate is later.
- Subscribe must not beachball. Purchase is async + timeout; Restore / Manage stay usable; Pro only after a verified transaction.

Clipboard #14 and Polish #15 on current `main` stay intact.

## P0: TestFlight empty-record (build 1.3.0 / 1302)

Hotkey opened the overlay, stop produced no text. Root cause is **not** a missing `NSMicrophoneUsageDescription` (it is in `Info.plist`) and **not** a missing `com.apple.security.device.audio-input` key in source (it is in `entitlements.mas.plist`). The pipeline failed open:

1. **Mic TCC / sandbox.** `cpal` opens a CoreAudio HAL stream. `stream.play()` succeeds and the overlay goes to Listening even when `kTCCServiceMicrophone` is not determined or denied. Under App Sandbox that often delivers **digital silence** (empty buffer or RMS ≈ 0) and **does not show the system prompt**. Developer ID / unsandboxed HAL is looser. The fix requests `AVCaptureDevice.requestAccess(for: .audio)` before capture and refuses to enter Recording if access is denied.
2. **Model not ready.** `start_recording` did not check `engine_ready`. Parakeet `downloadAndLoad` during stop can return empty or fail late. The fix fail-closes before the overlay shows if Parakeet / WhisperKit / ggml is not on disk.
3. **Empty capture / empty ASR treated as success.** `stop_and_save` only errored on a completely empty buffer. Zeroed HAL samples wrote a silent WAV; ASR returned `""`; paste was skipped; overlay went Ready. The fix fail-closes on empty buffer, digital-silence RMS, and empty transcript, and shows the reason on the overlay (the main-window `alert` is invisible while the user is dictating elsewhere).
4. **Paste.** Sandboxed `osascript` → System Events is blocked without `com.apple.security.temporary-exception.apple-events` for `com.apple.systemevents`. If a later build actually gets text, paste would fail next. The exception is now in `entitlements.mas.plist`, with a CGEvent Cmd+V fallback when Accessibility is trusted.

### How to prove on TestFlight

1. Install the new MAS-signed build. Confirm entitlements on the **installed** `.app` (not the source plist):
   ```
   codesign -d --entitlements :- /Applications/Mabel.app
   ```
   Must include `com.apple.security.app-sandbox`, `com.apple.security.device.audio-input`, and `com.apple.security.temporary-exception.apple-events` → `com.apple.systemevents`. `plutil` / `defaults read` the bundle `Info.plist` must still have `NSMicrophoneUsageDescription`.
2. Reset mic TCC: `tccutil reset Microphone com.mabel.app`. Launch via Finder / TestFlight, not a raw binary.
3. Finish Parakeet download (Settings → Engine shows ready).
4. **Deny path:** press the hotkey. The Microphone prompt must appear. Deny it. Overlay must show **Mic access needed** and must not return to Ready as a silent success.
5. **Grant path:** grant Microphone, Accessibility, and Automation (System Events). Press hotkey, speak, stop. Text must paste. Overlay must not show an error.
6. **Model-missing path:** before the model is downloaded, press the hotkey. Overlay must show **Model not ready**, not Listening.
7. Console.app filter `[Mabel]` / `[MabelASR]`: look for `WAV saved … rms=`, `transcription returned chars=`, `paste command completed`, or the fail-closed title.

### CIO signing / entitlement follow-ups

- Re-sign the next TF cut with this `entitlements.mas.plist`. **Do not drop `device.audio-input`.** If that key is missing on the signed binary, TCC denies the mic with no prompt (same class of bug as a hardened-runtime helper without the entitlement).
- Provisioning profile for `com.mabel.app` must allow the sandbox + audio-input + Apple Events entitlements.
- Identity is **Apple Distribution** (not Developer ID). Re-sign `libMabelASR.dylib` and `NemoTextProcessing.framework` with the same team.
- After the cut, attach the `codesign -d --entitlements` dump of the uploaded `.app` to the CIO notes so TF 1302-class drift is visible.

## Store listing (when a legal binary exists)

| Field | Value |
|---|---|
| Bundle ID | `com.mabel.app` (same as the DMG) |
| Price | Free |
| Category | Productivity (`public.app-category.productivity` is already in `Info.plist`) |
| Publisher | Chibitek Labs |

TestFlight Mac uses the **MAS-signed** `.app` / installer (`Apple Distribution` + Mac App Store provisioning), uploaded via Transporter or Xcode. It is not the notarized Developer ID DMG.

## Dual-flavor files

| Flavor | Entitlements | Tauri config | How to build |
|---|---|---|---|
| **Developer ID DMG (ship 1.3)** | `src-tauri/entitlements.plist` | `tauri.conf.json` + private `tauri.local.conf.json` | `scripts/release-macos.sh` or `npm run build:dmg` |
| **MAS / TestFlight (not shippable yet)** | `src-tauri/entitlements.mas.plist` | overlay `src-tauri/tauri.mas.conf.json` | `npm run build:mas` (refuses unless `MABEL_MAS_EXPERIMENT=1`) |

`release-macos.sh` is Developer ID only. It is not wired to the MAS overlay. Do not point it at `entitlements.mas.plist`.

### Developer ID DMG (current ship path)

```bash
# Private signing override (gitignored). Identity is Developer ID Application.
# scripts/release-macos.sh
```

`entitlements.plist` is Hardened Runtime **without** App Sandbox. It still includes the three codesigning entitlements the **whisper.cpp sidecar fallback** needs:

- `com.apple.security.cs.allow-jit`
- `com.apple.security.cs.allow-unsigned-executable-memory`
- `com.apple.security.cs.disable-library-validation` (whisper-cpp + `Contents/Frameworks/` ggml dylibs)

Those last two **fail MAS**. They stay on the DMG only so the Phase A sidecar remains usable during the transition. Parakeet / WhisperKit do not use them.

### MAS / TestFlight (draft flavor only)

```bash
# Prints the not-ready warning and exits unless you opt in:
MABEL_MAS_EXPERIMENT=1 npm run build:mas
```

That runs Tauri with `--config src-tauri/tauri.mas.conf.json --bundles app -- --no-default-features` so you get an `.app`, not a DMG, and the whisper-cpp sidecar is compiled out. Remaining store steps on a Mac with the right certs:

1. Build `libMabelASR.dylib` on macOS (`npm run vendor-asr`, Xcode 16 / Swift 6).
2. Sign with **Apple Distribution: Chibitek Labs (TEAMID)** and embed the **Mac App Store** provisioning profile for `com.mabel.app`. Re-sign `NemoTextProcessing.framework` if FluidAudio staged it.
3. Wrap with `productbuild` / Xcode using **3rd Party Mac Developer Installer**.
4. Upload that pkg to App Store Connect.
5. Enable **Mac TestFlight**, then submit the free Productivity listing.

`tauri.mas.conf.json` turns off GitHub updater artifacts and drops ggml / whisper-cpp from the bundle. Store updates go through App Store / TestFlight, not `latest.json`.

## MAS entitlements draft

`entitlements.mas.plist` is sandbox **ON** and **does not** set:

- `com.apple.security.cs.disable-library-validation`
- `com.apple.security.cs.allow-unsigned-executable-memory`

It does set:

- `com.apple.security.app-sandbox`
- `com.apple.security.device.audio-input`
- `com.apple.security.network.client` (Hugging Face model download, optional Groq)
- `com.apple.security.automation.apple-events` (paste)
- `com.apple.security.temporary-exception.apple-events` → `com.apple.systemevents` (sandboxed osascript paste)
- `com.apple.security.files.user-selected.read-write` (manual model file pick)
- `com.apple.security.temporary-exception.files.home-relative-path.read-only` → `/Library/Application Support/com.mabel.app` and `com.typr.app` (import prior DMG history into the TF container; #10 mic + System Events keys stay)

In-App Purchase is enabled on the App ID / MAS profile, not as a sandbox key in this file. See [app-store-iap.md](app-store-iap.md).

`allow-jit` is omitted. If sandboxed WebKit will not start, add it back with an App Review justification. Do **not** re-add library-validation disable or unsigned-executable-memory to the MAS file.

A 1.2.0-style bundle that still ships `externalBin: whisper-cpp` plus Frameworks ggml dylibs **will not load** under this entitlements file. The 1.3.0 MAS overlay excludes that sidecar. The default Parakeet path is the one that can live under these entitlements.

## Sandbox notes (future)

- Parakeet models download into FluidAudio's Application Support cache (container-safe). WhisperKit uses Mabel's app-dir cache.
- User-selected files are for an explicit “choose a model” path, not a substitute for container writes.
- llama-server must stay on `127.0.0.1` if it is ever enabled in this flavor. No inbound listen off loopback.
- Paste via System Events uses the temporary Apple Events exception plus an Accessibility CGEvent fallback.
- Soft nits (review screenshots, privacy nutrition labels, sandbox path polish) come later.

## Suggested order

1. Ship 1.3.0 as the GitHub notarized DMG (Erick on M5). Default new installs to Parakeet; keep whisper.cpp as a DMG fallback.
2. Create the App Store Connect record: free, Productivity, Chibitek Labs, `com.mabel.app`. Do not upload 1.1.7, 1.2.0, or an untested 1.3.0 MAS experiment.
3. On a Mac: `npm run vendor-asr`, then `MABEL_MAS_EXPERIMENT=1 npm run build:mas`, Apple Distribution + MAS provisioning, exercise Parakeet under the sandbox.
4. Only after that binary actually launches and dictates, consider TestFlight / store review.
