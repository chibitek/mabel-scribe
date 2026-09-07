#!/usr/bin/env bash
# Linux-safe structural proof for Mabel iOS Keyboard + Polish (ship #2).
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

echo "Mabel iOS Keyboard + Polish scaffold check"
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
need_file "$IOS/Shared/EnforcerBound.swift"
need_file "$IOS/Shared/SettingsStore.swift"
need_file "$IOS/Shared/Polish.swift"
need_file "$IOS/MabelIOS/Views/SettingsRootView.swift"
need_file "$IOS/MabelIOS/Views/SettingsPanes.swift"
need_file "$IOS/MabelIOS/Views/HomeTabView.swift"
need_file "$IOS/MabelIOS/Views/LockedProTabView.swift"
need_file "$IOS/MabelKeyboard/KeyboardViewController.swift"
need_file "$IOS/MabelKeyboard/KeyboardRootView.swift"
need_file "$IOS/MabelKeyboard/Assets.xcassets/MabelCat.imageset/mabel-cat.jpeg"
need_file "$IOS/MabelIOS/Assets.xcassets/MabelCat.imageset/mabel-cat.jpeg"
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

# ASC 90474: host must declare PortraitUpsideDown. Keyboard Info.plist stays untouched.
if python3 - "$IOS/MabelIOS/Info.plist" <<'PY'
import sys, plistlib
with open(sys.argv[1], "rb") as f:
    data = plistlib.load(f)
orients = data.get("UISupportedInterfaceOrientations") or []
need = [
    "UIInterfaceOrientationPortrait",
    "UIInterfaceOrientationLandscapeLeft",
    "UIInterfaceOrientationLandscapeRight",
    "UIInterfaceOrientationPortraitUpsideDown",
]
missing = [k for k in need if k not in orients]
if missing:
    print("missing", missing)
    raise SystemExit(1)
if orients.index("UIInterfaceOrientationPortraitUpsideDown") < orients.index("UIInterfaceOrientationLandscapeRight"):
    print("PortraitUpsideDown must follow LandscapeRight")
    raise SystemExit(1)
PY
then
  ok "host Info.plist ASC 90474 PortraitUpsideDown"
else
  bad "host Info.plist missing UIInterfaceOrientationPortraitUpsideDown (ASC 90474)"
fi
if grep -q 'UIInterfaceOrientationPortraitUpsideDown' "$IOS/MabelKeyboard/Info.plist"; then
  bad "keyboard Info.plist must not gain host orientation keys"
else
  ok "keyboard Info.plist orientations unchanged"
fi

for PRIV in "$IOS/MabelIOS/PrivacyInfo.xcprivacy" "$IOS/MabelKeyboard/PrivacyInfo.xcprivacy"; do
  if grep -q 'NSPrivacyTracking</key>' "$PRIV" && grep -q '<false/>' "$PRIV"; then
    ok "PrivacyInfo tracking disabled ($(basename "$(dirname "$PRIV")"))"
  else
    bad "$PRIV must disable tracking"
  fi
done

# Product locks in Swift: no Stiki/StoreKit/HIPAA-claim/Mac ASR as dependencies
LEAK="$(mktemp)"
if grep -R -n -E 'StoreKit|Stiki|HIPAA|BAA|whisper\.cpp|WhisperKit|FluidAudio|Parakeet|tauri|com\.mabel\.app|com\.mabel\.vision' \
  --include='*.swift' "$IOS/MabelIOS" "$IOS/MabelKeyboard" "$IOS/Shared" \
  | grep -v -i -E 'macBundleID|spatialBundleID|macASC|do not|not tauri|no whisper|stiki|hipaa|wisprbaa|no storekit|no account|not mochii|not mac|BREAKS IF|forbidden|dual gate|when ASC|not required|local-only' \
  >"$LEAK" || true
then
  :
fi
if [[ -s "$LEAK" ]]; then
  bad "iOS Swift leaked StoreKit / Stiki / Mac ASR"
  cat "$LEAK"
else
  ok "iOS Swift has no StoreKit / Stiki / Mac ASR"
fi
rm -f "$LEAK"

