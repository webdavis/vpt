import Foundation

func transcribeOutcome(
    path: String,
    locale: String,
    transcriber: some Transcriber
) async -> Outcome {
    do {
        let transcript = try await transcriber.transcribe(
            audio: URL(fileURLWithPath: path),
            locale: locale
        )
        try validateTranscript(transcript)
        let document = EngineDocument(language: transcript.language, segments: transcript.segments)
        let text = try encode(document) + "\n"
        guard text.utf8.count <= 16_777_216 else { throw SpeechFailure.outputLimit }
        return Outcome(stdout: text, stderr: "", exitCode: 0)
    } catch {
        let failure = (error as? SpeechFailure) ?? .failed
        return Outcome(stdout: "", stderr: "vpt-macos: \(failure.message)\n", exitCode: 1)
    }
}
