import AVFoundation
import Foundation
import Speech

enum SpeechEngineEvent: Sendable {
    case volatile(String)
    case final(String)
    case status(String)
    case failed(String)
}

enum SpeechEngineError: LocalizedError {
    case permissionDenied
    case engineUnavailable
    case localeUnsupported

    var errorDescription: String? {
        switch self {
        case .permissionDenied:
            return "Microphone permission denied."
        case .engineUnavailable:
            return "On-device Apple Speech is unavailable on this headset."
        case .localeUnsupported:
            return "No on-device speech model for this locale."
        }
    }
}

/// Apple Speech only. No whisper.cpp, Parakeet, WhisperKit, or Tauri.
///
/// Primary path: SpeechAnalyzer + SpeechTranscriber (visionOS 27 / XROS27).
/// Fallback: SFSpeechRecognizer with `requiresOnDeviceRecognition = true`.
/// The audio engine and AVAudioSession are torn down in `stop()` so the
/// microphone is powered only while listening.
///
/// XROS27 surface (do not regress):
/// - Construct SpeechDetector with detectionOptions + reportResults.
///   Sensitivity lives on SpeechDetector.DetectionOptions, not the
///   detector's own unlabeled sensitivity argument.
/// - SpeechTranscriber.Result.isFinal. Partial/live tail is
///   isFinal == false when reportingOptions includes volatileResults.
final class OnDeviceSpeechEngine: @unchecked Sendable {
    private let engine = AVAudioEngine()
    private var analyzer: SpeechAnalyzer?
    private var transcriber: SpeechTranscriber?
    private var inputBuilder: AsyncStream<AnalyzerInput>.Continuation?
    private var resultTask: Task<Void, Never>?
    private var recognizer: SFSpeechRecognizer?
    private var recognitionRequest: SFSpeechAudioBufferRecognitionRequest?
    private var recognitionTask: SFSpeechRecognitionTask?
    private var usingLegacyRecognizer = false
    private var sink: (@Sendable (SpeechEngineEvent) -> Void)?

    func start(handler: @escaping @Sendable (SpeechEngineEvent) -> Void) async throws {
        sink = handler
        try await requestMicrophone()

        do {
            try await startAnalyzer()
            return
        } catch {
            handler(.status("Analyzer unavailable — trying on-device dictation fallback"))
        }

        try await startOnDeviceSFSpeech()
    }

    func stop() async {
        resultTask?.cancel()
        resultTask = nil

        engine.inputNode.removeTap(onBus: 0)
        if engine.isRunning {
            engine.stop()
        }

        if usingLegacyRecognizer {
            recognitionRequest?.endAudio()
            recognitionTask?.cancel()
            recognitionRequest = nil
            recognitionTask = nil
            recognizer = nil
            usingLegacyRecognizer = false
        }

        inputBuilder?.finish()
        inputBuilder = nil

        if let analyzer {
            do {
                try await analyzer.finalizeAndFinishThroughEndOfInput()
            } catch {
                // Already finished or never started — safe to ignore on stop.
            }
        }
        self.analyzer = nil
        transcriber = nil

        deactivateAudioSession()
        sink = nil
    }

    // MARK: - Permissions

    private func requestMicrophone() async throws {
        let granted = await withCheckedContinuation { continuation in
            AVAudioApplication.requestRecordPermission { allowed in
                continuation.resume(returning: allowed)
            }
        }
        if granted == false {
            throw SpeechEngineError.permissionDenied
        }
    }

    // MARK: - SpeechAnalyzer (visionOS 27 / XROS27)

    private func startAnalyzer() async throws {
        let locale = preferredLocale()
        try await installAssetsIfNeeded(locale: locale)

        let transcriber = SpeechTranscriber(
            locale: locale,
            transcriptionOptions: [],
            reportingOptions: [.volatileResults],
            attributeOptions: []
        )
        // Official XROS27 init. Sensitivity is on DetectionOptions, not SpeechDetector.
        // reportResults: false — VAD is a power gate, not a second transcript stream.
        let detector = SpeechDetector(
            detectionOptions: SpeechDetector.DetectionOptions(sensitivityLevel: .high),
            reportResults: false
        )
        let analyzer = SpeechAnalyzer(modules: [transcriber, detector])
        self.transcriber = transcriber
        self.analyzer = analyzer

        let (stream, continuation) = AsyncStream<AnalyzerInput>.makeStream()
        inputBuilder = continuation
        try await analyzer.start(inputSequence: stream)

        resultTask = Task { [weak self] in
            guard let self else { return }
            do {
                for try await result in transcriber.results {
                    let piece = Self.plainText(result.text)
                    // SpeechTranscriber.Result.isFinal (WWDC25 / XROS27). No isVolatile.
                    if result.isFinal {
                        self.sink?(.final(piece))
                    } else {
                        self.sink?(.volatile(piece))
                    }
                }
            } catch is CancellationError {
                return
            } catch {
                self.sink?(.failed(error.localizedDescription))
            }
        }

        try await startMicTap(feedingAnalyzer: true)
    }