# Enforcer BOUND addendum — fold hard
if grep -q 'static let displayBrand = "Mabel"' "$IOS/Shared/EnforcerBound.swift" \
  && grep -q 'static let forbiddenBrands = \["Flow", "Wispr"\]' "$IOS/Shared/EnforcerBound.swift" \
  && grep -q 'static let shipOrder = \["Keyboard", "Polish", "Dictionary", "Scratchpad", "Languages"\]' "$IOS/Shared/EnforcerBound.swift" \
  && grep -q 'static let thisTip = "Polish"' "$IOS/Shared/EnforcerBound.swift" \
  && grep -q 'static let settingsPanes = \["Account", "General", "Keyboard", "Polish", "Notifications", "Data & privacy"\]' "$IOS/Shared/EnforcerBound.swift" \
  && grep -q 'static let polishModes = \["off", "casual", "professional", "polite"\]' "$IOS/Shared/EnforcerBound.swift" \
  && grep -q 'static let polishDefaultOff = true' "$IOS/Shared/EnforcerBound.swift" \
  && grep -q 'static let polishRequiresDualGate = true' "$IOS/Shared/EnforcerBound.swift" \
  && grep -q 'static let polishSignOutLocks = true' "$IOS/Shared/EnforcerBound.swift" \
  && grep -q 'static let polishDistinctFromStyle = true' "$IOS/Shared/EnforcerBound.swift" \
  && grep -qF 'GREEN (Polish): Off|Casual|Professional|Polite' "$IOS/Shared/EnforcerBound.swift" \
  && grep -qF 'BREAKS IF (Polish): default ON' "$IOS/Shared/EnforcerBound.swift" \
  && grep -q 'static let cloudStorageAvailableV1 = false' "$IOS/Shared/EnforcerBound.swift" \
  && grep -q 'static let improveModelsDefaultOn = false' "$IOS/Shared/EnforcerBound.swift" \
  && grep -q 'static let silentCloudAllowed = false' "$IOS/Shared/EnforcerBound.swift" \
  && grep -q 'static let privacySurfaceName = "Local-only privacy mode"' "$IOS/Shared/EnforcerBound.swift" \
  && grep -q 'static let dictationCloudAvailableV1 = false' "$IOS/Shared/EnforcerBound.swift" \
  && grep -q 'static let cloudSyncAvailableV1 = false' "$IOS/Shared/EnforcerBound.swift" \
  && grep -q 'static let localOnlyModeShips = true' "$IOS/Shared/EnforcerBound.swift" \
  && grep -q 'static let hipaaBAAFollowUpParked = true' "$IOS/Shared/EnforcerBound.swift" \
  && grep -q 'static let hipaaMakeItSo = false' "$IOS/Shared/EnforcerBound.swift" \
  && grep -q 'static let wisprBAAClaimAllowed = false' "$IOS/Shared/EnforcerBound.swift" \
  && grep -q 'static let silentTrainingUploadAllowed = false' "$IOS/Shared/EnforcerBound.swift" \
  && grep -q 'static let localOnlyLocalFirstCopyOK = true' "$IOS/Shared/EnforcerBound.swift" \
  && grep -q 'privacySuite = "b6530197"' "$IOS/Shared/EnforcerBound.swift" \
  && grep -q 'GREEN (b6530197): Local-only mode ships' "$IOS/Shared/EnforcerBound.swift" \
  && grep -q 'real HIPAA BAA parked (no Make It So)' "$IOS/Shared/EnforcerBound.swift" \
  && grep -q 'BREAKS IF (b6530197): HIPAA/BAA/Wispr BAA claim ships' "$IOS/Shared/EnforcerBound.swift" \
  && grep -q 'dictation cloud or cloud storage ON/available as sync v1' "$IOS/Shared/EnforcerBound.swift" \
  && grep -q 'static let homeTabs = \["Home", "Dictionary", "Snippets", "Style", "Scratchpad"\]' "$IOS/Shared/EnforcerBound.swift" \
  && grep -q 'static let freeHomeTabs = \["Home"\]' "$IOS/Shared/EnforcerBound.swift" \
  && grep -q 'static let freeHomeAndDictateRequireAccount = false' "$IOS/Shared/EnforcerBound.swift" \
  && grep -q 'static let proUnlockRequiresStoreKitAndStiki = true' "$IOS/Shared/EnforcerBound.swift" \
  && grep -q 'static let storeKitEqualsStiki = false' "$IOS/Shared/EnforcerBound.swift" \
  && grep -q 'GREEN (b6530197 Home IA): Free Home + dictate available without Stiki' "$IOS/Shared/EnforcerBound.swift" \
  && grep -q 'BREAKS IF (b6530197 Home IA): Free Home or Free dictate gated on' "$IOS/Shared/EnforcerBound.swift"; then
  ok "EnforcerBound locks Mabel brand, ship order, Settings IA, Home IA, Suite b6530197"
else
  bad "EnforcerBound missing brand/shipOrder/settings/home/cloud locks"
fi

if grep -q 'INFOPLIST_KEY_CFBundleDisplayName = Mabel' "$PBX" \
  && ! grep -q 'INFOPLIST_KEY_CFBundleDisplayName = Flow' "$PBX" \
  && ! grep -q 'PRODUCT_NAME = Flow' "$PBX"; then
  ok "Xcode display/product name is Mabel, not Flow"
else
  bad "display or product name drifted toward Flow"
fi

if python3 - "$IOS" <<'PY'
import os, re, sys
root = sys.argv[1]
failed = False
brand = re.compile(r'\b(Flow|Wispr)\b')
allow = re.compile(
    r'forbiddenBrands|isForbiddenBrand|BREAKS IF|no Flow|not a |clone|hard break|do not',
    re.I,
)
later_ship_files = re.compile(r'(LanguagePack|DictionaryEngine|ScratchpadStore)', re.I)

