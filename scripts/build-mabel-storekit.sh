#!/usr/bin/env bash
# Compile native/MabelStoreKit (StoreKit 2) into src-tauri/native-storekit/.
#
# Exact Apple Silicon / Xcode 16 command this script runs:
#
#   MACOSX_DEPLOYMENT_TARGET=14.0 \
#   xcrun swift build -c release --arch arm64 --product MabelStoreKit \
#     --package-path native/MabelStoreKit
#
# Then it copies libMabelStoreKit.dylib to src-tauri/native-storekit/ and
# sets LC_ID_DYLIB to @rpath/libMabelStoreKit.dylib so Tauri's
# Contents/Frameworks rpath can load it.
#
# macOS / full Xcode only. Linux and MABEL_SKIP_NATIVE_STOREKIT=1 are
# no-ops so cargo test still works off a Mac.
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

if ! command -v xcrun >/dev/null 2>&1; then
  echo "build-mabel-storekit: xcrun not found. Install Xcode 16+ from the App Store." >&2
  exit 1
fi

XCODE_PATH="$(xcode-select -p 2>/dev/null || true)"
if [[ -z "${XCODE_PATH}" || "${XCODE_PATH}" == *"CommandLineTools"* ]]; then
  echo "build-mabel-storekit: xcode-select points at Command Line Tools, not Xcode.app." >&2
  echo "build-mabel-storekit: sudo xcode-select -s /Applications/Xcode.app/Contents/Developer" >&2
  exit 1
fi

if ! xcrun --find swift >/dev/null 2>&1; then
  echo "build-mabel-storekit: swift not found under $(xcode-select -p). Install Xcode 16+ / Swift 6." >&2
  exit 1
fi

ARCH="$(uname -m)"
ARCH_ARGS=()
if [[ "${ARCH}" == "arm64" ]]; then
  ARCH_ARGS+=(--arch arm64)
fi

export MACOSX_DEPLOYMENT_TARGET="${MACOSX_DEPLOYMENT_TARGET:-14.0}"

echo "build-mabel-storekit: Xcode $(xcode-select -p)" >&2
echo "build-mabel-storekit: $(xcrun swift --version 2>&1 | head -n 1)" >&2
echo "build-mabel-storekit: compiling MabelStoreKit (StoreKit 2, ${ARCH}, macosx${MACOSX_DEPLOYMENT_TARGET})" >&2
mkdir -p "$OUT"
find "$OUT" -mindepth 1 -maxdepth 1 ! -name '.gitkeep' -exec rm -rf {} +

(
  cd "$PKG"
  xcrun swift build -c release --product MabelStoreKit "${ARCH_ARGS[@]}"
)

BIN=$(cd "$PKG" && xcrun swift build -c release --product MabelStoreKit "${ARCH_ARGS[@]}" --show-bin-path)
DYLIB=""
if [[ -f "$BIN/libMabelStoreKit.dylib" ]]; then
  DYLIB="$BIN/libMabelStoreKit.dylib"
else
  DYLIB=$(find "$PKG/.build" -name 'libMabelStoreKit.dylib' ! -path '*/index/*' ! -path '*/plugins/*' | head -n 1 || true)
fi

if [[ -z "${DYLIB}" || ! -f "${DYLIB}" ]]; then
  echo "build-mabel-storekit: libMabelStoreKit.dylib not produced." >&2
  echo "build-mabel-storekit: expected under $BIN or $PKG/.build" >&2
  echo "build-mabel-storekit: rerun: xcrun swift build -c release --arch arm64 --product MabelStoreKit --package-path native/MabelStoreKit" >&2
  exit 1
fi

cp "$DYLIB" "$OUT/libMabelStoreKit.dylib"
# Fail if install name cannot be rewritten — a leftover absolute
# .build path is why a "successful" compile still fails at app launch.
xcrun install_name_tool -id "@rpath/libMabelStoreKit.dylib" "$OUT/libMabelStoreKit.dylib"

echo "build-mabel-storekit: staged $OUT/libMabelStoreKit.dylib" >&2
echo "build-mabel-storekit: install name $(xcrun otool -D "$OUT/libMabelStoreKit.dylib" | tail -n 1)" >&2
