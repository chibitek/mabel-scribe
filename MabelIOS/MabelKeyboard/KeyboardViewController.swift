import SwiftUI
import UIKit

/// System keyboard extension. Not a silent spy.
/// Dictation starts only on an explicit orb tap after the fail-closed
/// permission gate. Never listen in viewDidLoad / viewWillAppear /
/// textDidChange. Ambient / always-on listen is a hard break.
final class KeyboardViewController: UIInputViewController {
    /// Full QWERTY + cat row. 276pt / defaultHigh was truncating the board.
    static let preferredHeight: CGFloat = 408

    private let session = SpeechSession()
    private let chrome = KeyboardChrome()
    private var hosting: UIHostingController<KeyboardRootView>?
    private var heightConstraint: NSLayoutConstraint?

    override func viewDidLoad() {
        super.viewDidLoad()
        view.backgroundColor = UIColor(IOSPalette.cream)
        chrome.controller = self
        chrome.needsInputModeSwitch = needsInputModeSwitchKey
        chrome.hasFullAccess = hasFullAccess

        let root = KeyboardRootView(session: session, chrome: chrome)
        let host = UIHostingController(rootView: root)
        host.view.translatesAutoresizingMaskIntoConstraints = false
        host.view.backgroundColor = .clear
        addChild(host)
        view.addSubview(host.view)
        applyKeyboardHeight()
        NSLayoutConstraint.activate([
            host.view.topAnchor.constraint(equalTo: view.topAnchor),
            host.view.leadingAnchor.constraint(equalTo: view.leadingAnchor),
            host.view.trailingAnchor.constraint(equalTo: view.trailingAnchor),
            host.view.bottomAnchor.constraint(equalTo: view.bottomAnchor),
        ])
        host.didMove(toParent: self)
        hosting = host
    }

    override func viewWillAppear(_ animated: Bool) {
        super.viewWillAppear(animated)
        // Status refresh only. Never start the microphone here.
        applyKeyboardHeight()
        chrome.hasFullAccess = hasFullAccess
        chrome.needsInputModeSwitch = needsInputModeSwitchKey
        session.refreshGate(context: .keyboard(hasFullAccess: hasFullAccess))
    }

    private func applyKeyboardHeight() {
        if let heightConstraint {
            heightConstraint.constant = Self.preferredHeight
            return
        }
        let height = view.heightAnchor.constraint(equalToConstant: Self.preferredHeight)
        height.priority = UILayoutPriority(999)
        height.isActive = true
        heightConstraint = height
    }

    override func viewWillDisappear(_ animated: Bool) {
        super.viewWillDisappear(animated)
        if session.isListening {
            session.stopListening()
        }
    }

    override func textDidChange(_ textInput: UITextInput?) {
        super.textDidChange(textInput)
        // Do not start or resume dictation when the document changes.
        chrome.hasFullAccess = hasFullAccess
    }
}

@MainActor
@Observable
final class KeyboardChrome {
    weak var controller: KeyboardViewController?
    var hasFullAccess = false
    var needsInputModeSwitch = true

    func insert(_ text: String) {
        controller?.textDocumentProxy.insertText(text)
    }

    func deleteBackward() {
        controller?.textDocumentProxy.deleteBackward()
    }

    func advanceToNextInputMode() {
        controller?.advanceToNextInputMode()
    }
}
