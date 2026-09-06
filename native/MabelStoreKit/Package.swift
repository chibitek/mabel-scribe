// swift-tools-version: 6.0
// StoreKit 2 bridge for Mabel Pro. Compiled on macOS by
// scripts/build-mabel-storekit.sh (Xcode 16 + Swift 6, Apple Silicon).
//
// Pure Swift target on purpose: a mixed-language dynamic library
// (C header path + Swift in one target) is a common `swift build`
// failure on Xcode 16. The C header stays in Sources/.../include for
// humans and matching the Rust FFI; it is not part of this product.
//
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
            exclude: ["include"],
            swiftSettings: [
                .swiftLanguageMode(.v5),
            ],
            linkerSettings: [
                .linkedFramework("StoreKit"),
                .linkedFramework("AppKit"),
            ]
        ),
    ]
)
