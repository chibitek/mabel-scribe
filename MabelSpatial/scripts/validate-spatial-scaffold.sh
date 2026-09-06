#!/usr/bin/env bash
# Linux-safe structural proof for Mabel Spatial. Does not compile visionOS.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
SP="$ROOT/MabelSpatial"
PBX="$SP/MabelSpatial.xcodeproj/project.pbxproj"
FAIL=0

ok() { printf '  PASS  %s\n' "$1"; }
bad() { printf '  FAIL  %s\n' "$1"; FAIL=1; }

need_file() {
  if [[ -f "$1" ]]; then ok "file ${1#$ROOT/}"; else bad "missing ${1#$ROOT/}"; fi
}

echo "Mabel Spatial scaffold check"
echo "repo: $ROOT"

need_file "$PBX"
need_file "$SP/MabelSpatial.xcodeproj/xcshareddata/xcschemes/MabelSpatial.xcscheme"
need_file "$SP/MabelSpatial/MabelSpatialApp.swift"
need_file "$SP/MabelSpatial/Info.plist"
need_file "$SP/MabelSpatial/MabelSpatial.entitlements"
need_file "$SP/MabelSpatial/PrivacyInfo.xcprivacy"
need_file "$SP/MabelSpatial/Models/SpatialConstants.swift"
need_file "$SP/MabelSpatial/Models/SpatialSession.swift"
need_file "$SP/MabelSpatial/Speech/OnDeviceSpeechEngine.swift"
need_file "$SP/MabelSpatial/Reality/ListeningOrbEntity.swift"
need_file "$SP/MabelSpatial/Reality/ImmersiveSpaceView.swift"
need_file "$SP/MabelSpatial/Views/WindowRootView.swift"
need_file "$SP/MabelSpatial/Views/ListeningOrbView.swift"
need_file "$SP/MabelSpatial/Views/FloatingTranscriptPanel.swift"
need_file "$SP/MabelSpatial/Assets.xcassets/AppIcon.solidimagestack/Contents.json"
need_file "$SP/README.md"

# Identity
for pair in \
  "PRODUCT_BUNDLE_IDENTIFIER = com.mabel.vision" \
  "DEVELOPMENT_TEAM = DF9FB764AR" \ # pragma: allowlist secret
  "SDKROOT = xros" \
  "TARGETED_DEVICE_FAMILY = 7" \
  "XROS_DEPLOYMENT_TARGET = 26.0" \
  "MARKETING_VERSION = 0.1.0" \
  "INFOPLIST_KEY_CFBundleDisplayName = \"Mabel Spatial\"" \
  "SUPPORTED_PLATFORMS = \"xros xrsimulator\"" \
  "SUPPORTS_XR_DESIGNED_FOR_IPHONE_IPAD = NO"
do
  if grep -qF "$pair" "$PBX"; then ok "pbxproj has $pair"; else bad "pbxproj missing $pair"; fi
done

if grep -q 'SDKROOT = iphoneos' "$PBX"; then
  bad "pbxproj sets iPhone SDK (Designed-for-iPhone risk)"
else
  ok "no iPhone SDKROOT"
fi
if grep -q 'SDKROOT = macosx' "$PBX"; then
  bad "pbxproj sets macOS SDK (must stay a sibling, not Tauri)"
else
  ok "no macOS SDKROOT"
fi

# Entitlements: mic + sandbox; no Mac DMG hardening
ENT="$SP/MabelSpatial/MabelSpatial.entitlements"
for key in com.apple.security.app-sandbox com.apple.security.device.audio-input; do
  if grep -q "$key" "$ENT"; then ok "entitlement $key"; else bad "entitlement missing $key"; fi
done
if python3 - "$ENT" <<'PY'
import sys, plistlib
with open(sys.argv[1], "rb") as f:
    data = plistlib.load(f)