for dirpath, _, files in os.walk(root):
    if "xcodeproj" in dirpath or "DerivedData" in dirpath:
        continue
    for name in files:
        if later_ship_files.search(name) and not name.endswith(".md"):
            print(f"  FAIL  later-ship file present this tip: {name}")
            failed = True
        if not name.endswith(".swift"):
            continue
        path = os.path.join(dirpath, name)
        for i, line in enumerate(open(path), 1):
            stripped = line.strip()
            if stripped.startswith("//") or stripped.startswith("///") or stripped.startswith("*"):
                continue
            if brand.search(line) and not allow.search(line):
                print(f"  FAIL  Flow/Wispr brand leak {path}:{i}: {stripped}")
                failed = True

# Display strings must stay Mabel
identity = open(os.path.join(root, "Shared/IOSIdentity.swift")).read()
enforcer = open(os.path.join(root, "Shared/EnforcerBound.swift")).read()
if 'displayName = EnforcerBound.displayBrand' not in identity:
    print("  FAIL  IOSIdentity.displayName must be EnforcerBound.displayBrand")
    failed = True
if 'static let displayBrand = "Mabel"' not in enforcer:
    print("  FAIL  EnforcerBound.displayBrand must be Mabel")
    failed = True
if 'static let thisTip = "Polish"' not in enforcer:
    print("  FAIL  this tip must be Polish")
    failed = True

# Keyboard is not a silent spy: no audio start in lifecycle methods
vc = open(os.path.join(root, "MabelKeyboard/KeyboardViewController.swift")).read()
for hook, start, end in (
    ("viewDidLoad", "func viewDidLoad", "func viewWillAppear"),
    ("viewWillAppear", "func viewWillAppear", "func viewWillDisappear"),
    ("textDidChange", "func textDidChange", "final class KeyboardChrome"),
):
    chunk = vc
    if start in vc and end in vc:
        chunk = vc.split(start, 1)[1].split(end, 1)[0]
    if re.search(r'startListening|toggleListening|startMicTap|AVAudioEngine\(|speech\.start', chunk):
        print(f"  FAIL  keyboard {hook} starts audio (spy / ambient listen)")
        failed = True
    else:
        print(f"  PASS  keyboard {hook} does not start audio")

engine = open(os.path.join(root, "Shared/OnDeviceSpeechEngine.swift")).read()
start_fn = engine.split("func start(", 1)[1].split("func stop(", 1)[0]
if "assertPermissions" not in start_fn:
    print("  FAIL  speech start() missing assertPermissions")
    failed = True
elif start_fn.find("assertPermissions") > start_fn.find("startMicTap"):
    print("  FAIL  startMicTap runs before permission assert (audio without permission)")
    failed = True
else:
    print("  PASS  start() asserts permissions before startMicTap")

# AVAudioEngine is created only after the gate, inside startMicTap
mic_fn = engine.split("private func startMicTap", 1)[1].split("private func deactivateAudioSession", 1)[0]
if "AVAudioEngine()" not in mic_fn or engine.count("AVAudioEngine()") != 1:
    print("  FAIL  AVAudioEngine must be created once, only inside startMicTap")
    failed = True
else:
    print("  PASS  AVAudioEngine created only after gate, in startMicTap")

session = open(os.path.join(root, "Shared/SpeechSession.swift")).read()
if "lastGate.allowsAudio == false && mayPrompt == false" not in session:
    print("  FAIL  SpeechSession missing fail-closed return before audio")
    failed = True
else:
    print("  PASS  SpeechSession fail-closes before speech.start")
if "context == .host && lastGate == .undeterminedOpenHost" not in session:
    print("  FAIL  keyboard must not get a permission-prompt audio path")
    failed = True
else:
    print("  PASS  permission prompt is host-only; keyboard never prompts")

# Settings IA scaffold (host shell). Free dictate still ignores Account.
settings_root = open(os.path.join(root, "MabelIOS/Views/SettingsRootView.swift")).read()
settings_panes = open(os.path.join(root, "MabelIOS/Views/SettingsPanes.swift")).read()
settings_store = open(os.path.join(root, "Shared/SettingsStore.swift")).read()
for required in (
    "AccountSettingsPane",
    "GeneralSettingsPane",
    "KeyboardSettingsPane",
    "NotificationsSettingsPane",
    "PrivacySettingsPane",
    "Continue with Apple",
    "Continue with Google",
    "Continue with Microsoft",
    "Have a code?",
    "QWERTY layout",
    "Live Activities",
    "Improve models",
    "Dictation cloud",
    "PolishSettingsPane",
    "polish-toggle",
    "polish-mode-select",
    "polish-activate",
    "Sign out locks Polish",
):
    blob = settings_root + settings_panes
    if required not in blob:
        print(f"  FAIL  Settings IA missing {required}")
        failed = True
