// ZenCoreCoarseTune.swift
//
// Mirror of the original `zen_core_tools.py` demo: nudge coarse tune five
// times for the track-2 ZEN-Core tone, reading back the value each time.
//
// Runs end-to-end against LoopbackTransport (an in-memory fake MC). To
// drive real hardware, swap the transport for a CoreMIDITransport.

import Foundation
import MCProgrammer

enum ZenCoreCoarseTune {
    static func run() throws {
        let transport = LoopbackTransport()
        let client = MCClient(transport: transport, modelID: .mc707)

        // Initialise the device's coarse-tune parameter to center (64).
        try client.writeCoarseTune(track: 1, clip: nil, value: 64)
        print("starting coarse tune = \(try client.readCoarseTune(track: 1, clip: nil))")

        // Nudge it +1 semitone five times.
        for i in 0..<5 {
            let v = try client.nudgeCoarseTune(track: 1, clip: nil, by: +1)
            print("step \(i + 1): coarse tune now = \(v)")
        }
    }
}
