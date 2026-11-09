import Foundation

func validateTranscript(_ transcript: SpeechTranscript) throws {
    guard transcript.duration.isFinite, transcript.duration >= 0,
          !transcript.language.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty,
          !transcript.segments.isEmpty else { throw SpeechFailure.invalidOutput }
    guard transcript.segments.count <= 65536 else { throw SpeechFailure.outputLimit }
    var previous = 0.0
    var wordCount = 0
    for segment in transcript.segments {
        guard segment.start >= previous else { throw SpeechFailure.invalidOutput }
        try validateSegment(segment, duration: transcript.duration)
        wordCount += segment.words.count
        previous = segment.start
    }
    guard wordCount > 0 else { throw SpeechFailure.invalidOutput }
}

func validateSegment(_ segment: EngineSegment, duration: Double) throws {
    try validateRange(segment.start, segment.end, within: 0 ... duration)
    guard segment.text.unicodeScalars.count <= 65536, segment.words.count <= 65536 else {
        throw SpeechFailure.outputLimit
    }
    guard !segment.text.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty else {
        throw SpeechFailure.invalidOutput
    }
    var previous = segment.start
    for word in segment.words {
        try validateRange(word.start, word.end, within: segment.start ... segment.end)
        guard word.start >= previous,
              !word.text.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
        else {
            throw SpeechFailure.invalidOutput
        }
        guard word.text.unicodeScalars.count <= 65536 else { throw SpeechFailure.outputLimit }
        if let confidence = word.confidence {
            guard confidence.isFinite, (0 ... 1).contains(confidence) else {
                throw SpeechFailure.invalidOutput
            }
        }
        previous = word.start
    }
}

private func validateRange(
    _ start: Double,
    _ end: Double,
    within range: ClosedRange<Double>
) throws {
    guard start.isFinite, end.isFinite, start <= end,
          range.contains(start), range.contains(end) else { throw SpeechFailure.invalidOutput }
}
