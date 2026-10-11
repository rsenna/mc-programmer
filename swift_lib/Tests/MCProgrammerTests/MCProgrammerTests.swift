import XCTest
@testable import MCProgrammer

final class MCProgrammerTests: XCTestCase {

    func testChecksumKnownVector() {
        // Roland checksum is the complement-to-128 of the byte sum.
        let payload: [UInt8] = [0x20, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01]
        let cs = RolandProtocol.checksum(payload)
        XCTAssertEqual(Int(cs), (128 - (Int(payload.reduce(0) { $0 + Int($1) }) % 128)) & 0x7F)
    }

    func testRQ1Framing() {
        let msg = RolandProtocol.buildRQ1(modelID: ModelID.mc707.rawValue,
                                          address: 0x20000000,
                                          count: 0x10)
        XCTAssertEqual(msg.first, 0xF0)
        XCTAssertEqual(msg.last,  0xF7)
        XCTAssertEqual(msg[1], 0x41)         // Roland
        XCTAssertEqual(msg[6], 0x5D)         // MC-707 model id
        XCTAssertEqual(msg[7], 0x11)         // RQ1
    }

    func testNoteStepRoundTrip() throws {
        var step = NoteStep()
        step.components[0] = NoteComponent(note: 60, velocity: 123, startCents: -10, endCents: 100)
        step.components[1] = NoteComponent(note: 67, velocity: 100, startCents: 6,   endCents: 56)
        step.substep = .triple

        let encoded = NoteStepCodec.encode(step)
        XCTAssertEqual(encoded.count, Int(SequencerMap.stepSize))

        let decoded = try NoteStepCodec.decode(encoded)
        XCTAssertEqual(decoded.components[0].note, 60)
        XCTAssertEqual(decoded.components[0].velocity, 123)
        XCTAssertEqual(decoded.components[0].startCents, -10)
        XCTAssertEqual(decoded.components[0].endCents, 100)
        XCTAssertTrue(decoded.components[0].tiesToNext)
        XCTAssertEqual(decoded.components[1].note, 67)
        XCTAssertEqual(decoded.components[1].startCents, 6)
        XCTAssertEqual(decoded.components[1].endCents, 56)
        XCTAssertEqual(decoded.substep, .triple)
        // Lanes 2..15 should still decode as empty
        XCTAssertTrue(decoded.components[2].isEmpty)
    }

    func testNoteStepFormattingShowsHexAndDecodedLanes() throws {
        var raw = NoteStepCodec.encode(NoteStep())
        raw[0x10] = 0x03
        raw[0x11] = 0x0C
        raw[0x12] = 0x08
        raw[0x13] = 0x00
        raw[0x30] = 123
        raw[0x31] = 100
        raw[0x40] = 0x0F
        raw[0x41] = 0x06
        raw[0x60] = 100
        raw[0x61] = 56
        raw[0x70] = 2
        raw[255] = 0xA5

        let rendered = try NoteStepCodec.format(raw)

        XCTAssertTrue(rendered.contains("0000: " + Array(repeating: "00", count: 16).joined(separator: " ")))
        XCTAssertTrue(rendered.contains("0010: 03 0C 08 00"))
        XCTAssertTrue(rendered.contains("00F0: " + (Array(repeating: "00", count: 15) + ["A5"]).joined(separator: " ")))
        XCTAssertTrue(rendered.contains("00: note=C3 (60), velocity=123, start=-10c, end=tie"))
        XCTAssertTrue(rendered.contains("01: note=invalid 0x80, velocity=100, start=+0c, end=56c"))
        XCTAssertTrue(rendered.contains("02: note=empty, velocity=100, start=+0c, end=80c"))
        XCTAssertTrue(rendered.contains("Substep: triple (raw 0x02)"))
    }

    func testNoteStepFormattingRequiresExactRecordSize() {
        for size in [255, 257] {
            XCTAssertThrowsError(try NoteStepCodec.format(Array(repeating: 0, count: size)))
        }
    }

    func testEmptyNoteSentinel() throws {
        let step = NoteStep()
        let encoded = NoteStepCodec.encode(step)
        let nb = Int(NoteStepLayout.notesOffs)
        XCTAssertEqual(encoded[nb], 0x0F)
        XCTAssertEqual(encoded[nb + 1], 0x0F)

        let decoded = try NoteStepCodec.decode(encoded)
        XCTAssertEqual(decoded.components[0].note, MC_EMPTY_NOTE)
        XCTAssertTrue(decoded.components[0].isEmpty)
    }

