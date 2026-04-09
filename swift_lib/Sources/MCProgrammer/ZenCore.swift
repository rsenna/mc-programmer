// ZenCore.swift — conventional ZEN-Core Tone parameter framework
// for MC-101 / MC-707 / MV-1.
//
// The MC family stores its synth patches as **conventional ZEN-Core Tone**
// descriptors — the PCM/sample-based half of the [ZCore] address block in
// Roland's Jupiter-X/Xm MIDI Implementation. Per the ZC programming-video
// transcript, the parameter offsets within a ZCore tone are the same
// across every Roland ZEN-Core instrument; only the *base address* of
// each tone descriptor differs from one product to the next, and that
// lives in `AddressMap`.
//
// The conventional ZEN-Core Tone is composed of these substructures (the
// SuperNATURAL Synth sub-block at [Synth Partial] is **not** part of this
// path and is intentionally excluded):
//
//   Substructure        Offset       Description
//   ------------------  -----------  -----------------------------------
//   Tone Common         0x0000       Tone Name, Level, Coarse/Fine Tune,
//                                    Octave Shift, Mono/Poly, Portamento,
//                                    Bend Range, etc. (24 params)
//   Tone MFX            0x0100       Multi-effect (not yet exposed)
//   Tone PMT            0x1000       Partial Mix Table (38 params)
//   Tone Partial 1..4   0x2000 +     OSC, filter, amplifier, modulation
//                       n*0x100      matrix, etc. (89 params)
//   Partial Pitch Env   0x2400 +     4 × pitch envelope (14 params each)
//                       n*0x100
//   Partial Filter Env  0x2800 +     4 × filter envelope (14 params each)
//                       n*0x100
//   Partial Amp Env     0x2C00 +     4 × amp envelope (9 params each)
//                       n*0x100
//   Partial LFO         0x3000 +     4 × { LFO 1 + LFO 2 } (98 params each
//                       n*0x200      — LFO struct is 0x11E bytes, so the
//                                    stride between adjacent partials is
//                                    0x200 not 0x100)
//   Partial EQ          0x3800 +     4 × 3-band EQ (8 params each)
//                       n*0x100
//
// The actual parameter table is auto-generated from the Jupiter-X PDF —
// see ZenCoreData.swift and the generator at tools/gen_zencore_params.py.

import Foundation

// MARK: - Substructure offsets within a single conventional ZEN-Core Tone

public enum ZenCoreLayout {
    /// Tone Common — verified at offset 0 by the parent repo's test scripts.
    public static let toneCommonOffs:        UInt32 = 0x0000

    /// Tone MFX (multi-effect). From Jupiter-X [ZCore] map; not yet exposed.
    public static let toneMFXOffs:           UInt32 = 0x0100

    /// Tone PMT (Partial Mix Table). From Jupiter-X [ZCore] map.
    public static let tonePMTOffs:           UInt32 = 0x1000

    public static let partialBaseOffs:       UInt32 = 0x2000
    public static let partialStride:         UInt32 = 0x0100

    public static let partialPitchEnvBase:   UInt32 = 0x2400
    public static let partialPitchEnvStride: UInt32 = 0x0100

    public static let partialFilterEnvBase:  UInt32 = 0x2800
    public static let partialFilterEnvStride: UInt32 = 0x0100

    public static let partialAmpEnvBase:     UInt32 = 0x2C00
    public static let partialAmpEnvStride:   UInt32 = 0x0100

    /// Partial LFO has a *bigger* stride because the LFO struct is
    /// 0x11E bytes (almost 286 bytes) — it carries both LFO 1 and LFO 2
    /// in one block.
    public static let partialLFOBase:        UInt32 = 0x3000
    public static let partialLFOStride:      UInt32 = 0x0200

    public static let partialEQBase:         UInt32 = 0x3800
    public static let partialEQStride:       UInt32 = 0x0100

    /// SuperNATURAL synth half of the ZCore tone descriptor — the MC's
    /// OSC type / structure / cross-mod / unison parameters live here.
    /// MC-707 Reference Manual page 62–63 (STRUCTURE / OSC MOD COM /
    /// UNISON / OSC categories).
    public static let toneSynthCommonOffs:   UInt32 = 0x3C00
    public static let toneSynthPMTOffs:      UInt32 = 0x3D00
    public static let toneSynthPartialBase:  UInt32 = 0x3E00
    public static let toneSynthPartialStride: UInt32 = 0x0100

    public static let numPartials                   = 4

