import Foundation
import Testing
@testable import VptMacos

private final class TrashOperationSpy {
    private(set) var received: [URL] = []
    var failure: OperationFailure?

    func trash(_ url: URL) throws {
        received.append(url)
        if let failure {
            throw failure
        }
    }
}

private enum OperationFailure: Error, Equatable {
    case expected
}

private func makeSUT() -> (sut: FileManagerTrasher, operation: TrashOperationSpy) {
    let operation = TrashOperationSpy()
    let sut = FileManagerTrasher(trashItem: operation.trash)
    return (sut, operation)
}

private func withSUT(
    sourceLocation: SourceLocation = #_sourceLocation,
    _ body: (FileManagerTrasher, TrashOperationSpy) throws -> Void
) rethrows {
    weak var weakOperation: TrashOperationSpy?
    defer {
        #expect(weakOperation == nil, sourceLocation: sourceLocation)
    }
    try {
        let (sut, operation) = makeSUT()
        weakOperation = operation
        try body(sut, operation)
    }()
}

private func fixtureURL() -> URL {
    URL(fileURLWithPath: #filePath)
        .deletingLastPathComponent()
        .deletingLastPathComponent()
        .deletingLastPathComponent()
        .appendingPathComponent(".build/isolated/tmp/fixture-\(UUID().uuidString)")
}

@Test func adapterForwardsExactURLToInjectedOperation() throws {
    try withSUT { sut, operation in
        let url = fixtureURL()

        try sut.trash(url)

        #expect(operation.received == [url])
    }
}

@Test func adapterPropagatesOriginalOperationError() {
    withSUT { sut, operation in
        operation.failure = OperationFailure.expected
        let url = fixtureURL()

        #expect(throws: OperationFailure.expected) {
            try sut.trash(url)
        }
        #expect(operation.received == [url])
    }
}
