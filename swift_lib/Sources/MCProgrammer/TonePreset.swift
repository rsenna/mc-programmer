// TonePreset.swift — capture and restore one ZEN-Core tone from an MC.
//
// This is the per-tone layer. It dumps every byte of the conventional
// ZEN-Core Tone substructures (Common + PMT + Partial 1-4 + Partial Pitch
// Env × 4 + Partial Filter Env × 4 + Partial Amp Env × 4 + Partial LFO × 4
// + Partial EQ × 4) for one track or clip tone, and can write the same
// bytes back to any other track or clip slot — even on a different MC.
//
// Example:
//
//     // Capture the active tone of track 1.
//     let preset = try client.dumpTone(trackNumber: 1)
//     try preset.save(toPath: "my_bass.tone")
//
//     // Later, load it onto track 3, clip 2.
//     let preset = try TonePreset.load(fromPath: "my_bass.tone")
//     try client.loadTone(trackNumber: 3, clipNumber: 2, preset: preset)
//
// The file format is plain JSON with hex byte strings — human-readable,
// diffable, and tagged with the model ID so you can't accidentally
// flash a 707 tone onto a 101.

import Foundation

public let TONE_PRESET_FORMAT = "mc_programmer.tone.v1"

public struct ToneRegion: Codable, Equatable {
    public let offset: UInt32
    public let label: String
    public let data: [UInt8]

    public init(offset: UInt32, label: String, data: [UInt8]) {
        self.offset = offset
        self.label = label
        self.data = data
    }

    // We hand-roll JSON to keep the format identical to the Python side.
    enum CodingKeys: String, CodingKey { case offset, label, data }

    public init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        let offsetStr = try c.decode(String.self, forKey: .offset)
        self.offset = UInt32(offsetStr.dropFirst(2), radix: 16) ?? 0
        self.label = try c.decode(String.self, forKey: .label)
        let hex = try c.decode(String.self, forKey: .data)
        self.data = TonePreset.bytes(fromHex: hex)
    }

    public func encode(to encoder: Encoder) throws {
        var c = encoder.container(keyedBy: CodingKeys.self)
        try c.encode(String(format: "0x%04X", offset), forKey: .offset)
        try c.encode(label, forKey: .label)
        try c.encode(TonePreset.hex(fromBytes: data), forKey: .data)
    }
}

public struct TonePreset: Codable, Equatable {
    public let format: String
    public let model: String
    public var name: String?
    public var regions: [ToneRegion]

    public init(model: String, name: String? = nil, regions: [ToneRegion] = []) {
        self.format = TONE_PRESET_FORMAT
        self.model = model
        self.name = name
        self.regions = regions
    }

    // ---- helpers used by ToneRegion's JSON coding ------------------------

    static func hex(fromBytes data: [UInt8]) -> String {
        data.map { String(format: "%02X", $0) }.joined(separator: " ")
    }

    static func bytes(fromHex s: String) -> [UInt8] {
        s.split(separator: " ").compactMap { UInt8($0, radix: 16) }
    }

    // ---- file I/O --------------------------------------------------------

    public func toJSON() throws -> String {
        let encoder = JSONEncoder()
        encoder.outputFormatting = [.prettyPrinted, .sortedKeys]
        let data = try encoder.encode(self)
        return String(data: data, encoding: .utf8) ?? ""
    }

    public static func fromJSON(_ raw: String) throws -> TonePreset {
        guard let data = raw.data(using: .utf8) else {
            throw MCProgrammerError.valueOutOfRange("invalid UTF-8 in tone preset JSON")
        }
        let preset = try JSONDecoder().decode(TonePreset.self, from: data)
        if preset.format != TONE_PRESET_FORMAT {
            throw MCProgrammerError.valueOutOfRange(
                "not an mc_programmer tone preset (format=\(preset.format))"
            )
        }
        return preset
    }

    public func save(toPath path: String) throws {
        let json = try toJSON()
        try json.write(toFile: path, atomically: true, encoding: .utf8)
    }

    public static func load(fromPath path: String) throws -> TonePreset {
        let raw = try String(contentsOfFile: path, encoding: .utf8)
        return try fromJSON(raw)
    }
}

// MARK: - Region table (which substructures get dumped)
//
// Offsets come from the [ZCore] address map in Roland's Jupiter-X/Xm MIDI
// Implementation v1.06, page 8. The MC family uses the conventional
// ZEN-Core Tone path so the same offsets apply (per the ZC programming-
// video transcript). Sizes come from the auto-generated SECTION_TOTAL_SIZE
// table in ZenCoreData.swift.

public struct ToneRegionSpec {
    public let label: String
    public let offset: UInt32
    public let sectionKey: String
}

public enum ToneRegionTable {
    public static let all: [ToneRegionSpec] = {
        var out: [ToneRegionSpec] = []
        out.append(.init(label: "tone_common", offset: 0x0000, sectionKey: "TONE_COMMON"))
        out.append(.init(label: "tone_mfx",    offset: 0x0100, sectionKey: "TONE_MFX"))
        out.append(.init(label: "tone_pmt",    offset: 0x1000, sectionKey: "TONE_PMT"))

        for n in 1...4 {
            out.append(.init(label: "partial_\(n)",
                             offset: 0x2000 + UInt32(n - 1) * 0x100,
                             sectionKey: "TONE_PARTIAL"))
        }
        for n in 1...4 {
            out.append(.init(label: "partial_pitch_env_\(n)",
                             offset: 0x2400 + UInt32(n - 1) * 0x100,
                             sectionKey: "PARTIAL_PITCH_ENV"))
        }
        for n in 1...4 {
            out.append(.init(label: "partial_filter_env_\(n)",
                             offset: 0x2800 + UInt32(n - 1) * 0x100,
                             sectionKey: "PARTIAL_FILTER_ENV"))
        }
        for n in 1...4 {
            out.append(.init(label: "partial_amp_env_\(n)",
                             offset: 0x2C00 + UInt32(n - 1) * 0x100,
                             sectionKey: "PARTIAL_AMP_ENV"))
        }
        // Partial LFO has a larger stride (0x200) — the struct is 0x11E bytes.
        for n in 1...4 {
            out.append(.init(label: "partial_lfo_\(n)",
                             offset: 0x3000 + UInt32(n - 1) * 0x200,
                             sectionKey: "PARTIAL_LFO"))
        }
        for n in 1...4 {
            out.append(.init(label: "partial_eq_\(n)",
                             offset: 0x3800 + UInt32(n - 1) * 0x100,
                             sectionKey: "PARTIAL_EQ"))
        }

        // SuperNATURAL synth half of the ZCore tone descriptor — the MC's
        // OSC type / structure / cross-mod / unison parameters live here.
        out.append(.init(label: "tone_synth_common",
                         offset: 0x3C00, sectionKey: "TONE_SYNTH_COMMON"))
        out.append(.init(label: "tone_synth_pmt",
                         offset: 0x3D00, sectionKey: "TONE_SYNTH_PMT"))
        for n in 1...4 {
            out.append(.init(label: "tone_synth_partial_\(n)",
                             offset: 0x3E00 + UInt32(n - 1) * 0x100,
                             sectionKey: "TONE_SYNTH_PARTIAL"))
        }
        return out
    }()

    public static func size(forSection key: String) -> UInt32 {
        ZenCoreData.sectionTotalSize[key] ?? 0
    }
}
