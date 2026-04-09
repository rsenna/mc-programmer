// ChordWithTie.swift
//
// Build a polyphonic chord step and a tied note crossing two steps,
// encode them, and pretty-print the resulting on-device step records.
// Runs offline — no MIDI hardware required.

import Foundation
import MCProgrammer

enum ChordWithTie {

    static func hexdump(_ label: String, _ raw: [UInt8]) {
        print("\(label) (\(raw.count) bytes):")
        var i = 0
        while i < raw.count {
            let row = raw[i..<min(i + 16, raw.count)]
            let hex = row.map { String(format: "%02X", $0) }.joined(separator: " ")
            print(String(format: "  %03X  %@", i, hex as CVarArg))
            i += 16
        }
    }

    static func run() {
        // 1) C-major chord on lanes 0..3 of one step
        var chord = NoteStep()
        for (lane, midi) in [60, 64, 67, 72].enumerated() {
            chord.components[lane] = NoteComponent(
                note: UInt8(midi), velocity: 100, startCents: 0, endCents: 80
            )
        }
        chord.compact()
        chord.substep = .none

        print("== Polyphonic chord step ==")
        for c in chord.activeComponents {
            print("  lane note=\(c.note) vel=\(c.velocity) end=\(c.endCents)c")
        }
        hexdump("encoded chord", NoteStepCodec.encode(chord))

        // 2) Tied note that spans two steps (per the MC video minute ~10 example)
        var stepA = NoteStep()
        stepA.components[0] = NoteComponent(
            note: 48, velocity: 100, startCents: -10, endCents: 100
        )

        var stepB = NoteStep()
        stepB.components[0] = NoteComponent(
            note: 48, velocity: 100, startCents: 0, endCents: 82
        )

        print("\n== Tied note across step A → step B ==")
        print("  step A: note=48 start=-10c end=100c (tie)")
        print("  step B: note=48 start=  0c end= 82c")
        hexdump("encoded step A", NoteStepCodec.encode(stepA))
        hexdump("encoded step B", NoteStepCodec.encode(stepB))
    }
}