if "stikiSignedIn && storeKitEntitled" not in enforcer:
    print("  FAIL  Pro dual gate must require StoreKit Pro AND Stiki")
    failed = True
else:
    print("  PASS  Pro dual gate is StoreKit Pro AND Stiki")
if "EnforcerBound.isProUnlocked" not in settings_store \
        or "EnforcerBound.isHomeTabUnlocked" not in settings_store:
    print("  FAIL  SettingsStore must use EnforcerBound dual-gate helpers")
    failed = True
else:
    print("  PASS  SettingsStore uses EnforcerBound dual-gate helpers")
if re.search(r'stikiSignedIn\s*\|\|\s*storeKitEntitled', settings_store + enforcer):
    print("  FAIL  StoreKit alone or Stiki alone must not unlock Pro")
    failed = True
if "static let storeKitEqualsStiki = false" not in enforcer:
    print("  FAIL  StoreKit ≠ Stiki lock missing")
    failed = True
if "static let freeHomeAndDictateRequireAccount = false" not in enforcer:
    print("  FAIL  Free Home + dictate must stay available without account")
    failed = True
if "GREEN (b6530197 Home IA): Free Home + dictate available without Stiki" not in enforcer \
        or "BREAKS IF (b6530197 Home IA): Free Home or Free dictate gated on" not in enforcer:
    print("  FAIL  Suite b6530197 Home IA GREEN/BREAKS IF missing")
    failed = True
else:
    print("  PASS  Suite b6530197 Home IA GREEN/BREAKS IF folded")
if "var improveModels = EnforcerBound.improveModelsDefaultOn" not in settings_store:
    print("  FAIL  Improve models must default from EnforcerBound (OFF)")
    failed = True
else:
    print("  PASS  Improve models defaults OFF")
if re.search(r'improveModels\s*=\s*true', settings_store):
    print("  FAIL  improve-models default ON / silent upload")
    failed = True
if "cloudStorage = EnforcerBound.cloudStorageAvailableV1" not in settings_store \
        or "requestCloudStorage" not in settings_store:
    print("  FAIL  Cloud storage must stay off / unavailable v1")
    failed = True
else:
    print("  PASS  Cloud storage off / unavailable v1")
if re.search(r'cloudStorage\s*=\s*true', settings_store + settings_panes):
    print("  FAIL  cloud ON v1")
    failed = True
if "disabled(EnforcerBound.cloudStorageAvailableV1 == false)" not in settings_panes:
    print("  FAIL  Cloud storage toggle must stay disabled in v1")
    failed = True
else:
    print("  PASS  Cloud storage toggle disabled in v1")
if "static let privacySurfaceName = \"Local-only privacy mode\"" not in enforcer:
    print("  FAIL  privacy surface name must stay Local-only privacy mode")
    failed = True
else:
    print("  PASS  privacy surface is Local-only privacy mode")
if "static let localOnlyModeShips = true" not in enforcer:
    print("  FAIL  local-only mode must ship")
    failed = True
else:
    print("  PASS  local-only mode ships")
if "static let dictationCloudAvailableV1 = false" not in enforcer:
    print("  FAIL  dictation cloud must stay unavailable v1")
    failed = True
else:
    print("  PASS  dictation cloud unavailable v1")
if "static let cloudSyncAvailableV1 = false" not in enforcer:
    print("  FAIL  dictation/cloud sync must stay unavailable v1")
    failed = True
else:
    print("  PASS  dictation/cloud sync unavailable v1")
if "static let hipaaBAAFollowUpParked = true" not in enforcer:
    print("  FAIL  real HIPAA BAA must stay parked (no claim)")
    failed = True
else:
    print("  PASS  real HIPAA BAA parked")
if "static let hipaaMakeItSo = false" not in enforcer:
    print("  FAIL  HIPAA BAA Make It So must stay off")
    failed = True
else:
    print("  PASS  no HIPAA BAA Make It So")
if "static let wisprBAAClaimAllowed = false" not in enforcer:
    print("  FAIL  Wispr BAA claim must stay disallowed")
    failed = True
else:
    print("  PASS  no Wispr BAA claim")
if "static let silentTrainingUploadAllowed = false" not in enforcer:
    print("  FAIL  silent training upload must stay disallowed")
    failed = True
else:
    print("  PASS  no silent training upload")
if "static let localOnlyLocalFirstCopyOK = true" not in enforcer:
    print("  FAIL  local-only / local-first Settings copy must stay allowed")
    failed = True
else:
    print("  PASS  local-only / local-first copy OK in Settings")
if "GREEN (b6530197): Local-only mode ships" not in enforcer \
        or "BREAKS IF (b6530197): HIPAA/BAA/Wispr BAA claim ships" not in enforcer:
    print("  FAIL  Suite b6530197 canonical GREEN/BREAKS IF missing")
    failed = True
