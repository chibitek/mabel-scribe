import AppKit
import Foundation
import StoreKit

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

private func runBlocking<T>(_ body: @escaping @Sendable () async throws -> T) throws -> T {
    let semaphore = DispatchSemaphore(value: 0)
    let box = BlockingBox<T>()
    Task.detached {
        do {
            let value = try await body()
            box.set(.success(value))
        } catch {
            box.set(.failure(error))
        }
        semaphore.signal()
    }
    semaphore.wait()
    return try box.take()
}

private final class BlockingBox<T>: @unchecked Sendable {
    private var result: Result<T, Error>?
    func set(_ value: Result<T, Error>) { result = value }
    func take() throws -> T {
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
    switch offer.type {
    case .introductory: offerType = "introductory"
    case .promotional: offerType = "promotional"
    case .winBack: offerType = "winBack"
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

@MainActor
private func showManageSubscriptions() async throws {
    if let window = NSApp.keyWindow ?? NSApp.mainWindow ?? NSApp.windows.first {
        try await AppStore.showManageSubscriptions(in: window)
        return
    }
    throw StoreBridgeError.failed("No window available for subscription management")
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
        try runBlocking {
            let products = try await Product.products(for: [id])
            guard let product = products.first else {
                throw StoreBridgeError.failed("Product \(id) is not available from App Store")
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
    do {
        try runBlocking { () -> Void in
            try await showManageSubscriptions()
        }
        return 0
    } catch {
        // Apple's subscription management page — not a marketing / upgrade URL.
        if let url = URL(string: "macappstore://apps.apple.com/account/subscriptions") {
            NSWorkspace.shared.open(url)
            return 0
        }
        setError(error.localizedDescription)
        return -1
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
