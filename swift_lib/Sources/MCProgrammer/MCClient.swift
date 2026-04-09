// MCClient.swift — high-level client tying a Transport to addresses + protocol.
//
// Typical entry point:
//
//   let client = try MCClient(transport: myTransport, modelID: .mc707)
//   var step = try client.readNoteStep(track: 0, clip: 0, step: 4)
//   step.components[0].note = 60          // C4
//   try client.writeNoteStep(track: 0, clip: 0, step: 4, step)

import Foundation

public final class MCClient {
    public let transport: Transport
    public let addresses: AddressMap
    public let modelID: UInt8

    public init(transport: Transport, modelID: ModelID, addresses: AddressMap = AddressMap()) {
        self.transport = transport
        self.modelID = modelID.rawValue
        self.addresses = addresses
    }

    public init(transport: Transport, modelIDByte: UInt8, addresses: AddressMap = AddressMap()) {
        self.transport = transport
        self.modelID = modelIDByte
        self.addresses = addresses
    }

    /// Detect a model ID via universal device inquiry. Throws if no reply.
    public static func detectModelID(transport: Transport) throws -> UInt8 {
        let reply = try transport.transfer(RolandProtocol.universalDeviceInquiry(), timeout: 0.2)
        guard reply.count >= 7 else { throw MCProgrammerError.noDeviceResponse }
        return reply[6]
    }

    // MARK: - Low-level RQ1 / DT1

    public func read(address: UInt32, count: UInt32, timeout: TimeInterval = 0.1) throws -> [UInt8] {
        let msg = RolandProtocol.buildRQ1(modelID: modelID, address: address, count: count)
        let reply = try transport.transfer(msg, timeout: timeout)
        if reply.isEmpty { return [] }
        return try RolandProtocol.parseDT1(reply).data
    }

    public func write(address: UInt32, data: [UInt8]) throws {
        let msg = RolandProtocol.buildDT1(modelID: modelID, address: address, data: data)
        _ = try transport.transfer(msg, timeout: 0.05)
    }

    /// Read a multi-byte parameter, MSB-first nibble-packed.
    public func readParam(address: UInt32, size: Int, timeout: TimeInterval = 0.1) throws -> UInt32 {
        let data = try read(address: address, count: UInt32(size), timeout: timeout)
        var value: UInt32 = 0
        for (i, b) in data.enumerated() {
            value |= UInt32(b & 0x0F) << (4 * (data.count - i - 1))
        }
        return value
    }

    public func writeParam(address: UInt32, value: UInt32, size: Int) throws {
        if size == 1 {
            try write(address: address, data: [UInt8(value & 0x7F)])
            return
        }
        var nibbles: [UInt8] = []
        for i in 0..<size {
            nibbles.append(UInt8((value >> (4 * (size - i - 1))) & 0x0F))
        }
        try write(address: address, data: nibbles)
    }

    // MARK: - Sequencer

    public func readNoteStep(track: Int, clip: Int, step: Int) throws -> NoteStep {
        let addr = try addresses.noteStepAddress(track: track, clip: clip, step: step)
        let raw = try read(address: addr, count: SequencerMap.stepSize)
        return try NoteStepCodec.decode(raw)
    }

    public func writeNoteStep(track: Int, clip: Int, step: Int, _ value: NoteStep) throws {
        let addr = try addresses.noteStepAddress(track: track, clip: clip, step: step)
        try write(address: addr, data: NoteStepCodec.encode(value))
    }

    public struct ClipStepParams {
        public let scale: StepScale
        public let stepCount: Int
    }

    /// Returns the clip's step scale and step count. Per the MC video the
    /// step length is encoded as two nibble bytes — e.g. 0x08 0x00 → 0x80 = 128.
    public func readClipStepParams(track: Int, clip: Int) throws -> ClipStepParams {
        let addr = try addresses.clipParamsAddress(track: track, clip: clip)
        let data = try read(address: addr, count: 4)
        guard data.count >= 3 else {
            return ClipStepParams(scale: .sixteenth, stepCount: 16)
        }
        let scale = StepScale(rawValue: data[0]) ?? .sixteenth
        let count = (Int(data[1]) << 4) | Int(data[2])
        return ClipStepParams(scale: scale, stepCount: count)
    }

