// main.swift — entry point for the MCProgrammerExamples executable.
//
// Run all examples:    swift run MCProgrammerExamples
// Run one example:     swift run MCProgrammerExamples four
//                      swift run MCProgrammerExamples chord
//                      swift run MCProgrammerExamples coarse

import Foundation

let args = CommandLine.arguments.dropFirst()
let pick = args.first?.lowercased() ?? "all"

func banner(_ s: String) {
    print("\n────────────────────────────────────────")
    print("  \(s)")
    print("────────────────────────────────────────")
}

do {
    if pick == "all" || pick == "four" {
        banner("FourOnTheFloor")
        try FourOnTheFloor.run()
    }
    if pick == "all" || pick == "chord" {
        banner("ChordWithTie")
        ChordWithTie.run()
    }
    if pick == "all" || pick == "coarse" {
        banner("ZenCoreCoarseTune")
        try ZenCoreCoarseTune.run()
    }
} catch {
    print("error: \(error)")
    exit(1)
}
