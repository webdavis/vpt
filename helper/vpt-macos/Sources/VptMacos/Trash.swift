import Foundation

package protocol Trasher {
    func trash(_ url: URL) throws
}

package struct FileManagerTrasher: Trasher {
    private let trashItem: (URL) throws -> Void

    package init() {
        trashItem = { url in try FileManager.default.trashItem(at: url, resultingItemURL: nil) }
    }

    init(trashItem: @escaping (URL) throws -> Void) {
        self.trashItem = trashItem
    }

    package func trash(_ url: URL) throws {
        try trashItem(url)
    }
}