    func testAddressMapSkipsHighMSB() throws {
        let map = AddressMap()
        for trk in 0..<SequencerMap.numTracks {
            for clip in 0..<SequencerMap.clipsPerTrack {
                let addr = try map.clipBase(track: trk, clip: clip)
                XCTAssertEqual(addr & 0x80808080, 0,
                               "clip address \(String(addr, radix: 16)) has a high-MSB byte")
            }
        }
    }

    func testZenCorePartialOffset() throws {
        // Conventional ZEN-Core: partial 1 at 0x2000, stride 0x100.
        XCTAssertEqual(try ZenCoreLayout.partialOffset(1), ZenCoreLayout.partialBaseOffs)
        XCTAssertEqual(try ZenCoreLayout.partialOffset(4),
                       ZenCoreLayout.partialBaseOffs &+ 3 &* ZenCoreLayout.partialStride)
        XCTAssertEqual(ZenCoreLayout.partialStride, 0x0100)
        XCTAssertEqual(ZenCoreLayout.partialLFOStride, 0x0200)
        XCTAssertThrowsError(try ZenCoreLayout.partialOffset(0))
        XCTAssertThrowsError(try ZenCoreLayout.partialOffset(5))
    }

    func testZenCoreToneCoarseTuneParam() throws {
        // The Tone Common field is "Tone Coarse Tune" at offset 0x18,
        // verified by zen_core_tools.py.
        let p = try ZenCore.lookup(.common, "tone_coarse_tune")
        XCTAssertEqual(p.offset, 0x18)
        XCTAssertEqual(p.size, 1)
        XCTAssertEqual(try p.encode(.int(64)), [64])
        if case .int(let v) = try p.decode([64]) {
            XCTAssertEqual(v, 64)
        } else {
            XCTFail("decode should return .int")
        }
        XCTAssertThrowsError(try p.encode(.int(0)))   // below min (16)
        XCTAssertThrowsError(try p.encode(.int(200))) // above max (112)
    }

    func testZenCoreToneNameRoundTrip() throws {
        // Roland labels this field just "Name" → snake_case "name", 16 bytes.
        let p = try ZenCore.lookup(.common, "name")
        XCTAssertEqual(p.size, 16)
        XCTAssertTrue(p.isASCII)
        let encoded = try p.encode(.string("INIT TONE"))
        XCTAssertEqual(encoded.count, 16)
        XCTAssertEqual(encoded.last, 0x20)            // right-padded with space
        if case .string(let s) = try p.decode(encoded) {
            XCTAssertEqual(s, "INIT TONE")
        } else {
            XCTFail("decode should return .string")
        }
    }

    func testZenCoreFourByteNibblePack() throws {
        // Real generated example: TVF Cutoff Frequency in Tone Partial,
        // 0..1023, packed across 4 nibble bytes.
        let p = try ZenCore.lookup(.partial, "tvf_cutoff_frequency")
        XCTAssertEqual(p.size, 4)
        XCTAssertEqual(try p.encode(.int(0x123)), [0x00, 0x01, 0x02, 0x03])
        if case .int(let v) = try p.decode([0x00, 0x01, 0x02, 0x03]) {
            XCTAssertEqual(v, 0x123)
        } else {
            XCTFail()
        }
    }

    func testZenCorePartialEnvLookup() throws {
        let p = try ZenCore.lookup(.partialPitchEnv, "pitch_env_time1")
        XCTAssertEqual(p.size, 4)
        XCTAssertEqual(p.minValue, 0)
        XCTAssertEqual(p.maxValue, 1023)
    }

    func testZenCoreEQLookup() throws {
        let p = try ZenCore.lookup(.partialEQ, "eq_low_gain")
        XCTAssertEqual(p.offset, 0x00)
        XCTAssertEqual(p.size, 4)
        XCTAssertEqual(p.minValue, 784)
        XCTAssertEqual(p.maxValue, 1264)
    }

