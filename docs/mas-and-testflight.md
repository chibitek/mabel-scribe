# Dual flavor: Developer ID DMG vs Mac App Store / TestFlight

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

- Sandboxed WebKit may still need `allow-jit` (omitted today; add only with an App Review justification).
- `llama-server` AI cleanup is a sidecar on the DMG. Keep it off or replace it before MAS.
- `NemoTextProcessing.framework` (FluidAudio) must be re-signed with the same team as the app.
- Paste via System Events may need a temporary Apple Events exception after the first sandbox run.
- No Apple Distribution + MAS provisioning run has been done here. Notarized DMG is still Erick-on-M5.

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
- `com.apple.security.files.user-selected.read-write` (manual model file pick)

`allow-jit` is omitted. If sandboxed WebKit will not start, add it back with an App Review justification. Do **not** re-add library-validation disable or unsigned-executable-memory to the MAS file.

A 1.2.0-style bundle that still ships `externalBin: whisper-cpp` plus Frameworks ggml dylibs **will not load** under this entitlements file. The 1.3.0 MAS overlay excludes that sidecar. The default Parakeet path is the one that can live under these entitlements.

## Sandbox notes (future)

- Parakeet models download into FluidAudio's Application Support cache (container-safe). WhisperKit uses Mabel's app-dir cache.
- User-selected files are for an explicit “choose a model” path, not a substitute for container writes.
- llama-server must stay on `127.0.0.1` if it is ever enabled in this flavor. No inbound listen off loopback.
- Paste via System Events may still need a temporary Apple Events exception for `com.apple.systemevents` after the first sandbox run.
- Soft nits (review screenshots, privacy nutrition labels, sandbox path polish) come later.

## Suggested order

1. Ship 1.3.0 as the GitHub notarized DMG (Erick on M5). Default new installs to Parakeet; keep whisper.cpp as a DMG fallback.
2. Create the App Store Connect record: free, Productivity, Chibitek Labs, `com.mabel.app`. Do not upload 1.1.7, 1.2.0, or an untested 1.3.0 MAS experiment.
3. On a Mac: `npm run vendor-asr`, then `MABEL_MAS_EXPERIMENT=1 npm run build:mas`, Apple Distribution + MAS provisioning, exercise Parakeet under the sandbox.
4. Only after that binary actually launches and dictates, consider TestFlight / store review.