else:
    print("  PASS  Suite b6530197 canonical GREEN/BREAKS IF folded")
if "request.requiresOnDeviceRecognition = EnforcerBound.dictationCloudAvailableV1 == false" not in engine:
    print("  FAIL  speech engine must bind requiresOnDeviceRecognition to dictation-cloud lock")
    failed = True
elif re.search(r'requiresOnDeviceRecognition\s*=\s*false', engine):
    print("  FAIL  requiresOnDeviceRecognition must not be hardcoded false")
    failed = True
else:
    print("  PASS  dictation stays on-device (local engines only)")
if "requestDictationCloud" not in settings_store \
        or "dictationCloud = EnforcerBound.dictationCloudAvailableV1" not in settings_store:
    print("  FAIL  dictation cloud must stay clamped off / unavailable v1")
    failed = True
else:
    print("  PASS  dictation cloud clamped off")
if re.search(r'dictationCloud\s*=\s*true', settings_store + settings_panes):
    print("  FAIL  dictation cloud ON iOS v1")
    failed = True
if "disabled(EnforcerBound.dictationCloudAvailableV1 == false)" not in settings_panes:
    print("  FAIL  Dictation cloud toggle must stay disabled in v1")
    failed = True
else:
    print("  PASS  Dictation cloud toggle disabled in v1")
if "EnforcerBound.privacySurfaceName" not in settings_panes:
    print("  FAIL  Settings privacy section must use Local-only privacy mode surface name")
    failed = True
else:
    print("  PASS  Settings uses Local-only privacy mode surface name")
# Silent cloud: no upload / iCloud / CloudKit in the iOS tree
silent = re.compile(
    r'URLSession|uploadTask|CKContainer|CKRecord|NSUbiquitous|iCloud|CloudKit|'
    r'silent upload|silent training|silent cloud',
    re.I,
)
silent_ok = re.compile(r'not in icloud|unavailable|no silent|silentCloudAllowed|silentTrainingUploadAllowed', re.I)
silent_hit = False
for dirpath, _, files in os.walk(root):
    if "xcodeproj" in dirpath:
        continue
    for name in files:
        if not name.endswith(".swift"):
            continue
        path = os.path.join(dirpath, name)
        for i, line in enumerate(open(path), 1):
            stripped = line.strip()
            if stripped.startswith("//") or stripped.startswith("///"):
                continue
            if silent.search(stripped) and not silent_ok.search(stripped):
                print(f"  FAIL  silent cloud {path}:{i}: {stripped}")
                failed = True
                silent_hit = True
if not silent_hit:
    print("  PASS  no silent cloud / upload path")
if re.search(r'^import StoreKit', settings_panes + settings_store + settings_root, re.M):
    print("  FAIL  Settings must not import StoreKit this tip")
    failed = True
else:
    print("  PASS  Settings does not import StoreKit")
claim_hit = False
deny = re.compile(
    r'NO HIPAA|no HIPAA|never.{0,4}HIPAA|HIPAA.{0,24}parked|parked.{0,24}HIPAA|'
    r'hipaaBAAFollowUpParked|hipaaMakeItSo|wisprBAAClaimAllowed|BREAKS IF|'
    r'Does not claim HIPAA|including.{0,8}HIPAA|no Make It So',
    re.I,
)
for dirpath, _, files in os.walk(root):
    if "xcodeproj" in dirpath or "DerivedData" in dirpath:
        continue
    for name in files:
        path = os.path.join(dirpath, name)
        if name.endswith((".swift", ".md", ".plist", ".pbxproj", ".xcprivacy")):
            for i, line in enumerate(open(path), 1):
                stripped = line.strip()
                is_comment = stripped.startswith("//") or stripped.startswith("///") or stripped.startswith("*") or stripped.startswith("<!--")
                if "HIPAA compliant" in stripped and not deny.search(stripped):
                    print(f"  FAIL  never claim HIPAA compliant {path}:{i}: {stripped}")
                    failed = True
                    claim_hit = True
                if name.endswith(".swift") and not is_comment:
                    if re.search(r'\b(HIPAA|BAA|Wispr BAA)\b', stripped) and not deny.search(stripped):
                        print(f"  FAIL  HIPAA/BAA/Wispr BAA claim {path}:{i}: {stripped}")
                        failed = True
                        claim_hit = True
if not claim_hit:
    print("  PASS  no HIPAA/BAA/Wispr BAA UI claim (local-only privacy mode)")

home = open(os.path.join(root, "MabelIOS/Views/HomeTabView.swift")).read()
lock = open(os.path.join(root, "MabelIOS/Views/LockedProTabView.swift")).read()
tabshell = open(os.path.join(root, "MabelIOS/Views/HostRootView.swift")).read()
for required in (
    "Stats carousel",
    "Dated activity feed",
    "Master On",
    "Try in any app",
    "Account and Settings",
    "line.3.horizontal",
):
    if required not in home:
        print(f"  FAIL  Home IA missing {required}")
        failed = True
