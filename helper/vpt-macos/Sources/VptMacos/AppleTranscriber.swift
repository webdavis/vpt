import AVFAudio
import Foundation
import Speech

package struct AppleTranscriber: Transcriber {
    package init() {}

    package func transcribe(audio: URL, locale: String) async throws -> SpeechTranscript {
        guard #available(macOS 26, *) else { throw SpeechFailure.unsupportedPlatform }
        let (transcriber, supported) = try await installedAppleTranscriber(locale: locale)
        let file = try AVAudioFile(forReading: audio)
        let duration = Double(file.length) / file.processingFormat.sampleRate
        guard duration.isFinite, duration >= 0 else { throw SpeechFailure.invalidOutput }
        let analyzer = SpeechAnalyzer(modules: [transcriber])
        do {
            try await analyzer.start(inputAudioFile: file, finishAfterFile: true)
            var segments: [EngineSegment] = []
            var encodedBytes = 0
            for try await result in transcriber.results {
                try Task.checkCancellation()
                if !result.isFinal {
                    continue
                }
                guard segments.count < 65536 else { throw SpeechFailure.outputLimit }
                let segment = try appleSegment(text: result.text, range: result.range)
                try validateSegment(segment, duration: duration)
                encodedBytes += try encode(segment).utf8.count
                guard encodedBytes <= 16_777_216 else { throw SpeechFailure.outputLimit }
                segments.append(segment)
            }
            return SpeechTranscript(
                language: supported.identifier(.bcp47),
                duration: duration,
                segments: segments
            )
        } catch {
            await analyzer.cancelAndFinishNow()
            throw error
        }
    }
}
