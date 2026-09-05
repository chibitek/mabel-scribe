#!/usr/bin/env bash
# Experimental MAS / TestFlight flavor. Does NOT replace scripts/release-macos.sh.
#
# 1.1.7 and 1.2.0 whisper-cpp sidecar builds are NOT Mac App Store ready.
# This script will not run unless MABEL_MAS_EXPERIMENT=1.
set -euo pipefail

ROOT=$(cd "$(dirname "$0")/.." && pwd)
cd "$ROOT"

echo "build-mas: MAS / TestFlight flavor (App Sandbox + entitlements.mas.plist)" >&2
echo "build-mas: 1.1.7 / 1.2.0 local whisper sidecar is NOT MAS-ready." >&2
echo "build-mas: disable-library-validation is stripped on this target; the sidecar will fail." >&2
echo "build-mas: ship MAS only after Phase B (WhisperKit/Parakeet) or a static-link whisper." >&2
echo "build-mas: the GitHub notarized DMG path is unchanged (scripts/release-macos.sh)." >&2

if [[ "${MABEL_MAS_EXPERIMENT:-}" != "1" ]]; then
  echo "build-mas: refusing to build. Set MABEL_MAS_EXPERIMENT=1 to compile the draft flavor." >&2
  exit 1
fi

exec npm run tauri -- build --config src-tauri/tauri.mas.conf.json --bundles app