    func testClientReadToneNameUsesCorrectAddress() throws {
        let p = try ZenCore.lookup(.common, "name")
        let encoded = try p.encode(.string("BASS"))
        let reply = RolandProtocol.buildDT1(modelID: ModelID.mc707.rawValue,
                                            address: 0x30000000, data: encoded)
        let transport = NullTransport(responses: [reply])
        let client = MCClient(transport: transport, modelID: .mc707)

        let name = try client.readToneName(track: 0, clip: 0)
        XCTAssertEqual(name, "BASS")

        XCTAssertEqual(transport.sent.count, 1)
        let sent = transport.sent[0]
        XCTAssertEqual(sent[7], 0x11) // RQ1
        let addr = (UInt32(sent[8]) << 24)
                 | (UInt32(sent[9]) << 16)
                 | (UInt32(sent[10]) << 8)
                 |  UInt32(sent[11])
        XCTAssertEqual(addr, 0x30000000)
    }

    // MARK: - Note parser

    func testNoteParserRolandConvention() throws {
        XCTAssertEqual(try Note.parse("C3"), 60)
        XCTAssertEqual(try Note.parse("C1"), 36)   // kick
        XCTAssertEqual(try Note.parse("D1"), 38)   // snare
        XCTAssertEqual(try Note.parse("F#1"), 42)  // closed hat
        XCTAssertEqual(try Note.parse("A#1"), 46)  // open hat
        XCTAssertEqual(try Note.parse("Bb3"), 70)
        XCTAssertEqual(try Note.parse("c1"), 36)
        XCTAssertEqual(try Note.parse("C-2"), 0)
        XCTAssertEqual(try Note.parse("C"),   60)  // default octave 3
    }

    func testNoteParserOutOfRange() {
        XCTAssertThrowsError(try Note.parse("Hello"))
        XCTAssertThrowsError(try Note.parse("C99"))
    }

    func testDrumKeyConstants() {
        XCTAssertEqual(DrumKey.KICK, 36)
        XCTAssertEqual(DrumKey.SNARE, 38)
        XCTAssertEqual(DrumKey.HAT_CLOSED, 42)
        XCTAssertEqual(DrumKey.HAT_OPEN, 46)
    }

    func testNotePitchLiteralFormsAreEquivalent() throws {
        let pitchA: Note.Pitch = "C1"
        let pitchB: Note.Pitch = 36
        XCTAssertEqual(try pitchA.resolve(), 36)
        XCTAssertEqual(try pitchB.resolve(), 36)
    }

    // MARK: - Front-panel high-level helpers (1-based)

    func testPlaceNoteWithName() throws {
        let client = MCClient(transport: LoopbackTransport(), modelID: .mc707)
        try client.placeNote(trackNumber: 1, clipNumber: 1, stepNumber: 1,
                             pitch: "C1", velocity: 120)
        let step = try client.readNoteStep(track: 0, clip: 0, step: 0)
        XCTAssertEqual(step.components[0].note, 36)
        XCTAssertEqual(step.components[0].velocity, 120)
    }

    func testPlaceNoteWithMidiNumber() throws {
        let client = MCClient(transport: LoopbackTransport(), modelID: .mc707)
        try client.placeNote(trackNumber: 1, clipNumber: 1, stepNumber: 1, pitch: 60)
        let step = try client.readNoteStep(track: 0, clip: 0, step: 0)
        XCTAssertEqual(step.components[0].note, 60)
    }

    func testPlaceNotePacksIntoLowestLane() throws {
        let client = MCClient(transport: LoopbackTransport(), modelID: .mc707)
        try client.placeNote(trackNumber: 1, clipNumber: 1, stepNumber: 1, pitch: "C1")
        try client.placeNote(trackNumber: 1, clipNumber: 1, stepNumber: 1, pitch: "E1")
        try client.placeNote(trackNumber: 1, clipNumber: 1, stepNumber: 1, pitch: "G1")
        let step = try client.readNoteStep(track: 0, clip: 0, step: 0)
        let notes = step.components.compactMap { $0.isEmpty ? nil : Int($0.note) }
        XCTAssertEqual(notes.sorted(), [36, 40, 43])
    }

    func testPlaceNoteLongTiesAcrossSteps() throws {
        let client = MCClient(transport: LoopbackTransport(), modelID: .mc707)
        try client.placeNote(trackNumber: 1, clipNumber: 1, stepNumber: 1,
                             pitch: "C3", lengthSteps: 2.5)
        let s1 = try client.readNoteStep(track: 0, clip: 0, step: 0)
        let s2 = try client.readNoteStep(track: 0, clip: 0, step: 1)
        let s3 = try client.readNoteStep(track: 0, clip: 0, step: 2)
        XCTAssertEqual(s1.components[0].endCents, MC_END_TIME_TIE)
        XCTAssertEqual(s2.components[0].endCents, MC_END_TIME_TIE)
        XCTAssertEqual(s3.components[0].endCents, 50)
    }

