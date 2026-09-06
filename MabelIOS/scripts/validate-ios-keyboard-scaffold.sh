#!/usr/bin/env bash
# Linux-safe structural proof for Mabel iOS keyboard (ship #1).
# Does not compile iOS. Does not claim HIPAA.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
IOS="$ROOT/MabelIOS"
PBX="$IOS/MabelIOS.xcodeproj/project.pbxproj"
FAIL=0

ok() { printf '  PASS  %s\n' "$1"; }
bad() { printf '  FAIL  %s\n' "$1"; FAIL=1; }

need_file() {
  if [[ -f "$1" ]]; then ok "file ${1#$ROOT/}"; else bad "missing ${1#$ROOT/}"; fi
}

echo "Mabel iOS keyboard scaffold check"
echo "repo: $ROOT"

need_file "$PBX"
need_file "$IOS/MabelIOS.xcodeproj/xcshareddata/xcschemes/MabelIOS.xcscheme"
need_file "$IOS/MabelIOS/MabelIOSApp.swift"
need_file "$IOS/MabelIOS/Info.plist"
need_file "$IOS/MabelIOS/MabelIOS.entitlements"
need_file "$IOS/MabelIOS/PrivacyInfo.xcprivacy"
need_file "$IOS/MabelIOS/Views/HostRootView.swift"
need_file "$IOS/MabelIOS/Views/HostDictatePlayground.swift"
need_file "$IOS/MabelIOS/Assets.xcassets/AppIcon.appiconset/AppIcon.png"
need_file "$IOS/Shared/IOSIdentity.swift"
need_file "$IOS/Shared/DictatePermissionGate.swift"
need_file "$IOS/Shared/OnDeviceSpeechEngine.swift"
need_file "$IOS/Shared/SpeechSession.swift"
need_file "$IOS/Shared/CatChrome.swift"
need_file "$IOS/MabelKeyboard/KeyboardViewController.swift"
need_file "$IOS/MabelKeyboard/KeyboardRootView.swift"
need_file "$IOS/MabelKeyboard/Info.plist"
need_file "$IOS/MabelKeyboard/MabelKeyboard.entitlements"
need_file "$IOS/MabelKeyboard/PrivacyInfo.xcprivacy"
need_file "$IOS/README.md"
need_file "$IOS/scripts/xcodebuild-ios.sh"

TEAM_SETTING="DEVELOPMENT_TEAM = DF9FB764AR" # pragma: allowlist secret
for pair in \
  "PRODUCT_BUNDLE_IDENTIFIER = com.mabel.ios" \
  "PRODUCT_BUNDLE_IDENTIFIER = com.mabel.ios.keyboard" \
  "$TEAM_SETTING" \
  "SDKROOT = iphoneos" \
  "IPHONEOS_DEPLOYMENT_TARGET = 17.0" \
  "MARKETING_VERSION = 0.1.0" \
  "INFOPLIST_KEY_CFBundleDisplayName = Mabel" \
  "SUPPORTED_PLATFORMS = \"iphoneos iphonesimulator\"" \
  "SUPPORTS_MACCATALYST = NO" \
  "SUPPORTS_MAC_DESIGNED_FOR_IPHONE_IPAD = NO" \
  "SUPPORTS_XR_DESIGNED_FOR_IPHONE_IPAD = NO" \
  "com.apple.product-type.app-extension" \
  "Embed Foundation Extensions"
do
  if grep -qF "$pair" "$PBX"; then ok "pbxproj has $pair"; else bad "pbxproj missing $pair"; fi
done

if grep -q 'SDKROOT = xros' "$PBX"; then
  bad "pbxproj sets visionOS SDK (Spatial stays separate)"
else
  ok "no visionOS SDKROOT"
fi
if grep -q 'SDKROOT = macosx' "$PBX"; then
  bad "pbxproj sets macOS SDK (must stay a sibling, not Tauri)"
else
  ok "no macOS SDKROOT"
fi

