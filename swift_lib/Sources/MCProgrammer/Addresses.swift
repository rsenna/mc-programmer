// Addresses.swift — MC-101 / MC-707 / MV-1 address-map constants and arithmetic.
//
// Values come straight from the test scripts (`trk_clip_tools.py`,
// `zen_core_tools.py`) and the MC / ZC programming-video transcripts.
//
// Address rules: addresses are 28 useful bits laid out across four bytes,
// and **no byte may have its MSB set** (the 0x80808080 mask in the dump
// routines), since they have to fit through the 7-bit MIDI pipe.

import Foundation

public enum SequencerMap {
    public static let baseAddress: UInt32   = 0x20000000
    public static let paramsOffs:  UInt32   = 0x00020000
    public static let noteOffs:    UInt32   = 0x00030000
    public static let motionOffs:  UInt32   = 0x00040000
    public static let pitchbendOffs: UInt32 = 0x00050000

    public static let stepSize:    UInt32   = 0x00000100
    public static let trkOffset:   UInt32   = 0x01080000
    public static let clipOffset:  UInt32   = 0x00080000

    public static let numTracks       = 8
    public static let clipsPerTrack   = 16
    public static let stepsPerClip    = 128

    public static let clip1BaseAddr: [UInt32] = [
        0x20000000, 0x21080000, 0x22100000, 0x23180000,
        0x24200000, 0x25280000, 0x26300000, 0x27380000,
    ]

    public static let trkBaseAddr: [UInt32] = [
        0x21000000, 0x22080000, 0x23100000, 0x24180000,
        0x25200000, 0x26280000, 0x27300000, 0x28380000,
    ]

    public static let trkSettingsBaseAddr: [UInt32] = [
        0x10000800, 0x10000A00, 0x10000C00, 0x10000E00,
        0x10001000, 0x10001200, 0x10001400, 0x10001600,
    ]
    public static let sndSrcCfgOffs: UInt32 = 0x04 // 0=track, 1=clip
    public static let trkColorOffs:  UInt32 = 0x19
    public static let trkTypeOffs:   UInt32 = 0x14 // 0=TONE, 1=DRUM, 0x7F=OFF
}

public enum ZenCoreMap {
    public static let toneAlignment: UInt32 = 0x00020000
    public static let trackBase: [UInt32] = [
        0x30000000, 0x30220000, 0x30440000, 0x30660000,
        0x31080000, 0x312A0000, 0x314C0000, 0x316E0000,
    ]
    /// Offset of the Coarse Tune byte inside a tone-common substructure.
    /// Demonstrated live in the ZC programming video. Range 16..112, center 64.
    public static let toneCommonCoarseTuneOffs: UInt32 = 0x0018
}

public enum NoteStepLayout {
    public static let notesOffs:   UInt32 = 0x10  // 16 notes × 2 bytes (nibble pairs)
    public static let velocityOffs: UInt32 = 0x30 // 16 velocities × 1 byte
    public static let startOffs:   UInt32 = 0x40  // 16 start times × 2 bytes (signed cents)
    public static let endOffs:     UInt32 = 0x60  // 16 end times × 1 byte (0..100)
    public static let substepOffs: UInt32 = 0x70  // 1 byte
    public static let notesPerStep        = 16
}

public enum MotionLayout {
    public static let lanesPerStep        = 8
    public static let laneOffset: UInt32  = 0x10
    public static let substeps            = 8
    public static let laneNames: [String] = [
        "C1/FILTER", "C2/MOD", "C3/FX", "C4/SOUND",
        "Ext 0x40", "Ext 0x50", "Ext 0x60", "Ext 0x70",
    ]
}

public enum StepScale: UInt8, CaseIterable {
    case eighth          = 0
    case sixteenth       = 1
    case thirtySecond    = 2
    case quarterTriplet  = 3
    case eighthTriplet   = 4
    case sixteenthTriplet = 5