    func testSetPatternKickAndHat() throws {
        let client = MCClient(transport: LoopbackTransport(), modelID: .mc707)
        try client.setPattern(trackNumber: 1, clipNumber: 1, notes: [
            "C1":  [1, 5, 9, 13],
            "F#1": [3, 7, 11, 15],
        ])
        var kicks: [Int] = []
        var hats: [Int] = []
        for s in 0..<16 {
            let step = try client.readNoteStep(track: 0, clip: 0, step: s)
            for c in step.activeComponents {
                if c.note == 36 { kicks.append(s + 1) }
                if c.note == 42 { hats.append(s + 1) }
            }
        }
        XCTAssertEqual(kicks.sorted(), [1, 5, 9, 13])
        XCTAssertEqual(hats.sorted(),  [3, 7, 11, 15])
    }

    func testTrackOutOfRangeMessageIsFriendly() {
        let client = MCClient(transport: LoopbackTransport(), modelID: .mc707)
        XCTAssertThrowsError(
            try client.placeNote(trackNumber: 9, clipNumber: 1,
                                 stepNumber: 1, pitch: "C1")
        ) { error in
            XCTAssertTrue("\(error)".contains("tracks 1..8"))
        }
    }

    func testCoarseTuneInSemitones() throws {
        let client = MCClient(transport: LoopbackTransport(), modelID: .mc707)
        try client.writeCoarseTuneSemitones(trackNumber: 2, clipNumber: nil, semitones: +5)
        XCTAssertEqual(try client.readCoarseTuneSemitones(trackNumber: 2, clipNumber: nil), 5)
        // Underlying wire value is 64 + 5 = 69
        XCTAssertEqual(try client.readCoarseTune(track: 1, clip: nil), 69)
    }

    func testListParametersFiltered() {
        let client = MCClient(transport: LoopbackTransport(), modelID: .mc707)
        let common = client.listParameters(group: .common)
        XCTAssertTrue(common.contains("tone_coarse_tune"))
        XCTAssertTrue(common.contains("name"))
        XCTAssertFalse(common.contains("tvf_cutoff_frequency"))
    }

    // MARK: - Scene / Clip selection (Program Change)

    func testSelectSceneEmitsProgramChange() throws {
        let transport = LoopbackTransport()
        let client = MCClient(transport: transport, modelID: .mc707)
        try client.selectScene(sceneNumber: 5, controlChannel: 1)
        XCTAssertEqual(transport.sentRaw.count, 1)
        XCTAssertEqual(transport.sentRaw[0], [0xC0, 4])
    }

    func testSelectSceneUsesConfiguredControlChannel() throws {
        let transport = LoopbackTransport()
        let client = MCClient(transport: transport, modelID: .mc707)
        try client.selectScene(sceneNumber: 128, controlChannel: 16)
        XCTAssertEqual(transport.sentRaw[0], [0xCF, 127])
    }

    func testSelectSceneOutOfRange() {
        let client = MCClient(transport: LoopbackTransport(), modelID: .mc707)
        XCTAssertThrowsError(try client.selectScene(sceneNumber: 0))
        XCTAssertThrowsError(try client.selectScene(sceneNumber: 129))
        XCTAssertThrowsError(
            try client.selectScene(sceneNumber: 1, controlChannel: 17)
        )
    }

    func testSelectClipDefaultsToTrackChannel() throws {
        let transport = LoopbackTransport()
        let client = MCClient(transport: transport, modelID: .mc707)
        try client.selectClip(trackNumber: 3, clipNumber: 12)
        // Default mapping: track 3 → channel 3 → status 0xC2; clip 12 → program 11
        XCTAssertEqual(transport.sentRaw[0], [0xC2, 11])
    }

    func testSelectClipWithExplicitChannel() throws {
        let transport = LoopbackTransport()
        let client = MCClient(transport: transport, modelID: .mc707)
        try client.selectClip(trackNumber: 1, clipNumber: 4, trackChannel: 10)
        XCTAssertEqual(transport.sentRaw[0], [0xC9, 3])
    }

