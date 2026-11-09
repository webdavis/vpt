import Foundation
import VptMacos

let outcome = await execute(
    arguments: Array(CommandLine.arguments.dropFirst()),
    trasher: FileManagerTrasher()
)
let status = emit(
    outcome,
    writeStdout: { try FileHandle.standardOutput.write(contentsOf: $0) },
    writeStderr: { try FileHandle.standardError.write(contentsOf: $0) }
)
exit(status)
