import AppKit
import Foundation
import StoreKit
import SwiftUI

/// C ABI for Mabel Pro StoreKit 2.
///
/// Product IDs (auto-renewable, same subscription group):
///   com.mabel.app.pro.monthly
///   com.mabel.app.pro.yearly
///
/// Fail-closed: unverified transactions never grant Pro. A 30-day
/// introductory free trial still counts as entitled.

private let productIds: Set<String> = [
    "com.mabel.app.pro.monthly",
    "com.mabel.app.pro.yearly",
]

/// C ABI callback. Kept in Swift so Package.swift can be a pure Swift
/// target (mixed-language + `.dynamic` is a common Xcode 16 build break).
public typealias mabel_storekit_update_cb = @convention(c) (UnsafePointer<CChar>?) -> Void

private let lock = NSLock()
private var lastErrorC: UnsafeMutablePointer<CChar>?
private var updateCallback: mabel_storekit_update_cb?
private var listenerStarted = false

private func setError(_ message: String) {
    lock.lock()
    defer { lock.unlock() }
    if let old = lastErrorC {
        free(old)
    }
    lastErrorC = strdup(message)
    fputs("[MabelStoreKit] \(message)\n", stderr)
}

private func clearError() {
    lock.lock()
    defer { lock.unlock() }
    if let old = lastErrorC {
        free(old)
        lastErrorC = nil
    }
}

/// Seconds the C ABI will wait for StoreKit. Must stay off the AppKit
/// main thread: `semaphore.wait()` on main deadlocks the purchase sheet.
private let bridgeTimeoutSeconds: TimeInterval = 120

private func runBlocking<T>(
    onMainActor: Bool = false,
    _ body: @escaping @Sendable () async throws -> T
) throws -> T {
    if Thread.isMainThread {
        throw StoreBridgeError.failed("StoreKit bridge must not block the main thread")
    }
    let semaphore = DispatchSemaphore(value: 0)
    let box = BlockingBox<T>()
    let work: @Sendable () async -> Void = {
        do {
            let value = try await body()
            box.set(.success(value))
        } catch {
            box.set(.failure(error))
        }
        semaphore.signal()
    }
    if onMainActor {
        Task { @MainActor in
            await work()
        }
    } else {
        Task.detached {
            await work()
        }
    }
    if semaphore.wait(timeout: .now() + bridgeTimeoutSeconds) == .timedOut {
        throw StoreBridgeError.failed(
            "StoreKit timed out after \(Int(bridgeTimeoutSeconds))s. Try Restore Purchases or Manage Subscriptions."
        )
    }
    return try box.take()
}

private final class BlockingBox<T>: @unchecked Sendable {
    private let lock = NSLock()
    private var result: Result<T, Error>?
    func set(_ value: Result<T, Error>) {
        lock.lock()
        defer { lock.unlock() }
        if result == nil {
            result = value
        }
    }
    func take() throws -> T {
        lock.lock()
        defer { lock.unlock() }
        guard let result else {
            throw StoreBridgeError.failed("async bridge produced no result")
        }
        return try result.get()
    }
}

private enum StoreBridgeError: Error, LocalizedError {
    case badArgument(String)
    case failed(String)
    case cancelled
    case pending
    var errorDescription: String? {
        switch self {
        case .badArgument(let s), .failed(let s): return s
        case .cancelled: return "Purchase cancelled"
        case .pending: return "Purchase pending"
        }
    }
}

private func verified<T>(_ result: VerificationResult<T>) throws -> T {
    switch result {
    case .unverified(_, let error):
        throw StoreBridgeError.failed("Unverified StoreKit transaction: \(error)")
    case .verified(let value):
        return value
    }
}

private func jsonString(_ object: Any) throws -> String {
    let data = try JSONSerialization.data(withJSONObject: object, options: [])
    guard let text = String(data: data, encoding: .utf8) else {
        throw StoreBridgeError.failed("Failed to encode StoreKit JSON")
    }
    return text
}

private func strdupJSON(_ object: Any) -> UnsafeMutablePointer<CChar>? {
    do {
        return try strdup(jsonString(object))
    } catch {
        setError(error.localizedDescription)
        return nil
    }
}

private func periodLabel(_ period: Product.SubscriptionPeriod) -> String {
    let n = period.value
    switch period.unit {
    case .day: return n == 1 ? "1 day" : "\(n) days"
    case .week: return n == 1 ? "1 week" : "\(n) weeks"
    case .month: return n == 1 ? "1 month" : "\(n) months"
    case .year: return n == 1 ? "1 year" : "\(n) years"
    @unknown default: return "\(n) period(s)"
    }
}

