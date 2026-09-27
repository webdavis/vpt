import Foundation

struct VersionDocument: Encodable {
    let schema = "vpt.helper/1"
    let version = "1.0.0"

    init() {}
}

struct TrashedDocument: Encodable {
    let trashed: String
}

func encode(_ document: some Encodable) throws -> String {
    let encoder = JSONEncoder()
    encoder.outputFormatting = [.sortedKeys, .withoutEscapingSlashes]
    let data = try encoder.encode(document)
    // JSONEncoder emits UTF-8. swiftlint:disable:next optional_data_string_conversion
    return String(decoding: data, as: UTF8.self)
}
