#!/usr/bin/env bash
# Compile native/MabelASR (FluidAudio Parakeet + WhisperKit) into
# src-tauri/native-asr/ for in-process linking. macOS / Xcode only.
#
# Exact Apple Silicon / Xcode 16 / Swift 6 command this script runs:
#
#   MACOSX_DEPLOYMENT_TARGET=14.0 \
#   xcrun swift build -c release --arch arm64 --product MabelASR \
#     --package-path native/MabelASR
#
# Linux and MABEL_SKIP_NATIVE_ASR=1 are no-ops so cargo test still works
# off a Mac. The MAS flavor needs this dylib (signed, library validation ON).
set -euo pipefail

ROOT=$(cd "$(dirname "$0")/.." && pwd)
OUT="$ROOT/src-tauri/native-asr"
PKG="$ROOT/native/MabelASR"

if [[ "${MABEL_SKIP_NATIVE_ASR:-}" == "1" ]]; then
  echo "build-mabel-asr: skipped (MABEL_SKIP_NATIVE_ASR=1)" >&2
  exit 0
fi

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "build-mabel-asr: not macOS; skipping Swift compile" >&2
  exit 0
fi

if ! command -v xcrun >/dev/null 2>&1; then
  echo "build-mabel-asr: xcrun not found. Install Xcode 16+ from the App Store." >&2
  exit 1
fi

XCODE_PATH="$(xcode-select -p 2>/dev/null || true)"
if [[ -z "${XCODE_PATH}" || "${XCODE_PATH}" == *"CommandLineTools"* ]]; then
  echo "build-mabel-asr: xcode-select points at Command Line Tools, not Xcode.app." >&2
  echo "build-mabel-asr: sudo xcode-select -s /Applications/Xcode.app/Contents/Developer" >&2
  exit 1
fi

if ! xcrun --find swift >/dev/null 2>&1; then
  echo "build-mabel-asr: swift not found under $(xcode-select -p). Install Xcode 16+ / Swift 6." >&2
  exit 1
fi

ARCH="$(uname -m)"
ARCH_ARGS=()
if [[ "${ARCH}" == "arm64" ]]; then
  ARCH_ARGS+=(--arch arm64)
fi

export MACOSX_DEPLOYMENT_TARGET="${MACOSX_DEPLOYMENT_TARGET:-14.0}"

echo "build-mabel-asr: Xcode $(xcode-select -p)" >&2
echo "build-mabel-asr: $(xcrun swift --version 2>&1 | head -n 1)" >&2
echo "build-mabel-asr: compiling MabelASR (FluidAudio 0.15.6 + WhisperKit 1.1.0, ${ARCH}, macosx${MACOSX_DEPLOYMENT_TARGET})" >&2
mkdir -p "$OUT"
find "$OUT" -mindepth 1 -maxdepth 1 ! -name '.gitkeep' -exec rm -rf {} +

(
  cd "$PKG"
  xcrun swift build -c release --product MabelASR "${ARCH_ARGS[@]}"
)

BIN=$(cd "$PKG" && xcrun swift build -c release --product MabelASR "${ARCH_ARGS[@]}" --show-bin-path)
DYLIB=""
if [[ -f "$BIN/libMabelASR.dylib" ]]; then
  DYLIB="$BIN/libMabelASR.dylib"
else
  DYLIB=$(find "$PKG/.build" -name 'libMabelASR.dylib' ! -path '*/index/*' ! -path '*/plugins/*' | head -n 1 || true)
fi

if [[ -z "${DYLIB}" || ! -f "${DYLIB}" ]]; then
  echo "build-mabel-asr: libMabelASR.dylib not produced." >&2
  echo "build-mabel-asr: rerun: xcrun swift build -c release --arch arm64 --product MabelASR --package-path native/MabelASR" >&2
  exit 1
fi

cp "$DYLIB" "$OUT/libMabelASR.dylib"
xcrun install_name_tool -id "@rpath/libMabelASR.dylib" "$OUT/libMabelASR.dylib"

FW=$(find "$PKG/.build" -type d -name 'NemoTextProcessing.framework' | head -n 1 || true)
if [[ -n "${FW:-}" ]]; then
  cp -R "$FW" "$OUT/NemoTextProcessing.framework"
  echo "build-mabel-asr: staged NemoTextProcessing.framework" >&2
else
  echo "build-mabel-asr: warning: NemoTextProcessing.framework not found in .build" >&2
fi

find "$(dirname "$DYLIB")" -maxdepth 1 -name '*.dylib' ! -name 'libMabelASR.dylib' -exec cp {} "$OUT/" \;

echo "build-mabel-asr: staged $(ls -1 "$OUT" | tr '\n' ' ')" >&2
