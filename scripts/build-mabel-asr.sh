#!/usr/bin/env bash
# Compile native/MabelASR (FluidAudio Parakeet + WhisperKit) into
# src-tauri/native-asr/ for in-process linking. macOS / Xcode only.
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

if ! command -v swift >/dev/null 2>&1; then
  echo "build-mabel-asr: swift not found. Install Xcode 16+ / Swift 6." >&2
  exit 1
fi

echo "build-mabel-asr: compiling MabelASR (FluidAudio 0.15.6 + WhisperKit 1.1.0)" >&2
mkdir -p "$OUT"
# Keep the gitkeep; replace previous Mach-O products.
find "$OUT" -mindepth 1 -maxdepth 1 ! -name '.gitkeep' -exec rm -rf {} +

(
  cd "$PKG"
  swift build -c release --product MabelASR
)

BIN=$(cd "$PKG" && swift build -c release --show-bin-path)
if [[ ! -f "$BIN/libMabelASR.dylib" ]]; then
  echo "build-mabel-asr: libMabelASR.dylib not at $BIN" >&2
  exit 1
fi

cp "$BIN/libMabelASR.dylib" "$OUT/libMabelASR.dylib"
install_name_tool -id "@rpath/libMabelASR.dylib" "$OUT/libMabelASR.dylib" || true

# FluidAudio 0.12+ ships NemoTextProcessing as an xcframework. Bundle the
# macOS slice so library validation can stay ON (same-team re-sign at Tauri pack).
FW=$(find "$PKG/.build" -type d -name 'NemoTextProcessing.framework' | head -n 1 || true)
if [[ -n "${FW:-}" ]]; then
  cp -R "$FW" "$OUT/NemoTextProcessing.framework"
  echo "build-mabel-asr: staged NemoTextProcessing.framework" >&2
else
  echo "build-mabel-asr: warning: NemoTextProcessing.framework not found in .build" >&2
fi

# Copy any extra dylibs the linker produced next to the product.
find "$BIN" -maxdepth 1 -name '*.dylib' ! -name 'libMabelASR.dylib' -exec cp {} "$OUT/" \;

echo "build-mabel-asr: staged $(ls -1 "$OUT" | tr '\n' ' ')" >&2