    // MARK: - ZEN-Core

    public func readCoarseTune(track: Int, clip: Int? = nil) throws -> UInt8 {
        let addr = try addresses.zenCoreToneAddress(track: track, clip: clip)
            &+ ZenCoreMap.toneCommonCoarseTuneOffs
        let data = try read(address: addr, count: 1)
        return data.first ?? 0
    }

    public func writeCoarseTune(track: Int, clip: Int? = nil, value: UInt8) throws {
        // Per ZC video / zen_core_tools.py: range 16..112, center 64.
        let clamped = max(16, min(112, value))
        let addr = try addresses.zenCoreToneAddress(track: track, clip: clip)
            &+ ZenCoreMap.toneCommonCoarseTuneOffs
        try write(address: addr, data: [clamped])
    }

    /// Read-modify-write helper matching `coarse_tune_rmw` in the test scripts.
    @discardableResult
    public func nudgeCoarseTune(track: Int, clip: Int? = nil, by step: Int) throws -> UInt8 {
        let current = Int(try readCoarseTune(track: track, clip: clip))
        let next = max(16, min(112, current + step))
        try writeCoarseTune(track: track, clip: clip, value: UInt8(next))
        return UInt8(next)
    }

    // MARK: - ZEN-Core: generic parameter access

    private func zcParamAddress(
        track: Int,
        clip: Int?,
        group: ZCParam.Group,
        param: ZCParam,
        partialIdx: Int
    ) throws -> UInt32 {
        let toneBase = try addresses.zenCoreToneAddress(track: track, clip: clip)
        let subOffs = try ZenCore.substructureOffset(group, partialIdx: partialIdx)
        return toneBase &+ subOffs &+ param.offset
    }

    /// Read any registered ZEN-Core parameter by name.
    /// Pass `clip == nil` for the *track* tone, or 0..15 for one of the
    /// clip tones. For `.partial`, also pass `partialIdx` in 1..4.
    public func readZCParam(
        track: Int,
        clip: Int? = nil,
        name: String,
        group: ZCParam.Group = .common,
        partialIdx: Int = 1
    ) throws -> ZCValue? {
        let param = try ZenCore.lookup(group, name)
        let addr = try zcParamAddress(track: track, clip: clip,
                                      group: group, param: param,
                                      partialIdx: partialIdx)
        let data = try read(address: addr, count: UInt32(param.size))
        if data.isEmpty { return nil }
        return try param.decode(data)
    }

    /// Write any registered ZEN-Core parameter by name.
    public func writeZCParam(
        track: Int,
        clip: Int? = nil,
        name: String,
        value: ZCValue,
        group: ZCParam.Group = .common,
        partialIdx: Int = 1
    ) throws {
        let param = try ZenCore.lookup(group, name)
        let addr = try zcParamAddress(track: track, clip: clip,
                                      group: group, param: param,
                                      partialIdx: partialIdx)
        try write(address: addr, data: try param.encode(value))
    }

    // Convenience helpers

    public func readToneName(track: Int, clip: Int? = nil) throws -> String {
        // Roland labels this field just "Name" → snake_case "name".
        guard case let .string(s)? = try readZCParam(track: track, clip: clip,
                                                     name: "name") else {
            return ""
        }
        return s
    }

    public func writeToneName(track: Int, clip: Int? = nil, _ name: String) throws {
        try writeZCParam(track: track, clip: clip, name: "name", value: .string(name))
    }

