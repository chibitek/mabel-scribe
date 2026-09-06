import StoreKit
import XCTest

/// Catalog smoke for `src-tauri/Mabel.storekit`.
///
/// This test target's scheme (MabelStoreKitProve) has StoreKit Configuration
/// set. Do **not** construct `SKTestSession` here — pairing a session with
/// a scheme config is a known hang (`SKTestSession` never returns).
///
/// Run:
///   xcodebuild test -project tools/MabelStoreKitProve/MabelStoreKitProve.xcodeproj \
///     -scheme MabelStoreKitProve -destination 'platform=macOS'
final class ProductLoadTests: XCTestCase {
    func testMonthlyAndYearlyLoadWithIntroTrial() async throws {
        let ids: Set<String> = [
            "com.mabel.app.pro.monthly",
            "com.mabel.app.pro.yearly",
        ]
        let products = try await Product.products(for: ids)
        XCTAssertEqual(
            Set(products.map(\.id)),
            ids,
            "NO_PRODUCTS: scheme StoreKit Configuration did not load src-tauri/Mabel.storekit"
        )

        for product in products {
            let intro = product.subscription?.introductoryOffer
            XCTAssertNotNil(intro, "\(product.id) missing introductory trial")
            XCTAssertEqual(intro?.paymentMode, .freeTrial)
            XCTAssertEqual(intro?.period.unit, .month)
            XCTAssertEqual(intro?.period.value, 1)
        }
    }
}
