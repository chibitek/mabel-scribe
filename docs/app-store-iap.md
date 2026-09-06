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

## What Pro unlocks

- Teams: on-device org name, seats, invite codes (no cloud sync in v1)
- Locked nav: Snippets, Style, Transforms, Scratchpad
- Free remains personal-only

There is no web upgrade. Settings → Plans and Billing is StoreKit purchase / restore / manage only. Do not point Activate Pro at chibiteklabs.com or chibiteklabs.ai.

## Local StoreKit testing

`src-tauri/Mabel.storekit` is an Xcode StoreKit Configuration (monthly + yearly + 1-month free trial). On a Mac:

1. Open the configuration in Xcode.
2. Product → Scheme → Edit Scheme → Run → Options → StoreKit Configuration → `Mabel.storekit`.
3. Or pass the file when running the `.app` from Xcode.

Sandbox Apple IDs are required for TestFlight / ASC sandbox. The configuration file does not ship paid entitlement.

## Build notes

- Native bridge: `native/MabelStoreKit` (StoreKit 2, C ABI) staged by `scripts/build-mabel-storekit.sh` / `npm run vendor-storekit`.
- Rust fail-closed parser: `src-tauri/src/storekit.rs`.
- MAS flavor still uses `entitlements.mas.plist` + `tauri.mas.conf.json`. IAP does not add sandbox keys and does not change Developer ID `entitlements.plist`.
- Release builds have **no** mock paid entitlement. Missing dylib / non-macOS / unverified transaction → Free.

## Still required before a free MAS ship

1. Products above exist on app `6809059582` and are Ready to Submit.
2. App ID IAP capability + MAS profile.
3. Erick runs a MAS/TF binary on a Mac and completes purchase, trial, restore, and manage.
4. Other MAS blockers in [mas-and-testflight.md](mas-and-testflight.md) (sandbox paste, signing, no untested upload).
