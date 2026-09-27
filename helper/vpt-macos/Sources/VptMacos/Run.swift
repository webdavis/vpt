import Foundation

package struct Outcome: Equatable {
    package let stdout: String
    package let stderr: String
    package let exitCode: Int32

    package init(stdout: String, stderr: String, exitCode: Int32) {
        self.stdout = stdout
        self.stderr = stderr
        self.exitCode = exitCode
    }
}

func run(_ command: Command, trasher: some Trasher) -> Outcome {
    switch command {
    case .version:
        return documentOutcome(VersionDocument())
    case let .trash(path):
        do {
            try trasher.trash(URL(fileURLWithPath: path))
            return documentOutcome(TrashedDocument(trashed: path))
        } catch {
            return Outcome(
                stdout: "",
                stderr: "vpt-macos: trash failed: \(error.localizedDescription)\n",
                exitCode: 1
            )
        }
    }
}

package func execute(arguments: [String], trasher: some Trasher) -> Outcome {
    switch parse(arguments) {
    case let .success(command):
        run(command, trasher: trasher)
    case let .failure(error):
        Outcome(
            stdout: "",
            stderr: "vpt-macos: \(error.message)\n\(usage)\n",
            exitCode: 2
        )
    }
}

private func documentOutcome(_ document: some Encodable) -> Outcome {
    do {
        return try Outcome(stdout: encode(document) + "\n", stderr: "", exitCode: 0)
    } catch {
        return Outcome(
            stdout: "",
            stderr: "vpt-macos: encoding failed: \(error.localizedDescription)\n",
            exitCode: 1
        )
    }
}
