# Public source of truth and secret audit

Date: 2026-09-06  
Scope: `https://github.com/erickgrau/Mabel` (this repo) and `https://github.com/chibitek/mabel-scribe`

## SoT recommendation

**`erickgrau/Mabel` is the public source of truth.** It is a public, non-fork repo. `main` at this audit is **1.4.0** after [#15 Polish](https://github.com/erickgrau/Mabel/pull/15) and [#16 P0 history + StoreKit](https://github.com/erickgrau/Mabel/pull/16) (`15999ec`). Clone, fork, and ship from here.

**`chibitek/mabel-scribe` is a public GitHub fork / mirror of that SoT, not a second SoT.** On 2026-09-06 its `main` was fast-forwarded from `526eb44` (v1.1.3, May) to **`15999ec`** (same commit as `erickgrau/Mabel` `main`; GitHub compare `identical`, ahead 0 / behind 0). Develop and open PRs on `erickgrau/Mabel` only.

### Keeping the mirror current

This Cloud Agent token can **read** `chibitek/mabel-scribe` and **cannot push** (no `push` permission). After each SoT `main` merge, someone with Chibitek org write access should:

1. Open https://github.com/chibitek/mabel-scribe
2. **Sync fork → Update branch** (fast-forward `main` from `erickgrau/Mabel`)
3. Or, from a machine that can push the fork:

```bash
git clone https://github.com/chibitek/mabel-scribe.git
cd mabel-scribe
git remote add upstream https://github.com/erickgrau/Mabel.git
git fetch upstream
git checkout main
git merge --ff-only upstream/main
git push origin main
```

Optional: archive the fork if a Chibitek-org mirror is not required. Do not develop on it. The May 2026 tip (`526eb44`) still exists in fork history with a real Developer ID identity string in `tauri.conf.json` — same public team/identity as old SoT history, not a private key.

## Feature train: public vs Pro-only

The public tree **already contains** the current main feature train. Nothing from #15 / #16 is withheld as a private tree.

| Surface | In public SoT? | Runtime |
|---|---|---|
| Parakeet / WhisperKit / whisper.cpp | Yes | Free |
| Clipboard history (opt-in, local) | Yes | Free; same-container import on TF upgrade |
| Polish (Off / Casual / Professional / Polite, local Gemma) | Yes | **Pro** (StoreKit fail-closed) |
| Dictionary (local terms) | Yes | **Pro + Stiki** |
| Snippets (local trigger → expansion) | Yes | **Pro + Stiki** |
| Style (Formal / Casual / Very casual, local register) | Yes | **Pro + Stiki** |
| Teams, transforms, scratchpad | Yes | **Pro** |
| StoreKit 2 monthly / yearly + 30-day trial | Yes | App Store / local `.storekit` prove |
| Mabel Spatial (visionOS sibling) | Yes | Separate listing later |

**Pro-only means a StoreKit entitlement gate, not a private source tree.** Official shipping secrets stay off git (see below). Forks that want their own paid SKU must change bundle ID + signing and create their own ASC products.

## What must stay private (never commit)

Use placeholders + local / CI secrets only:

| Item | Where it lives |
|---|---|
| Developer ID / Apple Distribution identity | `src-tauri/tauri.local.conf.json` (gitignored) and `MABEL_SIGNING_IDENTITY` |
| Notarization Apple ID + app-specific password | Keychain profile (`AC_PASSWORD` / `MABEL_NOTARY_PROFILE`) |
| Tauri updater **private** key | `$HOME/.tauri/mabel-updater.key` (`TAURI_SIGNING_PRIVATE_KEY_PATH`) |
| Groq API keys | macOS Keychain via `secrets.rs`, or `MABEL_GROQ_KEY` in the local shell |
| ASC API keys / `.p8` / issuer IDs | Not in this repo; never add them |
| StoreKit shared secret | Not used (StoreKit 2). Do not add a shared secret. |
| Signing certs / `.p12` / `.mobileprovision` | Local Keychain / profiles only |

Checked-in `tauri.conf.json` uses `Developer ID Application: Your Name (TEAMID)`. `scripts/release-macos.sh` **requires** `MABEL_SIGNING_IDENTITY`; it has no baked-in identity.

The Tauri updater **public** key in `tauri.conf.json` is a public key. That is expected.

## Findings (values redacted)

Hunt covered working tree, git history (added/deleted secret-like files and common token patterns), StoreKit config, docs, scripts, Spatial, and the `mabel-scribe` tip. No `.env*`, `.p12`, `.pem`, `.p8`, `.key`, or provisioning profiles are tracked. No Groq keys, ASC API keys, 1Password `op://` secret refs, Railway/Supabase URLs with credentials, or StoreKit shared secrets.

| Sev | Path / location | What | Action |
|---|---|---|---|
| **Medium** | `scripts/release-macos.sh` (main, pre-this-PR) | Default `MABEL_SIGNING_IDENTITY` was a real `Developer ID Application: <name> (<team>)` string. Not a private key, but it taught forks the official identity. | **Scrubbed this PR.** Set the env var locally. History still has the old default — see rotation. |
| **Medium** | `chibitek/mabel-scribe` history @ `526eb44` `src-tauri/tauri.conf.json` | Same identity was on the May 2026 tip. **`main` now matches SoT `15999ec`.** | Keep the mirror synced via GitHub Sync fork after each SoT merge. |
| **Low** | Git history of `erickgrau/Mabel` (`0a4b932` … `aff7605`) | Same identity lived in `tauri.conf.json` until 2026-05-12. | No history rewrite (would break #15/#16 SHAs). Team IDs are public on signed binaries. |
| **Info** | `MabelSpatial/**` (`DEVELOPMENT_TEAM`, `SpatialConstants.swift`, README) | Apple Team ID for the official visionOS target. Marked `pragma: allowlist secret`. | **Keep.** This is a public Apple team identifier, required for the official Xcode project. Not a credential. |
| **Info** | `src-tauri/src/storekit.rs`, docs, `.storekit` | ASC app Apple ID `6809059582`. | **Keep.** Public App Store record id. |
| **Info** | `src-tauri/src/storage.rs` tests | Used `/Users/erick/...` as a container-path fixture. | **Scrubbed this PR** to `/Users/dev/...`. |
| **Info** | README notarization examples | `APPLE_PASSWORD="abcd-efgh-ijkl-mnop"` is Apple’s documented example shape, not a live password. | Leave as placeholder. |
| **Info** | `src-tauri/src/clipboard_history.rs` tests | Synthetic token fixtures, assembled at runtime. JWT header prefix split further this PR. | Not live keys. |
| **Clear** | Groq / ASC API / 1Password / Railway / Supabase / p12 / pem / shared secret | None found on either public tip. | No rotation from those classes. |

## Rotation (Erick / CIO)

Do **not** treat this list as leaked key material. Nothing that looks like a live API token was found.

1. **No Groq, ASC `.p8`, updater private key, or notarization password was in git.** If any of those were ever pasted into a chat, ticket, or CI log outside this audit, rotate that specific item. Do not publish the new value.
2. **Apple Team ID / Developer ID display name** cannot be “rotated” without a new team. They already appear on notarized binaries. Confirm the Developer ID certificate itself was never exported into git (this audit found no cert files).
3. **`mabel-scribe`:** `main` matches `15999ec`. Assume crawlers copied the May 2026 `tauri.conf.json` identity string from history. Same as (2) — identity name, not a key. Re-sync the fork after this PR merges.
4. If an App Store Connect **shared secret** was created for the old subscription API, do not put it in this repo. StoreKit 2 on 1.4.0 does not need it.

## Hygiene lock

`storekit::tests::checked_in_signing_is_placeholder` fails if `tauri.conf.json` or `scripts/release-macos.sh` grow a real Developer ID identity again.