    /// Offset of partial *N* (1..4) within a tone descriptor for the given
    /// substructure. Defaults to the Tone Partial layout.
    public static func partialOffset(
        _ partialIdx: Int,
        base: UInt32 = partialBaseOffs,
        stride: UInt32 = partialStride
    ) throws -> UInt32 {
        guard (1...numPartials).contains(partialIdx) else {
            throw MCProgrammerError.indexOutOfRange("partial=\(partialIdx) (must be 1..4)")
        }
        return base &+ UInt32(partialIdx - 1) &* stride
    }
}

// MARK: - ZCParam: descriptor + codec for one parameter

public enum ZCValue: Equatable {
    case int(Int)
    case string(String)
}

public struct ZCParam {
    public enum Group: String, CaseIterable {
        case common              = "common"
        case pmt                 = "pmt"
        case partial             = "partial"
        case partialPitchEnv     = "partial_pitch_env"
        case partialFilterEnv    = "partial_filter_env"
        case partialAmpEnv       = "partial_amp_env"
        case partialLFO          = "partial_lfo"
        case partialEQ           = "partial_eq"
        case synthCommon         = "synth_common"
        case synthPMT            = "synth_pmt"
        case synthPartial        = "synth_partial"
        case mfx                 = "mfx"
    }

    public let name: String          // machine name (snake_case)
    public let offset: UInt32        // offset within containing substructure
    public let size: Int             // bytes on the wire (1, 2, 4, or N for ASCII)
    public let minValue: Int
    public let maxValue: Int
    public let defaultValue: Int
    public let label: String
    public let isASCII: Bool
    public let isSigned: Bool
    public let notes: String

    public init(
        name: String,
        offset: UInt32,
        size: Int,
        minValue: Int = 0,
        maxValue: Int = 127,
        defaultValue: Int = 0,
        label: String = "",
        isASCII: Bool = false,
        isSigned: Bool = false,
        notes: String = ""
    ) {
        self.name = name
        self.offset = offset
        self.size = size
        self.minValue = minValue
        self.maxValue = maxValue
        self.defaultValue = defaultValue
        self.label = label
        // Auto-detect ASCII fields the same way the Python side does:
        // any 8+ byte field whose Roland range is 32..127 is a name string.
        self.isASCII = isASCII || (size >= 8 && minValue == 32 && maxValue == 127)
        self.isSigned = isSigned
        self.notes = notes
    }

    /// Encode a value into the data bytes that go inside a DT1 packet.
    public func encode(_ value: ZCValue) throws -> [UInt8] {
        if isASCII {
            guard case let .string(s) = value else {
                throw MCProgrammerError.valueOutOfRange("\(name): expected string")
            }
            return encodeASCII(s)
        }

        guard case let .int(v) = value else {
            throw MCProgrammerError.valueOutOfRange("\(name): expected int")
        }

        if !(minValue...maxValue).contains(v) {
            throw MCProgrammerError.valueOutOfRange(
                "\(name)=\(v) (must be \(minValue)...\(maxValue))"
            )
        }

        switch size {
        case 1: return [UInt8(v & 0x7F)]
        case 2: return [UInt8((v >> 4) & 0x0F), UInt8(v & 0x0F)]
        case 4:
            return [
                UInt8((v >> 12) & 0x0F),
                UInt8((v >> 8)  & 0x0F),
                UInt8((v >> 4)  & 0x0F),
                UInt8(v         & 0x0F),
            ]
        default:
            throw MCProgrammerError.valueOutOfRange("\(name): unsupported wire size \(size)")
        }
    }

    /// Decode the data bytes from a DT1 reply back into a value.
    public func decode(_ raw: [UInt8]) throws -> ZCValue {
        if isASCII {
            return .string(decodeASCII(raw))
        }

        var v: Int
        switch size {
        case 1:
            v = Int(raw[0] & 0x7F)
        case 2:
            v = (Int(raw[0] & 0x0F) << 4) | Int(raw[1] & 0x0F)
        case 4:
            v =   (Int(raw[0] & 0x0F) << 12)
                | (Int(raw[1] & 0x0F) << 8)
                | (Int(raw[2] & 0x0F) << 4)
                |  Int(raw[3] & 0x0F)
        default:
            throw MCProgrammerError.valueOutOfRange("\(name): unsupported wire size \(size)")
        }
        return .int(v)
    }

    // MARK: - ASCII helpers

    private func encodeASCII(_ s: String) -> [UInt8] {
        var bytes = s.unicodeScalars
            .prefix(size)
            .map { UInt8($0.value & 0x7F) }
        while bytes.count < size { bytes.append(0x20) }
        return bytes.map { (0x20...0x7E).contains($0) ? $0 : UInt8(ascii: "?") }
    }

