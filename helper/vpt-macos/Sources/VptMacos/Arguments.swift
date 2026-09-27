enum Command: Equatable {
    case version
    case trash(path: String)
}

struct UsageError: Error, Equatable {
    let message: String
}

let usage = "usage: vpt-macos --version | trash <path>"

func parse(_ arguments: [String]) -> Result<Command, UsageError> {
    switch arguments.first {
    case "--version":
        guard arguments.count == 1 else {
            return .failure(UsageError(message: "--version takes no arguments"))
        }
        return .success(.version)
    case "trash":
        guard arguments.count == 2 else {
            return .failure(UsageError(message: "trash takes exactly one path"))
        }
        guard !arguments[1].isEmpty else {
            return .failure(UsageError(message: "trash takes a nonempty path"))
        }
        return .success(.trash(path: arguments[1]))
    default:
        return .failure(UsageError(message: "unknown subcommand"))
    }
}
