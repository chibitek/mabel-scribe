#pragma once
/* C ABI mirror for the Rust FFI. Not compiled into the Swift package
 * (Package.swift excludes this header so the dylib stays a pure Swift
 * dynamic library — mixed-language + .dynamic broke Xcode 16 builds). */

#ifdef __cplusplus
extern "C" {
#endif

/* Entitlement / product JSON is UTF-8. Caller frees with mabel_storekit_free. */
typedef void (*mabel_storekit_update_cb)(const char *entitlement_json);

void mabel_storekit_set_update_callback(mabel_storekit_update_cb cb);
void mabel_storekit_start_listener(void);

char *mabel_storekit_products_json(void);
char *mabel_storekit_entitlement_json(void);

/* 0 = purchased / restored / redeemed, 1 = user cancelled, 2 = pending,
 * 3 = offer codes need a newer macOS, -1 = error.
 * Redeem never grants Pro by itself — entitlement still requires a
 * verified StoreKit transaction. */
int mabel_storekit_purchase(const char *product_id);
int mabel_storekit_restore(void);
int mabel_storekit_manage(void);
int mabel_storekit_redeem_offer_code(void);
int mabel_storekit_offer_codes_supported(void);

void mabel_storekit_free(char *s);
const char *mabel_storekit_last_error(void);

#ifdef __cplusplus
}
#endif