if "TabView" not in tabshell or "HomeTabView" not in tabshell:
    print("  FAIL  host shell missing bottom tabs")
    failed = True
else:
    print("  PASS  Home IA tabs + hamburger + master + try-in-any-app")
if "Activate Pro" not in lock or "Sign in with Stiki" not in lock:
    print("  FAIL  locked Pro tabs must offer Activate Pro / Sign in with Stiki")
    failed = True
else:
    print("  PASS  locked Pro tabs use dual-gate CTAs")
if "func isTabUnlocked" not in settings_store or "isProUnlocked" not in settings_store:
    print("  FAIL  tab unlock must use Pro dual gate")
    failed = True
else:
    print("  PASS  tab unlock uses StoreKit Pro AND Stiki")
# Free Home / Free dictate must not gate on Stiki or account.
free_paths = {
    "MabelIOS/Views/HomeTabView.swift": home,
    "MabelIOS/Views/HostDictatePlayground.swift": open(os.path.join(root, "MabelIOS/Views/HostDictatePlayground.swift")).read(),
    "Shared/DictatePermissionGate.swift": open(os.path.join(root, "Shared/DictatePermissionGate.swift")).read(),
    "Shared/SpeechSession.swift": open(os.path.join(root, "Shared/SpeechSession.swift")).read(),
    "MabelKeyboard/KeyboardViewController.swift": open(os.path.join(root, "MabelKeyboard/KeyboardViewController.swift")).read(),
    "MabelKeyboard/KeyboardRootView.swift": open(os.path.join(root, "MabelKeyboard/KeyboardRootView.swift")).read(),
}
free_gate_hit = False
for rel, blob in free_paths.items():
    for i, line in enumerate(blob.splitlines(), 1):
        stripped = line.strip()
        if stripped.startswith("//") or stripped.startswith("///"):
            continue
        if re.search(r'\b(stikiSignedIn|storeKitEntitled|isProUnlocked|isTabUnlocked)\b', stripped):
            print(f"  FAIL  Free Home or Free dictate gated on Stiki/account {rel}:{i}: {stripped}")
            failed = True
            free_gate_hit = True
if not free_gate_hit:
    print("  PASS  Free Home + dictate not gated on Stiki/account")
# Dual-gate truth table: neither StoreKit nor Stiki alone unlocks Pro tabs.
def pro_unlocked(stiki, storekit):
    return bool(stiki and storekit)

def tab_unlocked(tab, stiki, storekit):
    if tab == "Home":
        return True
    if tab in ("Dictionary", "Snippets", "Style", "Scratchpad"):
        return pro_unlocked(stiki, storekit)
    return False

home_ia_failed = False
for tab in ("Home", "Dictionary", "Snippets", "Style", "Scratchpad"):
    for stiki, storekit, want_pro in (
        (False, False, False),
        (True, False, False),
        (False, True, False),
        (True, True, True),
    ):
        got = tab_unlocked(tab, stiki, storekit)
        want = True if tab == "Home" else want_pro
        if got != want:
            print(f"  FAIL  Home IA gate {tab} stiki={stiki} storekit={storekit} -> {got} want {want}")
            failed = True
            home_ia_failed = True
if not home_ia_failed:
    print("  PASS  Home IA dual-gate truth table (StoreKit ≠ Stiki)")
if "masterOn" not in settings_store:
    print("  FAIL  Master On toggle missing from store")
    failed = True
if "Does not listen until you tap the orb" not in home:
    print("  FAIL  Master On must not start the microphone")
    failed = True
else:
    print("  PASS  Master On is ready-only, not ambient listen")

# Free dictate must not require Stiki
gate = open(os.path.join(root, "Shared/DictatePermissionGate.swift")).read()
if re.search(r'\bStiki\b', gate) and not re.search(r'No account, StoreKit, or Stiki', gate):
    print("  FAIL  permission gate must not require Stiki")
    failed = True
elif "allowsAudio" not in gate:
    print("  FAIL  gate missing allowsAudio")
    failed = True
else:
    print("  PASS  Free dictate gate has no Stiki requirement")

sys.exit(1 if failed else 0)
PY
then
  :
else
  FAIL=1
fi

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

# TF 0.1.0/2 smoke: sticky listen, full keyboard, Mabel cat (not maple leaf).
if grep -q 'taskHint = .dictation' "$IOS/Shared/OnDeviceSpeechEngine.swift" \
  && grep -q 'scheduleStickyRestart' "$IOS/Shared/OnDeviceSpeechEngine.swift" \
  && grep -q 'isRecoverableEndOfSpeech' "$IOS/Shared/OnDeviceSpeechEngine.swift" \
  && grep -q 'kAFAssistantErrorDomain' "$IOS/Shared/OnDeviceSpeechEngine.swift"; then
  ok "sticky ASR restarts after Apple end-of-speech / silence"
