// swift-tools-version: 6.0
// In-process ASR for Mabel. CoreML / ANE only — no JIT, no unsigned
// executable memory, no disable-library-validation.
//
// Pins:
//   FluidAudio 0.15.6  — Parakeet TDT v2 (en) / v3 (multi)
//   WhisperKit 1.1.0   — large-v3-turbo
//
// Built on macOS by scripts/build-mabel-asr.sh (Xcode 16 + Swift 6).
// Linux / CI skips this package.

import PackageDescription

let package = Package(
    name: "MabelASR",
    platforms: [
        .macOS(.v14),
    ],
    products: [
        .library(name: "MabelASR", type: .dynamic, targets: ["MabelASR"]),
    ],
    dependencies: [
        .package(url: "https://github.com/FluidInference/FluidAudio.git", exact: "0.15.6"),
        .package(url: "https://github.com/argmaxinc/WhisperKit.git", exact: "1.1.0"),
    ],
    targets: [
        .target(
            name: "MabelASR",
            dependencies: [
                .product(name: "FluidAudio", package: "FluidAudio"),
                .product(name: "WhisperKit", package: "WhisperKit"),
            ],
            path: "Sources/MabelASR",
            publicHeadersPath: "include"
        ),
    ]
)
