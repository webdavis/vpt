@testable import VptMacos
import XCTest

final class ArgumentsTests: XCTestCase {
    func testVersionAndTrashParseExactly() {
        XCTAssertEqual(parse(["--version"]), .success(.version))
        XCTAssertEqual(parse(["trash", "/tmp/owned"]), .success(.trash(path: "/tmp/owned")))
    }

    func testMalformedAndUnsupportedArgumentsAreUsageErrors() {
        XCTAssertEqual(parse([]), .failure(UsageError(message: "unknown subcommand")))
        XCTAssertEqual(
            parse(["notify", "--title", "x"]),
            .failure(UsageError(message: "unknown subcommand"))
        )
        XCTAssertEqual(
            parse(["transcribe", "x"]),
            .failure(UsageError(message: "transcribe requires one audio path and --locale <tag>"))
        )
        XCTAssertEqual(
            parse(["--version", "extra"]),
            .failure(UsageError(message: "--version takes no arguments"))
        )
        XCTAssertEqual(
            parse(["trash"]),
            .failure(UsageError(message: "trash takes exactly one path"))
        )
        XCTAssertEqual(
            parse(["trash", "a", "b"]),
            .failure(UsageError(message: "trash takes exactly one path"))
        )
        XCTAssertEqual(
            parse(["trash", ""]),
            .failure(UsageError(message: "trash takes a nonempty path"))
        )
    }
}
