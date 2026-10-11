// Sequencer.swift — note-step and motion-step encode / decode for the MC.
//
// Layout (per the MC programming-video transcript)
// =================================================
//
// Each step record occupies 0x100 bytes inside its clip's note-sequence
// area. The first 0x10 bytes are unknown / not decoded here. After that:
//
//   offset 0x10  16 notes        — 2 bytes/note, packed as two 4-bit nibbles
//                                   (e.g. C4 = 0x3C → 0x03 0x0C). An empty
//                                   note slot is encoded as 0x0F 0x0F.
//   offset 0x30  16 velocities   — 1 byte each (0..127)
//   offset 0x40  16 start times  — 2 bytes each, signed cents-of-a-step,
//                                   two's-complement nibble pairs
//                                   (e.g. -10 → 0xF6 → 0x0F 0x06)
//   offset 0x60  16 end times    — 1 byte each, 0..100. 100 ties to next step.
//   offset 0x70  1 byte substep  — 0/1/2/3/4 = none/×2/×3/×4/flam
//
// Polyphony: each step holds up to 16 *components* indexed 0..15. The MC
// firmware packs concurrent notes into the lowest free lane and rebalances
// when notes are deleted, so editor code that wants stable round-trips
// should call `compact()` on a step before encoding.

import Foundation

public let MC_EMPTY_NOTE: UInt8 = 0xFF
public let MC_END_TIME_TIE: UInt8 = 100
public let MC_VELOCITY_DEFAULT: UInt8 = 100
public let MC_END_TIME_DEFAULT: UInt8 = 0x50  // 80 cents

public enum SubStep: UInt8 {
    case none      = 0
    case double    = 1
    case triple    = 2
    case quadruple = 3
    case flam      = 4
}

public struct NoteComponent: Equatable {
    public var note: UInt8           // MIDI note 0..127, or MC_EMPTY_NOTE
    public var velocity: UInt8       // 0..127
    public var startCents: Int8      // -50..+99 in practice
    public var endCents: UInt8       // 0..100; 100 ties into next step
    public var processed: Bool       // scratch flag for tie-resolution code

    public init(
        note: UInt8 = MC_EMPTY_NOTE,
        velocity: UInt8 = MC_VELOCITY_DEFAULT,
        startCents: Int8 = 0,
        endCents: UInt8 = MC_END_TIME_DEFAULT,
        processed: Bool = false
    ) {
        self.note = note
        self.velocity = velocity
        self.startCents = startCents
        self.endCents = endCents
        self.processed = processed
    }

    public var isEmpty: Bool { note == MC_EMPTY_NOTE }
    public var tiesToNext: Bool { endCents == MC_END_TIME_TIE }
}

public struct NoteStep: Equatable {
    public var components: [NoteComponent]
    public var substep: SubStep

    public init(
        components: [NoteComponent] = Array(repeating: NoteComponent(),
                                            count: NoteStepLayout.notesPerStep),
        substep: SubStep = .none
    ) {
        precondition(components.count == NoteStepLayout.notesPerStep,
                     "NoteStep needs exactly \(NoteStepLayout.notesPerStep) components")
        self.components = components
        self.substep = substep
    }

    public var activeComponents: [NoteComponent] {
        components.filter { !$0.isEmpty }
    }

    /// Push live notes down to the lowest lane indices, the way the MC
    /// firmware does internally.
    public mutating func compact() {
        let live = components.filter { !$0.isEmpty }
        let pad = Array(repeating: NoteComponent(),
                        count: NoteStepLayout.notesPerStep - live.count)
        components = live + pad
    }
}

public enum NoteStepCodec {

    public static func packNibblePair(_ value: UInt8) -> (UInt8, UInt8) {
        return ((value >> 4) & 0x0F, value & 0x0F)
    }

    public static func unpackNibblePair(_ hi: UInt8, _ lo: UInt8) -> UInt8 {
        return ((hi & 0x0F) << 4) | (lo & 0x0F)
    }

    public static func encode(_ step: NoteStep) -> [UInt8] {
        var buf = [UInt8](repeating: 0, count: Int(SequencerMap.stepSize))

        for i in 0..<NoteStepLayout.notesPerStep {
            let comp = step.components[i]

            // notes (nibble-packed) at notesOffs
            let nb = Int(NoteStepLayout.notesOffs) + i * 2
            if comp.isEmpty {
                buf[nb] = 0x0F
                buf[nb + 1] = 0x0F
            } else {
                let (hi, lo) = packNibblePair(comp.note & 0x7F)
                buf[nb] = hi
                buf[nb + 1] = lo
            }

            // velocity (clamp to 0..127)
            buf[Int(NoteStepLayout.velocityOffs) + i] = comp.velocity & 0x7F

            // signed start cents — encode as raw 8-bit two's complement
            let sb = Int(NoteStepLayout.startOffs) + i * 2
            let signedByte = UInt8(bitPattern: comp.startCents)
            let (shi, slo) = packNibblePair(signedByte)
            buf[sb] = shi
            buf[sb + 1] = slo

            // end time (clamped to 0..100)
            let end = min(comp.endCents, MC_END_TIME_TIE)
            buf[Int(NoteStepLayout.endOffs) + i] = end
        }

        buf[Int(NoteStepLayout.substepOffs)] = step.substep.rawValue & 0x7F
        return buf
    }