    func testLoopbackRecordsActiveSceneAndClip() throws {
        let transport = LoopbackTransport()
        let client = MCClient(transport: transport, modelID: .mc707)
        try client.selectScene(sceneNumber: 7)               // PC on channel 1
        try client.selectClip(trackNumber: 2, clipNumber: 15) // PC on channel 2
        XCTAssertEqual(transport.activeScene, 15)
        XCTAssertEqual(transport.activeClipPerChannel[1], 7)
        XCTAssertEqual(transport.activeClipPerChannel[2], 15)
    }

    // MARK: - Tone preset dump / restore

    func testTonePresetDumpThenLoadRoundtrip() throws {
        let client = MCClient(transport: LoopbackTransport(), modelID: .mc707)
        // Make the source tone non-trivial.
        try client.writeZCParam(track: 0, clip: nil,
                                name: "tone_coarse_tune", value: .int(70))
        try client.writeToneName(track: 0, clip: nil, "TEST")

        let preset = try client.dumpTone(trackNumber: 1, clipNumber: nil)
        XCTAssertFalse(preset.regions.isEmpty)
        XCTAssertEqual(preset.model, "MC-707")

        try client.loadTone(trackNumber: 5, clipNumber: nil, preset: preset)
        if case .int(let v) = try client.readZCParam(
            track: 4, clip: nil, name: "tone_coarse_tune") {
            XCTAssertEqual(v, 70)
        } else {
            XCTFail()
        }
        XCTAssertEqual(try client.readToneName(track: 4, clip: nil), "TEST")
    }

    func testTonePresetSaveLoadFileRoundtrip() throws {
        let client = MCClient(transport: LoopbackTransport(), modelID: .mc707)
        try client.writeToneName(track: 0, clip: nil, "BASS")

        let path = NSTemporaryDirectory() + "test_\(UUID().uuidString).tone"
        defer { try? FileManager.default.removeItem(atPath: path) }

        try client.dumpTone(trackNumber: 1, clipNumber: nil).save(toPath: path)
        let preset = try TonePreset.load(fromPath: path)
        XCTAssertEqual(preset.name, "BASS")

        try client.loadTone(trackNumber: 2, clipNumber: nil, preset: preset)
        XCTAssertEqual(try client.readToneName(track: 1, clip: nil), "BASS")
    }

    func testLoadTonePresetRefusesWrongModel() throws {
        let client = MCClient(transport: LoopbackTransport(), modelID: .mc707)
        let wrong = TonePreset(model: "MC-101", regions: [
            ToneRegion(offset: 0x18, label: "x", data: [70])
        ])
        XCTAssertThrowsError(
            try client.loadTone(trackNumber: 1, clipNumber: nil, preset: wrong)
        ) { error in
            XCTAssertTrue("\(error)".contains("MC-101"))
            XCTAssertTrue("\(error)".contains("MC-707"))
        }
    }

    // MARK: - Project dump / restore

    func testProjectDumpThenLoadIntoBlankDevice() throws {
        let src = MCClient(transport: LoopbackTransport(), modelID: .mc707)
        try src.setPattern(trackNumber: 1, clipNumber: 1, notes: [
            "C1":  [1, 5, 9, 13],
            "F#1": [3, 7, 11, 15],
        ])
        try src.writeZCParam(track: 0, clip: 0,
                             name: "tone_coarse_tune", value: .int(72))
        try src.writeToneName(track: 0, clip: 0, "BIG BASS")

        let project = try Project.dumpFrom(client: src, name: "snapshot")
        XCTAssertEqual(project.model, "MC-707")
        XCTAssertEqual(project.tracks.count, 8)
        XCTAssertEqual(project.tracks[0].clips.count, 16)

        let dst = MCClient(transport: LoopbackTransport(), modelID: .mc707)
        try project.loadInto(client: dst)

        var kicks: [Int] = []
        for s in 0..<16 {
            let step = try dst.readNoteStep(track: 0, clip: 0, step: s)
            for c in step.activeComponents where c.note == 36 {
                kicks.append(s + 1)
            }
        }
        XCTAssertEqual(kicks.sorted(), [1, 5, 9, 13])
        if case .int(let v) = try dst.readZCParam(
            track: 0, clip: 0, name: "tone_coarse_tune") {
            XCTAssertEqual(v, 72)
        } else {
            XCTFail()
        }
        XCTAssertEqual(try dst.readToneName(track: 0, clip: 0), "BIG BASS")
    }

