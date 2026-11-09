import Foundation
@testable import VptMacos
import XCTest

private struct FakeTranscriber: Transcriber {
    let transcript: SpeechTranscript
    func transcribe(audio: URL, locale: String) async throws -> SpeechTranscript {
        XCTAssertEqual(audio.path, "/owned/audio with spaces.m4a")
        XCTAssertEqual(locale, "fr-CA")
        return transcript
    }
}

private struct FailedTranscriber: Transcriber {
    let failure: SpeechFailure
    func transcribe(audio _: URL, locale _: String) async throws -> SpeechTranscript {
        throw failure
    }
}

private struct NoTrash: Trasher {
    func trash(_: URL) throws {
        XCTFail("speech invoked Trash")
    }
}

private struct PrivateError: Error, LocalizedError {
    var errorDescription: String? {
        "private-canary"
    }
}

private struct UnexpectedFailure: Transcriber {
    func transcribe(audio _: URL, locale _: String) async throws -> SpeechTranscript {
        throw PrivateError()
    }
}

@MainActor final class TranscriptionTests: XCTestCase {
    func testCommandCompositionRoutesSpeechWithoutTrash() async {
        let transcript = SpeechTranscript(language: "fr-CA", duration: 1.5, segments: [segment()])
        let result = await execute(
            arguments: ["transcribe", "/owned/audio with spaces.m4a", "--locale", "fr-CA"],
            trasher: NoTrash(), transcriber: FakeTranscriber(transcript: transcript)
        )
        XCTAssertEqual(result.exitCode, 0)
        XCTAssertTrue(result.stdout.contains("vpt.engine/1"))
    }

    func testFrameworkErrorDetailsStayPrivate() async {
        let result = await transcribeOutcome(path: "/owned/audio", locale: "fr-CA",
                                             transcriber: UnexpectedFailure())
        XCTAssertEqual(result.exitCode, 1)
        XCTAssertEqual(result.stdout, "")
        XCTAssertFalse(result.stderr.contains("private-canary"))
    }

    private func outcome(_ transcript: SpeechTranscript) async -> Outcome {
        await transcribeOutcome(path: "/owned/audio with spaces.m4a", locale: "fr-CA",
                                transcriber: FakeTranscriber(transcript: transcript))
    }

    private func segment(
        start: Double = 0,
        end: Double = 1.5,
        confidence: Double? = 0.8
    ) -> EngineSegment {
        EngineSegment(start: start, end: end, text: "private-canary",
                      words: [EngineWord(text: "private-canary", start: start, end: end,
                                         confidence: confidence)])
    }

    func testEncodesWordTimingsConfidenceAndEngineMetadata() async throws {
        let result = await outcome(SpeechTranscript(language: "fr-CA", duration: 1.5,
                                                    segments: [segment()]))
        XCTAssertEqual(result.exitCode, 0)
        XCTAssertEqual(result.stderr, "")
        let value = try XCTUnwrap(JSONSerialization
            .jsonObject(with: Data(result.stdout.utf8)) as? [String: Any])
        XCTAssertEqual(value["schema"] as? String, "vpt.engine/1")
        XCTAssertEqual(value["local"] as? Bool, true)
        XCTAssertEqual(value["language"] as? String, "fr-CA")
        let engine = try XCTUnwrap(value["engine"] as? [String: String])
        XCTAssertEqual(engine["name"], "apple")
        XCTAssertEqual(engine["family"], "apple")
        let segments = try XCTUnwrap(value["segments"] as? [[String: Any]])
        let words = try XCTUnwrap(segments[0]["words"] as? [[String: Any]])
        XCTAssertEqual(words[0]["end"] as? Double, 1.5)
        XCTAssertEqual(words[0]["confidence"] as? Double, 0.8)
    }

    func testMissingConfidenceStaysAbsent() async {
        let result = await outcome(SpeechTranscript(language: "fr-CA", duration: 1.5,
                                                    segments: [segment(confidence: nil)]))
        XCTAssertEqual(result.exitCode, 0)
        XCTAssertFalse(result.stdout.contains("confidence"))
    }

    func testInvalidRangesConfidenceAndEmptyWordsNeverProduceSuccess() async {
        let cases = [
            [segment(start: -0.1)], [segment(end: 1.6)], [segment(start: 1, end: 0.5)],
            [segment(end: .infinity)], [segment(confidence: 1.1)], [segment(confidence: .nan)],
            [EngineSegment(start: 0, end: 1, text: "private-canary", words: [])],
            [segment(start: 0.5), segment(start: 0)], []
        ]
        for segments in cases {
            let result = await outcome(SpeechTranscript(language: "fr-CA", duration: 1.5,
                                                        segments: segments))
            XCTAssertEqual(result.exitCode, 1)
            XCTAssertEqual(result.stdout, "")
            XCTAssertFalse(result.stderr.contains("private-canary"))
        }
    }

    func testAvailabilityErrorsHaveSafeUsefulMessages() async {
        for failure in [
            SpeechFailure.unsupportedPlatform,
            .unavailable,
            .unsupportedLocale,
            .missingAssets
        ] {
            let result = await transcribeOutcome(path: "/owned/audio", locale: "en-US",
                                                 transcriber: FailedTranscriber(failure: failure))
            XCTAssertEqual(
                result,
                Outcome(stdout: "", stderr: "vpt-macos: \(failure.message)\n", exitCode: 1)
            )
        }
    }
}
