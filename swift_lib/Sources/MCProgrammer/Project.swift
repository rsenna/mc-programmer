// Project.swift — full-state dump and restore for an entire MC.
//
// A `Project` is the everything-snapshot of an MC: every clip's note
// steps, motion lanes, and pitch-bend recordings; every track-tone and
// clip-tone; per-track settings; and the device's system block.
//
// Layered above `TonePreset` (one tone) — `Project.dumpFrom(client:)`
// just calls `client.dumpTone(...)` 8 + 128 times under the hood, plus
// the per-clip sequencer reads and the system block.
//
// Example:
//
//     // Snapshot the live device:
//     let project = try Project.dumpFrom(client: client, name: "live set")
//     try project.save(toPath: "liveset.mctproject")
//
//     // Later (or on a different MC of the same model):
//     let project = try Project.load(fromPath: "liveset.mctproject")
//     try project.loadInto(client: client)
//
// What's NOT in v1: the scene block (no MC-verified address), arpeggio
// user patterns. Use `client.read(address:count:)` to dump those manually.

import Foundation

public let PROJECT_FORMAT = "mc_programmer.project.v1"

/// Tiny system header. `hack_tools.py` is known to read this range.
public let SYSTEM_HEADER_ADDRESS: UInt32 = 0x00000000
public let SYSTEM_HEADER_SIZE = 0x30

/// System / setup block. Verified by `hack_tools.dump_settings(basic=False)`.
public let SYSTEM_BLOCK_ADDRESS: UInt32 = 0x10000000
public let SYSTEM_BLOCK_SIZE = 0x4000

public let TRACK_SETTINGS_SIZE = 0x20

public struct ClipDump: Codable, Equatable {
    public var clipNumber: Int
    public var stepScale: Int
    public var stepCount: Int
    public var noteSteps: [UInt8]
    public var motionSteps: [UInt8]
    public var pitchbend: [UInt8]
    public var tone: TonePreset

    enum CodingKeys: String, CodingKey {
        case clipNumber = "clip_number"
        case stepScale = "step_scale"
        case stepCount = "step_count"
        case noteSteps = "note_steps"
        case motionSteps = "motion_steps"
        case pitchbend
        case tone
    }

    public init(clipNumber: Int, stepScale: Int, stepCount: Int,
                noteSteps: [UInt8], motionSteps: [UInt8],
                pitchbend: [UInt8], tone: TonePreset) {
        self.clipNumber = clipNumber
        self.stepScale = stepScale
        self.stepCount = stepCount
        self.noteSteps = noteSteps
        self.motionSteps = motionSteps
        self.pitchbend = pitchbend
        self.tone = tone
    }

    public init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        self.clipNumber = try c.decode(Int.self, forKey: .clipNumber)
        self.stepScale = try c.decode(Int.self, forKey: .stepScale)
        self.stepCount = try c.decode(Int.self, forKey: .stepCount)
        self.noteSteps = TonePreset.bytes(fromHex: try c.decode(String.self, forKey: .noteSteps))
        self.motionSteps = TonePreset.bytes(fromHex: try c.decode(String.self, forKey: .motionSteps))
        self.pitchbend = TonePreset.bytes(fromHex: try c.decode(String.self, forKey: .pitchbend))
        self.tone = try c.decode(TonePreset.self, forKey: .tone)
    }

    public func encode(to encoder: Encoder) throws {
        var c = encoder.container(keyedBy: CodingKeys.self)
        try c.encode(clipNumber, forKey: .clipNumber)
        try c.encode(stepScale, forKey: .stepScale)
        try c.encode(stepCount, forKey: .stepCount)
        try c.encode(TonePreset.hex(fromBytes: noteSteps), forKey: .noteSteps)
        try c.encode(TonePreset.hex(fromBytes: motionSteps), forKey: .motionSteps)
        try c.encode(TonePreset.hex(fromBytes: pitchbend), forKey: .pitchbend)
        try c.encode(tone, forKey: .tone)
    }
}

public struct TrackDump: Codable, Equatable {
    public var trackNumber: Int
    public var settings: [UInt8]
    public var trackTone: TonePreset
    public var clips: [ClipDump]

    enum CodingKeys: String, CodingKey {
        case trackNumber = "track_number"
        case settings
        case trackTone = "track_tone"
        case clips
    }

    public init(trackNumber: Int, settings: [UInt8],
                trackTone: TonePreset, clips: [ClipDump]) {
        self.trackNumber = trackNumber
        self.settings = settings
        self.trackTone = trackTone
        self.clips = clips
    }

