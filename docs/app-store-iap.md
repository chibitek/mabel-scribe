# Mabel Pro — App Store Connect checklist

CIO / CoS create these in App Store Connect **before** any free Mac App Store or TestFlight ship. The app is fail-closed: no verified StoreKit 2 subscription (trial included) means Free, personal-only.

Do **not** invent other prices or product IDs unless this document is updated first.

## App identity

| Field | Value |
|---|---|
| Bundle ID | `com.mabel.app` |
| ASC app Apple ID | `6809059582` (Mabel Dictation) |
| App price | Free |
| Category | Productivity |

## Subscription products

Create **one** subscription group, then two auto-renewable subscriptions.

| Product ID | Reference name | Duration | Price (locked) | Intro offer |
|---|---|---|---|---|
| `com.mabel.app.pro.monthly` | Mabel Pro Monthly | 1 month | **$5 / user / month** | 30-day free trial |
| `com.mabel.app.pro.yearly` | Mabel Pro Yearly | 1 year | **$48 / user / year** | 30-day free trial |

ASC introductory offers use calendar units. Use **Free trial → 1 month** (that is Apple's 30-day trial). Same intro on both products. Same subscription group so users can switch monthly ↔ yearly.

### Suggested group

- Group reference name: `Mabel Pro`
- Localization (en-US): display name `Mabel Pro`, description `Teams plus every Pro feature. Personal use stays free.`

### Suggested localizations (en-US)

**Monthly**

- Display name: `Mabel Pro Monthly`
- Description: `Unlock teams (org, seats, invites) and every Pro feature. 30-day free trial, then the App Store price.`

**Yearly**

- Display name: `Mabel Pro Yearly`
- Description: `Unlock teams (org, seats, invites) and every Pro feature. 30-day free trial, then the App Store yearly price.`

The UI **must** show StoreKit `displayPrice` and intro-offer text from `Product`. Do not treat "$5" / "$48" as the only price source in the client.

## Capabilities (MAS / TestFlight only)

1. Apple Developer → Identifiers → `com.mabel.app` → enable **In-App Purchase**.
2. Regenerate the **Mac App Store** provisioning profile so it includes IAP.
3. Sign MAS / TestFlight with **Apple Distribution** + that profile.

In-App Purchase is an **App ID + profile** capability. It is **not** a key in `entitlements.plist` / `entitlements.mas.plist`. Do **not** add `com.apple.developer.in-app-payments` (that is Apple Pay). Developer ID DMG entitlements stay unchanged so notarized builds are not broken.

MAS flavor still keeps the #10 keys: `com.apple.security.device.audio-input` and `temporary-exception.apple-events` → `com.apple.systemevents`.

## What Pro unlocks

- Teams: on-device org name, seats, invite codes (no cloud sync in v1)
- Locked nav: Snippets, Style, Transforms, Scratchpad
- Polish: Off / Casual / Professional / Polite (default Off; local Gemma; never invents)
- Free remains personal-only

Those surfaces unlock only with a verified StoreKit transaction **and** a live Stiki session. StoreKit alone is not enough. Have a code? is not gated on Stiki.

There is no web upgrade. Settings → Plans and Billing is StoreKit purchase / restore / manage / **Have a code?** only. Do not point Activate Pro at chibiteklabs.com or chibiteklabs.ai.

## Subscription offer codes

CIO / ASC create Apple **Subscription Offer Codes** (and win-back / promotional codes as App Store Connect allows) on app **`6809059582`**. Eng does **not** invent prices, product IDs, or a custom redeem field.

- Plans → **Have a code?** opens StoreKit `offerCodeRedemption` (the system sheet).
- macOS 15+ only. Deployment target stays **14.0**. On unsupported OS the button stays (Cat UI on Plans) and shows recoverable copy: **Offer codes need a newer macOS**. Never crash. Never mock-grant Pro.
- Cancel, fail, timeout, or an unverified transaction → stay Free. Entitlement is still verified StoreKit tx only (monthly / yearly + trial).
- **Sign-on RE-LOCK:** the redeem sheet is **not** gated on Stiki. Pro surface unlock needs **both** a verified StoreKit subscription **and** a live Stiki session. StoreKit alone is not enough after redeem (or any other purchase).
- No website redeem. No home-rolled code UI that grants Pro.
- Do **not** name `OfferType.winBack` or `AppStore.showManageSubscriptions` in `MabelStoreKit.swift` (those broke `swift build` for macosx14.0).

## CIO Mac prove (purchase + trial)

Local StoreKit products exist **only** when the running process was launched by Xcode with **StoreKit Configuration = `src-tauri/Mabel.storekit`**. That file is a local (not ASC-synced) Xcode 16 v4 config: monthly + yearly, 30-day (`P1M` free) intro, no `storefrontTimeZone` (that field made Xcode report `_lastMigrationError` and left `SKTestSession` hung / `NO_PRODUCTS`).

`npm run tauri -- dev`, double-clicking a `.app`, and anything under `/Applications` (including **`Mabel 2.app`**) do **not** attach the configuration. Those paths are why prove went RED on tip `0178e46c` even after the scheme path was edited.

### 0. Use this branch's 1.4.0 binary

Wrong binary for prove:

- Installed `/Applications/Mabel.app` or `/Applications/Mabel 2.app` at **1.3.0 without StoreKit**
- Any build where `Contents/Frameworks/libMabelStoreKit.dylib` is missing
- Any build where Swift compile failed and cargo used to warn-and-continue (that is now a hard error on macOS)

```bash
git checkout cursor/storekit2-pro-iap-cfa6
git pull
# One-shot: dylib + 1.4.0 .app + catalog smoke + open Xcode
bash scripts/prove-storekit-mac.sh
```

### 1. MabelStoreKit dylib (Apple Silicon / Xcode 16)

Full **Xcode.app** (not Command Line Tools):

```bash
sudo xcode-select -s /Applications/Xcode.app/Contents/Developer
xcode-select -p   # must contain Xcode.app
xcrun swift --version   # Swift 6 / Xcode 16+
```

Exact compile command. **Deployment target stays macOS 14.0** (same as `Package.swift` `.macOS(.v14)`). Do **not** raise the package or the app to macOS 15. `OfferType.winBack` and `AppStore.showManageSubscriptions(in:)` are macOS 15 / iOS-scene APIs and are not referenced; Manage Subscriptions opens Apple’s account URL instead.

```bash
export MACOSX_DEPLOYMENT_TARGET=14.0
xcrun swift build -c release --arch arm64 --product MabelStoreKit \
  --package-path native/MabelStoreKit
```

Or the wrapper that stages the dylib for Tauri (`@rpath/libMabelStoreKit.dylib`):

```bash
npm run vendor-storekit
# → src-tauri/native-storekit/libMabelStoreKit.dylib
ls -l src-tauri/native-storekit/libMabelStoreKit.dylib
xcrun otool -D src-tauri/native-storekit/libMabelStoreKit.dylib
```

`tauri.conf.json` `beforeDevCommand` / `beforeBuildCommand` now run `vendor-storekit`. On macOS, `src-tauri/build.rs` **fails the Rust compile** if the dylib is missing so you cannot get a silent 1.4.0-without-StoreKit binary. `MABEL_SKIP_NATIVE_STOREKIT=1` is fail-closed Free only.

The prove script also needs **Parakeet / WhisperKit** (`vendor-asr`) before `tauri build`. Same Xcode 16 / Swift 6 / macosx14.0:

```bash
export MACOSX_DEPLOYMENT_TARGET=14.0
xcrun swift build -c release --arch arm64 --product MabelASR \
  --package-path native/MabelASR
# or
npm run vendor-asr
# → src-tauri/native-asr/libMabelASR.dylib
```

`MabelASR.swift` is Swift 6-clean: locked Sendable last-error box, `mabel_asr_progress_cb` typealias (no mixed-language header), `await` on actor `isAvailable`, WhisperKit `[TranscriptionResult]` + non-optional `text`.

### 2. Package 1.4.0 with the dylib inside the .app

```bash
npx tauri build --bundles app
APP=src-tauri/target/release/bundle/macos/Mabel.app
defaults read "$APP/Contents/Info" CFBundleShortVersionString   # must be 1.4.0
ls "$APP/Contents/Frameworks/libMabelStoreKit.dylib"            # must exist
```

If Finder already has `Mabel.app`, Tauri may write `Mabel 2.app` in the bundle folder. Check **that** tree's version and Frameworks — still do not use `/Applications/Mabel 2.app`.

MAS overlay (`tauri.mas.conf.json`) also lists `native-storekit/libMabelStoreKit.dylib`.

### 3. Catalog smoke (`xcodebuild test`) vs purchase UI (⌘R)

Checked-in project: `tools/MabelStoreKitProve/MabelStoreKitProve.xcodeproj`

| Path | What it does |
|---|---|
| **MabelStoreKitProve** + `xcodebuild test` | XCTest `ProductLoadTests` creates `SKTestSession(contentsOf: src-tauri/Mabel.storekit)` (or the copied bundle resource). This is the automated catalog prove. |
| **Mabel-StoreKit** + **⌘R** | PathRunnable launches the Tauri `Mabel.app` with scheme StoreKit Configuration `container:Mabel.storekit`. **This is the purchase / trial UI prove.** |

`xcodebuild test` does **not** apply a scheme `StoreKitConfigurationFileReference` (CIO tried relative, absolute, and copy-into-project on tip `446378a6` — still `NO_PRODUCTS`). Do **not** also set StoreKit Configuration on the **MabelStoreKitProve** Test action: pairing that with `SKTestSession` hangs.

Temporary limit: there is no supported way for `xcodebuild test` to launch the prebuilt Tauri `Mabel.app` under a scheme StoreKit Configuration. Purchase + trial sheets still need a human **⌘R** on **Mabel-StoreKit**.

Catalog-only smoke:

```bash
xcodebuild test \
  -project tools/MabelStoreKitProve/MabelStoreKitProve.xcodeproj \
  -scheme MabelStoreKitProve \
  -destination 'platform=macOS'
```

Purchase / trial UI:

1. Open `tools/MabelStoreKitProve/MabelStoreKitProve.xcodeproj` (the prove script does this).
2. Select scheme **Mabel-StoreKit** (not ProveHost / not MabelStoreKitProve Run).
3. Product → Scheme → Edit Scheme → **Run → Options → StoreKit Configuration** → `Mabel.storekit` (`container:Mabel.storekit`).
4. Press **Run (⌘R)**. Confirm the debug title / console is the `target/release/bundle/macos/Mabel.app` path, not `/Applications`.

### 4. Confirm products loaded

Settings → Plans and Billing must show **two** cards with StoreKit `displayPrice` and a **30-day / 1 month free trial** line for:

- `com.mabel.app.pro.monthly`
- `com.mabel.app.pro.yearly`

If the UI says `NO_PRODUCTS` or “App Store prices are unavailable”, the process is not under that configuration (or you launched 1.3.0). Stop; do not treat restore-empty as a product prove.

### 5. Checklist

| Case | Expect |
|---|---|
| **Purchase + trial** | Subscribe Monthly (or Yearly). Button shows **Waiting for App Store…**. StoreKit sheet from the Xcode config. Success or a visible error within ~125s — the window must not hang. Entitlement `status=trial`, `isTrial=true`. Pro surfaces (teams + locked nav + Polish) still need a live Stiki session — StoreKit alone is not enough. |
| **Restore / Manage** | Stay clickable during purchase. Restore times out with an error instead of hanging. Manage still opens the App Store subscriptions URL immediately. |
| **Fail-closed** | Quit, or Debug → StoreKit → refund / expire in Xcode. App is Free. Timeout / cancel / pending never grant Pro. No mock paid path. Missing dylib cannot compile on macOS unless `MABEL_SKIP_NATIVE_STOREKIT=1`, which stays Free. |
| **Have a code?** | Plans Cat UI opens `offerCodeRedemption` on macOS 15+. Cancel / fail / unverified stay Free. macOS 14 shows **Offer codes need a newer macOS**. Sheet is not gated on Stiki. Pro surfaces stay locked until StoreKit **and** a live Stiki session. |

Sandbox Apple IDs are for TestFlight / ASC sandbox, not this local configuration. The `.storekit` file does not ship paid entitlement.

Do **not** merge this draft on a RED purchase+trial.

## Build notes

- Native bridge: `native/MabelStoreKit` (StoreKit 2, C ABI, pure Swift package) staged by `scripts/build-mabel-storekit.sh` / `npm run vendor-storekit`.
- Rust fail-closed parser: `src-tauri/src/storekit.rs`.
- MAS flavor still uses `entitlements.mas.plist` + `tauri.mas.conf.json` (device.audio-input + System Events AE exception + home-relative read of prior Application Support for upgrade import). IAP does not add Apple Pay keys and does not change Developer ID `entitlements.plist`.
- Purchase / restore FFI must not run on the AppKit main thread. Swift waits at most 120s; Rust IPC 125s. Unverified transactions still never grant Pro.
- Release builds have **no** mock paid entitlement. Missing dylib / non-macOS / unverified transaction → Free.

## Still required before a free MAS ship

1. Products above exist on app `6809059582` and are Ready to Submit.
2. App ID IAP capability + MAS profile.
3. Erick runs a MAS/TF binary on a Mac and completes purchase, trial, restore, and manage.
4. Other MAS blockers in [mas-and-testflight.md](mas-and-testflight.md) (sandbox paste, signing, no untested upload).