private func introOfferDict(_ offer: Product.SubscriptionOffer) -> [String: Any] {
    var paymentMode = "unknown"
    switch offer.paymentMode {
    case .freeTrial: paymentMode = "freeTrial"
    case .payAsYouGo: paymentMode = "payAsYouGo"
    case .payUpFront: paymentMode = "payUpFront"
    default: break
    }
    var offerType = "unknown"
    // Do not name the macOS 15 win-back OfferType case here.
    // `swift build` for macosx14.0 fails even when the rest of StoreKit 2
    // is fine. Intro / promo cover the locked Pro catalog.
    switch offer.type {
    case .introductory: offerType = "introductory"
    case .promotional: offerType = "promotional"
    default: break
    }
    let period = periodLabel(offer.period)
    let display: String
    if offer.paymentMode == .freeTrial {
        display = "\(period) free trial"
    } else {
        display = "\(offer.displayPrice) for \(period)"
    }
    return [
        "paymentMode": paymentMode,
        "offerType": offerType,
        "displayPrice": offer.displayPrice,
        "period": period,
        "periodCount": offer.periodCount,
        "display": display,
    ]
}

private func productDict(_ product: Product) -> [String: Any] {
    var dict: [String: Any] = [
        "id": product.id,
        "displayName": product.displayName,
        "description": product.description,
        "displayPrice": product.displayPrice,
        "price": NSDecimalNumber(decimal: product.price).stringValue,
        "kind": "autoRenewable",
    ]
    if let sub = product.subscription {
        dict["subscriptionPeriod"] = periodLabel(sub.subscriptionPeriod)
        if let intro = sub.introductoryOffer {
            dict["introOffer"] = introOfferDict(intro)
        }
    }
    return dict
}

private func iso(_ date: Date?) -> String? {
    guard let date else { return nil }
    return ISO8601DateFormatter().string(from: date)
}

private func entitlementPayload() async -> [String: Any] {
    var best: (
        productId: String,
        expiration: Date?,
        isTrial: Bool,
        willRenew: Bool,
        environment: String
    )?

    for await result in Transaction.currentEntitlements {
        let transaction: Transaction
        do {
            transaction = try verified(result)
        } catch {
            continue
        }
        guard productIds.contains(transaction.productID) else { continue }
        if transaction.revocationDate != nil { continue }
        if let exp = transaction.expirationDate, exp < Date() { continue }

        var isTrial = false
        var willRenew = false
        if let statusList = try? await Product.products(for: [transaction.productID]).first?.subscription?.status {
            for status in statusList {
                guard status.state == .subscribed || status.state == .inGracePeriod else { continue }
                if let renewal = try? verified(status.renewalInfo) {
                    willRenew = renewal.willAutoRenew
                    if renewal.offerType == .introductory {
                        isTrial = true
                    }
                }
            }
        }
        if #available(macOS 14.2, *) {
            if let offer = transaction.offer, offer.type == .introductory {
                isTrial = true
            }
        }

        if let current = best {
            let currentExp = current.expiration ?? Date.distantFuture
            let nextExp = transaction.expirationDate ?? Date.distantFuture
            if nextExp < currentExp { continue }
        }
        best = (
            productId: transaction.productID,
            expiration: transaction.expirationDate,
            isTrial: isTrial,
            willRenew: willRenew,
            environment: String(describing: transaction.environment)
        )
    }

    guard let best else {
        return [
            "entitled": false,
            "status": "none",
            "productId": NSNull(),
            "isTrial": false,
            "willAutoRenew": false,
            "expirationDate": NSNull(),
            "environment": NSNull(),
        ]
    }

    return [
        "entitled": true,
        "status": best.isTrial ? "trial" : "subscribed",
        "productId": best.productId,
        "isTrial": best.isTrial,
        "willAutoRenew": best.willRenew,
        "expirationDate": iso(best.expiration) as Any,
        "environment": best.environment,
    ]
}

private func notifyUpdate() {
    Task {
        let payload = await entitlementPayload()
        guard let text = try? jsonString(payload), let cb = updateCallback else { return }
        text.withCString { cb($0) }
    }
}

@_cdecl("mabel_storekit_set_update_callback")
public func mabel_storekit_set_update_callback(_ cb: mabel_storekit_update_cb?) {
    updateCallback = cb
}

