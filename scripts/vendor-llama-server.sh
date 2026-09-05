#!/usr/bin/env bash
# Download the official llama.cpp macOS-arm64 release and stage llama-server
# plus its dylibs into src-tauri/llama-runtime/.
#
# Official binaries already have an @loader_path rpath, so they load sibling
# dylibs from this folder. That keeps llama.cpp's libggml* out of
# Contents/Frameworks/ (where Whisper's older ggml dylibs live).
#
# On Darwin we also re-apply @loader_path as belt-and-suspenders, the same
# class of rpath fix as the v1.1.0 whisper-cpp Frameworks change.
set -euo pipefail

LLAMA_TAG="${MABEL_LLAMA_TAG:-b10809}"
TARBALL="llama-${LLAMA_TAG}-bin-macos-arm64.tar.gz"
URL="https://github.com/ggml-org/llama.cpp/releases/download/${LLAMA_TAG}/${TARBALL}"
SHA256="${MABEL_LLAMA_SHA256:-7d692df9e1e386e62f1c12b843903218041e6cd74c9415aa39a7ed3176f9eaa2}"

SCRIPT_DIR=$(cd "$(dirname "$0")" && pwd)
ROOT=$(cd "$SCRIPT_DIR/.." && pwd)
DEST="$ROOT/src-tauri/llama-runtime"
STAMP="$DEST/.mabel-llama-version"

if [[ -x "$DEST/llama-server" && -f "$STAMP" && "$(cat "$STAMP")" == "$LLAMA_TAG" ]]; then
  touch "$DEST/.gitkeep"
  echo "vendor-llama-server: $DEST already has $LLAMA_TAG"
  exit 0
fi

WORKDIR=$(mktemp -d)
cleanup() { rm -rf "$WORKDIR"; }
trap cleanup EXIT

echo "vendor-llama-server: downloading $URL"
curl -fL --retry 3 --retry-delay 2 -o "$WORKDIR/$TARBALL" "$URL"

actual=$(sha256sum "$WORKDIR/$TARBALL" | awk '{print $1}')
if [[ "$actual" != "$SHA256" ]]; then
  echo "vendor-llama-server: sha256 mismatch (got $actual, want $SHA256)" >&2
  exit 1
fi

tar -xzf "$WORKDIR/$TARBALL" -C "$WORKDIR"
SRC=$(find "$WORKDIR" -maxdepth 2 -type f -name llama-server -print -quit)
if [[ -z "$SRC" ]]; then
  echo "vendor-llama-server: llama-server missing from tarball" >&2
  exit 1
fi
SRC_DIR=$(dirname "$SRC")

rm -rf "$DEST"
mkdir -p "$DEST"
# Keep the tracked placeholder so a fresh clone still has the directory.
touch "$DEST/.gitkeep"
# Thin llama-server wrapper plus every dylib/symlink it needs. Skip the extra
# CLI tools (llama-cli, bench, …) so the bundle stays ~26 MB, not the full zip.
cp -a "$SRC_DIR/llama-server" "$DEST/"
# Prefer copying the Metal tuner if present; it is tiny and used at runtime.
if [[ -f "$SRC_DIR/ggml-metal-tuning" ]]; then
  cp -a "$SRC_DIR/ggml-metal-tuning" "$DEST/"
fi
# -a preserves the versioned dylib + symlink layout the official rpaths expect.
find "$SRC_DIR" -maxdepth 1 \( -name '*.dylib' -o -name '*.dylib.*' \) -exec cp -a {} "$DEST/" \;

if [[ "$(uname -s)" == "Darwin" ]]; then
  if command -v install_name_tool >/dev/null 2>&1; then
    install_name_tool -add_rpath "@loader_path" "$DEST/llama-server" 2>/dev/null || true
    echo "vendor-llama-server: ensured @loader_path rpath on llama-server"
  fi
fi

chmod +x "$DEST/llama-server"
echo "$LLAMA_TAG" > "$STAMP"
echo "vendor-llama-server: staged $DEST (tag $LLAMA_TAG)"
