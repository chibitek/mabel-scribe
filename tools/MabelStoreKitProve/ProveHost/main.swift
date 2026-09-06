import AppKit

/// Tiny host so the XCTest target has a macOS process. Do not use this
/// scheme to prove Mabel IAP. Launch **Mabel-StoreKit**, which runs the
/// Tauri-built 1.4.0 `Mabel.app` under `src-tauri/Mabel.storekit`.
final class AppDelegate: NSObject, NSApplicationDelegate {
    func applicationDidFinishLaunching(_ notification: Notification) {
        fputs(
            "[MabelStoreKitProve] Host only. Switch the scheme to Mabel-StoreKit and press Run.\n",
            stderr
        )
    }
}

let app = NSApplication.shared
let delegate = AppDelegate()
app.delegate = delegate
app.setActivationPolicy(.accessory)
app.run()
