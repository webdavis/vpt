import Foundation

enum FixtureScratch {
    static func make() throws -> URL {
        let temporary = URL(fileURLWithPath: #filePath)
            .deletingLastPathComponent()
            .deletingLastPathComponent()
            .deletingLastPathComponent()
            .appendingPathComponent(".build/isolated/tmp")
        let base = temporary
            .appendingPathComponent("vpt-macos-tests-\(UUID().uuidString)")
        try FileManager.default.createDirectory(
            at: base.appendingPathComponent("trash"),
            withIntermediateDirectories: true
        )
        return base
    }
}
