#!/usr/bin/env bash
# Mac-only build helper for Mabel Spatial (com.mabel.vision).
# Linux / CI cannot compile visionOS. This script exits 2 there and prints
# the exact destinations CIO/Erick should run on Apple Silicon Xcode.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
PROJECT="$ROOT/MabelSpatial/MabelSpatial.xcodeproj"
SCHEME="MabelSpatial"

if [[ "$(uname -s)" != "Darwin" ]]; then
  cat <<'EOF' >&2
Mabel Spatial cannot be compiled on Linux. Run these on an Apple Silicon Mac
with Xcode-beta + XROS27 (visionOS 27 SDK), from the repo root:

  xcodebuild -project MabelSpatial/MabelSpatial.xcodeproj \
    -scheme MabelSpatial \
    -showdestinations

  # Simulator
  xcodebuild -project MabelSpatial/MabelSpatial.xcodeproj \
    -scheme MabelSpatial \
    -destination 'platform=visionOS Simulator,name=Apple Vision Pro,OS=latest' \
    -configuration Debug \
    build

  # Generic visionOS device (compile-check)
  xcodebuild -project MabelSpatial/MabelSpatial.xcodeproj \
    -scheme MabelSpatial \
    -destination 'generic/platform=visionOS' \
    -configuration Debug \
    build

  # Physical Vision Pro (RealityDevice14,1 / visionOS 27 beta)
  # Pair in Xcode → Devices and Simulators, then substitute DEVICE_UDID:
  xcodebuild -project MabelSpatial/MabelSpatial.xcodeproj \
    -scheme MabelSpatial \
    -destination 'platform=visionOS,id=DEVICE_UDID' \
    -allowProvisioningUpdates \
    -configuration Debug \
    build

See MabelSpatial/README.md.
EOF
  exit 2
fi

DEST="${1:-generic/platform=visionOS}"
echo "xcodebuild -project $PROJECT -scheme $SCHEME -destination '$DEST' -configuration Debug build"
xcodebuild -project "$PROJECT" \
  -scheme "$SCHEME" \
  -destination "$DEST" \
  -configuration Debug \
  build