    // =====================================================================
    // MARK: - High-level helpers (front-panel numbering)
    // =====================================================================
    //
    // The methods in this section accept ``trackNumber``/``clipNumber``/
    // ``stepNumber`` parameters that match the **MC's front-panel display
    // exactly**: tracks 1..8, clips 1..16, steps 1..128. So:
    //
    //     try client.placeNote(trackNumber: 1, clipNumber: 1,
    //                          stepNumber: 1, pitch: "C1")
    //
    // places a kick on the first step of the first clip on the first track,
    // which is what the device shows as TRACK 1 / CLIP 1 / STEP 1.
    //
    // The lower-level primitives in this file (``readNoteStep``,
    // ``writeZCParam``, …) still take ``track:``/``clip:``/``step:``
    // parameters that are 0-based — those mirror the underlying address
    // arithmetic. The ``Number`` suffix on the helpers is the discriminator.
    //
    // Pitches accept both literals — ``"C1"`` and ``36`` are equivalent
    // because of ``Note.Pitch``'s ExpressibleBy*Literal conformances. The
    // MC family follows the Roland octave convention where C3 = MIDI 60,
    // so the kick drum lives at C1 = 36.

    /// Number of physical tracks on the connected device. The MC-101 has
    /// only 4 tracks; the MC-707 and MV-1 have 8.
    public var numTracks: Int {
        switch modelID {
        case 0x5E: return 4   // MC-101
        default:   return 8   // MC-707 / MV-1 / unknown
        }
    }

    private func trackToIdx(_ n: Int) throws -> Int {
        let max = numTracks
        guard (1...max).contains(n) else {
            let model: String
            switch modelID {
            case 0x5D: model = "MC-707"
            case 0x5E: model = "MC-101"
            case 0x5F: model = "MV-1"
            default:   model = "your device"
            }
            throw MCProgrammerError.indexOutOfRange(
                "track=\(n) doesn't exist — \(model) has tracks 1..\(max) (front-panel labels)"
            )
        }
        return n - 1
    }

    private func clipToIdx(_ n: Int) throws -> Int {
        guard (1...16).contains(n) else {
            throw MCProgrammerError.indexOutOfRange(
                "clip=\(n) doesn't exist — each track has clips 1..16"
            )
        }
        return n - 1
    }

    private func stepToIdx(_ n: Int) throws -> Int {
        guard (1...128).contains(n) else {
            throw MCProgrammerError.indexOutOfRange(
                "step=\(n) doesn't exist — each clip has steps 1..128"
            )
        }
        return n - 1
    }

    /// Add one note to a step, packing it into the lowest free lane.
    ///
    /// ``trackNumber``, ``clipNumber`` and ``stepNumber`` are 1-based,
    /// matching the MC's screen labels. ``pitch`` accepts a MIDI number
    /// (``36``) or a note name (``"C1"``).
    ///
    /// ``lengthSteps`` is the duration in step units — 0.8 is the
    /// default 80-cent length the MC writes when you punch in a note
    /// from the front panel; 1.0 is a full step; values > 1.0 tie the
    /// note across the appropriate number of following steps.
    public func placeNote(
        trackNumber: Int,
        clipNumber: Int,
        stepNumber: Int,
        pitch: Note.Pitch,
        velocity: UInt8 = 100,
        lengthSteps: Double = 0.8
    ) throws {
        let ti = try trackToIdx(trackNumber)
        let ci = try clipToIdx(clipNumber)
        let si = try stepToIdx(stepNumber)
        let midi = UInt8(try pitch.resolve())

        var existing = try readNoteStep(track: ti, clip: ci, step: si)
        var placed = false
        for i in 0..<existing.components.count where existing.components[i].isEmpty {
            existing.components[i].note = midi
            existing.components[i].velocity = velocity
            existing.components[i].startCents = 0
            existing.components[i].endCents = lengthSteps > 1.0
                ? MC_END_TIME_TIE
                : UInt8(min(100, max(0, Int((lengthSteps * 100).rounded()))))
            try writeNoteStep(track: ti, clip: ci, step: si, existing)
            placed = true
            break
        }
        guard placed else { return }   // all 16 lanes full — drop, like the hardware

        var remaining = lengthSteps - 1.0
        var cur = si + 1
        while remaining > 0 && cur < SequencerMap.stepsPerClip {
            var tail = try readNoteStep(track: ti, clip: ci, step: cur)
            for i in 0..<tail.components.count where tail.components[i].isEmpty {
                tail.components[i].note = midi
                tail.components[i].velocity = velocity
                tail.components[i].startCents = 0
                tail.components[i].endCents = remaining > 1.0
                    ? MC_END_TIME_TIE
                    : UInt8(min(100, max(0, Int((remaining * 100).rounded()))))
                try writeNoteStep(track: ti, clip: ci, step: cur, tail)
                break
            }
            remaining -= 1.0
            cur += 1
        }
    }