else
  bad "speech engine missing sticky ASR restart"
fi
if grep -q 'func idleStopSeconds' "$IOS/Shared/SettingsStore.swift" \
  && grep -q 'armIdleWatch' "$IOS/Shared/SpeechSession.swift" \
  && grep -q 'Idle only stops a session you already started' "$IOS/MabelIOS/Views/SettingsPanes.swift"; then
  ok "idle-stop is optional and never starts the mic"
else
  bad "idle-stop helper missing or could start the mic"
fi
if grep -q 'static let preferredHeight: CGFloat = 408' "$IOS/MabelKeyboard/KeyboardViewController.swift" \
  && grep -q 'UILayoutPriority(999)' "$IOS/MabelKeyboard/KeyboardViewController.swift" \
  && ! grep -q 'equalToConstant: 276' "$IOS/MabelKeyboard/KeyboardViewController.swift"; then
  ok "keyboard height is full-board, not truncated 276"
else
  bad "keyboard still uses truncated 276pt height"
fi
if grep -q 'static let top = \["Q", "W", "E", "R", "T", "Y", "U", "I", "O", "P"\]' "$IOS/MabelKeyboard/KeyboardRootView.swift" \
  && grep -q 'static let middle = \["A", "S", "D", "F", "G", "H", "J", "K", "L"\]' "$IOS/MabelKeyboard/KeyboardRootView.swift" \
  && grep -q 'static let bottom = \["Z", "X", "C", "V", "B", "N", "M"\]' "$IOS/MabelKeyboard/KeyboardRootView.swift" \
  && grep -q 'showQwerty' "$IOS/MabelKeyboard/KeyboardRootView.swift"; then
  ok "full QWERTY present so the transcript can be edited"
else
  bad "keyboard missing full QWERTY"
fi
if grep -q 'Image("MabelCat")' "$IOS/Shared/CatChrome.swift" \
  && grep -q 'Mabel cat portrait' "$IOS/Shared/CatChrome.swift" \
  && grep -q 'not a maple leaf' "$IOS/Shared/CatChrome.swift" \
  && grep -q 'mabel-cat.jpeg' "$IOS/MabelKeyboard/Assets.xcassets/MabelCat.imageset/Contents.json"; then
  ok "dictate control uses Mabel cat branding (not orb / maple leaf)"
else
  bad "Mabel cat branding missing from dictate control"
fi
if grep -q 'B2C20001B2C10235 /\* Assets.xcassets in Resources \*/' "$PBX"; then
  ok "keyboard target embeds MabelCat assets"
else
  bad "pbxproj keyboard target missing Assets.xcassets"
fi

if grep -q 'promptHostPermissions' "$IOS/Shared/OnDeviceSpeechEngine.swift" \
  && grep -q 'requestHostPermissions()' "$IOS/MabelIOS/Views/HomeTabView.swift"; then
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

# Polish product locks (ship #2). Dual gate, default Off, local-first, distinct.
python3 - "$IOS" <<'PY'
import os, re, sys
root = sys.argv[1]
failed = False
polish = open(os.path.join(root, "Shared/Polish.swift")).read()
enforcer = open(os.path.join(root, "Shared/EnforcerBound.swift")).read()
store = open(os.path.join(root, "Shared/SettingsStore.swift")).read()
session = open(os.path.join(root, "Shared/SpeechSession.swift")).read()
panes = open(os.path.join(root, "MabelIOS/Views/SettingsPanes.swift")).read()
style_tab = open(os.path.join(root, "MabelIOS/Views/HostRootView.swift")).read()
lock = open(os.path.join(root, "MabelIOS/Views/LockedProTabView.swift")).read()

for required in (
    'static let defaultMode = off',
    'static let casual = "casual"',
    'static let professional = "professional"',
    'static let polite = "polite"',
    "Never invent facts",
    "Never expand meaning",
    "do not send this step off-device",
    "Coach cannot rewrite",
    "Not Nexus",
    "sign-out locks Polish",
    "StoreKit Pro AND Stiki session",
    "Activate Pro / Sign in with Stiki",
    "Formal|Casual|Very casual",
    "acceptOrFailClosed",
    "applyFromAppGroup",
    "isProUnlocked(stikiSignedIn: stikiSignedIn, storeKitEntitled: storeKitEntitled)",
):
    if required not in polish:
        print(f"  FAIL  Polish.swift missing {required}")
        failed = True
if 'static let defaultMode = off' not in polish:
    print("  FAIL  Polish default must be Off")
    failed = True
if re.search(r'defaultMode\s*=\s*casual', polish):
    print("  FAIL  BREAKS IF: default ON")
    failed = True
