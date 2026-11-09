import Foundation
@testable import VptMacos
import XCTest

private final class TemporaryTrasher: Trasher {
    let directory: URL
    private(set) var received: [URL] = []

    init(directory: URL) {
        self.directory = directory
    }

    func trash(_ url: URL) throws {
        received.append(url)
        try FileManager.default.moveItem(
            at: url,
            to: directory.appendingPathComponent(url.lastPathComponent)
        )
    }
}

private struct TrashFailure: Error, LocalizedError {
    let detail: String

    var errorDescription: String? {
        detail
    }
}

private struct FailingTrasher: Trasher {
    func trash(_: URL) throws {
        throw TrashFailure(detail: "blocked")
    }
}

private enum EncodingMarker: Error, Equatable {
    case expected
}

private struct FailingDocument: Encodable {
    func encode(to _: any Encoder) throws {
        throw EncodingMarker.expected
    }
}

@MainActor final class RunTests: XCTestCase {
    private func makeSUT(
        directory: URL,
        file: StaticString = #filePath,
        line: UInt = #line
    ) -> TemporaryTrasher {
        let trasher = TemporaryTrasher(directory: directory)
        trackForMemoryLeaks(trasher, file: file, line: line)
        return trasher
    }

    private func trackForMemoryLeaks(
        _ instance: AnyObject,
        file: StaticString = #filePath,
        line: UInt = #line
    ) {
        addTeardownBlock { [weak instance] in
            XCTAssertNil(instance, "Potential memory leak", file: file, line: line)
        }
    }

    private func scratch() throws -> URL {
        let base = try FixtureScratch.make()
        addTeardownBlock { try FileManager.default.removeItem(at: base) }
        return base
    }

    func testVersionPrintsExactDocumentAndEmptyError() async throws {
        let base = try scratch()
        let trasher = makeSUT(directory: base.appendingPathComponent("trash"))
        let outcome = await VptMacos.run(.version, trasher: trasher)
        XCTAssertEqual(
            outcome,
            Outcome(
                stdout: "{\"schema\":\"vpt.helper/1\",\"version\":\"1.0.0\"}\n",
                stderr: "",
                exitCode: 0
            )
        )
        XCTAssertTrue(trasher.received.isEmpty)
    }

    func testTrashMovesOnlyOwnedFixtureThroughTheProtocol() async throws {
        let base = try scratch()
        let victim = base.appendingPathComponent("victim.txt")
        try Data("fixture".utf8).write(to: victim)
        let trasher = makeSUT(directory: base.appendingPathComponent("trash"))

        let outcome = await VptMacos.run(.trash(path: victim.path), trasher: trasher)

        XCTAssertEqual(
            outcome,
            Outcome(stdout: "{\"trashed\":\"\(victim.path)\"}\n", stderr: "", exitCode: 0)
        )
        XCTAssertEqual(trasher.received, [victim])
        XCTAssertFalse(FileManager.default.fileExists(atPath: victim.path))
        XCTAssertEqual(
            try Data(contentsOf: base.appendingPathComponent("trash/victim.txt")),
            Data("fixture".utf8)
        )
    }

    func testTrashFailureKeepsSourceAndWritesOnlyError() async throws {
        let base = try scratch()
        let victim = base.appendingPathComponent("victim.txt")
        try Data("fixture".utf8).write(to: victim)

        let outcome = await VptMacos.run(.trash(path: victim.path), trasher: FailingTrasher())

        XCTAssertEqual(
            outcome,
            Outcome(stdout: "", stderr: "vpt-macos: trash failed: blocked\n", exitCode: 1)
        )
        XCTAssertEqual(try Data(contentsOf: victim), Data("fixture".utf8))
    }

    func testTrashDocumentEscapesHostilePathAndRoundTrips() async throws {
        let base = try scratch()
        let victim = base.appendingPathComponent("quote\"back\\line\n.txt")
        try Data("fixture".utf8).write(to: victim)
        let trasher = makeSUT(directory: base.appendingPathComponent("trash"))

        let outcome = await VptMacos.run(.trash(path: victim.path), trasher: trasher)

        XCTAssertEqual(outcome.exitCode, 0)
        XCTAssertEqual(outcome.stderr, "")
        XCTAssertTrue(outcome.stdout.hasSuffix("\n"))
        let body = String(outcome.stdout.dropLast())
        XCTAssertFalse(body.contains("\n"))
        XCTAssertTrue(body.contains("\\\""))
        XCTAssertTrue(body.contains("\\\\"))
        XCTAssertTrue(body.contains("\\n"))
        let document = try XCTUnwrap(JSONSerialization
            .jsonObject(with: Data(body.utf8)) as? [String: String])
        XCTAssertEqual(document, ["trashed": victim.path])
        XCTAssertEqual(trasher.received, [victim])
    }

    func testEncodingFailurePropagatesWithoutSuccessDocument() {
        XCTAssertThrowsError(try encode(FailingDocument())) { error in
            XCTAssertEqual(error as? EncodingMarker, .expected)
        }
    }

    func testExecutableCompositionDispatchesVersionWithoutTrash() async throws {
        let base = try scratch()
        let trasher = makeSUT(directory: base.appendingPathComponent("trash"))

        let outcome = await execute(arguments: ["--version"], trasher: trasher)
        XCTAssertEqual(
            outcome,
            Outcome(
                stdout: "{\"schema\":\"vpt.helper/1\",\"version\":\"1.0.0\"}\n",
                stderr: "",
                exitCode: 0
            )
        )
        XCTAssertTrue(trasher.received.isEmpty)
    }

    func testExecutableCompositionDispatchesTrashThroughTheAdapter() async throws {
        let base = try scratch()
        let victim = base.appendingPathComponent("victim.txt")
        try Data("fixture".utf8).write(to: victim)
        let trasher = makeSUT(directory: base.appendingPathComponent("trash"))

        let outcome = await execute(arguments: ["trash", victim.path], trasher: trasher)

        XCTAssertEqual(
            outcome,
            Outcome(stdout: "{\"trashed\":\"\(victim.path)\"}\n", stderr: "", exitCode: 0)
        )
        XCTAssertEqual(trasher.received, [victim])
        XCTAssertFalse(FileManager.default.fileExists(atPath: victim.path))
        XCTAssertEqual(
            try Data(contentsOf: base.appendingPathComponent("trash/victim.txt")),
            Data("fixture".utf8)
        )
    }

    func testExecutableCompositionReportsEveryUsageFailureOnStderr() async {
        let cases: [([String], String)] = [
            ([], "unknown subcommand"),
            (["notify", "--title", "x"], "unknown subcommand"),
            (["transcribe", "x"], "transcribe requires one audio path and --locale <tag>"),
            (["--version", "extra"], "--version takes no arguments"),
            (["trash"], "trash takes exactly one path"),
            (["trash", "a", "b"], "trash takes exactly one path"),
            (["trash", ""], "trash takes a nonempty path")
        ]

        for (arguments, message) in cases {
            let outcome = await execute(arguments: arguments, trasher: FailingTrasher())
            XCTAssertEqual(
                outcome,
                Outcome(stdout: "", stderr: "vpt-macos: \(message)\n\(usage)\n", exitCode: 2),
                "arguments: \(arguments)"
            )
        }
    }
}
