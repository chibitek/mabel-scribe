// swift-tools-version: 6.0
// StoreKit 2 bridge for Mabel Pro. Compiled on macOS by
// scripts/build-mabel-storekit.sh (Xcode 16 + Swift 6).
// Linux / CI skips this package.

import PackageDescription

let package = Package(
    name: "MabelStoreKit",
    platforms: [
        .macOS(.v14),
    ],
    products: [
        .library(name: "MabelStoreKit", type: .dynamic, targets: ["MabelStoreKit"]),
    ],
    targets: [
        .target(
            name: "MabelStoreKit",
            dependencies: [],
            path: "Sources/MabelStoreKit",
            publicHeadersPath: "include",
            linkerSettings: [
                .linkedFramework("StoreKit"),
                .linkedFramework("AppKit"),
            ]
        ),
    ]
)
