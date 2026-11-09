@testable import VptMacos
import XCTest

final class TranscribeArgumentsTests: XCTestCase {
    func testAcceptsOneAudioPathAndPreservesRegionalLocale() {
        XCTAssertEqual(parse(["transcribe", "/owned/audio with spaces.m4a", "--locale", "fr-CA"]),
                       .success(.transcribe(path: "/owned/audio with spaces.m4a", locale: "fr-CA")))
    }

    func testRefusesMissingDuplicatedAndEmptyOptions() {
        for arguments in [
            ["transcribe", "audio"], ["transcribe", "audio", "--language", "en"],
            ["transcribe", "", "--locale", "en"], ["transcribe", "audio", "--locale", " "],
            ["transcribe", "audio", "--locale", "en\nUS"],
            ["transcribe", "audio", "--locale", "en", "extra"]
        ] {
            guard case .failure = parse(arguments) else {
                XCTFail("invalid speech arguments accepted")
                continue
            }
        }
    }
}
