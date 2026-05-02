#!/bin/sh
# Builds a self-contained `llama-server` binary for aarch64-apple-darwin and
# drops it into src-tauri/binaries/llama-server-aarch64-apple-darwin so Tauri's
# bundler can ship it as a sidecar inside Mabel Scribe.app.
#
# Why static? llama.cpp's prebuilt macOS arm64 archive ships llama-server with
# dependencies on libllama, libggml, libmtmd, etc. via @rpath. Those dylibs
# share filenames (libggml.0.dylib) with whisper.cpp's bundled dylibs but
# different ABI versions, which collides inside Contents/Frameworks/. A static
# build sidesteps this entirely: one binary, no shared lib lookups.
#
# Why latest? The medical-polish stage 2 relies on the OpenAI chat-completions
# endpoint accepting the `grammar` field and on `--cache-reuse` working as
# advertised. Both are stable in recent llama.cpp builds. Pinning to the
# latest release at build time captures a reproducible version in the
# .llama-server-version file alongside the binary.
#
# Run order:
#   1. Run this script after cloning the repo (one-time, ~5 minutes on M-series).
#   2. Then `npm run tauri build` will pick up the binary as a sidecar.
#
# Re-run when you want to bump llama.cpp.

set -e

SCRIPT_DIR=$(cd "$(dirname "$0")" && pwd)
ROOT=$(cd "$SCRIPT_DIR/.." && pwd)
DEST_DIR="$ROOT/src-tauri/binaries"
DEST_BIN="$DEST_DIR/llama-server-aarch64-apple-darwin"
VERSION_FILE="$DEST_DIR/.llama-server-version"

# Resolve the current latest release tag at build time. Captured into the
# version file so the bundled binary's provenance is recorded in the repo.
TAG=$(curl -fsSL "https://api.github.com/repos/ggml-org/llama.cpp/releases/latest" \
  | sed -nE 's/.*"tag_name": *"([^"]+)".*/\1/p' | head -n 1)

if [ -z "$TAG" ]; then
  echo "Failed to resolve latest llama.cpp release tag." >&2
  exit 1
fi

echo "Building llama-server from llama.cpp@${TAG}"

WORK=$(mktemp -d -t scribe-llama-build)
trap "rm -rf '$WORK'" EXIT

cd "$WORK"
git clone --depth 1 --branch "$TAG" https://github.com/ggml-org/llama.cpp.git src
cd src

# Static build: no shared libs, no rpath dependencies. The resulting binary
# only links against macOS system frameworks (Metal, Foundation, Security,
# libSystem, libc++) which are guaranteed-present and don't conflict with
# anything Tauri bundles.
#
# Disable example/test/tools targets we don't use to cut compile time and
# binary size.
cmake -B build \
  -DCMAKE_BUILD_TYPE=Release \
  -DBUILD_SHARED_LIBS=OFF \
  -DGGML_METAL=ON \
  -DGGML_METAL_EMBED_LIBRARY=ON \
  -DLLAMA_BUILD_SERVER=ON \
  -DLLAMA_BUILD_EXAMPLES=OFF \
  -DLLAMA_BUILD_TESTS=OFF \
  -DLLAMA_BUILD_TOOLS=ON \
  -DLLAMA_CURL=OFF \
  -DLLAMA_SERVER_SSL=OFF \
  -DCMAKE_DISABLE_FIND_PACKAGE_OpenSSL=ON

cmake --build build --config Release -j --target llama-server

mkdir -p "$DEST_DIR"
cp build/bin/llama-server "$DEST_BIN"
strip -S "$DEST_BIN" 2>/dev/null || true
chmod +x "$DEST_BIN"

# Record the resolved tag so we know what's in the repo.
echo "$TAG" > "$VERSION_FILE"

# Sanity check: every dependency must come from /System or /usr/lib. Anything
# else (homebrew openssl, @rpath, dev-machine paths) won't exist on a
# clinician's Mac and the bundled binary will fail at runtime. Fail loud.
BAD_DEPS=$(otool -L "$DEST_BIN" \
  | awk 'NR>1 {print $1}' \
  | grep -vE '^[[:space:]]*(/System/|/usr/lib/)' \
  || true)
if [ -n "$BAD_DEPS" ]; then
  echo "ERROR: bundled llama-server has non-system dependencies; will fail on user machines:" >&2
  echo "$BAD_DEPS" >&2
  exit 1
fi

echo ""
echo "Built llama-server (${TAG}):"
ls -lh "$DEST_BIN"
echo "Version recorded at $VERSION_FILE"