    private func installAssetsIfNeeded(locale: Locale) async throws {
        let supported = await SpeechTranscriber.supportedLocales
        let localeID = locale.identifier(.bcp47)
        let supportedMatch = supported.contains { $0.identifier(.bcp47) == localeID }
        if supportedMatch == false {
            throw SpeechEngineError.localeUnsupported
        }

        let installed = await SpeechTranscriber.installedLocales
        if installed.contains(where: { $0.identifier(.bcp47) == localeID }) {
            return
        }

        sink?(.status("Installing on-device speech model…"))
        let probe = SpeechTranscriber(locale: locale, preset: .transcription)
        if let request = try await AssetInventory.assetInstallationRequest(supporting: [probe]) {
            try await request.downloadAndInstall()
        }
    }

    // MARK: - On-device SFSpeech fallback

    private func startOnDeviceSFSpeech() async throws {
        let recognizer = SFSpeechRecognizer(locale: preferredLocale())
        guard let recognizer, recognizer.isAvailable else {
            throw SpeechEngineError.engineUnavailable
        }
        guard recognizer.supportsOnDeviceRecognition else {
            throw SpeechEngineError.engineUnavailable
        }

        let request = SFSpeechAudioBufferRecognitionRequest()
        request.shouldReportPartialResults = true
        request.requiresOnDeviceRecognition = true
        request.addsPunctuation = true

        self.recognizer = recognizer
        recognitionRequest = request
        usingLegacyRecognizer = true

        recognitionTask = recognizer.recognitionTask(with: request) { [weak self] result, error in
            if let result {
                let text = result.bestTranscription.formattedString
                if result.isFinal {
                    self?.sink?(.final(text))
                } else {
                    self?.sink?(.volatile(text))
                }
            }
            if let error {
                self?.sink?(.failed(error.localizedDescription))
            }
        }

        try await startMicTap(feedingAnalyzer: false)
    }

    // MARK: - Microphone (only while listening)

    private func startMicTap(feedingAnalyzer: Bool) async throws {
        let session = AVAudioSession.sharedInstance()
        try session.setCategory(.record, mode: .measurement, options: [])
        try session.setActive(true)

        let input = engine.inputNode
        let hwFormat = input.outputFormat(forBus: 0)

        if feedingAnalyzer {
            guard let transcriber else {
                throw SpeechEngineError.engineUnavailable
            }
            let targetFormat = await SpeechAnalyzer.bestAvailableAudioFormat(compatibleWith: [transcriber]) ?? hwFormat
            let converter = AVAudioConverter(from: hwFormat, to: targetFormat)

            input.installTap(onBus: 0, bufferSize: 4096, format: hwFormat) { [weak self] buffer, _ in
                guard let self, let converter else { return }
                let ratio = targetFormat.sampleRate / hwFormat.sampleRate
                let capacity = AVAudioFrameCount(Double(buffer.frameLength) * ratio) + 32
                guard let converted = AVAudioPCMBuffer(pcmFormat: targetFormat, frameCapacity: capacity) else {
                    return
                }
                var error: NSError?
                converter.convert(to: converted, error: &error) { _, status in
                    status.pointee = .haveData
                    return buffer
                }
                if error == nil {
                    self.inputBuilder?.yield(AnalyzerInput(buffer: converted))
                }
            }
        } else {
            input.installTap(onBus: 0, bufferSize: 1024, format: hwFormat) { [weak self] buffer, _ in
                self?.recognitionRequest?.append(buffer)
            }
        }

        engine.prepare()
        try engine.start()
    }

    private func deactivateAudioSession() {
        do {
            try AVAudioSession.sharedInstance().setActive(false, options: .notifyOthersOnDeactivation)
        } catch {
            // Headset audio session may already be idle.
        }
    }

    private func preferredLocale() -> Locale {
        let current = Locale.current
        if current.language.languageCode != nil {
            return current
        }
        return Locale(identifier: "en-US")
    }

    private static func plainText(_ attributed: AttributedString) -> String {
        String(attributed.characters)
    }
}
