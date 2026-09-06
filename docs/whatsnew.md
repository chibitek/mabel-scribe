# What's New in Mabel

The in-app first-launch popup reads from this file. Every Mabel release MUST add an entry here. Newest version on top.

## v1.4.0 (2026-09-06)

### New
- Dictionary (Pro): personal terms and jargon, first in the Pro catalog. Free is locked with an in-app Plans upsell. Pro can add, edit, and delete local terms. Mabel feeds them into the existing whisper.cpp prompt hook and local Gemma spelling hints. Terms stay on this Mac — no cloud sync, no Nexus write, no team share.
- Polish (Pro): toggle default Off, modes Off / Casual / Professional / Polite. Free is locked with an in-app upsell. After ASR, local Gemma autocorrects and lightly rewords — never invents facts, never expands meaning, never leaves this Mac. Not Nexus or company memory; Coach cannot rewrite dictation. Settings → Engine and the menu-bar Polish submenu. Upgrade stays in Settings → Plans.
- Mabel Pro is an App Store subscription: monthly and yearly, each with a 30-day free trial. Prices come from the App Store.
- Activate Pro, Restore Purchases, Manage Subscriptions, and Have a code? live in Settings → Plans and Billing. There is no website upgrade or redeem. Offer codes open Apple’s system sheet (macOS 15+); cancel or an unverified code stays Free. Pro surfaces also need a Stiki sign-on — StoreKit alone is not enough. The redeem sheet does not require Stiki.
- Pro unlocks dictionary, teams (organization, seats, local invites), snippets, style, transforms, and scratchpad. Free stays personal-only.
- Transcribe a local WAV, MP3, OGG, or FLAC file to TXT, JSON, SRT, or VTT. Processing stays on this Mac (whisper.cpp sidecar / Developer ID).
- The whisper.cpp path can download a small Silero VAD model and drop silence before decode, which cuts empty-audio hallucinations.

### Fixed
- TestFlight version bumps on the same install keep Insights and local history. Application Support files are not wiped on upgrade. If a file cannot be read, Mabel says so and leaves it alone.
- Subscribe no longer freezes the window. The App Store sheet runs off the main thread, shows progress, and returns success or a recoverable error within two minutes. Restore Purchases and Manage Subscriptions stay usable. Pro still requires a verified StoreKit transaction.

## v1.3.0 (2026-09-05)

### New
- Parakeet is the default local engine on new installs. It runs in-process via FluidAudio / CoreML on the Neural Engine. No whisper.cpp sidecar for that path.
- WhisperKit large-v3-turbo is an optional local engine, also in-process.
- Settings → Engine lets you pick Parakeet, WhisperKit, or the Phase A whisper.cpp Large v3 Q5 sidecar (Developer ID builds only).

### Fixed
- Existing 1.2.0 installs keep their whisper.cpp model and are not forced onto Parakeet.

## v1.2.0 (2026-09-05)

### New
- Whisper Large v3 Q5 (~1.1 GB) is now a downloadable local model and the recommended default for new Apple Silicon installs. Small and Medium stay available as fallbacks. There is no official English-only large-v3 Q5; the multilingual Q5 is used, and the language setting still forces the English decoder or auto-detect.
- New installs default to the local engine. Groq stays opt-in behind your own API key.
- AI cleanup no longer needs Homebrew. Mabel vendors `llama-server` and its dylibs from the official llama.cpp macOS-arm64 release and loads them from a bundled `llama-runtime` folder (same rpath idea as the v1.1.0 Whisper Frameworks fix). The Gemma 4 E4B model still downloads on first use (~5 GB).
- The local LLM unloads after five minutes idle so the ~5 GB weights are not pinned for the whole session.

### Fixed
- After a successful transcription, leftover dictation audio is deleted — including any leftover `last_recording.wav` in Application Support from older builds.

## v1.1.7 (2026-05-13)

### New
- Startup update prompt. Mabel now checks for signed updates after launch and asks before downloading or installing anything.
- Repeatable release automation. The signed release script builds the app, updater bundle, icon-bearing DMG, notarizes everything, creates `latest.json`, and uploads GitHub Release assets.

### Fixed
- The About → Check for Updates flow and the startup update prompt now share the same install path and progress messaging.

## v1.1.6 (2026-05-13)

### New
- Signed and notarized test build. Mabel now uses a stable Developer ID identity so macOS Accessibility and Automation permissions stick across installs.
- In-app updates are ready. Settings → About now includes a signed update checker backed by GitHub Releases, so future versions can be installed from inside Mabel.
- The share DMG now includes the Mabel app icon, volume icon, and Applications shortcut before signing and notarization.

### Fixed
- Push-to-talk transcription no longer stalls after releasing the hotkey.
- Paste failures now report the real AppleScript error in the debug log.
- If automatic paste fails, Mabel restores the previous clipboard instead of leaving the dictated text there.
- Image-only clipboard contents are preserved around dictation paste.

## v1.1.3 (2026-05-02)

### New
- English-only Whisper models. Settings → Engine → Language lets you pick English-only (recommended) or Multilingual. The English-only models are noticeably more accurate for English dictation. New installs default to English-only Small. Existing installs keep their current model and can switch in Settings, which will download the matching new model.
- Custom Dictionary. Add proper nouns, acronyms, and jargon you dictate often, and Mabel will spell them correctly. Open the Dictionary tab in the sidebar. Words are stored locally on this Mac and never uploaded, even when using the cloud engine.

### Fixed
- More reliable transcription. Whisper now runs with steadier defaults. A previous tuning pass that bumped thread count caused streaming chunks to come back blank or with the classic "Thanks for watching" hallucination on quieter speech. Reverted.

## v1.1.2 (2026-05-01)

### Fixed
- English transcription accuracy. v1.1.1 enabled Whisper auto-detect for multilingual support, but auto-detect picks the wrong language on short utterances and produces garbage that looks English-ish but isn't what you said. Reverted to English-only by default. A proper language picker is coming in the next release.
- "What's New" popup can no longer be dismissed by an accidental backdrop click before you've actually read it. Only the "Got it" button dismisses now.

## v1.1.1 (2026-05-01)

### New
- Mabel can now walk across your desktop. Optional, off by default. Settings → System → "Mabel on your desktop". Pick how big she is, how often she visits, and how long each visit lasts.
- Multilingual transcription. Mabel auto-detects the language you're speaking and transcribes in the source language. Works for 100+ languages.
- This "What's New" popup. From now on, every Mabel update will show you what changed on first launch.

### Fixed
- Default hotkey changed from Cmd+Shift+Space to Cmd+D (the old default conflicted with several other apps).
- Groq API key no longer prompts for keychain access on every dictation. The key is cached in memory after the first read.

## v1.1.0 (2026-05-01)

### Fixed
- Mabel now actually transcribes on every Mac, not just the build machine. Previous v1.0.5 had a missing-library bug that silently broke dictation on fresh installs (the wave overlay still moved, but nothing pasted).
- AppleEvents permission is now requested during first-run setup alongside Accessibility, so you grant both up front instead of being interrupted on first paste.
- Groq API key now shows as "Saved" after a fresh install if the key is still in your macOS Keychain from a previous version.

### New
- AI cleanup tier (opt-in, ~5 GB download) now only appears when the local LLM runtime is available. When enabled, dictations are run through a local LLM after Whisper to remove filler words, fix punctuation, and normalize numbers and proper nouns. Runs fully on-device. Toggle in Settings → Engine.
