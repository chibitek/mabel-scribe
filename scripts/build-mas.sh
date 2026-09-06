#!/usr/bin/env bash
# Experimental MAS / TestFlight flavor. Does NOT replace scripts/release-macos.sh.
#
# Phase B default engine (FluidAudio Parakeet, in-process CoreML) does not
# require disable-library-validation or allow-unsigned-executable-memory.
# This flavor still is NOT claimed MAS-ready: WebKit JIT, llama-server,
# sandboxed paste, and a real Apple Distribution + provisioning run are
# unfinished. Do not upload to App Store Connect.
set -euo pipefail

ROOT=$(cd "$(dirname "$0")/.." && pwd)
cd "$ROOT"

echo "build-mas: MAS / TestFlight flavor (App Sandbox + entitlements.mas.plist)" >&2
echo "build-mas: default local engine is in-process Parakeet (CoreML)." >&2
echo "build-mas: whisper-cpp sidecar + ggml dylibs are excluded from this flavor." >&2
echo "build-mas: StoreKit 2 Pro IAP is wired; ASC products + App ID IAP capability are still CIO follow-ups." >&2
echo "build-mas: NOT MAS-ready — do not upload. See docs/mas-and-testflight.md and docs/app-store-iap.md." >&2
echo "build-mas: the GitHub notarized DMG path is unchanged (scripts/release-macos.sh)." >&2

if [[ "${MABEL_MAS_EXPERIMENT:-}" != "1" ]]; then
  echo "build-mas: refusing to build. Set MABEL_MAS_EXPERIMENT=1 to compile the draft flavor." >&2
  exit 1
fi

exec npm run tauri -- build --config src-tauri/tauri.mas.conf.json --bundles app -- --no-default-features
