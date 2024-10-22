// swift-tools-version:5.5

import PackageDescription

let package = Package(
  name: "ID3Kit",
  platforms: [
    .macOS(.v10_15),
  ],
  products: [
    .library(name: "ID3Kit", targets: ["ID3Kit"]),
  ],
  targets: [
    .binaryTarget(
      name: "ID3Kit",
      path: "./ID3.xcframework"
    )
  ]
)
