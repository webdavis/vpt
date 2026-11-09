import Foundation

enum Command: Equatable {
    case version
    case trash(path: String)
    case transcribe(path: String, locale: String)
}

struct UsageError: Error, Equatable {
    let message: String
}

let usage = "usage: vpt-macos --version | trash <path> | transcribe <audio-path> --locale <tag>"

func parse(_ arguments: [String]) -> Result<Command, UsageError> {
    switch arguments.first {
    case "transcribe":
        guard arguments.count == 4, arguments[2] == "--locale" else {
            return .failure(
                UsageError(message: "transcribe requires one audio path and --locale <tag>")
            )
        }
        let path = arguments[1]
        let locale = arguments[3]
        guard !path.isEmpty, !path.contains("\0"),
              !locale.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty,
              locale.rangeOfCharacter(from: .controlCharacters) == nil
        else {
            return .failure(UsageError(message: "transcribe requires a nonempty path and locale"))
        }
        return .success(.transcribe(path: path, locale: locale))
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
