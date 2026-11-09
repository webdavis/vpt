import Foundation
@testable import VptMacos
import XCTest

@MainActor final class NativeSpeechTests: XCTestCase {
    func testInstalledEnglishAssetsTranscribeGeneratedSpeech() async throws {
        guard let path = ProcessInfo.processInfo.environment["VPT_TEST_APPLE_SPEECH_AUDIO"] else {
            throw XCTSkip("Set VPT_TEST_APPLE_SPEECH_AUDIO to generated quick-brown-fox speech")
        }
        let transcript = try await AppleTranscriber().transcribe(
            audio: URL(fileURLWithPath: path), locale: "en-US"
        )
        try validateTranscript(transcript)
        XCTAssertEqual(transcript.language, "en-US")
        let text = transcript.segments.map(\.text).joined(separator: " ").lowercased()
        XCTAssertTrue(text.contains("quick brown fox"))
        XCTAssertTrue(text.contains("lazy dog"))
        let words = transcript.segments.flatMap(\.words)
        XCTAssertGreaterThanOrEqual(words.count, 9)
        for word in words {
            let confidence = try XCTUnwrap(word.confidence)
            XCTAssertTrue(confidence.isFinite && (0 ... 1).contains(confidence))
            XCTAssertGreaterThan(word.end, word.start)
        }
    }
}