@_cdecl("mabel_storekit_start_listener")
public func mabel_storekit_start_listener() {
    lock.lock()
    if listenerStarted {
        lock.unlock()
        return
    }
    listenerStarted = true
    lock.unlock()

    Task.detached {
        for await _ in Transaction.updates {
            notifyUpdate()
        }
    }
}

@_cdecl("mabel_storekit_products_json")
public func mabel_storekit_products_json() -> UnsafeMutablePointer<CChar>? {
    clearError()
    do {
        let json = try runBlocking {
            let products = try await Product.products(for: productIds)
            if products.isEmpty {
                throw StoreBridgeError.failed(
                    "NO_PRODUCTS: StoreKit returned no catalog for com.mabel.app.pro.monthly / com.mabel.app.pro.yearly. Launch the 1.4.0 Mabel.app from the Mabel-StoreKit Xcode scheme (StoreKit Configuration = src-tauri/Mabel.storekit). Do not run /Applications/Mabel.app or Mabel 2.app. See docs/app-store-iap.md."
                )
            }
            let ordered = products.sorted { lhs, rhs in
                if lhs.id.contains("monthly") { return true }
                if rhs.id.contains("monthly") { return false }
                return lhs.id < rhs.id
            }
            return try jsonString(ordered.map(productDict))
        }
        return strdup(json)
    } catch {
        setError(error.localizedDescription)
        return nil
    }
}

@_cdecl("mabel_storekit_entitlement_json")
public func mabel_storekit_entitlement_json() -> UnsafeMutablePointer<CChar>? {
    clearError()
    do {
        let json = try runBlocking {
            try jsonString(await entitlementPayload())
        }
        return strdup(json)
    } catch {
        setError(error.localizedDescription)
        return nil
    }
}

@_cdecl("mabel_storekit_purchase")
public func mabel_storekit_purchase(_ productId: UnsafePointer<CChar>?) -> Int32 {
    clearError()
    guard let productId, let id = String(validatingCString: productId), productIds.contains(id) else {
        setError("Unknown product")
        return -1
    }
    do {
        try runBlocking(onMainActor: true) {
            let products = try await Product.products(for: [id])
            guard let product = products.first else {
                throw StoreBridgeError.failed(
                    "NO_PRODUCTS: \(id) is not available. The process is not running under src-tauri/Mabel.storekit (Xcode scheme Mabel-StoreKit) and App Store Connect has not returned the product. See docs/app-store-iap.md."
                )
            }
            let result = try await product.purchase()
            switch result {
            case .success(let verification):
                let transaction = try verified(verification)
                await transaction.finish()
            case .userCancelled:
                throw StoreBridgeError.cancelled
            case .pending:
                throw StoreBridgeError.pending
            @unknown default:
                throw StoreBridgeError.failed("Unknown purchase result")
            }
        }
        notifyUpdate()
        return 0
    } catch let err as StoreBridgeError {
        switch err {
        case .cancelled:
            setError(err.errorDescription ?? "cancelled")
            return 1
        case .pending:
            setError(err.errorDescription ?? "pending")
            return 2
        default:
            setError(err.errorDescription ?? "purchase failed")
            return -1
        }
    } catch {
        setError(error.localizedDescription)
        return -1
    }
}

@_cdecl("mabel_storekit_restore")
public func mabel_storekit_restore() -> Int32 {
    clearError()
    do {
        try runBlocking {
            try await AppStore.sync()
        }
        notifyUpdate()
        return 0
    } catch {
        setError(error.localizedDescription)
        return -1
    }
}

@_cdecl("mabel_storekit_manage")
public func mabel_storekit_manage() -> Int32 {
    clearError()
    // The StoreKit manage-subscriptions sheet is UIWindowScene-based and
    // is not in the macOS 14 SDK (compile error on macosx14.0). Native
    // Mac opens Apple's account subscriptions page instead — not a
    // marketing / upgrade URL.
    if let url = URL(string: "macappstore://apps.apple.com/account/subscriptions") {
        NSWorkspace.shared.open(url)
        return 0
    }
    if let url = URL(string: "https://apps.apple.com/account/subscriptions") {
        NSWorkspace.shared.open(url)
        return 0
    }
    setError("Could not open App Store subscription management")
    return -1
}

