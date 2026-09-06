#!/usr/bin/env bash
# Mac-only build helper for Mabel iOS (com.mabel.ios + keyboard).
# Linux / CI cannot compile iOS. This script exits 2 there and prints
# the destinations CIO/Erick should run on Apple Silicon Xcode.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
PROJECT="$ROOT/MabelIOS/MabelIOS.xcodeproj"
SCHEME="MabelIOS"

if [[ "$(uname -s)" != "Darwin" ]]; then
  cat <<'EOF' >&2
Mabel iOS cannot be compiled on Linux. Run these on an Apple Silicon Mac
with Xcode 16+ (iOS 17 SDK or newer), from the repo root:

  xcodebuild -project MabelIOS/MabelIOS.xcodeproj \
    -scheme MabelIOS \
    -showdestinations

  # Simulator
  xcodebuild -project MabelIOS/MabelIOS.xcodeproj \
    -scheme MabelIOS \
    -destination 'platform=iOS Simulator,name=iPhone 16,OS=latest' \
    -configuration Debug \
    build

  # Generic iOS device (compile-check)
  xcodebuild -project MabelIOS/MabelIOS.xcodeproj \
    -scheme MabelIOS \
    -destination 'generic/platform=iOS' \
    -configuration Debug \
    build

  # Physical iPhone. Pair in Xcode → Devices and Simulators, then
  # substitute DEVICE_UDID:
  xcodebuild -project MabelIOS/MabelIOS.xcodeproj \
    -scheme MabelIOS \
    -destination 'platform=iOS,id=DEVICE_UDID' \
    -allowProvisioningUpdates \
    -configuration Debug \
    build

See MabelIOS/README.md.
EOF
  exit 2
fi

DEST="${1:-generic/platform=iOS}"
echo "xcodebuild -project $PROJECT -scheme $SCHEME -destination '$DEST' -configuration Debug build"
xcodebuild -project "$PROJECT" \
  -scheme "$SCHEME" \
  -destination "$DEST" \
  -configuration Debug \
  build