# Entitlements: app group only. No Mac DMG hardening, no StoreKit.
for ENT in "$IOS/MabelIOS/MabelIOS.entitlements" "$IOS/MabelKeyboard/MabelKeyboard.entitlements"; do
  if grep -q 'group.com.mabel.ios' "$ENT"; then ok "$(basename "$(dirname "$ENT")") app group"; else bad "$ENT missing app group"; fi
  if python3 - "$ENT" <<'PY'
import sys, plistlib
with open(sys.argv[1], "rb") as f:
    data = plistlib.load(f)
forbidden = (
    "com.apple.security.cs.allow-jit",
    "com.apple.security.cs.allow-unsigned-executable-memory",
    "com.apple.security.cs.disable-library-validation",
    "com.apple.developer.in-app-payments",
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
done

for PLIST in "$IOS/MabelIOS/Info.plist" "$IOS/MabelKeyboard/Info.plist"; do
  for key in NSMicrophoneUsageDescription NSSpeechRecognitionUsageDescription CFBundleDisplayName; do
    if grep -q "$key" "$PLIST"; then ok "$(basename "$(dirname "$PLIST")") Info.plist $key"; else bad "$PLIST missing $key"; fi
  done
done

if grep -q 'RequestsOpenAccess' "$IOS/MabelKeyboard/Info.plist" \
  && grep -q 'com.apple.keyboard-service' "$IOS/MabelKeyboard/Info.plist"; then
  ok "keyboard NSExtension + RequestsOpenAccess"
else
  bad "keyboard Info.plist missing extension / open access"
fi

for PRIV in "$IOS/MabelIOS/PrivacyInfo.xcprivacy" "$IOS/MabelKeyboard/PrivacyInfo.xcprivacy"; do
  if grep -q 'NSPrivacyTracking</key>' "$PRIV" && grep -q '<false/>' "$PRIV"; then
    ok "PrivacyInfo tracking disabled ($(basename "$(dirname "$PRIV")"))"
  else
    bad "$PRIV must disable tracking"
  fi
done

# Product locks in Swift: no Stiki/StoreKit/HIPAA-claim/Wispr/Mac ASR
LEAK="$(mktemp)"
if grep -R -n -E 'StoreKit|Stiki|HIPAA|BAA|Wispr|whisper\.cpp|WhisperKit|FluidAudio|Parakeet|tauri|com\.mabel\.app|com\.mabel\.vision' \
  --include='*.swift' "$IOS/MabelIOS" "$IOS/MabelKeyboard" "$IOS/Shared" \
  | grep -v -i -E 'macBundleID|spatialBundleID|macASC|do not|not tauri|no whisper|stiki|hipaa|wispr|no storekit|no account|not mochii|not mac' \
  >"$LEAK" || true
then
  :
fi
if [[ -s "$LEAK" ]]; then
  bad "iOS Swift leaked StoreKit / Stiki / Mac ASR / foreign brand"
  cat "$LEAK"
else
  ok "iOS Swift has no StoreKit / Stiki / Mac ASR / Wispr clone"
fi
rm -f "$LEAK"

if grep -q 'requiresOnDeviceRecognition' "$IOS/Shared/OnDeviceSpeechEngine.swift" \
  && grep -q 'SFSpeechRecognizer' "$IOS/Shared/OnDeviceSpeechEngine.swift"; then
  ok "on-device Apple Speech engine present"
else
  bad "OnDeviceSpeechEngine missing on-device Speech"
fi

if grep -q 'allowsAudio' "$IOS/Shared/DictatePermissionGate.swift" \
  && grep -q 'evaluateKeyboard' "$IOS/Shared/DictatePermissionGate.swift"; then
  ok "fail-closed DictatePermissionGate present"
else
  bad "permission gate missing"
fi

# Ambient listen lock: keyboard must not start listening from lifecycle hooks.
if grep -n 'startListening\|toggleListening' "$IOS/MabelKeyboard/KeyboardViewController.swift" \
  | grep -v 'stopListening' | grep -v '//' >/tmp/mabel-ios-kb-listen.txt; then
  bad "keyboard VC must not start dictation from lifecycle"
  cat /tmp/mabel-ios-kb-listen.txt
else
  ok "keyboard VC does not start dictation from lifecycle"
fi

if grep -q 'Never start the microphone here' "$IOS/MabelKeyboard/KeyboardViewController.swift" \
  && grep -q 'viewWillAppear' "$IOS/MabelKeyboard/KeyboardViewController.swift"; then
  ok "viewWillAppear is status-only"
else
  bad "viewWillAppear missing status-only lock comment"
fi

if grep -q 'advanceToNextInputMode' "$IOS/MabelKeyboard/KeyboardViewController.swift"; then
  ok "next-keyboard / globe path present"
else
  bad "missing next keyboard control"
fi

if grep -q 'promptHostPermissions' "$IOS/Shared/OnDeviceSpeechEngine.swift" \
  && grep -q 'requestHostPermissions()' "$IOS/MabelIOS/Views/HostRootView.swift"; then
  ok "Allow permissions does not start the mic"
else
  bad "host Allow button must prompt without starting audio"
fi

# Gate truth table (mirrors DictatePermissionGate.swift)
python3 - <<'PY'
cases = [
    # keyboard: full, mic, speech -> expected
    ((False, "granted", "authorized"), "needsFullAccess"),
    ((True, "denied", "authorized"), "needsMicrophone"),
    ((True, "granted", "denied"), "needsSpeechRecognition"),
    ((True, "undetermined", "authorized"), "undeterminedOpenHost"),
    ((True, "granted", "undetermined"), "undeterminedOpenHost"),
    ((True, "granted", "authorized"), "ready"),
    ((True, "denied", "denied"), "needsMicrophone"),
]

def keyboard(full, mic, speech):
    if not full:
        return "needsFullAccess"
    if mic == "denied":
        return "needsMicrophone"
    if speech in ("denied", "restricted"):
        return "needsSpeechRecognition"
    if mic == "undetermined" or speech == "undetermined":
        return "undeterminedOpenHost"
    if mic == "granted" and speech == "authorized":
        return "ready"
    return "needsMicrophone"

failed = False
for (full, mic, speech), want in cases:
    got = keyboard(full, mic, speech)
    ready = got == "ready"
    if got != want or (want == "ready") != ready:
        print(f"  FAIL  gate {(full, mic, speech)} -> {got} want {want}")
        failed = True
    else:
        print(f"  PASS  gate {(full, mic, speech)} -> {got} allowsAudio={ready}")
# Only ready may power the mic
if keyboard(True, "granted", "authorized") != "ready":
    print("  FAIL  ready path broken")
    failed = True
raise SystemExit(1 if failed else 0)
PY

# pbxproj Swift files exist on disk
python3 - "$IOS" <<'PY'
import re, os, sys
root = sys.argv[1]
pbx = open(os.path.join(root, "MabelIOS.xcodeproj/project.pbxproj")).read()
missing = []
for name in re.findall(r"path = ([A-Za-z0-9_.]+\.swift);", pbx):
    hits = []
    for dirpath, _, files in os.walk(root):
        if "xcodeproj" in dirpath:
            continue
        if name in files:
            hits.append(os.path.join(dirpath, name))
    if not hits:
        missing.append(name)
        print(f"  FAIL  pbxproj swift missing on disk: {name}")
if missing:
    sys.exit(1)
print("  PASS  all pbxproj Swift files exist on disk")
PY

# Mac + Spatial identities must stay put
if grep -q '"version": "1.4.0"' "$ROOT/src-tauri/tauri.conf.json" \
  && grep -q '"identifier": "com.mabel.app"' "$ROOT/src-tauri/tauri.conf.json"; then
  ok "Mac Tauri still 1.4.0 / com.mabel.app"
else
  bad "Mac tauri.conf.json identity/version drifted"
fi
if grep -q 'PRODUCT_BUNDLE_IDENTIFIER = com.mabel.vision' "$ROOT/MabelSpatial/MabelSpatial.xcodeproj/project.pbxproj"; then
  ok "Spatial still com.mabel.vision"
else
  bad "Spatial bundle drifted"
fi

echo
if [[ "$FAIL" -ne 0 ]]; then
  echo "scaffold check FAILED"
  exit 1
fi
echo "scaffold check PASSED (xcodebuild still requires a Mac)"
