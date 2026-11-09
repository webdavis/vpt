import CoreMedia
import Foundation
import Speech

@available(macOS 26, *)
func appleSegment(text: AttributedString, range: CMTimeRange) throws -> EngineSegment {
    guard text.characters.count <= 65536 else { throw SpeechFailure.outputLimit }
    var words: [EngineWord] = []
    for run in text.runs {
        let word = String(text[run.range].characters)
            .trimmingCharacters(in: .whitespacesAndNewlines)
        if word.isEmpty {
            continue
        }
        guard words.count < 65536 else { throw SpeechFailure.outputLimit }
        guard let time = run.audioTimeRange else {
            guard word.unicodeScalars.allSatisfy(CharacterSet.punctuationCharacters.contains) else {
                throw SpeechFailure.invalidOutput
            }
            continue
        }
        words.append(EngineWord(text: word, start: time.start.seconds, end: time.end.seconds,
                                confidence: run.transcriptionConfidence))
    }
    return EngineSegment(start: range.start.seconds, end: range.end.seconds,
                         text: String(text.characters), words: words)
}
