import Foundation

package func emit(
    _ outcome: Outcome,
    writeStdout: (Data) throws -> Void,
    writeStderr: (Data) throws -> Void
) -> Int32 {
    if !outcome.stdout.isEmpty {
        do {
            try writeStdout(Data(outcome.stdout.utf8))
        } catch {
            try? writeStderr(Data("vpt-macos: output failed\n".utf8))
            return 1
        }
    }

    if !outcome.stderr.isEmpty {
        do {
            try writeStderr(Data(outcome.stderr.utf8))
        } catch {
            return 1
        }
    }

    return outcome.exitCode
}
