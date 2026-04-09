// swift-tools-version:5.7
import PackageDescription

let package = Package(
    name: "MCProgrammer",
    platforms: [
        .macOS(.v11),
        .iOS(.v14),
    ],
    products: [
        .library(name: "MCProgrammer", targets: ["MCProgrammer"]),
        .executable(name: "MCProgrammerExamples", targets: ["MCProgrammerExamples"]),
    ],
    targets: [
        .target(
            name: "MCProgrammer",
            path: "Sources/MCProgrammer"
        ),
        .executableTarget(
            name: "MCProgrammerExamples",
            dependencies: ["MCProgrammer"],
            path: "Examples"
        ),
        .testTarget(
            name: "MCProgrammerTests",
            dependencies: ["MCProgrammer"],
            path: "Tests/MCProgrammerTests"
        ),
    ]
)
