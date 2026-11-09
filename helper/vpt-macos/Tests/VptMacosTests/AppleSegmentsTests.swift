import CoreMedia
import Foundation
import Speech
@testable import VptMacos
import XCTest

@available(macOS 26, *)
final class AppleSegmentsTests: XCTestCase {
    private func range(_ start: Double, _ end: Double) -> CMTimeRange {
        CMTimeRange(start: CMTime(seconds: start, preferredTimescale: 1000),
                    end: CMTime(seconds: end, preferredTimescale: 1000))
    }

    func testAttributeRunsKeepWordRangesConfidenceAndOriginalSegmentText() throws {
        var first = AttributedString("Hello")
        first.audioTimeRange = range(0.1, 0.4)
        first.transcriptionConfidence = 0.95
        var second = AttributedString(" world.")
        second.audioTimeRange = range(0.5, 1.2)
        let text = first + second
        let segment = try appleSegment(text: text, range: range(0, 1.5))
        XCTAssertEqual(segment.text, "Hello world.")
        XCTAssertEqual(segment.start, 0)
        XCTAssertEqual(segment.end, 1.5)
        XCTAssertEqual(segment.words.map(\.text), ["Hello", "world."])
        XCTAssertEqual(segment.words[0].start, 0.1, accuracy: 0.0001)
        XCTAssertEqual(segment.words[0].confidence, 0.95)
        XCTAssertNil(segment.words[1].confidence)
    }

    func testUntimedPunctuationStaysInSegmentWithoutInventingWordTiming() throws {
        var word = AttributedString("Hello")
        word.audioTimeRange = range(0.1, 0.4)
        word.transcriptionConfidence = 0.9
        for punctuation in [".", "!", "?", "…", "。", "！", "？", "\"", "\n."] {
            let text = AttributedString("“") + word + AttributedString(punctuation)
            let segment = try appleSegment(text: text, range: range(0, 1))
            XCTAssertEqual(segment.text, "“Hello" + punctuation)
            XCTAssertEqual(segment.words.map(\.text), ["Hello"])
            XCTAssertEqual(segment.words[0].start, 0.1, accuracy: 0.0001)
            XCTAssertEqual(segment.words[0].end, 0.4, accuracy: 0.0001)
            XCTAssertEqual(segment.words[0].confidence, 0.9)
        }
    }

    func testMissingWordTimingFailsWithoutInventingRanges() {
        for text in ["private-canary", "123", "$42", "。word", "é", "你好"] {
            XCTAssertThrowsError(try appleSegment(
                text: AttributedString(text),
                range: range(0, 1)
            ))
        }
    }
}