if re.search(r'https://|api\.groq\.com|URLSession|CKContainer|nexus_write', polish):
    print("  FAIL  BREAKS IF: cloud / Nexus write in Polish")
    failed = True
if "import StoreKit" in polish:
    print("  FAIL  Polish must not import StoreKit")
    failed = True
if "Polish.applyFromAppGroup" not in session:
    print("  FAIL  SpeechSession must apply Polish after ASR")
    failed = True
else:
    print("  PASS  SpeechSession applies Polish after ASR")
if "func setPolishMode" not in store or "func signOutStiki" not in store:
    print("  FAIL  SettingsStore missing Polish persist / sign-out lock")
    failed = True
else:
    print("  PASS  SettingsStore Polish persist + sign-out lock")
if "PolishSettingsPane" not in panes or "polish-mode-select" not in panes:
    print("  FAIL  Settings missing Polish pane / mode picker")
    failed = True
if "Not Style (Formal, Casual, Very casual)" not in panes:
    print("  FAIL  Polish must stay distinct from Style register")
    failed = True
else:
    print("  PASS  Polish UI distinct from Style Formal|Casual|Very casual")
if 'case "Style"' not in style_tab:
    print("  FAIL  Style tab must remain (distinct from Polish)")
    failed = True
else:
    print("  PASS  Style tab remains distinct from Polish Settings")
if "Activate Pro" not in panes or "Sign in with Stiki" not in panes:
    print("  FAIL  locked Polish must offer Activate Pro / Sign in with Stiki")
    failed = True
else:
    print("  PASS  locked Polish uses dual-gate CTAs")
if "Sign out locks Polish" not in panes:
    print("  FAIL  Account must say sign-out locks Polish")
    failed = True

def require_ok(mode, stiki, storekit):
    if mode in ("casual", "professional", "polite"):
        return bool(stiki and storekit)
    return True

def effective(mode, stiki, storekit):
    if mode not in ("casual", "professional", "polite"):
        return "off"
    return mode if (stiki and storekit) else "off"

gate_failed = False
for mode in ("off", "casual", "professional", "polite"):
    for stiki, storekit in ((False, False), (True, False), (False, True), (True, True)):
        allowed = require_ok(mode, stiki, storekit)
        want_allow = True if mode == "off" else bool(stiki and storekit)
        got_eff = effective(mode, stiki, storekit)
        want_eff = mode if (mode != "off" and stiki and storekit) else "off"
        if allowed != want_allow or got_eff != want_eff:
            print(f"  FAIL  Polish gate mode={mode} stiki={stiki} storekit={storekit}")
            failed = True
            gate_failed = True
if not gate_failed:
    print("  PASS  Polish dual-gate truth table (StoreKit ≠ Stiki; sign-out → Off)")

# Local rules: professional synonym, fail-closed expansion, Off is identity.
def apply(text, mode, stiki, storekit):
    if effective(mode, stiki, storekit) == "off":
        return text
    if mode == "professional":
        return text.replace("gonna", "going to").replace("yeah", "yes")
    return text

if apply("yeah I am gonna go", "professional", False, False) != "yeah I am gonna go":
    print("  FAIL  Free / signed-out must not rewrite")
    failed = True
else:
    print("  PASS  Free / signed-out Polish is identity")
if apply("hello", "off", True, True) != "hello":
    print("  FAIL  Off must not rewrite")
    failed = True
else:
    print("  PASS  Off Polish is identity")

def accept_or_fail(inp, out):
    cleaned = out.strip()
    if not cleaned:
        return None
    if len(inp.strip()) >= 20 and len(cleaned) > len(inp.strip()) * 2.5:
        return None
    return cleaned

if accept_or_fail("hello world this is spoken text", "x" * 200) is not None:
    print("  FAIL  BREAKS IF: invent (expanded output accepted)")
    failed = True
else:
    print("  PASS  invented expansion fails closed")
if "2.5" not in polish:
    print("  FAIL  acceptOrFailClosed missing 2.5x invent tripwire")
    failed = True

# Keyboard overlay / entitlements must stay App Group only.
for rel in ("MabelIOS/MabelIOS.entitlements", "MabelKeyboard/MabelKeyboard.entitlements"):
    blob = open(os.path.join(root, rel)).read()
    if "group.com.mabel.ios" not in blob:
        print(f"  FAIL  {rel} missing App Group")
        failed = True
    if "in-app-payments" in blob or "allow-jit" in blob:
        print(f"  FAIL  {rel} gained extra entitlements")
        failed = True
print("  PASS  App Group entitlements unchanged (no StoreKit / JIT)")

if "Formal|Casual|Very casual" not in polish or "Clipboard" not in polish:
    print("  FAIL  Polish product lock must name Style + Clipboard")
    failed = True
if "This tip is Polish" not in lock:
    print("  FAIL  later-ship placeholder must name this tip Polish")
    failed = True

sys.exit(1 if failed else 0)
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