forbidden = (
    "com.apple.security.cs.allow-jit",
    "com.apple.security.cs.allow-unsigned-executable-memory",
    "com.apple.security.cs.disable-library-validation",
)
failed = False
for key in forbidden:
    if key in data:
        print(f"  FAIL  entitlement must not include {key}")
        failed = True
    else:
        print(f"  PASS  no {key}")
sys.exit(1 if failed else 0)
PY
then
  :
else
  FAIL=1
fi

# Privacy strings
PLIST="$SP/MabelSpatial/Info.plist"
for key in NSMicrophoneUsageDescription NSSpeechRecognitionUsageDescription CFBundleDisplayName; do
  if grep -q "$key" "$PLIST"; then ok "Info.plist $key"; else bad "Info.plist missing $key"; fi
done
if grep -q 'NSPrivacyTracking</key>' "$SP/MabelSpatial/PrivacyInfo.xcprivacy" \
  && grep -q '<false/>' "$SP/MabelSpatial/PrivacyInfo.xcprivacy"; then
  ok "PrivacyInfo tracking disabled"
else
  bad "PrivacyInfo must disable tracking"
fi

# Apple Speech only — no Mac ASR / Tauri in the spatial tree
if grep -R -n -E 'whisper\.cpp|WhisperKit|FluidAudio|Parakeet|tauri|com\.mabel\.app' \
  --include='*.swift' "$SP/MabelSpatial" | grep -v 'macBundleID\|macASC\|do not\|not a Tauri\|No whisper' >/tmp/mabel-spatial-leak.txt || true
then
  :
fi
if [[ -s /tmp/mabel-spatial-leak.txt ]]; then
  bad "spatial Swift leaked Mac ASR / Tauri / com.mabel.app"
  cat /tmp/mabel-spatial-leak.txt
else
  ok "spatial Swift does not import Mac ASR / Tauri"
fi

if grep -q 'SpeechAnalyzer\|SpeechTranscriber\|requiresOnDeviceRecognition' \
  "$SP/MabelSpatial/Speech/OnDeviceSpeechEngine.swift"; then
  ok "Apple Speech on-device engine present"
else
  bad "OnDeviceSpeechEngine missing Apple Speech types"
fi

if grep -q 'ListenOrbMarker\|HoverEffectComponent\|SpatialTapGesture' \
  "$SP/MabelSpatial/Reality/ImmersiveSpaceView.swift" \
  "$SP/MabelSpatial/Reality/ListeningOrbEntity.swift"; then
  ok "eyes+hands orb gesture wiring present"
else
  bad "missing gaze/pinch orb wiring"
fi

# Resolve listed Swift paths from pbxproj
python3 - "$SP" <<'PY'
import re, os, sys
root = sys.argv[1]
pbx = open(os.path.join(root, "MabelSpatial.xcodeproj/project.pbxproj")).read()
missing = []
for name in re.findall(r"path = ([A-Za-z0-9_.]+\.swift);", pbx):
    hits = []
    for dirpath, _, files in os.walk(os.path.join(root, "MabelSpatial")):
        if name in files:
            hits.append(os.path.join(dirpath, name))
    if not hits:
        missing.append(name)
        print(f"  FAIL  pbxproj swift missing on disk: {name}")
if missing:
    sys.exit(1)
print("  PASS  all pbxproj Swift files exist on disk")
PY

# Mac SoT must still be 1.4.0 / com.mabel.app
if grep -q '"version": "1.4.0"' "$ROOT/src-tauri/tauri.conf.json" \
  && grep -q '"identifier": "com.mabel.app"' "$ROOT/src-tauri/tauri.conf.json"; then
  ok "Mac Tauri still 1.4.0 / com.mabel.app"
else
  bad "Mac tauri.conf.json identity/version drifted"
fi

echo
if [[ "$FAIL" -ne 0 ]]; then
  echo "scaffold check FAILED"
  exit 1
fi
echo "scaffold check PASSED (xcodebuild still requires a Mac)"
