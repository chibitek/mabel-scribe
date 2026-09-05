# Mac App Store and TestFlight (follow-up)

Mabel 1.2 ships as a **signed Developer ID DMG**, not as a Mac App Store build. This note is the cheap MAS / TestFlight flavor so Erick can take it later. Do not point `scripts/release-macos.sh` or the checked-in `tauri.conf.json` at the MAS entitlements.

## Store listing

| Field | Value |
|---|---|
| Bundle ID | `com.mabel.app` (unchanged) |
| Price | Free |
| Category | Productivity (`public.app-category.productivity` is already in `Info.plist`) |
| Publisher | Chibitek Labs |

TestFlight uses the same MAS-signed binary (`Apple Distribution` / Mac App Store signing), uploaded via Transporter or Xcode Organizer. It is not the Developer ID notarized DMG.

## Files in this repo

- `src-tauri/entitlements.plist` — **current ship path**. Hardened Runtime for Developer ID + notarization. Includes `allow-jit`, `allow-unsigned-executable-memory`, and `disable-library-validation` so the whisper-cpp sidecar can load the bundled ggml dylibs in `Contents/Frameworks/`.
- `src-tauri/entitlements.mas.plist` — **skeleton only**. App Sandbox, microphone, outbound network (model download + optional Groq), Apple Events (paste), user-selected files (manual model import).
- `src-tauri/tauri.mas.conf.json` — unused Tauri overlay. Example: `npm run tauri build -- --config src-tauri/tauri.mas.conf.json`. Not wired into the release script.

## Why the current binary cannot ship on MAS

`disable-library-validation` and `allow-unsigned-executable-memory` are rejected (or are a non-starter) for the Mac App Store. Today they exist because:

1. The whisper-cpp sidecar loads unsigned-from-Apple's-POV ggml dylibs from `Contents/Frameworks/`.
2. WebKit / the sidecar load path historically needed the unsigned-executable-memory entitlement.

A MAS binary must **drop `disable-library-validation`**. That means one of:

- Statically link whisper.cpp into the main executable (no sidecar dylibs), or
- Wait for Phase B (WhisperKit / FluidAudio Parakeet) so there is no whisper-cpp sidecar.

Do not try to "sandbox the current sidecar" by keeping library-validation disabled. That will fail MAS.

`allow-jit` is omitted from the MAS skeleton. If a sandboxed Tauri/WebKit build will not start without it, add it back with an App Review explanation. Do not re-add the other two.

## Sandbox notes for a future MAS flavor

- Model downloads should land in the app container (Application Support). The sandbox allows that without extra entitlements.
- `files.user-selected.read-write` is for an explicit "choose a model file" path, not a substitute for container writes.
- `network.client` covers Hugging Face model downloads and optional Groq. No server inbound listener should be bound off loopback; llama-server already uses `127.0.0.1`.
- Paste via System Events may still need a temporary Apple Events exception for `com.apple.systemevents` after the first sandbox test. Add that only if the sandboxed build cannot paste.
- In-app GitHub Releases updater (`createUpdaterArtifacts`) does not apply on the App Store. The MAS overlay turns that off. Store updates go through App Store / TestFlight.

## Suggested later work (not 1.2)

1. Keep shipping Developer ID DMGs from `scripts/release-macos.sh`.
2. Create a Mac App Store record: free, Productivity, Chibitek Labs, bundle `com.mabel.app`.
3. After Phase B or a statically linked whisper, build with `tauri.mas.conf.json` + Apple Distribution identity.
4. Upload that build to TestFlight, then submit for MAS review.
