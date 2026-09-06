#!/usr/bin/env bash
# CIO Mac prove: build 1.4.0 with StoreKit 2 and open the Xcode scheme that
# actually attaches src-tauri/Mabel.storekit to the running process.
#
# This is the only local path that can turn purchase + trial green.
# `tauri dev`, Finder, and /Applications/Mabel*.app do NOT attach the
# StoreKit Configuration — they produce NO_PRODUCTS.
set -euo pipefail

ROOT=$(cd "$(dirname "$0")/.." && pwd)
cd "$ROOT"

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "prove-storekit: this recipe is Mac / Xcode only." >&2
  exit 1
fi

echo "prove-storekit: refusing leftover /Applications binaries" >&2
for leftover in "/Applications/Mabel.app" "/Applications/Mabel 2.app"; do
  if [[ -d "$leftover" ]]; then
    echo "prove-storekit: WARNING: $leftover exists — do not launch it for this prove." >&2
    if [[ -f "$leftover/Contents/Info.plist" ]]; then
      echo "prove-storekit:   version $(defaults read "$leftover/Contents/Info" CFBundleShortVersionString 2>/dev/null || echo unknown)" >&2
    fi
  fi
done

echo "prove-storekit: 1/4 native StoreKit dylib (Apple Silicon / Xcode 16)" >&2
npm run vendor-storekit

DYLIB="$ROOT/src-tauri/native-storekit/libMabelStoreKit.dylib"
if [[ ! -f "$DYLIB" ]]; then
  echo "prove-storekit: libMabelStoreKit.dylib missing after vendor-storekit" >&2
  exit 1
fi
echo "prove-storekit: dylib $(ls -lh "$DYLIB" | awk '{print $5}')  id=$(xcrun otool -D "$DYLIB" | tail -n 1)" >&2

echo "prove-storekit: 2/4 Tauri 1.4.0 .app (this branch, not MAS skip)" >&2
# Skip the heavy llama vendor if it is already staged; StoreKit prove
# does not need the LLM sidecar.
if [[ ! -d "$ROOT/src-tauri/llama-runtime" ]] || [[ -z "$(ls -A "$ROOT/src-tauri/llama-runtime" 2>/dev/null | grep -v .gitkeep || true)" ]]; then
  npm run vendor-llama || true
fi
npm run vendor-asr
npx tauri build --bundles app

APP=""
for candidate in \
  "$ROOT/src-tauri/target/release/bundle/macos/Mabel.app" \
  "$ROOT/src-tauri/target/release/bundle/macos/Mabel 2.app"
do
  if [[ -d "$candidate" ]]; then
    APP="$candidate"
    break
  fi
done

if [[ -z "$APP" ]]; then
  echo "prove-storekit: Tauri did not produce Mabel.app under src-tauri/target/release/bundle/macos/" >&2
  exit 1
fi

if [[ "$(basename "$APP")" == "Mabel 2.app" ]]; then
  echo "prove-storekit: WARNING: bundled as Mabel 2.app (Finder collision). Still OK if version/dylib checks pass." >&2
fi

VERSION="$(defaults read "$APP/Contents/Info" CFBundleShortVersionString)"
if [[ "$VERSION" != "1.4.0" ]]; then
  echo "prove-storekit: REFUSING $APP — CFBundleShortVersionString is $VERSION, want 1.4.0" >&2
  echo "prove-storekit: you are on the wrong binary (1.3.0 has no StoreKit)." >&2
  exit 1
fi

SK_DYLIB="$APP/Contents/Frameworks/libMabelStoreKit.dylib"
if [[ ! -f "$SK_DYLIB" ]]; then
  echo "prove-storekit: REFUSING $APP — libMabelStoreKit.dylib not in Contents/Frameworks" >&2
  echo "prove-storekit: this is a 1.4.0-labelled binary without StoreKit." >&2
  ls -la "$APP/Contents/Frameworks" >&2 || true
  exit 1
fi

echo "prove-storekit: 3/4 verified $APP" >&2
echo "prove-storekit:   CFBundleShortVersionString=$VERSION" >&2
echo "prove-storekit:   Frameworks/libMabelStoreKit.dylib present" >&2

SCHEME="$ROOT/tools/MabelStoreKitProve/MabelStoreKitProve.xcodeproj/xcshareddata/xcschemes/Mabel-StoreKit.xcscheme"
python3 - "$SCHEME" "$APP" <<'PY'
import pathlib, sys
scheme = pathlib.Path(sys.argv[1])
app = pathlib.Path(sys.argv[2]).resolve()
text = scheme.read_text()
# Xcode PathRunnable needs an absolute FilePath or Run launches nothing.
import re
text2, n = re.subn(
    r'FilePath = "[^"]*Mabel(?: 2)?\.app"',
    f'FilePath = "{app}"',
    text,
)
if n == 0:
    sys.exit("prove-storekit: could not patch Mabel-StoreKit.xcscheme FilePath")
scheme.write_text(text2)
print(f"prove-storekit: scheme PathRunnable -> {app}", file=sys.stderr)
PY

PROJECT="$ROOT/tools/MabelStoreKitProve/MabelStoreKitProve.xcodeproj"
echo "prove-storekit: 4/4 catalog smoke + open Xcode" >&2
echo "prove-storekit: xcodebuild test (MabelStoreKitProve scheme, StoreKit Configuration on)" >&2
if xcodebuild test \
  -project "$PROJECT" \
  -scheme MabelStoreKitProve \
  -destination 'platform=macOS' \
  -only-testing:ProveTests/ProductLoadTests/testMonthlyAndYearlyLoadWithIntroTrial
then
  echo "prove-storekit: catalog smoke GREEN (monthly + yearly + 30-day intro)" >&2
else
  echo "prove-storekit: catalog smoke RED — open the .storekit in Xcode and confirm it migrates without timezone errors." >&2
  echo "prove-storekit: continuing to open the Run scheme so you can still launch Mabel.app under the config." >&2
fi

open -a Xcode "$PROJECT"
echo "" >&2
echo "=== CIO prove next steps ===" >&2
echo "1. In Xcode, select scheme **Mabel-StoreKit** (not ProveHost)." >&2
echo "2. Product → Scheme → Edit Scheme → Run → Options → StoreKit Configuration" >&2
echo "   must be src-tauri/Mabel.storekit (already set on this scheme)." >&2
echo "3. Press Run (⌘R). Xcode must launch:" >&2
echo "   $APP" >&2
echo "4. Confirm About / Settings shows 1.4.0. Settings → Plans and Billing" >&2
echo "   must list Monthly + Yearly with a 30-day trial line (not NO_PRODUCTS)." >&2
echo "5. Checklist: purchase monthly (trial), restore (empty or that trial)," >&2
echo "   fail-closed (quit / no subscription → Free)." >&2
echo "6. Do NOT double-click /Applications/Mabel.app or Mabel 2.app." >&2
echo "7. Do NOT construct SKTestSession inside Mabel.app (hangs when the" >&2
echo "   scheme already has a StoreKit Configuration)." >&2
echo "" >&2
echo "Docs: docs/app-store-iap.md" >&2
