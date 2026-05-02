# What's New in Mabel Scribe

The in-app first-launch popup reads from this file. Every release adds an entry here. Newest version on top.

## v0.1.0 (2026-05-02)

### New

- Mabel Scribe forks Mabel v1.1.3. Separate brand, separate bundle ID (`com.chibitek.mabelscribe`), separate update channel. Installs alongside Mabel without collision.
- Optional medical-terminology polish. A second pass after the AI cleanup that fixes drug names, anatomy, and medical abbreviations using BioMistral 7B running locally. Runs only on text already cleaned by Gemma. ~5.1 GB one-time download. Off by default. Settings → Engine → Cleanup → Medical terminology polish.

### Changed

- Removed the desktop walking companion from the Mabel feature set.
- Microphone and Apple Events permission prompts now reflect the Scribe brand and clinical context.

### Important

- Mabel Scribe is not a medical device. Not FDA-cleared. The transcript and note output require review and editing by a licensed provider before any clinical use.
