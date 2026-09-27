import Foundation
import Testing

private func isolatedRoot() -> URL {
    URL(fileURLWithPath: #filePath)
        .deletingLastPathComponent()
        .deletingLastPathComponent()
        .deletingLastPathComponent()
        .appendingPathComponent(".build/isolated")
}

private func canonicalPath(_ url: URL) -> String {
    url.standardizedFileURL.resolvingSymlinksInPath().path
}

@Test func processReceivesIsolatedEnvironment() throws {
    let root = isolatedRoot()
    let expectedPaths = [
        "HOME": root.appendingPathComponent("home"),
        "XDG_STATE_HOME": root.appendingPathComponent("state"),
        "XDG_CONFIG_HOME": root.appendingPathComponent("config"),
        "XDG_CACHE_HOME": root.appendingPathComponent("cache"),
        "XDG_DATA_HOME": root.appendingPathComponent("data"),
        "VPT_CONFIG": root.appendingPathComponent("config/vpt.toml"),
        "TMPDIR": root.appendingPathComponent("tmp")
    ]

    for (key, expected) in expectedPaths {
        let value = try #require(ProcessInfo.processInfo.environment[key], "missing \(key)")
        #expect(canonicalPath(URL(fileURLWithPath: value)) == canonicalPath(expected), "\(key)")
    }
}

@Test func scratchUsesIsolatedFixtureRoot() throws {
    let base = try FixtureScratch.make()
    defer {
        do {
            try FileManager.default.removeItem(at: base)
        } catch {
            Issue.record(error)
        }
    }

    let actual = base.deletingLastPathComponent()
    let expected = isolatedRoot().appendingPathComponent("tmp")
    #expect(canonicalPath(actual) == canonicalPath(expected))
}
