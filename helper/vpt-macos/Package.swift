// swift-tools-version: 6.0
import PackageDescription

let package = Package(
    name: "vpt-macos",
    platforms: [.macOS(.v15)],
    targets: [
        .target(name: "VptMacos"),
        .executableTarget(name: "vpt-macos", dependencies: ["VptMacos"]),
        .testTarget(name: "VptMacosTests", dependencies: ["VptMacos"])
    ],
    swiftLanguageModes: [.v6]
)
