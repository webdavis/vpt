import Foundation
import Testing
@testable import VptMacos

private enum WriteFailure: Error {
    case injected
}

private final class OutputSpy {
    private(set) var stdout = Data()
    private(set) var stderr = Data()
    private(set) var stdoutCalls = 0
    private(set) var stderrCalls = 0
    var failsStdout = false
    var failsStderr = false

    func writeStdout(_ data: Data) throws {
        stdoutCalls += 1
        if failsStdout {
            throw WriteFailure.injected
        }
        stdout.append(data)
    }

    func writeStderr(_ data: Data) throws {
        stderrCalls += 1
        if failsStderr {
            throw WriteFailure.injected
        }
        stderr.append(data)
    }
}

private func makeSUT() -> OutputSpy {
    OutputSpy()
}

private func withSUT(
    sourceLocation: SourceLocation = #_sourceLocation,
    _ body: (OutputSpy) throws -> Void
) rethrows {
    weak var weakSpy: OutputSpy?
    defer {
        #expect(weakSpy == nil, sourceLocation: sourceLocation)
    }
    try {
        let spy = makeSUT()
        weakSpy = spy
        try body(spy)
    }()
}

@Test func emitsSuccessfulDocumentWithoutTouchingEmptyErrorStream() {
    withSUT { spy in
        let outcome = Outcome(stdout: "{\"version\":\"1.0.0\"}\n", stderr: "", exitCode: 0)

        let status = emit(outcome, writeStdout: spy.writeStdout, writeStderr: spy.writeStderr)

        #expect(status == 0)
        #expect(spy.stdout == Data(outcome.stdout.utf8))
        #expect(spy.stderr.isEmpty)
        #expect(spy.stdoutCalls == 1)
        #expect(spy.stderrCalls == 0)
    }
}

@Test func emitsUsageOnlyToStderrWithOriginalStatus() {
    withSUT { spy in
        let outcome = Outcome(stdout: "", stderr: "vpt-macos: usage\n", exitCode: 2)

        let status = emit(outcome, writeStdout: spy.writeStdout, writeStderr: spy.writeStderr)

        #expect(status == 2)
        #expect(spy.stdout.isEmpty)
        #expect(spy.stderr == Data(outcome.stderr.utf8))
        #expect(spy.stdoutCalls == 0)
        #expect(spy.stderrCalls == 1)
    }
}

@Test func emitsTrashErrorOnlyToStderrWithOriginalStatus() {
    withSUT { spy in
        let outcome = Outcome(stdout: "", stderr: "vpt-macos: trash failed: fixture\n", exitCode: 1)

        let status = emit(outcome, writeStdout: spy.writeStdout, writeStderr: spy.writeStderr)

        #expect(status == 1)
        #expect(spy.stdout.isEmpty)
        #expect(spy.stderr == Data(outcome.stderr.utf8))
        #expect(spy.stdoutCalls == 0)
        #expect(spy.stderrCalls == 1)
    }
}

@Test func stdoutFailureWritesOneBoundedDiagnosticAndExitsOne() {
    withSUT { spy in
        spy.failsStdout = true
        let outcome = Outcome(stdout: "document\n", stderr: "original\n", exitCode: 0)

        let status = emit(outcome, writeStdout: spy.writeStdout, writeStderr: spy.writeStderr)

        #expect(status == 1)
        #expect(spy.stdout.isEmpty)
        #expect(spy.stderr == Data("vpt-macos: output failed\n".utf8))
        #expect(spy.stdoutCalls == 1)
        #expect(spy.stderrCalls == 1)
    }
}

@Test func stderrFailureExitsOneWithoutRetry() {
    withSUT { spy in
        spy.failsStderr = true
        let outcome = Outcome(stdout: "", stderr: "vpt-macos: usage\n", exitCode: 2)

        let status = emit(outcome, writeStdout: spy.writeStdout, writeStderr: spy.writeStderr)

        #expect(status == 1)
        #expect(spy.stdoutCalls == 0)
        #expect(spy.stderrCalls == 1)
    }
}

@Test func diagnosticWriteFailureExitsOneWithoutRetry() {
    withSUT { spy in
        spy.failsStdout = true
        spy.failsStderr = true
        let outcome = Outcome(stdout: "document\n", stderr: "", exitCode: 0)

        let status = emit(outcome, writeStdout: spy.writeStdout, writeStderr: spy.writeStderr)

        #expect(status == 1)
        #expect(spy.stdoutCalls == 1)
        #expect(spy.stderrCalls == 1)
    }
}