    /// Erase every note from one step. ``trackNumber``/``clipNumber``/
    /// ``stepNumber`` are 1-based.
    public func clearStep(trackNumber: Int, clipNumber: Int, stepNumber: Int) throws {
        let ti = try trackToIdx(trackNumber)
        let ci = try clipToIdx(clipNumber)
        let si = try stepToIdx(stepNumber)
        try writeNoteStep(track: ti, clip: ci, step: si, NoteStep())
    }

    /// Erase every step of a clip. ``trackNumber``/``clipNumber`` are 1-based.
    public func clearClip(trackNumber: Int, clipNumber: Int) throws {
        let ti = try trackToIdx(trackNumber)
        let ci = try clipToIdx(clipNumber)
        let empty = NoteStep()
        for s in 0..<SequencerMap.stepsPerClip {
            try writeNoteStep(track: ti, clip: ci, step: s, empty)
        }
    }

    /// Program a whole pattern in one call.
    ///
    /// ``trackNumber`` / ``clipNumber`` are 1-based. The dictionary maps
    /// pitches (literals or strings or ints) to lists of **1-based** step
    /// positions. If ``clearFirst`` is true (default) the clip is wiped
    /// before the new notes are written.
    ///
    /// Example — a 4-on-the-floor kick + offbeat hat:
    ///
    ///     try client.setPattern(trackNumber: 1, clipNumber: 1, notes: [
    ///         "C1":  [1, 5, 9, 13],   // kick on every downbeat
    ///         "F#1": [3, 7, 11, 15],  // hat on the offbeats
    ///     ])
    public func setPattern(
        trackNumber: Int,
        clipNumber: Int,
        notes: [String: [Int]],
        velocity: UInt8 = 100,
        clearFirst: Bool = true
    ) throws {
        if clearFirst {
            try clearClip(trackNumber: trackNumber, clipNumber: clipNumber)
        }
        for (pitchString, steps) in notes {
            for step in steps {
                try placeNote(
                    trackNumber: trackNumber,
                    clipNumber: clipNumber,
                    stepNumber: step,
                    pitch: .named(pitchString),
                    velocity: velocity
                )
            }
        }
    }

    // ---- Coarse tune in semitones ----------------------------------------

    /// Coarse tune in semitones (-48..+48). 0 = no transposition.
    /// ``trackNumber`` is 1-based; pass ``clipNumber: nil`` for the
    /// track-wide tone, or 1..16 for one of the clip tones.
    public func readCoarseTuneSemitones(
        trackNumber: Int,
        clipNumber: Int? = nil
    ) throws -> Int {
        let ti = try trackToIdx(trackNumber)
        let ci: Int? = try clipNumber.map { try clipToIdx($0) }
        return Int(try readCoarseTune(track: ti, clip: ci)) - 64
    }

    /// Set coarse tune in semitones (-48..+48). 0 = no transposition.
    public func writeCoarseTuneSemitones(
        trackNumber: Int,
        clipNumber: Int? = nil,
        semitones: Int
    ) throws {
        guard (-48...48).contains(semitones) else {
            throw MCProgrammerError.valueOutOfRange(
                "semitones=\(semitones) — must be -48..+48 (your MC's hardware limit)"
            )
        }
        let ti = try trackToIdx(trackNumber)
        let ci: Int? = try clipNumber.map { try clipToIdx($0) }
        try writeCoarseTune(track: ti, clip: ci, value: UInt8(64 + semitones))
    }

    // ---- Discoverability -------------------------------------------------