    public static func decode(_ raw: [UInt8]) throws -> NoteStep {
        let needed = Int(NoteStepLayout.substepOffs) + 1
        if raw.count < needed {
            throw MCProgrammerError.valueOutOfRange("step buffer too short: \(raw.count)")
        }

        var components = [NoteComponent]()
        components.reserveCapacity(NoteStepLayout.notesPerStep)

        for i in 0..<NoteStepLayout.notesPerStep {
            let nb = Int(NoteStepLayout.notesOffs) + i * 2
            let noteByte = unpackNibblePair(raw[nb], raw[nb + 1])

            let vel = raw[Int(NoteStepLayout.velocityOffs) + i] & 0x7F

            let sb = Int(NoteStepLayout.startOffs) + i * 2
            let startByte = unpackNibblePair(raw[sb], raw[sb + 1])
            let startCents = Int8(bitPattern: startByte)

            let endCents = raw[Int(NoteStepLayout.endOffs) + i]

            components.append(
                NoteComponent(
                    note: noteByte,
                    velocity: vel,
                    startCents: startCents,
                    endCents: endCents
                )
            )
        }

        let substepByte = raw[Int(NoteStepLayout.substepOffs)] & 0x7F
        let substep = SubStep(rawValue: substepByte) ?? .none

        return NoteStep(components: components, substep: substep)
    }

    /// Render a raw 256-byte note step as a hex dump and decoded lane list.
    /// Unknown bytes remain visible in the raw dump.
    public static func format(_ raw: [UInt8]) throws -> String {
        let expected = Int(SequencerMap.stepSize)
        guard raw.count == expected else {
            throw MCProgrammerError.valueOutOfRange(
                "step record must be exactly \(expected) bytes, got \(raw.count)"
            )
        }

        let step = try decode(raw)
        var output = "MC note step (\(raw.count) bytes)\n"
        output.reserveCapacity(2400)
        for offset in stride(from: 0, to: raw.count, by: 16) {
            appendHexOffset(offset, to: &output)
            output.append(": ")
            for column in 0..<16 {
                if column != 0 { output.append(" ") }
                appendHexByte(raw[offset + column], to: &output)
            }
            output.append("\n")
        }

        output.append("Decoded lanes:\n")
        for (lane, component) in step.components.enumerated() {
            if lane < 10 { output.append("0") }
            output.append(String(lane))
            output.append(": note=")
            if component.isEmpty {
                output.append("empty")
            } else if let pitch = try? Note.name(of: Int(component.note)) {
                output.append(pitch)
                output.append(" (")
                output.append(String(component.note))
                output.append(")")
            } else {
                output.append("invalid 0x")
                appendHexByte(component.note, to: &output)
            }

            output.append(", velocity=")
            output.append(String(component.velocity))
            output.append(", start=")
            if component.startCents >= 0 { output.append("+") }
            output.append(String(component.startCents))
            output.append("c, end=")
            if component.tiesToNext {
                output.append("tie")
            } else {
                output.append(String(component.endCents))
                output.append("c")
            }
            output.append("\n")
        }

        let substep: String
        switch step.substep {
        case .none: substep = "none"
        case .double: substep = "double"
        case .triple: substep = "triple"
        case .quadruple: substep = "quadruple"
        case .flam: substep = "flam"
        }
        output.append("Substep: ")
        output.append(substep)
        output.append(" (raw 0x")
        appendHexByte(raw[Int(NoteStepLayout.substepOffs)], to: &output)
        output.append(")")
        return output
    }

    private static let hexDigits = Array("0123456789ABCDEF")

    private static func appendHexByte(_ byte: UInt8, to output: inout String) {
        output.append(hexDigits[Int(byte >> 4)])
        output.append(hexDigits[Int(byte & 0x0F)])
    }

    private static func appendHexOffset(_ offset: Int, to output: inout String) {
        output.append(hexDigits[(offset >> 12) & 0x0F])
        output.append(hexDigits[(offset >> 8) & 0x0F])
        output.append(hexDigits[(offset >> 4) & 0x0F])
        output.append(hexDigits[offset & 0x0F])
    }
}

// MARK: - Motion lanes

public struct MotionStep {
    /// Eight lane slots; each lane carries 8 substep values when present.
    public var lanes: [[UInt8]?]

    public init(lanes: [[UInt8]?] = Array(repeating: nil, count: MotionLayout.lanesPerStep)) {
        precondition(lanes.count == MotionLayout.lanesPerStep,
                     "MotionStep requires \(MotionLayout.lanesPerStep) lane slots")
        self.lanes = lanes
    }
}