    public var label: String {
        switch self {
        case .eighth: return "1/8"
        case .sixteenth: return "1/16"
        case .thirtySecond: return "1/32"
        case .quarterTriplet: return "1/4T"
        case .eighthTriplet: return "1/8T"
        case .sixteenthTriplet: return "1/16T"
        }
    }
}

/// Caches per-track lists of clip base addresses (skipping any address that
/// would have a high MSB byte) and computes derived addresses by arithmetic.
public struct AddressMap {
    public let clipBaseAddresses: [[UInt32]]

    public init() {
        var rows: [[UInt32]] = []
        for trk in 0..<SequencerMap.numTracks {
            var row: [UInt32] = [SequencerMap.clip1BaseAddr[trk]]
            for _ in 1..<SequencerMap.clipsPerTrack {
                row.append(AddressMap.nextValidBaseAddress(row.last!))
            }
            rows.append(row)
        }
        self.clipBaseAddresses = rows
    }

    private static func nextValidBaseAddress(_ previous: UInt32, offset: UInt32 = SequencerMap.clipOffset) -> UInt32 {
        var next = previous &+ offset
        while (next & 0x80808080) != 0 {
            next = next &+ offset
        }
        return next
    }

    // MARK: - Track / clip base addresses

    public func trackBase(_ trk: Int) throws -> UInt32 {
        try check(trk, max: SequencerMap.numTracks, label: "track")
        return SequencerMap.trkBaseAddr[trk]
    }

    public func trackSettingsBase(_ trk: Int) throws -> UInt32 {
        try check(trk, max: SequencerMap.numTracks, label: "track")
        return SequencerMap.trkSettingsBaseAddr[trk]
    }

    public func clipBase(track: Int, clip: Int) throws -> UInt32 {
        try check(track, max: SequencerMap.numTracks, label: "track")
        try check(clip,  max: SequencerMap.clipsPerTrack, label: "clip")
        return clipBaseAddresses[track][clip]
    }

    // MARK: - Sequencer step addresses

    public func clipParamsAddress(track: Int, clip: Int) throws -> UInt32 {
        return try clipBase(track: track, clip: clip) &+ SequencerMap.paramsOffs
    }

    public func noteStepAddress(track: Int, clip: Int, step: Int) throws -> UInt32 {
        try check(step, max: SequencerMap.stepsPerClip, label: "step")
        return try clipBase(track: track, clip: clip)
            &+ SequencerMap.noteOffs
            &+ UInt32(step) &* SequencerMap.stepSize
    }

    public func motionStepAddress(track: Int, clip: Int, step: Int, lane: Int = 0) throws -> UInt32 {
        try check(step, max: SequencerMap.stepsPerClip, label: "step")
        try check(lane, max: MotionLayout.lanesPerStep, label: "motion lane")
        return try clipBase(track: track, clip: clip)
            &+ SequencerMap.motionOffs
            &+ UInt32(step) &* SequencerMap.stepSize
            &+ UInt32(lane) &* MotionLayout.laneOffset
    }

    // MARK: - ZEN-Core tone descriptor addresses

    /// Pass `clip == nil` for the *track* tone (loaded when track sound source
    /// is TRACK), or 0..15 for one of the clip tones.
    public func zenCoreToneAddress(track: Int, clip: Int? = nil) throws -> UInt32 {
        try check(track, max: SequencerMap.numTracks, label: "track")
        let base = ZenCoreMap.trackBase[track]
        if let clip = clip {
            try check(clip, max: SequencerMap.clipsPerTrack, label: "clip")
            return base &+ ZenCoreMap.toneAlignment &* UInt32(clip)
        }
        return base &+ ZenCoreMap.toneAlignment &* UInt32(SequencerMap.clipsPerTrack)
    }

    private func check(_ i: Int, max: Int, label: String) throws {
        if i < 0 || i >= max {
            throw MCProgrammerError.indexOutOfRange("\(label)=\(i) (must be 0..<\(max))")
        }
    }
}
