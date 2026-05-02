# Mabel Scribe

Privacy-first clinical scribe for macOS. A fork of [Mabel](https://github.com/erickgrau/Mabel) tailored for medical and clinical use. Audio is transcribed on-device. Audio and transcripts never leave the Mac.

**Status: v0.1, internal release.** This is an early build for a single client engagement, not a public product. Not a medical device. Not for any safety-critical clinical use.

## What's in v0.1

Inherited from Mabel v1.1.3:

- Whisper.cpp transcription on-device (English-only and multilingual models, Small and Medium).
- Floating overlay with live waveform during recording.
- Toggle and push-to-talk hotkey modes.
- Custom dictionary that primes Whisper with proper nouns, drug names, and jargon.
- Optional Gemma 4 E4B local LLM cleanup pass for filler removal, punctuation, and self-correction collapse.
- Local stats only. No telemetry.
- Optional Groq cloud transcription (opt-in, requires user-supplied API key). **Not appropriate for HIPAA-covered audio.**

New in Mabel Scribe:

- Optional second-pass medical-terminology polish using BioMistral 7B Q5_K_M (Apache 2.0). Runs locally after Gemma cleanup. Fixes drug names, anatomy, and medical abbreviations via a constrained edit list. ~5.1 GB additional download. Off by default.
- Bundle ID `com.chibitek.mabelscribe`. Installs and runs alongside Mabel without collision.
- Desktop walking companion removed.

## What's planned

Not in v0.1, but on the roadmap:

- Session model (Start visit, End visit) replacing the dictation hotkey.
- Dual-source audio capture (provider mic plus ScreenCaptureKit system audio for the patient side).
- SOAP, HPI, ROS, A&P, and follow-up note templates.
- Configurable retention and per-session AES-encrypted storage.
- Required pre-visit consent gate.

## Privacy posture (v0.1)

- Audio is recorded to a temp file, transcribed, deleted.
- Transcripts are not logged to disk.
- No telemetry, no analytics, no remote logging.
- API keys (Groq) live in the macOS Keychain.
- Whisper, Gemma 4 cleanup, and BioMistral 7B polish all run on-device. Nothing is sent to any server unless the user enables Groq cloud transcription.

## System requirements

- macOS 12 (Monterey) or later.
- Apple Silicon (M1 or newer). Intel build is not currently distributed.
- Disk: ~500 MB for Whisper Small, ~1.5 GB for Medium, ~5 GB for Gemma 4 cleanup, ~5.1 GB for BioMistral 7B polish. Pick what you enable.

## Build

The build flow is mostly identical to Mabel. See the [Mabel README](https://github.com/erickgrau/Mabel/blob/main/README.md#build-it-yourself) for prerequisites, signing, and notarization. Differences for Mabel Scribe:

- Bundle identifier is `com.chibitek.mabelscribe` (not `com.mabel.app`).
- Package name is `mabel-scribe`, Cargo crate name is `mabel-scribe`, Rust lib name is `scribe_lib`.
- Optional dev env vars are `SCRIBE_GROQ_KEY` and `SCRIBE_LLAMA_SERVER`.
- Signing identity is shared with Mabel (`Developer ID Application: Erick Grau (DF9FB764AR)`).
- A static `llama-server` is bundled alongside `whisper-cpp` so users don't need to install llama.cpp separately. The binary is produced by [scripts/build-llama-server.sh](scripts/build-llama-server.sh), which clones llama.cpp at the latest release tag and links the server statically with Metal acceleration.

```bash
git clone https://github.com/chibitek/mabel-scribe.git
cd mabel-scribe
npm install

# One-time: build the bundled llama-server sidecar (~5 minutes on M-series).
# Re-run this script when you want to bump llama.cpp.
scripts/build-llama-server.sh

npm run tauri dev
```

The `whisper-cpp` sidecar binary is not currently fetched by a script. If `src-tauri/binaries/whisper-cpp-aarch64-apple-darwin` is missing on a fresh clone, copy it over from your existing Mabel checkout, or build whisper.cpp locally and place it there.

## License

MIT. See [LICENSE](LICENSE).

## Important disclaimer

Mabel Scribe is a transcription and note-drafting tool. It does not make clinical decisions. It does not suggest diagnoses, codes, or treatments. The output requires review and editing by a licensed provider before use in any patient record. Mabel Scribe is not FDA-cleared and not registered as a medical device.