    // ---- Scene / Clip selection via Program Change -----------------------
    //
    // Per the MC-707 / MC-101 MIDI Implementation Chart:
    //
    //   "Program Change (0-127) on the control channel selects a Scene.
    //    Program Change (0-15) on a track channel selects that track's Clip."
    //
    // The MC family does **not** use Bank Select for any kind of tone or
    // patch selection — only the front-panel scene/clip change is exposed
    // over MIDI. There is no MIDI message that loads an arbitrary preset
    // tone into a track.

    /// Switch the MC to a stored scene via Program Change.
    ///
    /// `sceneNumber` is 1-based (1..128). `controlChannel` is the MIDI
    /// channel that the MC's system menu has set as the *control* channel
    /// — usually 1, but you can change it on the device. Pass whatever
    /// your MC is configured to.
    public func selectScene(sceneNumber: Int, controlChannel: Int = 1) throws {
        guard (1...128).contains(sceneNumber) else {
            throw MCProgrammerError.indexOutOfRange("sceneNumber=\(sceneNumber) — must be 1..128")
        }
        guard (1...16).contains(controlChannel) else {
            throw MCProgrammerError.indexOutOfRange("controlChannel=\(controlChannel) — must be 1..16")
        }
        let status: UInt8 = 0xC0 | UInt8((controlChannel - 1) & 0x0F)
        try transport.sendRaw([status, UInt8(sceneNumber - 1)])
    }

    /// Trigger a stored clip on a track via Program Change.
    ///
    /// `trackNumber` and `clipNumber` are 1-based (1..8 and 1..16). The
    /// message goes out on `trackChannel` if you pass it; otherwise the
    /// convention "track N → MIDI channel N" is used (the MC's default
    /// mapping — change it in the MC's system menu and pass an explicit
    /// channel here if you've remapped).
    public func selectClip(
        trackNumber: Int,
        clipNumber: Int,
        trackChannel: Int? = nil
    ) throws {
        guard (1...16).contains(clipNumber) else {
            throw MCProgrammerError.indexOutOfRange("clipNumber=\(clipNumber) — must be 1..16")
        }
        let ti = try trackToIdx(trackNumber)
        let channel = trackChannel ?? (ti + 1)
        guard (1...16).contains(channel) else {
            throw MCProgrammerError.indexOutOfRange("trackChannel=\(channel) — must be 1..16")
        }
        let status: UInt8 = 0xC0 | UInt8((channel - 1) & 0x0F)
        try transport.sendRaw([status, UInt8(clipNumber - 1)])
    }

    // =====================================================================
    // MARK: - Chunked I/O (multi-RQ1 / multi-DT1 for blocks > 128 bytes)
    // =====================================================================

    /// Conservative upper bound for one Sys-Ex payload — the MC's per-
    /// request size limit isn't documented but the reference Python
    /// scripts use ≤128 bytes per RQ1, so we do the same.
    public static let chunkSize: Int = 0x80

    public func readChunked(address: UInt32, count: Int) throws -> [UInt8] {
        var out: [UInt8] = []
        out.reserveCapacity(count)
        var offset = 0
        while offset < count {
            let n = min(MCClient.chunkSize, count - offset)
            var chunk = try read(address: address &+ UInt32(offset), count: UInt32(n))
            if chunk.isEmpty {
                // Pad with zeros so the caller gets a constant-length result
                // and can detect missing data.
                chunk = Array(repeating: 0, count: n)
            }
            out.append(contentsOf: chunk)
            offset += n
        }
        return out
    }

    public func writeChunked(address: UInt32, data: [UInt8]) throws {
        var offset = 0
        while offset < data.count {
            let n = min(MCClient.chunkSize, data.count - offset)
            try write(address: address &+ UInt32(offset),
                      data: Array(data[offset..<(offset + n)]))
            offset += n
        }
    }

    // =====================================================================
    // MARK: - Tone preset dump / restore
    // =====================================================================

