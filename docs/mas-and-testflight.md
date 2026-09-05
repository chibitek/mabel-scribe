# Dual flavor: Developer ID DMG vs Mac App Store / TestFlight

**1.1.7 and 1.2.0 local Whisper (whisper-cpp sidecar + ggml dylibs) are not Mac App Store ready.** Do not upload those binaries to MAS or Mac TestFlight. Do not ship 1.1.7 to the store.

The outside-store channel stays the **GitHub notarized Developer ID DMG** (`scripts/release-macos.sh`). MAS / TestFlight is a second flavor that needs App Sandbox, Apple Distribution signing, a Mac App Store provisioning profile, and an engine that does not require `disable-library-validation`.

The clean long-term MAS engine is **Phase B** (in-process WhisperKit / FluidAudio Parakeet). Until then, a MAS binary would need whisper **statically linked** into the main executable. This repo does not do that yet. Phase B is not started here.

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
| **Developer ID DMG (ship 1.2)** | `src-tauri/entitlements.plist` | `tauri.conf.json` + private `tauri.local.conf.json` | `scripts/release-macos.sh` or `npm run build:dmg` |
| **MAS / TestFlight (not shippable yet)** | `src-tauri/entitlements.mas.plist` | overlay `src-tauri/tauri.mas.conf.json` | `npm run build:mas` (refuses unless `MABEL_MAS_EXPERIMENT=1`) |

`release-macos.sh` is Developer ID only. It is not wired to the MAS overlay. Do not point it at `entitlements.mas.plist`.

### Developer ID DMG (current ship path)

```bash
# Private signing override (gitignored). Identity is Developer ID Application.
# scripts/release-macos.sh
```

`entitlements.plist` is Hardened Runtime **without** App Sandbox. It includes the three codesigning entitlements the current sidecar needs:

- `com.apple.security.cs.allow-jit`
- `com.apple.security.cs.allow-unsigned-executable-memory`
- `com.apple.security.cs.disable-library-validation` (whisper-cpp + `Contents/Frameworks/` ggml dylibs)

Those last two **fail MAS**. `disable-library-validation` is the hard blocker for the current local engine.

### MAS / TestFlight (draft flavor only)

```bash
# Prints the not-ready warning and exits unless you opt in:
MABEL_MAS_EXPERIMENT=1 npm run build:mas
```

That runs Tauri with `--config src-tauri/tauri.mas.conf.json --bundles app` so you get an `.app`, not a DMG. After Phase B (or a static-link whisper), the remaining store steps on a Mac with the right certs are:

1. Sign with **Apple Distribution: Chibitek Labs (TEAMID)** and embed the **Mac App Store** provisioning profile for `com.mabel.app`.
2. Wrap with `productbuild` / Xcode using **3rd Party Mac Developer Installer**.
3. Upload that pkg to App Store Connect.
4. Enable **Mac TestFlight**, then submit the free Productivity listing.

`tauri.mas.conf.json` turns off GitHub updater artifacts. Store updates go through App Store / TestFlight, not `latest.json`.

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

A 1.2.0 build that still ships `externalBin: whisper-cpp` plus Frameworks ggml dylibs **will not load** under this entitlements file. That is expected. It is why MAS waits on Phase B or a static-link whisper.

## Sandbox notes (future)

- Model downloads belong in the app container (Application Support). The sandbox allows that without extra entitlements.
- User-selected files are for an explicit “choose a model” path, not a substitute for container writes.
- llama-server must stay on `127.0.0.1`. No inbound listen off loopback.
- Paste via System Events may still need a temporary Apple Events exception for `com.apple.systemevents` after the first sandbox run.
- Soft nits (review screenshots, privacy nutrition labels, sandbox path polish) come later.

## Suggested order

1. Ship 1.2.0 as the GitHub notarized DMG. That is the only supported distribution for this PR.
2. Create the App Store Connect record: free, Productivity, Chibitek Labs, `com.mabel.app`. Do not upload 1.1.7 or 1.2.0 sidecar builds.
3. Land Phase B (or statically link whisper) so MAS entitlements can stay stripped of library-validation disable.
4. Then `MABEL_MAS_EXPERIMENT=1 npm run build:mas`, Apple Distribution + MAS provisioning, TestFlight, store review.