    public init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        self.trackNumber = try c.decode(Int.self, forKey: .trackNumber)
        self.settings = TonePreset.bytes(fromHex: try c.decode(String.self, forKey: .settings))
        self.trackTone = try c.decode(TonePreset.self, forKey: .trackTone)
        self.clips = try c.decode([ClipDump].self, forKey: .clips)
    }

    public func encode(to encoder: Encoder) throws {
        var c = encoder.container(keyedBy: CodingKeys.self)
        try c.encode(trackNumber, forKey: .trackNumber)
        try c.encode(TonePreset.hex(fromBytes: settings), forKey: .settings)
        try c.encode(trackTone, forKey: .trackTone)
        try c.encode(clips, forKey: .clips)
    }
}

public struct Project: Codable {
    public var format: String
    public var model: String
    public var name: String?
    public var tracks: [TrackDump]
    public var systemHeader: [UInt8]
    public var systemBlock: [UInt8]

    enum CodingKeys: String, CodingKey {
        case format, model, name, tracks
        case systemHeader = "system_header"
        case systemBlock = "system_block"
    }

    public init(model: String, name: String? = nil,
                tracks: [TrackDump] = [],
                systemHeader: [UInt8] = [],
                systemBlock: [UInt8] = []) {
        self.format = PROJECT_FORMAT
        self.model = model
        self.name = name
        self.tracks = tracks
        self.systemHeader = systemHeader
        self.systemBlock = systemBlock
    }

