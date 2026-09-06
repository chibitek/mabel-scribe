import StoreKit
import StoreKitTest
import XCTest

/// Catalog smoke for `src-tauri/Mabel.storekit`.
///
/// `xcodebuild test` does **not** apply a scheme StoreKit Configuration
/// FileReference (relative, absolute, or copy-into-project). That is why
/// Product.products returned NO_PRODUCTS on tip 446378a6.
///
/// Load the catalog with `SKTestSession(contentsOf:)` only. Do **not**
/// also set StoreKit Configuration on this scheme's Test action — pairing
/// session + scheme config hangs (`SKTestSession` never returns).
///
/// Purchase / trial in the real Mabel.app is still scheme **Mabel-StoreKit**
/// + ⌘R (Xcode must launch that process under the config).
///
///   xcodebuild test -project tools/MabelStoreKitProve/MabelStoreKitProve.xcodeproj \
///     -scheme MabelStoreKitProve -destination 'platform=macOS'
final class ProductLoadTests: XCTestCase {
    /// Retained for the whole test so StoreKit Testing stays attached.
    private var session: SKTestSession?

    override func setUp() async throws {
        try await super.setUp()
        let url = Self.storeKitConfigurationURL()
        XCTAssertTrue(
            FileManager.default.fileExists(atPath: url.path),
            "StoreKit configuration missing at \(url.path)"
        )
        let session = try SKTestSession(contentsOf: url)
        session.disableDialogs = true
        session.resetToDefaultState()
        self.session = session
    }

    override func tearDown() async throws {
        session = nil
        try await super.tearDown()
    }

    func testMonthlyAndYearlyLoadWithIntroTrial() async throws {
        let ids: Set<String> = [
            "com.mabel.app.pro.monthly",
            "com.mabel.app.pro.yearly",
        ]
        let products = try await Product.products(for: ids)
        XCTAssertEqual(
            Set(products.map(\.id)),
            ids,
            "NO_PRODUCTS: SKTestSession did not load \(Self.storeKitConfigurationURL().path)"
        )

        for product in products {
            let intro = product.subscription?.introductoryOffer
            XCTAssertNotNil(intro, "\(product.id) missing introductory trial")
            XCTAssertEqual(intro?.paymentMode, .freeTrial)
            XCTAssertEqual(intro?.period.unit, .month)
            XCTAssertEqual(intro?.period.value, 1)
        }
    }

    private static func storeKitConfigurationURL() -> URL {
        if let bundled = Bundle(for: Self.self).url(forResource: "Mabel", withExtension: "storekit") {
            return bundled
        }
        return URL(fileURLWithPath: #filePath)
            .deletingLastPathComponent()
            .deletingLastPathComponent()
            .deletingLastPathComponent()
            .deletingLastPathComponent()
            .appendingPathComponent("src-tauri/Mabel.storekit")
    }
}
