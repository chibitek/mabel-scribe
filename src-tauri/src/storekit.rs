//! StoreKit 2 Pro entitlement.
//!
//! Fail-closed: Pro is off unless a verified StoreKit 2 subscription
//! (monthly or yearly, including the 30-day introductory trial) is live.
//! Release builds have no mock paid path. Linux / missing dylib = Free.

use serde::{Deserialize, Serialize};
use std::os::raw::c_int;
use std::sync::OnceLock;

#[cfg(all(target_os = "macos", mabel_native_storekit))]
use std::ffi::{CStr, CString};
#[cfg(all(target_os = "macos", mabel_native_storekit))]
use std::os::raw::c_char;
#[cfg(all(target_os = "macos", mabel_native_storekit))]
use tauri::Emitter;

pub const PRODUCT_MONTHLY: &str = "com.mabel.app.pro.monthly";
pub const PRODUCT_YEARLY: &str = "com.mabel.app.pro.yearly";
pub const ASC_APP_APPLE_ID: &str = "6809059582";
pub const BUNDLE_ID: &str = "com.mabel.app";

static APP: OnceLock<tauri::AppHandle> = OnceLock::new();

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IntroOffer {
    #[serde(rename = "paymentMode", default)]
    pub payment_mode: String,
    #[serde(rename = "offerType", default)]
    pub offer_type: String,
    #[serde(rename = "displayPrice", default)]
    pub display_price: String,
    #[serde(default)]
    pub period: String,
    #[serde(rename = "periodCount", default)]
    pub period_count: u32,
    #[serde(default)]
    pub display: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StoreProduct {
    pub id: String,
    #[serde(rename = "displayName", default)]
    pub display_name: String,
    #[serde(default)]
    pub description: String,
    #[serde(rename = "displayPrice", default)]
    pub display_price: String,
    #[serde(default)]
    pub price: String,
    #[serde(default)]
    pub kind: String,
    #[serde(rename = "subscriptionPeriod", default)]
    pub subscription_period: String,
    #[serde(rename = "introOffer")]
    pub intro_offer: Option<IntroOffer>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Entitlement {
    pub entitled: bool,
    pub status: String,
    #[serde(rename = "productId")]
    pub product_id: Option<String>,
    #[serde(rename = "isTrial", default)]
    pub is_trial: bool,
    #[serde(rename = "willAutoRenew", default)]
    pub will_auto_renew: bool,
    #[serde(rename = "expirationDate")]
    pub expiration_date: Option<String>,
    pub environment: Option<String>,
}

impl Entitlement {
    pub fn none() -> Self {
        Self {
            entitled: false,
            status: "none".into(),
            product_id: None,
            is_trial: false,
            will_auto_renew: false,
            expiration_date: None,
            environment: None,
        }
    }
}

pub fn is_known_product(id: &str) -> bool {
    id == PRODUCT_MONTHLY || id == PRODUCT_YEARLY
}

/// Fail-closed parse. Unknown products, missing status, or `entitled: false`
/// never unlock Pro. Trial (`status == "trial"`) is entitled.
pub fn entitlement_from_json(raw: &str) -> Entitlement {
    let Ok(parsed) = serde_json::from_str::<Entitlement>(raw) else {
        return Entitlement::none();
    };
    let product_ok = parsed
        .product_id
        .as_deref()
        .is_some_and(is_known_product);
    let status_ok = parsed.status == "trial" || parsed.status == "subscribed";
    if parsed.entitled && product_ok && status_ok {
        parsed
    } else {
        Entitlement::none()
    }
}

pub fn require_pro() -> Result<Entitlement, String> {
    let entitlement = current_entitlement();
    if entitlement.entitled {
        Ok(entitlement)
    } else {
        Err("Mabel Pro requires an active App Store subscription (the 30-day trial counts).".into())
    }
}

pub fn attach(app: tauri::AppHandle) {
    let _ = APP.set(app);
    native_start_listener();
}

pub fn current_entitlement() -> Entitlement {
    match native_entitlement_json() {
        Ok(json) => entitlement_from_json(&json),
        Err(_) => Entitlement::none(),
    }
}

pub fn load_products() -> Result<Vec<StoreProduct>, String> {
    let json = native_products_json()?;
    let products: Vec<StoreProduct> =
        serde_json::from_str(&json).map_err(|e| format!("StoreKit product JSON: {e}"))?;
    Ok(products
        .into_iter()
        .filter(|p| is_known_product(&p.id))
        .collect())
}

pub fn purchase(product_id: &str) -> Result<Entitlement, String> {
    if !is_known_product(product_id) {
        return Err("Unknown Mabel Pro product".into());
    }
    match native_purchase(product_id)? {
        0 => Ok(current_entitlement()),
        1 => Err("Purchase cancelled".into()),
        2 => Err("Purchase pending approval".into()),
        _ => Err(native_last_error().unwrap_or_else(|| "Purchase failed".into())),
    }
}

pub fn restore() -> Result<Entitlement, String> {
    match native_restore()? {
        0 => Ok(current_entitlement()),
        _ => Err(native_last_error().unwrap_or_else(|| "Restore failed".into())),
    }
}

pub fn manage_subscriptions() -> Result<(), String> {
    match native_manage()? {
        0 => Ok(()),
        _ => Err(native_last_error().unwrap_or_else(|| "Could not open subscription management".into())),
    }
}

#[tauri::command]
pub fn storekit_entitlement() -> Entitlement {
    current_entitlement()
}

#[tauri::command]
pub fn storekit_products() -> Result<Vec<StoreProduct>, String> {
    load_products()
}

#[tauri::command]
pub fn storekit_purchase(product_id: String) -> Result<Entitlement, String> {
    purchase(&product_id)
}

#[tauri::command]
pub fn storekit_restore() -> Result<Entitlement, String> {
    restore()
}

#[tauri::command]
pub fn storekit_manage_subscriptions() -> Result<(), String> {
    manage_subscriptions()
}

#[cfg(all(target_os = "macos", mabel_native_storekit))]
mod ffi {
    use super::*;

    unsafe extern "C" {
        pub fn mabel_storekit_set_update_callback(cb: Option<extern "C" fn(*const c_char)>);
        pub fn mabel_storekit_start_listener();
        pub fn mabel_storekit_products_json() -> *mut c_char;
        pub fn mabel_storekit_entitlement_json() -> *mut c_char;
        pub fn mabel_storekit_purchase(product_id: *const c_char) -> c_int;
        pub fn mabel_storekit_restore() -> c_int;
        pub fn mabel_storekit_manage() -> c_int;
        pub fn mabel_storekit_free(s: *mut c_char);
        pub fn mabel_storekit_last_error() -> *const c_char;
    }

    pub extern "C" fn on_update(json: *const c_char) {
        if json.is_null() {
            return;
        }
        let raw = unsafe { CStr::from_ptr(json) }
            .to_string_lossy()
            .into_owned();
        let entitlement = entitlement_from_json(&raw);
        if let Some(app) = APP.get() {
            let _ = app.emit("pro-entitlement-changed", entitlement);
        }
    }
}

#[cfg(all(target_os = "macos", mabel_native_storekit))]
fn take_c_string(ptr: *mut c_char) -> Result<String, String> {
    if ptr.is_null() {
        return Err(native_last_error().unwrap_or_else(|| "StoreKit returned no data".into()));
    }
    let text = unsafe { CStr::from_ptr(ptr) }.to_string_lossy().into_owned();
    unsafe { ffi::mabel_storekit_free(ptr) };
    Ok(text)
}

#[cfg(all(target_os = "macos", mabel_native_storekit))]
fn native_products_json() -> Result<String, String> {
    take_c_string(unsafe { ffi::mabel_storekit_products_json() })
}

#[cfg(all(target_os = "macos", mabel_native_storekit))]
fn native_entitlement_json() -> Result<String, String> {
    take_c_string(unsafe { ffi::mabel_storekit_entitlement_json() })
}

#[cfg(all(target_os = "macos", mabel_native_storekit))]
fn native_purchase(product_id: &str) -> Result<c_int, String> {
    let c = CString::new(product_id).map_err(|e| e.to_string())?;
    Ok(unsafe { ffi::mabel_storekit_purchase(c.as_ptr()) })
}

#[cfg(all(target_os = "macos", mabel_native_storekit))]
fn native_restore() -> Result<c_int, String> {
    Ok(unsafe { ffi::mabel_storekit_restore() })
}

#[cfg(all(target_os = "macos", mabel_native_storekit))]
fn native_manage() -> Result<c_int, String> {
    Ok(unsafe { ffi::mabel_storekit_manage() })
}

#[cfg(all(target_os = "macos", mabel_native_storekit))]
fn native_last_error() -> Option<String> {
    let ptr = unsafe { ffi::mabel_storekit_last_error() };
    if ptr.is_null() {
        None
    } else {
        Some(unsafe { CStr::from_ptr(ptr) }.to_string_lossy().into_owned())
    }
}

#[cfg(all(target_os = "macos", mabel_native_storekit))]
fn native_start_listener() {
    unsafe {
        ffi::mabel_storekit_set_update_callback(Some(ffi::on_update));
        ffi::mabel_storekit_start_listener();
    }
}

#[cfg(not(all(target_os = "macos", mabel_native_storekit)))]
fn native_products_json() -> Result<String, String> {
    Err(unavailable_message())
}

#[cfg(not(all(target_os = "macos", mabel_native_storekit)))]
fn native_entitlement_json() -> Result<String, String> {
    Err(unavailable_message())
}

#[cfg(not(all(target_os = "macos", mabel_native_storekit)))]
fn native_purchase(_product_id: &str) -> Result<c_int, String> {
    Err(unavailable_message())
}

#[cfg(not(all(target_os = "macos", mabel_native_storekit)))]
fn native_restore() -> Result<c_int, String> {
    Err(unavailable_message())
}

#[cfg(not(all(target_os = "macos", mabel_native_storekit)))]
fn native_manage() -> Result<c_int, String> {
    Err(unavailable_message())
}

#[cfg(not(all(target_os = "macos", mabel_native_storekit)))]
fn native_last_error() -> Option<String> {
    Some(unavailable_message())
}

#[cfg(not(all(target_os = "macos", mabel_native_storekit)))]
fn native_start_listener() {}

fn unavailable_message() -> String {
    "StoreKit 2 is only available in the macOS App Store / TestFlight build. Pro stays locked without a verified subscription.".into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn product_ids_are_the_locked_catalog() {
        assert_eq!(PRODUCT_MONTHLY, "com.mabel.app.pro.monthly");
        assert_eq!(PRODUCT_YEARLY, "com.mabel.app.pro.yearly");
        assert!(is_known_product(PRODUCT_MONTHLY));
        assert!(is_known_product(PRODUCT_YEARLY));
        assert!(!is_known_product("com.mabel.app.pro.lifetime"));
        assert!(!is_known_product(""));
    }

    #[test]
    fn asc_identity_is_documented() {
        assert_eq!(ASC_APP_APPLE_ID, "6809059582");
        assert_eq!(BUNDLE_ID, "com.mabel.app");
    }

    #[test]
    fn fail_closed_on_garbage_json() {
        assert_eq!(entitlement_from_json(""), Entitlement::none());
        assert_eq!(entitlement_from_json("{"), Entitlement::none());
        assert_eq!(entitlement_from_json("null"), Entitlement::none());
    }

    #[test]
    fn fail_closed_when_not_entitled() {
        let json = r#"{"entitled":false,"status":"none","productId":null,"isTrial":false}"#;
        assert!(!entitlement_from_json(json).entitled);
    }

    #[test]
    fn fail_closed_on_unknown_product() {
        let json = r#"{
            "entitled": true,
            "status": "subscribed",
            "productId": "com.evil.app.pro",
            "isTrial": false
        }"#;
        assert!(!entitlement_from_json(json).entitled);
    }

    #[test]
    fn fail_closed_on_expired_or_unknown_status() {
        let expired = r#"{
            "entitled": true,
            "status": "expired",
            "productId": "com.mabel.app.pro.monthly",
            "isTrial": false
        }"#;
        assert!(!entitlement_from_json(expired).entitled);
    }

    #[test]
    fn trial_counts_as_entitled() {
        let json = r#"{
            "entitled": true,
            "status": "trial",
            "productId": "com.mabel.app.pro.yearly",
            "isTrial": true,
            "willAutoRenew": true,
            "expirationDate": "2026-10-06T00:00:00Z"
        }"#;
        let e = entitlement_from_json(json);
        assert!(e.entitled);
        assert!(e.is_trial);
        assert_eq!(e.status, "trial");
        assert_eq!(e.product_id.as_deref(), Some(PRODUCT_YEARLY));
    }

    #[test]
    fn subscribed_monthly_is_entitled() {
        let json = r#"{
            "entitled": true,
            "status": "subscribed",
            "productId": "com.mabel.app.pro.monthly",
            "isTrial": false,
            "willAutoRenew": true
        }"#;
        let e = entitlement_from_json(json);
        assert!(e.entitled);
        assert!(!e.is_trial);
        assert_eq!(e.product_id.as_deref(), Some(PRODUCT_MONTHLY));
    }

    #[test]
    fn entitled_flag_alone_is_not_enough() {
        let json = r#"{"entitled":true,"status":"subscribed"}"#;
        assert!(!entitlement_from_json(json).entitled);
    }

    #[test]
    fn purchase_rejects_unknown_ids_without_storekit() {
        let err = purchase("com.other.app.monthly").unwrap_err();
        assert!(err.contains("Unknown"));
    }

    #[test]
    fn stub_environment_fails_closed() {
        assert!(!current_entitlement().entitled);
        assert!(load_products().is_err());
        assert!(require_pro().is_err());
    }

    #[test]
    fn mas_docs_name_the_iap_products() {
        let docs = include_str!("../../docs/app-store-iap.md");
        assert!(docs.contains(PRODUCT_MONTHLY));
        assert!(docs.contains(PRODUCT_YEARLY));
        assert!(docs.contains(ASC_APP_APPLE_ID));
        assert!(docs.contains("30-day"));
        assert!(docs.contains("In-App Purchase"));
    }

    #[test]
    fn mas_overlay_bundles_storekit_and_keeps_sandbox() {
        let overlay = include_str!("../tauri.mas.conf.json");
        assert!(overlay.contains("native-storekit/libMabelStoreKit.dylib"));
        assert!(overlay.contains("entitlements.mas.plist"));
        let mas = include_str!("../entitlements.mas.plist");
        assert!(mas.contains("In-App Purchase"));
        assert!(!mas.contains("<key>com.apple.developer.in-app-payments</key>"));
        assert!(
            mas.contains("<key>com.apple.security.device.audio-input</key>"),
            "#10 MAS mic entitlement must stay on this branch"
        );
        assert!(
            mas.contains("com.apple.systemevents"),
            "#10 System Events AE exception must stay on this branch"
        );
        let dmg = include_str!("../entitlements.plist");
        assert!(!dmg.contains("com.apple.security.app-sandbox"));
    }

    #[test]
    fn storekit_config_has_locked_product_ids_and_intro() {
        let cfg = include_str!("../Mabel.storekit");
        assert!(cfg.contains(PRODUCT_MONTHLY));
        assert!(cfg.contains(PRODUCT_YEARLY));
        assert!(cfg.contains("\"paymentMode\" : \"free\""));
        assert!(cfg.contains("6809059582"));
        assert!(
            cfg.contains("\"major\" : 4"),
            "Xcode 16 loads v4; older handwritten files failed to migrate"
        );
        assert!(
            cfg.contains("winbackOffers"),
            "Xcode 16 schema expects winbackOffers on subscriptions"
        );
        assert!(
            !cfg.contains("storefrontTimeZone"),
            "storefrontTimeZone caused '_lastMigrationError' / SKTestSession hang"
        );
        assert!(!cfg.contains("_lastMigrationError"));
    }

    #[test]
    fn tauri_vendors_storekit_before_dev_and_build() {
        let conf = include_str!("../tauri.conf.json");
        assert!(conf.contains("vendor-storekit"));
        assert!(conf.contains("\"version\": \"1.4.0\""));
        assert!(conf.contains("native-storekit/libMabelStoreKit.dylib"));
        let pkg = include_str!("../../package.json");
        assert!(pkg.contains("prove-storekit"));
    }

    #[test]
    fn storekit_package_is_pure_swift_dynamic_library() {
        let manifest = include_str!("../../native/MabelStoreKit/Package.swift");
        assert!(manifest.contains(".dynamic"));
        assert!(
            !manifest.contains("publicHeadersPath"),
            "mixed-language + dynamic is a common Xcode 16 swift build failure"
        );
        assert!(manifest.contains("swiftLanguageMode"));
    }

    #[test]
    fn cio_prove_recipe_names_scheme_and_dylib_command() {
        let docs = include_str!("../../docs/app-store-iap.md");
        assert!(docs.contains("Mabel-StoreKit"));
        assert!(docs.contains("xcrun swift build"));
        assert!(docs.contains("--arch arm64"));
        assert!(docs.contains("libMabelStoreKit.dylib"));
        assert!(docs.contains("StoreKit Configuration"));
        assert!(docs.contains("Mabel 2.app"));
        assert!(docs.contains("device.audio-input"));
        assert!(docs.contains("com.apple.systemevents"));
        let script = include_str!("../../scripts/prove-storekit-mac.sh");
        assert!(script.contains("Mabel-StoreKit"));
        assert!(script.contains("1.4.0"));
        assert!(script.contains("libMabelStoreKit.dylib"));
        assert!(script.contains("SKTestSession"));
        assert!(docs.contains("MACOSX_DEPLOYMENT_TARGET=14.0"));
        assert!(docs.contains("Do **not** raise the package or the app to macOS 15"));
        assert!(docs.contains("npm run vendor-asr"));
        assert!(docs.contains("SKTestSession(contentsOf:"));
        let tests = include_str!("../../tools/MabelStoreKitProve/ProveTests/ProductLoadTests.swift");
        assert!(tests.contains("SKTestSession(contentsOf:"));
        assert!(tests.contains("disableDialogs"));
        let prove_scheme = include_str!(
            "../../tools/MabelStoreKitProve/MabelStoreKitProve.xcodeproj/xcshareddata/xcschemes/MabelStoreKitProve.xcscheme"
        );
        assert!(
            !prove_scheme.contains("StoreKitConfigurationFileReference"),
            "scheme Test/Run config + SKTestSession hangs; session-only for xcodebuild test"
        );
    }

    #[test]
    fn storekit_swift_compiles_for_macosx14() {
        let swift = include_str!("../../native/MabelStoreKit/Sources/MabelStoreKit/MabelStoreKit.swift");
        assert!(
            !swift.contains("case .winBack"),
            "OfferType.winBack is macOS 15+ and breaks swift build for macosx14.0"
        );
        assert!(
            !swift.contains("showManageSubscriptions"),
            "AppStore.showManageSubscriptions is not in the macOS 14 SDK"
        );
        assert!(swift.contains("macappstore://apps.apple.com/account/subscriptions"));
        assert!(swift.contains(PRODUCT_MONTHLY));
        assert!(swift.contains(PRODUCT_YEARLY));
    }

    #[test]
    fn frontend_does_not_open_marketing_upgrade() {
        let ts = include_str!("../../src/main.ts");
        let html = include_str!("../../index.html");
        assert!(!ts.contains("chibiteklabs.com"));
        assert!(!ts.contains("chibiteklabs.ai"));
        assert!(!ts.contains("PRO_URL"));
        assert!(!html.contains("chibiteklabs.com"));
        assert!(!html.contains("$10/month"));
    }
}
