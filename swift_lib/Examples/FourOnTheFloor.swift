// FourOnTheFloor.swift
//
// Write a 4-on-the-floor kick to clip 1 of track 1, then read it back.
//
// This example uses LoopbackTransport — an in-memory fake MC device — so
// it runs end-to-end without any MIDI hardware. The MCClient calls below
// are exactly what you would write against a real device; just swap the
// transport for a CoreMIDITransport when you're ready to drive hardware:
//
//     let transport = try CoreMIDITransport(deviceName: "MC-707")

import Foundation
import MCProgrammer

enum FourOnTheFloor {
    static func run() throws {
        let client = MCClient(transport: LoopbackTransport(), modelID: .mc707)

        // 1) Wipe the first 16 steps of clip 1, track 1.
        for s in 0..<16 {
            try client.writeNoteStep(track: 0, clip: 0, step: s, NoteStep())
        }

        // 2) Place a kick (C1 = MIDI note 36) on every downbeat.
        for s in [0, 4, 8, 12] {
            var step = NoteStep()
            step.components[0] = NoteComponent(note: 36, velocity: 120)
            try client.writeNoteStep(track: 0, clip: 0, step: s, step)
        }

        // 3) Read the steps back to verify.
        var hits: [Int] = []
        for s in 0..<16 {
            let step = try client.readNoteStep(track: 0, clip: 0, step: s)
            if !step.activeComponents.isEmpty {
                hits.append(s)
            }
        }
        print("Steps with notes: \(hits)")
        print("Step 4 lane 0 note: \(try client.readNoteStep(track: 0, clip: 0, step: 4).components[0].note)")
    }
}