/// 1 if StoreKit `offerCodeRedemption` can present the system sheet.
@_cdecl("mabel_storekit_offer_codes_supported")
public func mabel_storekit_offer_codes_supported() -> Int32 {
    if #available(macOS 15.0, *) {
        return 1
    }
    return 0
}

/// Present Apple's offer-code sheet. Does not invent a price or grant Pro.
/// Cancel / fail / unverified → stay Free (entitlement is still verified tx only).
@_cdecl("mabel_storekit_redeem_offer_code")
public func mabel_storekit_redeem_offer_code() -> Int32 {
    clearError()
    if #available(macOS 15.0, *) {
        return redeemOfferCodeOnSupportedOS()
    }
    setError("Offer codes need a newer macOS")
    return 3
}

@available(macOS 15.0, *)
private func redeemOfferCodeOnSupportedOS() -> Int32 {
    do {
        try runBlocking(onMainActor: true) {
            try await presentOfferCodeRedemption()
        }
        // Fail-closed: only Transaction.currentEntitlements after verify() grants Pro.
        notifyUpdate()
        return 0
    } catch let err as StoreBridgeError {
        switch err {
        case .cancelled:
            setError("Offer code redemption cancelled. You are still on Free.")
            return 1
        default:
            setError(err.errorDescription ?? "Could not redeem the offer code. You are still on Free.")
            return -1
        }
    } catch {
        let ns = error as NSError
        if ns.domain == NSCocoaErrorDomain && ns.code == NSUserCancelledError {
            setError("Offer code redemption cancelled. You are still on Free.")
            return 1
        }
        let text = error.localizedDescription
        if text.localizedCaseInsensitiveContains("cancel") {
            setError("Offer code redemption cancelled. You are still on Free.")
            return 1
        }
        setError("Could not redeem the offer code. You are still on Free. \(text)")
        return -1
    }
}

/// Hidden SwiftUI anchor so we can call `offerCodeRedemption` from AppKit/Tauri.
/// Do not add a custom code field — Apple's system sheet is the only redeem UI.
@available(macOS 15.0, *)
private struct OfferCodeRedemptionRoot: View {
    @State private var isPresented = false
    let onCompletion: (Result<Void, Error>) -> Void

    var body: some View {
        Color.clear
            .frame(width: 1, height: 1)
            .offerCodeRedemption(isPresented: $isPresented, onCompletion: onCompletion)
            .onAppear { isPresented = true }
    }
}

private final class ResumeOnce: @unchecked Sendable {
    private let lock = NSLock()
    private var continuation: CheckedContinuation<Void, Error>?

    init(_ continuation: CheckedContinuation<Void, Error>) {
        self.continuation = continuation
    }

    func resume(returning: Void) {
        lock.lock()
        let cont = continuation
        continuation = nil
        lock.unlock()
        cont?.resume(returning: returning)
    }

    func resume(throwing error: Error) {
        lock.lock()
        let cont = continuation
        continuation = nil
        lock.unlock()
        cont?.resume(throwing: error)
    }
}

private var offerCodeHosting: NSViewController?

@available(macOS 15.0, *)
@MainActor
private func presentOfferCodeRedemption() async throws {
    guard let parent = NSApp.keyWindow ?? NSApp.mainWindow ?? NSApp.windows.first(where: { $0.isVisible }) else {
        throw StoreBridgeError.failed("Could not open the App Store offer code sheet. Try again from Plans.")
    }

    try await withCheckedThrowingContinuation { (continuation: CheckedContinuation<Void, Error>) in
        let gate = ResumeOnce(continuation)
        let root = OfferCodeRedemptionRoot { result in
            offerCodeHosting?.view.removeFromSuperview()
            offerCodeHosting = nil
            switch result {
            case .success:
                gate.resume(returning: ())
            case .failure(let error):
                gate.resume(throwing: error)
            }
        }
        let host = NSHostingController(rootView: root)
        offerCodeHosting = host
        host.view.frame = NSRect(x: 0, y: 0, width: 1, height: 1)
        host.view.alphaValue = 0
        parent.contentView?.addSubview(host.view)
    }
}

@_cdecl("mabel_storekit_free")
public func mabel_storekit_free(_ s: UnsafeMutablePointer<CChar>?) {
    if let s { free(s) }
}

@_cdecl("mabel_storekit_last_error")
public func mabel_storekit_last_error() -> UnsafePointer<CChar>? {
    lock.lock()
    defer { lock.unlock() }
    return UnsafePointer(lastErrorC)
}