    func testProjectFileRoundtrip() throws {
        let src = MCClient(transport: LoopbackTransport(), modelID: .mc707)
        try src.writeToneName(track: 0, clip: 0, "PROJECT TEST")

        let path = NSTemporaryDirectory() + "test_\(UUID().uuidString).mctproject"
        defer { try? FileManager.default.removeItem(atPath: path) }

        try Project.dumpFrom(client: src, name: "snapshot").save(toPath: path)
        let project = try Project.load(fromPath: path)
        XCTAssertEqual(project.name, "snapshot")

        let dst = MCClient(transport: LoopbackTransport(), modelID: .mc707)
        try project.loadInto(client: dst)
        XCTAssertEqual(try dst.readToneName(track: 0, clip: 0), "PROJECT TEST")
    }

    func testProjectLoadRefusesWrongModelUnlessForced() throws {
        let client = MCClient(transport: LoopbackTransport(), modelID: .mc707)
        let wrong = Project(model: "MC-101", tracks: [])
        XCTAssertThrowsError(try wrong.loadInto(client: client))
        // force: true should bypass.
        try wrong.loadInto(client: client, force: true)
    }

    // MARK: - MC-101 compatibility

    func testMC101NumTracksIs4() {
        let mc101 = MCClient(transport: LoopbackTransport(), modelID: .mc101)
        let mc707 = MCClient(transport: LoopbackTransport(), modelID: .mc707)
        XCTAssertEqual(mc101.numTracks, 4)
        XCTAssertEqual(mc707.numTracks, 8)
    }

    func testMC101RejectsTracks5Through8() throws {
        let client = MCClient(transport: LoopbackTransport(modelID: .mc101),
                              modelID: .mc101)
        for t in 5...8 {
            XCTAssertThrowsError(
                try client.placeNote(trackNumber: t, clipNumber: 1,
                                     stepNumber: 1, pitch: "C1")
            ) { error in
                XCTAssertTrue("\(error)".contains("MC-101"))
                XCTAssertTrue("\(error)".contains("1..4"))
            }
        }
    }

    func testMC101AcceptsTracks1Through4() throws {
        let client = MCClient(transport: LoopbackTransport(modelID: .mc101),
                              modelID: .mc101)
        for t in 1...4 {
            try client.placeNote(trackNumber: t, clipNumber: 1,
                                 stepNumber: 1, pitch: "C1")
            let step = try client.readNoteStep(track: t - 1, clip: 0, step: 0)
            XCTAssertEqual(step.components[0].note, 36)
        }
    }

    func testMC101LoadRefusesMC707Project() throws {
        let client = MCClient(transport: LoopbackTransport(modelID: .mc101),
                              modelID: .mc101)
        let wrong = Project(model: "MC-707", tracks: [])
        XCTAssertThrowsError(try wrong.loadInto(client: client)) { error in
            XCTAssertTrue("\(error)".contains("MC-707"))
            XCTAssertTrue("\(error)".contains("MC-101"))
        }
    }

    func testClientWritesPartialParamToCorrectAddress() throws {
        // Per the [ZCore] address map, Tone Partial 2 is at offset 0x2100
        // within the tone descriptor; the Coarse Tune of a partial is at
        // offset 0x01 inside [Tone Partial]. So writing partial 2 of the
        // track-0 clip-0 tone (base 0x30000000) should target 0x30002101.
        let transport = NullTransport()
        let client = MCClient(transport: transport, modelID: .mc707)
        try client.writeZCParam(track: 0, clip: 0,
                                name: "coarse_tune", value: .int(70),
                                group: .partial, partialIdx: 2)

        let sent = transport.sent[0]
        XCTAssertEqual(sent[7], 0x12)  // DT1
        let addr = (UInt32(sent[8]) << 24)
                 | (UInt32(sent[9]) << 16)
                 | (UInt32(sent[10]) << 8)
                 |  UInt32(sent[11])
        XCTAssertEqual(addr, 0x30000000 + 0x2100 + 0x01)
    }

    func testZenCoreToneAddresses() throws {
        let map = AddressMap()
        // Track tone for track 0 = trackBase + alignment * 16
        XCTAssertEqual(try map.zenCoreToneAddress(track: 0, clip: nil),
                       0x30000000 + 0x20000 * 16)
        // Clip 0 of track 0 = trackBase
        XCTAssertEqual(try map.zenCoreToneAddress(track: 0, clip: 0), 0x30000000)
    }
}