    public init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        self.format = try c.decode(String.self, forKey: .format)
        self.model = try c.decode(String.self, forKey: .model)
        self.name = try c.decodeIfPresent(String.self, forKey: .name)
        self.tracks = try c.decode([TrackDump].self, forKey: .tracks)
        self.systemHeader = TonePreset.bytes(
            fromHex: (try? c.decode(String.self, forKey: .systemHeader)) ?? "")
        self.systemBlock = TonePreset.bytes(
            fromHex: (try? c.decode(String.self, forKey: .systemBlock)) ?? "")
    }

    public func encode(to encoder: Encoder) throws {
        var c = encoder.container(keyedBy: CodingKeys.self)
        try c.encode(format, forKey: .format)
        try c.encode(model, forKey: .model)
        try c.encode(name, forKey: .name)
        try c.encode(tracks, forKey: .tracks)
        try c.encode(TonePreset.hex(fromBytes: systemHeader), forKey: .systemHeader)
        try c.encode(TonePreset.hex(fromBytes: systemBlock), forKey: .systemBlock)
    }

    // ---- file I/O --------------------------------------------------------

    public func toJSON() throws -> String {
        let encoder = JSONEncoder()
        encoder.outputFormatting = [.prettyPrinted, .sortedKeys]
        return String(data: try encoder.encode(self), encoding: .utf8) ?? ""
    }

    public static func fromJSON(_ raw: String) throws -> Project {
        guard let data = raw.data(using: .utf8) else {
            throw MCProgrammerError.valueOutOfRange("invalid UTF-8 in project JSON")
        }
        let project = try JSONDecoder().decode(Project.self, from: data)
        if project.format != PROJECT_FORMAT {
            throw MCProgrammerError.valueOutOfRange(
                "not an mc_programmer project file (format=\(project.format))")
        }
        return project
    }

    public func save(toPath path: String) throws {
        try toJSON().write(toFile: path, atomically: true, encoding: .utf8)
    }

    public static func load(fromPath path: String) throws -> Project {
        let raw = try String(contentsOfFile: path, encoding: .utf8)
        return try fromJSON(raw)
    }

    // ---- dump (device → memory) ------------------------------------------

    public static func dumpFrom(
        client: MCClient,
        name: String? = nil,
        includeSystem: Bool = true
    ) throws -> Project {
        let modelStr: String
        switch client.modelID {
        case 0x5D: modelStr = "MC-707"
        case 0x5E: modelStr = "MC-101"
        case 0x5F: modelStr = "MV-1"
        default:   modelStr = String(format: "0x%02X", client.modelID)
        }

        var project = Project(model: modelStr, name: name)

        // Walk only the tracks the connected device actually has —
        // MC-101 has 4, MC-707/MV-1 have 8.
        for trackIdx in 0..<client.numTracks {
            let settingsAddr = SequencerMap.trkSettingsBaseAddr[trackIdx]
            let settings = try client.readChunked(address: settingsAddr, count: TRACK_SETTINGS_SIZE)

            let trackTone = try client.dumpTone(trackNumber: trackIdx + 1, clipNumber: nil)

            var clips: [ClipDump] = []
            for clipIdx in 0..<16 {
                let clipBase = try client.addresses.clipBase(track: trackIdx, clip: clipIdx)

                let params = try client.readChunked(
                    address: clipBase &+ SequencerMap.paramsOffs, count: 4)
                let stepScale = params.count >= 1 ? Int(params[0]) : 1
                var stepCount = params.count >= 3
                    ? ((Int(params[1]) << 4) | Int(params[2]))
                    : 16
                stepCount = max(1, min(128, stepCount))

                let length = stepCount * Int(SequencerMap.stepSize)
                let noteSteps = try client.readChunked(
                    address: clipBase &+ SequencerMap.noteOffs, count: length)
                let motionSteps = try client.readChunked(
                    address: clipBase &+ SequencerMap.motionOffs, count: length)
                let pitchbend = try client.readChunked(
                    address: clipBase &+ SequencerMap.pitchbendOffs, count: length)

                let clipTone = try client.dumpTone(
                    trackNumber: trackIdx + 1, clipNumber: clipIdx + 1)

                clips.append(ClipDump(
                    clipNumber: clipIdx + 1,
                    stepScale: stepScale,
                    stepCount: stepCount,
                    noteSteps: noteSteps,
                    motionSteps: motionSteps,
                    pitchbend: pitchbend,
                    tone: clipTone
                ))
            }

            project.tracks.append(TrackDump(
                trackNumber: trackIdx + 1,
                settings: settings,
                trackTone: trackTone,
                clips: clips
            ))
        }

        if includeSystem {
            project.systemHeader = try client.readChunked(
                address: SYSTEM_HEADER_ADDRESS, count: SYSTEM_HEADER_SIZE)
            project.systemBlock = try client.readChunked(
                address: SYSTEM_BLOCK_ADDRESS, count: SYSTEM_BLOCK_SIZE)
        }
        return project
    }

    // ---- restore (memory → device) ---------------------------------------

    public func loadInto(client: MCClient, force: Bool = false) throws {
        let expected: String
        switch client.modelID {
        case 0x5D: expected = "MC-707"
        case 0x5E: expected = "MC-101"
        case 0x5F: expected = "MV-1"
        default:   expected = String(format: "0x%02X", client.modelID)
        }
        if !force && !model.isEmpty && model != expected {
            throw MCProgrammerError.valueOutOfRange(
                "project model '\(model)' doesn't match the client's model '\(expected)'. " +
                "Pass force: true to override."
            )
        }

        for trackDump in tracks {
            let trackIdx = trackDump.trackNumber - 1
            try client.writeChunked(
                address: SequencerMap.trkSettingsBaseAddr[trackIdx],
                data: trackDump.settings)
            try client.loadTone(trackNumber: trackDump.trackNumber,
                                clipNumber: nil, preset: trackDump.trackTone)

            for clipDump in trackDump.clips {
                let clipIdx = clipDump.clipNumber - 1
                let clipBase = try client.addresses.clipBase(track: trackIdx, clip: clipIdx)

                try client.writeChunked(
                    address: clipBase &+ SequencerMap.paramsOffs,
                    data: [
                        UInt8(clipDump.stepScale & 0x7F),
                        UInt8((clipDump.stepCount >> 4) & 0x0F),
                        UInt8(clipDump.stepCount & 0x0F),
                        0x00,
                    ])
                if !clipDump.noteSteps.isEmpty {
                    try client.writeChunked(
                        address: clipBase &+ SequencerMap.noteOffs,
                        data: clipDump.noteSteps)
                }
                if !clipDump.motionSteps.isEmpty {
                    try client.writeChunked(
                        address: clipBase &+ SequencerMap.motionOffs,
                        data: clipDump.motionSteps)
                }
                if !clipDump.pitchbend.isEmpty {
                    try client.writeChunked(
                        address: clipBase &+ SequencerMap.pitchbendOffs,
                        data: clipDump.pitchbend)
                }

                try client.loadTone(trackNumber: trackDump.trackNumber,
                                    clipNumber: clipDump.clipNumber,
                                    preset: clipDump.tone)
            }
        }

        if !systemHeader.isEmpty {
            try client.writeChunked(address: SYSTEM_HEADER_ADDRESS, data: systemHeader)
        }
        if !systemBlock.isEmpty {
            try client.writeChunked(address: SYSTEM_BLOCK_ADDRESS, data: systemBlock)
        }
    }
}
