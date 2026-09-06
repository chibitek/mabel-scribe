#!/usr/bin/env bash
# Compile native/MabelStoreKit (StoreKit 2) into src-tauri/native-storekit/.
# macOS / Xcode only. Linux and MABEL_SKIP_NATIVE_STOREKIT=1 are no-ops
# so cargo test still works off a Mac.
set -euo pipefail

ROOT=$(cd "$(dirname "$0")/.." && pwd)
OUT="$ROOT/src-tauri/native-storekit"
PKG="$ROOT/native/MabelStoreKit"

if [[ "${MABEL_SKIP_NATIVE_STOREKIT:-}" == "1" ]]; then
  echo "build-mabel-storekit: skipped (MABEL_SKIP_NATIVE_STOREKIT=1)" >&2
  exit 0
fi

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "build-mabel-storekit: not macOS; skipping Swift compile" >&2
  exit 0
fi

if ! command -v swift >/dev/null 2>&1; then
  echo "build-mabel-storekit: swift not found. Install Xcode 16+ / Swift 6." >&2
  exit 1
fi

echo "build-mabel-storekit: compiling MabelStoreKit (StoreKit 2)" >&2
mkdir -p "$OUT"
find "$OUT" -mindepth 1 -maxdepth 1 ! -name '.gitkeep' -exec rm -rf {} +

(
  cd "$PKG"
  swift build -c release --product MabelStoreKit
)

BIN=$(cd "$PKG" && swift build -c release --show-bin-path)
if [[ ! -f "$BIN/libMabelStoreKit.dylib" ]]; then
  echo "build-mabel-storekit: libMabelStoreKit.dylib not at $BIN" >&2
  exit 1
fi

cp "$BIN/libMabelStoreKit.dylib" "$OUT/libMabelStoreKit.dylib"
install_name_tool -id "@rpath/libMabelStoreKit.dylib" "$OUT/libMabelStoreKit.dylib" || true

echo "build-mabel-storekit: staged $(ls -1 "$OUT" | tr '\n' ' ')" >&2
