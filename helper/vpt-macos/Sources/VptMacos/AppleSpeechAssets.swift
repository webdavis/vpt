import Foundation
import Speech

@available(macOS 26, *)
func installedAppleTranscriber(locale: String) async throws -> (SpeechTranscriber, Locale) {
    guard SpeechTranscriber.isAvailable else { throw SpeechFailure.unavailable }
    guard let supported = await SpeechTranscriber
        .supportedLocale(equivalentTo: Locale(identifier: locale))
    else {
        throw SpeechFailure.unsupportedLocale
    }
    guard await SpeechTranscriber.installedLocales.contains(supported) else {
        throw SpeechFailure.missingAssets
    }
    try await AssetInventory.reserve(locale: supported)
    let transcriber = SpeechTranscriber(locale: supported, transcriptionOptions: [],
                                        reportingOptions: [], attributeOptions: [
                                            .audioTimeRange,
                                            .transcriptionConfidence
                                        ])
    guard await AssetInventory.status(forModules: [transcriber]) == .installed else {
        throw SpeechFailure.missingAssets
    }
    return (transcriber, supported)
}