    /// Capture every byte of one ZEN-Core tone into a `TonePreset`.
    /// `trackNumber` is 1-based; pass `clipNumber: nil` to dump the
    /// track-wide tone.
    public func dumpTone(trackNumber: Int, clipNumber: Int? = nil,
                         name: String? = nil) throws -> TonePreset {
        let ti = try trackToIdx(trackNumber)
        let ci: Int? = try clipNumber.map { try clipToIdx($0) }
        let base = try addresses.zenCoreToneAddress(track: ti, clip: ci)

        var regions: [ToneRegion] = []
        for spec in ToneRegionTable.all {
            let size = Int(ToneRegionTable.size(forSection: spec.sectionKey))
            let bytes = try readChunked(address: base &+ spec.offset, count: size)
            regions.append(ToneRegion(offset: spec.offset, label: spec.label, data: bytes))
        }

        var displayName = name
        if displayName == nil {
            displayName = (try? readToneName(track: ti, clip: ci))
        }

        let modelStr: String
        switch modelID {
        case 0x5D: modelStr = "MC-707"
        case 0x5E: modelStr = "MC-101"
        case 0x5F: modelStr = "MV-1"
        default:   modelStr = String(format: "0x%02X", modelID)
        }
        return TonePreset(model: modelStr, name: displayName, regions: regions)
    }

    /// Write a `TonePreset` into a tone slot. `trackNumber` is 1-based;
    /// pass `clipNumber: nil` for the track-wide tone slot.
    ///
    /// The preset's `model` field is checked against this client's model
    /// ID; loading a 707 preset onto a 101 raises an error.
    public func loadTone(trackNumber: Int, clipNumber: Int? = nil,
                         preset: TonePreset) throws {
        let expected: String
        switch modelID {
        case 0x5D: expected = "MC-707"
        case 0x5E: expected = "MC-101"
        case 0x5F: expected = "MV-1"
        default:   expected = String(format: "0x%02X", modelID)
        }
        if !preset.model.isEmpty && preset.model != expected {
            throw MCProgrammerError.valueOutOfRange(
                "preset model '\(preset.model)' doesn't match this client's model '\(expected)'"
            )
        }

        let ti = try trackToIdx(trackNumber)
        let ci: Int? = try clipNumber.map { try clipToIdx($0) }
        let base = try addresses.zenCoreToneAddress(track: ti, clip: ci)

        for region in preset.regions {
            try writeChunked(address: base &+ region.offset, data: region.data)
        }
    }

    /// Return the names of every ZEN-Core parameter we know how to set.
    /// Pass a ``group`` to filter (e.g. ``.common`` for tone-wide params).
    public func listParameters(group: ZCParam.Group? = nil) -> [String] {
        if let group = group {
            let table: [String: ZCParam]
            switch group {
            case .common:           table = ZenCore.toneCommonParams
            case .pmt:              table = ZenCore.tonePMTParams
            case .partial:          table = ZenCore.tonePartialParams
            case .partialPitchEnv:  table = ZenCore.partialPitchEnvParams
            case .partialFilterEnv: table = ZenCore.partialFilterEnvParams
            case .partialAmpEnv:    table = ZenCore.partialAmpEnvParams
            case .partialLFO:       table = ZenCore.partialLFOParams
            case .partialEQ:        table = ZenCore.partialEQParams
            case .synthCommon:      table = ZenCore.toneSynthCommonParams
            case .synthPMT:         table = ZenCore.toneSynthPMTParams
            case .synthPartial:     table = ZenCore.toneSynthPartialParams
            case .mfx:              table = ZenCore.toneMFXParams
            }
            return table.keys.sorted()
        }
        var seen = Set<String>()
        for table in [
            ZenCore.toneCommonParams, ZenCore.tonePMTParams,
            ZenCore.tonePartialParams,
            ZenCore.partialPitchEnvParams, ZenCore.partialFilterEnvParams,
            ZenCore.partialAmpEnvParams, ZenCore.partialLFOParams,
            ZenCore.partialEQParams,
            ZenCore.toneSynthCommonParams, ZenCore.toneSynthPMTParams,
            ZenCore.toneSynthPartialParams, ZenCore.toneMFXParams,
        ] {
            seen.formUnion(table.keys)
        }
        return seen.sorted()
    }
}
