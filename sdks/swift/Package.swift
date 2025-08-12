// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "SpacetimeDB",
    products: [
        .library(
            name: "SpacetimeDB",
            targets: ["SpacetimeDB"]
        ),
    ],
    targets: [
        .target(
            name: "SpacetimeDB"
        )
    ]
)
