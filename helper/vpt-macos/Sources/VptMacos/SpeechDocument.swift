import Foundation

struct EngineDocument: Encodable, Sendable {
    let schema = "vpt.engine/1"
    let engine = AppleEngineMetadata()
    let language: String
    let local = true
    let segments: [EngineSegment]
}

struct AppleEngineMetadata: Encodable, Sendable {
    let name = "apple"
    let tool = "vpt-macos"
    let version = "1.0.0"
    let model = "SpeechTranscriber"
    let family = "apple"
}

struct EngineSegment: Encodable, Sendable {
    let start: Double
    let end: Double
    let text: String
    let words: [EngineWord]
}

struct EngineWord: Encodable, Sendable {
    let text: String
    let start: Double
    let end: Double
    let confidence: Double?
}

package struct SpeechTranscript: Sendable {
    let language: String
    let duration: Double
    let segments: [EngineSegment]
}

package protocol Transcriber: Sendable {
    func transcribe(audio: URL, locale: String) async throws -> SpeechTranscript
}

enum SpeechFailure: Error, Sendable {
    case unsupportedPlatform
    case unavailable
    case unsupportedLocale
    case missingAssets
    case invalidOutput
    case outputLimit
    case failed

    var message: String {
        switch self {
        case .unsupportedPlatform: "Apple speech requires macOS 26 or later"
        case .unavailable: "Apple SpeechTranscriber is unavailable on this Mac"
        case .unsupportedLocale: "Apple speech does not support the requested locale"
        case .missingAssets: "Apple speech assets are not installed for the requested locale"
        case .invalidOutput: "Apple speech returned an invalid transcript"
        case .outputLimit: "Apple speech output exceeded the document limits"
        case .failed: "Apple speech could not complete transcription"
        }
    }
}