    private func decodeASCII(_ raw: [UInt8]) -> String {
        var chars = ""
        for b in raw.prefix(size) {
            if (0x20..<0x7F).contains(b) {
                chars.append(Character(UnicodeScalar(b)))
            } else {
                chars.append(" ")
            }
        }
        return chars.trimmingCharacters(in: .whitespaces)
    }
}

// MARK: - Group lookup (parameters live in ZenCoreData.*)

public enum ZenCore {
    public static var toneCommonParams:       [String: ZCParam] { ZenCoreData.TONE_COMMON }
    public static var tonePMTParams:          [String: ZCParam] { ZenCoreData.TONE_PMT }
    public static var tonePartialParams:      [String: ZCParam] { ZenCoreData.TONE_PARTIAL }
    public static var partialPitchEnvParams:  [String: ZCParam] { ZenCoreData.PARTIAL_PITCH_ENV }
    public static var partialFilterEnvParams: [String: ZCParam] { ZenCoreData.PARTIAL_FILTER_ENV }
    public static var partialAmpEnvParams:    [String: ZCParam] { ZenCoreData.PARTIAL_AMP_ENV }
    public static var partialLFOParams:       [String: ZCParam] { ZenCoreData.PARTIAL_LFO }
    public static var partialEQParams:        [String: ZCParam] { ZenCoreData.PARTIAL_EQ }
    public static var toneSynthCommonParams:  [String: ZCParam] { ZenCoreData.TONE_SYNTH_COMMON }
    public static var toneSynthPMTParams:     [String: ZCParam] { ZenCoreData.TONE_SYNTH_PMT }
    public static var toneSynthPartialParams: [String: ZCParam] { ZenCoreData.TONE_SYNTH_PARTIAL }
    public static var toneMFXParams:          [String: ZCParam] { ZenCoreData.TONE_MFX }

    public static func lookup(_ group: ZCParam.Group, _ name: String) throws -> ZCParam {
        let table: [String: ZCParam]
        switch group {
        case .common:           table = toneCommonParams
        case .pmt:              table = tonePMTParams
        case .partial:          table = tonePartialParams
        case .partialPitchEnv:  table = partialPitchEnvParams
        case .partialFilterEnv: table = partialFilterEnvParams
        case .partialAmpEnv:    table = partialAmpEnvParams
        case .partialLFO:       table = partialLFOParams
        case .partialEQ:        table = partialEQParams
        case .synthCommon:      table = toneSynthCommonParams
        case .synthPMT:         table = toneSynthPMTParams
        case .synthPartial:     table = toneSynthPartialParams
        case .mfx:              table = toneMFXParams
        }
        guard let p = table[name] else {
            throw MCProgrammerError.indexOutOfRange("unknown \(group.rawValue) parameter: \(name)")
        }
        return p
    }

    public static func substructureOffset(
        _ group: ZCParam.Group,
        partialIdx: Int = 1
    ) throws -> UInt32 {
        switch group {
        case .common:           return ZenCoreLayout.toneCommonOffs
        case .pmt:              return ZenCoreLayout.tonePMTOffs
        case .mfx:              return ZenCoreLayout.toneMFXOffs
        case .synthCommon:      return ZenCoreLayout.toneSynthCommonOffs
        case .synthPMT:         return ZenCoreLayout.toneSynthPMTOffs
        case .partial:
            return try ZenCoreLayout.partialOffset(
                partialIdx,
                base: ZenCoreLayout.partialBaseOffs,
                stride: ZenCoreLayout.partialStride)
        case .partialPitchEnv:
            return try ZenCoreLayout.partialOffset(
                partialIdx,
                base: ZenCoreLayout.partialPitchEnvBase,
                stride: ZenCoreLayout.partialPitchEnvStride)
        case .partialFilterEnv:
            return try ZenCoreLayout.partialOffset(
                partialIdx,
                base: ZenCoreLayout.partialFilterEnvBase,
                stride: ZenCoreLayout.partialFilterEnvStride)
        case .partialAmpEnv:
            return try ZenCoreLayout.partialOffset(
                partialIdx,
                base: ZenCoreLayout.partialAmpEnvBase,
                stride: ZenCoreLayout.partialAmpEnvStride)
        case .partialLFO:
            return try ZenCoreLayout.partialOffset(
                partialIdx,
                base: ZenCoreLayout.partialLFOBase,
                stride: ZenCoreLayout.partialLFOStride)
        case .partialEQ:
            return try ZenCoreLayout.partialOffset(
                partialIdx,
                base: ZenCoreLayout.partialEQBase,
                stride: ZenCoreLayout.partialEQStride)
        case .synthPartial:
            return try ZenCoreLayout.partialOffset(
                partialIdx,
                base: ZenCoreLayout.toneSynthPartialBase,
                stride: ZenCoreLayout.toneSynthPartialStride)
        }
    }
}
