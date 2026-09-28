//! Integration tests mirroring the Python / Swift test suites.
//!
//! The memory `LoopbackTransport` lets these exercise the full client path —
//! address arithmetic, Sys-Ex framing, sequencer codec, ZEN-Core parameter
//! access, tone presets and whole-project dump/restore — without hardware.

use mc_programmer::{
    build_dt1, decode_note_step, encode_note_step, lookup, note, note_name, parse_dt1_payload, partial_offset,
    roland_checksum, substructure_offset, AddressMap, ClipDump, LoopbackTransport, MCClient, ModelId, NoteComponent,
    NoteStep, NullTransport, ParamGroup, Pitch, Project, SubStep, TonePreset, ToneRegion, TrackDump, ZCValue,
    EMPTY_NOTE,
};

/// Create an MC-707 memory transport whose unwritten bytes are `0xFF`.
fn loopback() -> LoopbackTransport {
    LoopbackTransport::new(ModelId::Mc707, 0xFF)
}

/// Create an MC-707 client over a fresh loopback device.
fn client() -> MCClient<LoopbackTransport> {
    MCClient::new(loopback(), ModelId::Mc707.value())
}

/// Create a four-track MC-101 client over a fresh loopback device.
fn client_101() -> MCClient<LoopbackTransport> {
    let t = LoopbackTransport::new(ModelId::Mc101, 0xFF);
    MCClient::new(t, ModelId::Mc101.value())
}

// ---------------------------------------------------------------------------
// Protocol
// ---------------------------------------------------------------------------

#[test]
fn checksum_is_complement_to_128() {
    let payload = [0x20, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01];
    assert_eq!(
        roland_checksum(&payload),
        (128 - payload.iter().map(|b| *b as usize).sum::<usize>() % 128) as u8 & 0x7F
    );
}

#[test]
fn dt1_round_trip() {
    let msg = build_dt1(ModelId::Mc101.value(), 0x30000018, &[0x40]).unwrap();
    let parsed = parse_dt1_payload(&msg).unwrap();
    assert_eq!(parsed.address, 0x30000018);
    assert_eq!(parsed.data, [0x40]);
}

// ---------------------------------------------------------------------------
// Address map
// ---------------------------------------------------------------------------

#[test]
fn no_clip_address_has_high_msb() {
    let map = AddressMap::new();
    for trk in 0..8 {
        for clip in 0..16 {
            let addr = map.clip_base(trk, clip).unwrap();
            assert_eq!(addr & 0x80808080, 0);
        }
    }
}

#[test]
fn zencore_track_tone_address() {
    let map = AddressMap::new();
    assert_eq!(
        map.zencore_tone_address(0, None).unwrap(),
        0x30000000 + 0x20000 * 16
    );
    assert_eq!(map.zencore_tone_address(0, Some(0)).unwrap(), 0x30000000);
}

// ---------------------------------------------------------------------------
// Note step codec
// ---------------------------------------------------------------------------

#[test]
fn round_trip_complex_step() {
    let mut comps = vec![NoteComponent::empty(); 16];
    comps[0] = NoteComponent {
        note: 60,
        velocity: 123,
        start_cents: -10,
        end_cents: 100,
        processed: false,
    };
    comps[1] = NoteComponent {
        note: 67,
        velocity: 100,
        start_cents: 6,
        end_cents: 56,
        processed: false,
    };
    let step = NoteStep::new(comps, SubStep::Triple).unwrap();

    let encoded = encode_note_step(&step);
    assert_eq!(encoded.len(), 0x100);

    let decoded = decode_note_step(&encoded).unwrap();
    assert_eq!(decoded.components[0].note, 60);
    assert_eq!(decoded.components[0].velocity, 123);
    assert_eq!(decoded.components[0].start_cents, -10);
    assert_eq!(decoded.components[0].end_cents, 100);
    assert!(decoded.components[0].ties_to_next());
    assert_eq!(decoded.components[1].note, 67);
    assert_eq!(decoded.components[1].start_cents, 6);
    assert_eq!(decoded.substep, SubStep::Triple);
    for c in &decoded.components[2..] {
        assert_eq!(c.note, EMPTY_NOTE);
    }
}

#[test]
fn empty_note_sentinel() {
    let encoded = encode_note_step(&NoteStep::default());
    assert_eq!(encoded[0x10], 0x0F);
    assert_eq!(encoded[0x11], 0x0F);
}

#[test]
fn signed_start_cents_extremes() {
    for cents in [-50i8, -1, 0, 1, 50, 99] {
        let mut step = NoteStep::default();
        step.components[0] = NoteComponent {
            note: 60,
            start_cents: cents,
            ..NoteComponent::empty()
        };
        let decoded = decode_note_step(&encode_note_step(&step)).unwrap();
        assert_eq!(decoded.components[0].start_cents, cents);
    }
}

// ---------------------------------------------------------------------------
// MCClient low-level
// ---------------------------------------------------------------------------

#[test]
fn write_note_step_emits_sysex() {
    let transport = NullTransport::new();
    let c = MCClient::new(transport, ModelId::Mc707.value());
    let mut step = NoteStep::default();
    step.components[0].note = 60;
    c.write_note_step(0, 0, 0, &step).unwrap();

    let sent = c.transport.sent();
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0][0], 0xF0);
    assert_eq!(*sent[0].last().unwrap(), 0xF7);
    assert_eq!(sent[0][7], 0x12); // DT1
}

#[test]
fn read_note_step_decodes_dt1_reply() {
    let empty = encode_note_step(&NoteStep::default());
    let reply = build_dt1(ModelId::Mc707.value(), 0x20030000, &empty).unwrap();
    let transport = NullTransport::with_responses(vec![reply]);
    let c = MCClient::new(transport, ModelId::Mc707.value());
    let step = c.read_note_step(0, 0, 0).unwrap();
    assert_eq!(step.components[0].note, EMPTY_NOTE);
}

#[test]
fn chunked_io_crosses_7_bit_address_boundaries() {
    let c = client();
    let data: Vec<u8> = (0..300)
        .map(|index| ((index + (index / 128) * 17) % 128) as u8)
        .collect();
    let base = 0x1000_0800;

    c.write_chunked(base, &data).unwrap();
    assert_eq!(c.read_chunked(base, data.len()).unwrap(), data);
}

#[test]
fn every_zencore_tone_address_is_7_bit_clean() {
    let map = AddressMap::new();
    for track in 0..8 {
        for clip in (0..16).map(Some).chain(std::iter::once(None)) {
            let address = map.zencore_tone_address(track, clip).unwrap();
            assert_eq!(address & 0x8080_8080, 0, "0x{address:08X}");
        }
    }
}

// ---------------------------------------------------------------------------
// ZEN-Core framework
// ---------------------------------------------------------------------------

#[test]
fn partial_offset_layout() {
    assert_eq!(partial_offset(1, 0x2000, 0x100).unwrap(), 0x2000);
    assert_eq!(partial_offset(2, 0x2000, 0x100).unwrap(), 0x2100);
    assert_eq!(partial_offset(4, 0x2000, 0x100).unwrap(), 0x2300);
    assert!(partial_offset(0, 0x2000, 0x100).is_err());
    assert!(partial_offset(5, 0x2000, 0x100).is_err());
}

#[test]
fn tone_coarse_tune_descriptor() {
    let param = lookup(ParamGroup::Common, "tone_coarse_tune").unwrap();
    assert_eq!(param.offset, 0x18);
    assert_eq!(param.size, 1);
    assert_eq!(param.encode(&ZCValue::Int(64)).unwrap(), [64]);
    assert_eq!(param.decode(&[64]).unwrap(), ZCValue::Int(64));
    assert!(param.encode(&ZCValue::Int(0)).is_err()); // below min 16
    assert!(param.encode(&ZCValue::Int(200)).is_err()); // above max 112
}

#[test]
fn tone_name_round_trip() {
    let param = lookup(ParamGroup::Common, "name").unwrap();
    assert_eq!(param.size, 16);
    assert!(param.ascii);
    let encoded = param.encode(&ZCValue::Str("INIT TONE".into())).unwrap();
    assert_eq!(encoded.len(), 16);
    assert_eq!(encoded[15], 0x20); // right-padded with space
    assert_eq!(
        param.decode(&encoded).unwrap(),
        ZCValue::Str("INIT TONE".into())
    );
}

#[test]
fn nibble_packed_4_byte_value() {
    let param = lookup(ParamGroup::Partial, "tvf_cutoff_frequency").unwrap();
    assert_eq!(param.size, 4);
    assert_eq!(
        param.encode(&ZCValue::Int(0x123)).unwrap(),
        [0x00, 0x01, 0x02, 0x03]
    );
    assert_eq!(
        param.decode(&[0x00, 0x01, 0x02, 0x03]).unwrap(),
        ZCValue::Int(0x123)
    );
}

#[test]
fn substructure_offset_dispatch() {
    use mc_programmer::{PARTIAL_LFO_BASE, PARTIAL_LFO_STRIDE, TONE_COMMON_OFFS, TONE_MFX_OFFS, TONE_PMT_OFFS};
    assert_eq!(
        substructure_offset(ParamGroup::Common, 1).unwrap(),
        TONE_COMMON_OFFS
    );
    assert_eq!(
        substructure_offset(ParamGroup::Mfx, 1).unwrap(),
        TONE_MFX_OFFS
    );
    assert_eq!(
        substructure_offset(ParamGroup::Pmt, 1).unwrap(),
        TONE_PMT_OFFS
    );
    assert_eq!(
        substructure_offset(ParamGroup::SynthCommon, 1).unwrap(),
        0x3C00
    );
    assert_eq!(
        substructure_offset(ParamGroup::SynthPmt, 1).unwrap(),
        0x3D00
    );
    assert_eq!(
        substructure_offset(ParamGroup::SynthPartial, 1).unwrap(),
        0x3E00
    );
    assert_eq!(
        substructure_offset(ParamGroup::SynthPartial, 4).unwrap(),
        0x4100
    );
    assert_eq!(
        substructure_offset(ParamGroup::PartialLfo, 3).unwrap(),
        PARTIAL_LFO_BASE + 2 * PARTIAL_LFO_STRIDE
    );
}

#[test]
fn synth_and_mfx_tables_present() {
    assert_eq!(
        lookup(ParamGroup::SynthCommon, "unison_switch")
            .unwrap()
            .size,
        1
    );
    assert_eq!(
        lookup(ParamGroup::SynthCommon, "unison_size")
            .unwrap()
            .max_value,
        8
    );
    assert_eq!(
        lookup(ParamGroup::SynthCommon, "unison_detune")
            .unwrap()
            .max_value,
        100
    );
    assert_eq!(
        lookup(ParamGroup::SynthPmt, "structure1_2")
            .unwrap()
            .max_value,
        4
    );
    assert_eq!(
        lookup(ParamGroup::SynthPartial, "osc_type").unwrap().offset,
        0x00
    );
    assert_eq!(lookup(ParamGroup::Mfx, "type").unwrap().offset, 0x00);
    assert_eq!(lookup(ParamGroup::Mfx, "mfx_switch").unwrap().offset, 0x01);
}

// ---------------------------------------------------------------------------
// Note parser
// ---------------------------------------------------------------------------

#[test]
fn note_name_conventions() {
    assert_eq!(note("C3").unwrap(), 60);
    assert_eq!(note("C1").unwrap(), 36); // kick
    assert_eq!(note("D1").unwrap(), 38); // snare
    assert_eq!(note("F#1").unwrap(), 42); // closed hat
    assert_eq!(note("A#1").unwrap(), 46); // open hat
    assert_eq!(note("C#3").unwrap(), 61);
    assert_eq!(note("Db3").unwrap(), 61);
    assert_eq!(note("c3").unwrap(), 60);
    assert_eq!(note("C-2").unwrap(), 0);
    assert_eq!(note("C").unwrap(), 60);
    assert!(note("Hello")
        .unwrap_err()
        .to_string()
        .contains("unrecognised note name"));
}

#[test]
fn note_name_round_trips() {
    for n in 0..=127u8 {
        assert_eq!(note(&note_name(n).unwrap()).unwrap(), n);
    }
}

// ---------------------------------------------------------------------------
// High-level helpers
// ---------------------------------------------------------------------------

#[test]
fn place_note_with_name_and_midi() {
    let c = client();
    c.place_note(1, 1, 1, Pitch::from("C1"), 120, 0.8).unwrap();
    let midi = c.read_note_step(0, 0, 0).unwrap();
    assert_eq!(midi.components[0].note, 36);
    assert_eq!(midi.components[0].velocity, 120);

    c.place_note(1, 1, 1, Pitch::from(60u8), 100, 0.8).unwrap();
    let notes: Vec<u8> = c
        .read_note_step(0, 0, 0)
        .unwrap()
        .active_components()
        .iter()
        .map(|c| c.note)
        .collect();
    assert!(notes.contains(&36) && notes.contains(&60));
}
#[test]
fn place_note_long_ties_across_steps() {
    let c = client();
    c.place_note(1, 1, 1, Pitch::from("C3"), 100, 2.5).unwrap();
    let s1 = c.read_note_step(0, 0, 0).unwrap();
    let s2 = c.read_note_step(0, 0, 1).unwrap();
    let s3 = c.read_note_step(0, 0, 2).unwrap();
    assert_eq!(s1.components[0].end_cents, 100);
    assert_eq!(s2.components[0].end_cents, 100);
    assert_eq!(s3.components[0].end_cents, 50);
}

#[test]
fn set_pattern_kick_and_hat() {
    let c = client();
    let notes = [
        (Pitch::from("C1"), vec![1u32, 5, 9, 13]),
        (Pitch::from("F#1"), vec![3u32, 7, 11, 15]),
    ];
    c.set_pattern(1, 1, &notes, 100, true).unwrap();

    let mut kicks = Vec::new();
    let mut hats = Vec::new();
    for s in 0..16u32 {
        let step = c.read_note_step(0, 0, s as usize).unwrap();
        for comp in step.active_components() {
            if comp.note == 36 {
                kicks.push(s + 1);
            }
            if comp.note == 42 {
                hats.push(s + 1);
            }
        }
    }
    assert_eq!(kicks, [1, 5, 9, 13]);
    assert_eq!(hats, [3, 7, 11, 15]);
}

#[test]
fn track_out_of_range_is_friendly() {
    let c = client();
    let err = c
        .place_note(9, 1, 1, Pitch::from("C1"), 100, 0.8)
        .unwrap_err();
    assert!(err.to_string().contains("tracks 1..8"));
}

#[test]
fn coarse_tune_semitones() {
    let c = client();
    c.write_coarse_tune_semitones(2, None, 5).unwrap();
    assert_eq!(c.read_coarse_tune_semitones(2, None).unwrap(), 5);
    assert_eq!(c.read_coarse_tune(1, None).unwrap(), 69);
    let err = c.write_coarse_tune_semitones(2, None, 50).unwrap_err();
    assert!(err.to_string().contains("-48..+48"));
}

#[test]
fn list_and_describe_parameters() {
    let c = client();
    let common = c.list_parameters(Some(ParamGroup::Common));
    assert!(common.contains(&"tone_coarse_tune"));
    assert!(common.contains(&"name"));
    assert!(!common.contains(&"tvf_cutoff_frequency"));

    let desc = c.describe_parameter("tone_coarse_tune").unwrap();
    assert!(desc.contains("Tone Coarse Tune"));
    assert!(desc.contains("0x0018"));
}

// ---------------------------------------------------------------------------
// Scene / clip selection
// ---------------------------------------------------------------------------

#[test]
fn select_scene_emits_program_change() {
    let c = client();
    c.select_scene(5, 1).unwrap();
    assert_eq!(c.transport.sent_raw(), vec![vec![0xC0, 4]]);

    c.select_scene(128, 16).unwrap();
    assert_eq!(c.transport.sent_raw()[1], [0xCF, 127]);
    assert!(c.select_scene(0, 1).is_err());
}

#[test]
fn select_clip_channel_mapping() {
    let c = client();
    c.select_clip(3, 12, None).unwrap(); // track 3 -> channel 3
    assert_eq!(c.transport.sent_raw()[0], [0xC2, 11]);

    c.select_clip(1, 4, Some(10)).unwrap();
    assert_eq!(c.transport.sent_raw()[1], [0xC9, 3]);

    assert!(c.select_clip(1, 17, None).is_err());
    assert!(c.select_clip(9, 1, None).is_err());
}

// ---------------------------------------------------------------------------
// ZEN-Core client writes + dump/restore
// ---------------------------------------------------------------------------

#[test]
fn client_writes_and_reads_zc_params() {
    let c = client();
    c.write_zc_param(0, Some(0), "tone_coarse_tune", 70, ParamGroup::Common, 1)
        .unwrap();
    assert_eq!(
        c.read_zc_param(0, Some(0), "tone_coarse_tune", ParamGroup::Common, 1)
            .unwrap(),
        Some(ZCValue::Int(70))
    );

    c.write_zc_param(0, None, "unison_switch", 1, ParamGroup::SynthCommon, 1)
        .unwrap();
    assert_eq!(
        c.read_zc_param(0, None, "unison_switch", ParamGroup::SynthCommon, 1)
            .unwrap(),
        Some(ZCValue::Int(1))
    );
}

#[test]
fn write_partial_uses_correct_address() {
    let c = client();
    c.write_zc_param(0, Some(0), "coarse_tune", 70, ParamGroup::Partial, 2)
        .unwrap();
    // Tone Partial 2 lives at 0x2100 within the descriptor; `coarse_tune` is
    // at offset 0x01 inside [Tone Partial]. Clip 0 tone base is 0x30000000.
    let addr = 0x30000000u32 + 0x2100 + 0x01;
    assert_eq!(c.transport.snapshot(addr, 1), vec![70]);
}

#[test]
fn dump_tone_roundtrip_across_tracks() {
    let c = client();
    c.write_zc_param(0, None, "tone_coarse_tune", 70, ParamGroup::Common, 1)
        .unwrap();
    c.write_zc_param(0, None, "unison_switch", 1, ParamGroup::SynthCommon, 1)
        .unwrap();
    c.write_tone_name(0, None, "TEST").unwrap();

    let preset = c.dump_tone(1, None, None).unwrap();
    assert!(!preset.regions.is_empty());
    assert_eq!(preset.model, "MC-707");

    c.load_tone(5, None, &preset).unwrap();
    assert_eq!(
        c.read_zc_param(4, None, "tone_coarse_tune", ParamGroup::Common, 1)
            .unwrap(),
        Some(ZCValue::Int(70))
    );
    assert_eq!(
        c.read_zc_param(4, None, "unison_switch", ParamGroup::SynthCommon, 1)
            .unwrap(),
        Some(ZCValue::Int(1))
    );
    assert_eq!(c.read_tone_name(4, None).unwrap(), "TEST");
}

// ---------------------------------------------------------------------------
// Tone preset (JSON)
// ---------------------------------------------------------------------------

#[test]
fn tone_preset_json_roundtrip() {
    let preset = TonePreset::new(
        "MC-707",
        Some("BASS".to_string()),
        vec![ToneRegion {
            offset: 0x18,
            label: "tone_common".to_string(),
            data: vec![70, 0x00, 0xFF],
        }],
    );
    let json = preset.to_json();
    assert!(json.contains("mc_programmer.tone.v1"));
    let back = TonePreset::from_json(&json).unwrap();
    assert_eq!(back, preset);
}

#[test]
fn load_tone_refuses_wrong_model() {
    let c = client();
    let wrong = TonePreset::new(
        "MC-101",
        None,
        vec![ToneRegion {
            offset: 0x18,
            label: "x".into(),
            data: vec![70],
        }],
    );
    let err = c.load_tone(1, None, &wrong).unwrap_err();
    assert!(err.to_string().contains("MC-101"));
    assert!(err.to_string().contains("MC-707"));
}

// ---------------------------------------------------------------------------
// Project (JSON) dump/restore
// ---------------------------------------------------------------------------

#[test]
fn project_dump_load_into_blank_device() {
    let src = client();
    let notes = [
        (Pitch::from("C1"), vec![1u32, 5, 9, 13]),
        (Pitch::from("F#1"), vec![3u32, 7, 11, 15]),
    ];
    src.set_pattern(1, 1, &notes, 100, true).unwrap();
    src.write_zc_param(0, Some(0), "tone_coarse_tune", 72, ParamGroup::Common, 1)
        .unwrap();
    src.write_tone_name(0, Some(0), "BIG BASS").unwrap();

    let project = Project::dump_from(&src, Some("snapshot".into()), true).unwrap();
    assert_eq!(project.model, "MC-707");
    assert_eq!(project.tracks.len(), 8);
    assert_eq!(project.tracks[0].clips.len(), 16);

    // Round-trip through JSON.
    let json = project.to_json();
    let project = Project::from_json(&json).unwrap();

    let dst = client();
    project.load_into(&dst, false).unwrap();

    let mut kicks = Vec::new();
    for s in 0..16u32 {
        for comp in dst
            .read_note_step(0, 0, s as usize)
            .unwrap()
            .active_components()
        {
            if comp.note == 36 {
                kicks.push(s + 1);
            }
        }
    }
    assert_eq!(kicks, [1, 5, 9, 13]);
    assert_eq!(
        dst.read_zc_param(0, Some(0), "tone_coarse_tune", ParamGroup::Common, 1)
            .unwrap(),
        Some(ZCValue::Int(72))
    );
    assert_eq!(dst.read_tone_name(0, Some(0)).unwrap(), "BIG BASS");
}

#[test]
fn project_load_refuses_wrong_model_unless_forced() {
    let c = client();
    let wrong = Project::new("MC-101", None, Vec::new());
    let err = wrong.load_into(&c, false).unwrap_err();
    assert!(err.to_string().contains("MC-101"));
    assert!(err.to_string().contains("MC-707"));
    wrong.load_into(&c, true).unwrap(); // force bypasses the check
}

// ---------------------------------------------------------------------------
// MC-101 compatibility
// ---------------------------------------------------------------------------

#[test]
fn mc101_num_tracks_and_bounds() {
    let c101 = client_101();
    let c707 = client();
    assert_eq!(c101.num_tracks(), 4);
    assert_eq!(c707.num_tracks(), 8);

    for t in [5u32, 6, 7, 8] {
        let err = c101
            .place_note(t, 1, 1, Pitch::from("C1"), 100, 0.8)
            .unwrap_err();
        assert!(err.to_string().contains("MC-101"));
        assert!(err.to_string().contains("1..4"));
    }
    for t in [1u32, 2, 3, 4] {
        c101.place_note(t, 1, 1, Pitch::from("C1"), 100, 0.8)
            .unwrap();
        assert_eq!(
            c101.read_note_step((t - 1) as usize, 0, 0)
                .unwrap()
                .components[0]
                .note,
            36
        );
    }
}

#[test]
fn mc101_project_roundtrip_respects_four_tracks() {
    let src = client_101();
    let notes = [(Pitch::from("C1"), vec![1u32, 5, 9, 13])];
    src.set_pattern(1, 1, &notes, 100, true).unwrap();
    src.write_zc_param(0, Some(0), "tone_coarse_tune", 70, ParamGroup::Common, 1)
        .unwrap();

    let project = Project::dump_from(&src, Some("mc101".into()), true).unwrap();
    assert_eq!(project.model, "MC-101");
    assert_eq!(project.tracks.len(), 4);

    let dst = client_101();
    project.load_into(&dst, false).unwrap();
    assert_eq!(
        dst.read_zc_param(0, Some(0), "tone_coarse_tune", ParamGroup::Common, 1)
            .unwrap(),
        Some(ZCValue::Int(70))
    );
}

#[test]
fn mc101_load_refuses_mc707_project() {
    let c = client_101();
    let wrong = Project::new("MC-707", None, Vec::new());
    let err = wrong.load_into(&c, false).unwrap_err();
    assert!(err.to_string().contains("MC-707"));
    assert!(err.to_string().contains("MC-101"));
}

/// Verify invalid in-memory track and clip numbers return errors without panicking.
#[test]
fn malformed_project_track_clip_numbers_error_not_panic() {
    let c = client();
    let bad_track = Project {
        model: "MC-707".to_string(),
        name: None,
        tracks: vec![TrackDump {
            track_number: 9,
            settings: vec![],
            track_tone: TonePreset::new("MC-707", None, vec![]),
            clips: vec![],
        }],
        system_header: vec![],
        system_block: vec![],
    };
    let err = bad_track.load_into(&c, false).unwrap_err();
    assert!(err.to_string().contains("track_number"));

    let bad_clip = Project {
        model: "MC-707".to_string(),
        name: None,
        tracks: vec![TrackDump {
            track_number: 1,
            settings: vec![],
            track_tone: TonePreset::new("MC-707", None, vec![]),
            clips: vec![ClipDump {
                clip_number: 0,
                step_scale: 0,
                step_count: 0,
                note_steps: vec![],
                motion_steps: vec![],
                pitchbend: vec![],
                tone: TonePreset::new("MC-707", None, vec![]),
            }],
        }],
        system_header: vec![],
        system_block: vec![],
    };
    let err = bad_clip.load_into(&c, false).unwrap_err();
    assert!(err.to_string().contains("clip_number"));
}

/// Verify JSON parsing rejects track zero before a project can be restored.
#[test]
fn malformed_project_json_track_zero_is_rejected_during_parse() {
    let json = r#"{
      "format": "mc_programmer.project.v1",
      "model": "MC-707",
      "name": null,
      "tracks": [
        {"track_number": 0, "settings": "", "track_tone":
          {"format": "mc_programmer.tone.v1", "model": "MC-707", "name": null, "regions": []},
         "clips": []}
      ],
      "system_header": "",
      "system_block": ""
    }"#;
    let err = Project::from_json(json).unwrap_err();
    assert!(err.to_string().contains("track_number"));
}

/// Verify missing track numbers and string-valued clip numbers fail JSON parsing.
#[test]
fn malformed_project_json_requires_numbered_fields() {
    let missing_track_number = r#"{
      "format": "mc_programmer.project.v1",
      "model": "MC-707",
      "name": null,
      "tracks": [{"settings": "", "track_tone":
        {"format": "mc_programmer.tone.v1", "model": "MC-707", "name": null, "regions": []},
        "clips": []}],
      "system_header": "",
      "system_block": ""
    }"#;
    let err = Project::from_json(missing_track_number).unwrap_err();
    assert!(err.to_string().contains("track_number"));

    let wrong_clip_type = r#"{
      "format": "mc_programmer.project.v1",
      "model": "MC-707",
      "name": null,
      "tracks": [{"track_number": 1, "settings": "", "track_tone":
        {"format": "mc_programmer.tone.v1", "model": "MC-707", "name": null, "regions": []},
        "clips": [{"clip_number": "1", "step_scale": 1, "step_count": 16,
          "note_steps": "", "motion_steps": "", "pitchbend": "", "tone":
          {"format": "mc_programmer.tone.v1", "model": "MC-707", "name": null, "regions": []}}]}],
      "system_header": "",
      "system_block": ""
    }"#;
    let err = Project::from_json(wrong_clip_type).unwrap_err();
    assert!(err.to_string().contains("clip_number"));
}

/// Verify forced MC-707 restores reject tracks absent on the MC-101 before writing track 1.
#[test]
fn forced_cross_model_load_validates_every_track_before_writing() {
    let transport = LoopbackTransport::new(ModelId::Mc101, 0xFF);
    let probe = transport.clone();
    let c = MCClient::new(transport, ModelId::Mc101.value());
    let project = Project {
        model: "MC-707".to_string(),
        name: None,
        tracks: vec![
            TrackDump {
                track_number: 1,
                settings: vec![0x11],
                track_tone: TonePreset::new("MC-707", None, vec![]),
                clips: vec![],
            },
            TrackDump {
                track_number: 5,
                settings: vec![0x55],
                track_tone: TonePreset::new("MC-707", None, vec![]),
                clips: vec![],
            },
        ],
        system_header: vec![],
        system_block: vec![],
    };

    let err = project.load_into(&c, true).unwrap_err();
    assert!(err.to_string().contains("track_number"));
    assert_eq!(
        probe.snapshot(mc_programmer::addresses::TRK_SETTINGS_BASE_ADDR[0], 1),
        vec![0xFF]
    );
}

#[test]
fn wrong_format_json_rejected() {
    assert!(TonePreset::from_json(r#"{"format": "other", "model": "MC-707"}"#).is_err());
    assert!(Project::from_json(r#"{"format": "other", "model": "MC-707"}"#).is_err());
}

#[test]
fn zencore_parameter_decode_rejects_truncated_payload() {
    let param = lookup(ParamGroup::PartialEq, "eq_low_gain").unwrap();
    let error = param.decode(&[0x00, 0x01]).unwrap_err();
    assert!(error.to_string().contains("expected 4 bytes"));
}

#[test]
fn zencore_client_rejects_truncated_parameter_reply() {
    let reply = build_dt1(ModelId::Mc707.value(), 0x3000_0000, &[0x00, 0x01]).unwrap();
    let transport = NullTransport::with_responses(vec![reply]);
    let c = MCClient::new(transport, ModelId::Mc707.value());
    let error = c
        .read_zc_param(0, Some(0), "eq_low_gain", ParamGroup::PartialEq, 1)
        .unwrap_err();
    assert!(error.to_string().contains("expected 4 bytes"));
}

#[test]
fn unicode_note_name_returns_error_without_panicking() {
    assert!(note("Ｃ3").is_err());
}

#[test]
fn short_rq1_returns_empty_reply_without_panicking() {
    let transport = loopback();
    let request = [0xF0, 0x41, 0x10, 0, 0, 0, 0x5D, 0x11, 0, 0, 0, 0, 0, 0xF7];
    assert!(
        mc_programmer::Transport::transfer(&transport, &request, 0.1)
            .unwrap()
            .is_empty()
    );
}

/// Verify a short settings block leaves the destination track untouched.
#[test]
fn project_rejects_wrong_track_settings_size_before_writing() {
    let transport = loopback();
    let probe = transport.clone();
    let c = MCClient::new(transport, ModelId::Mc707.value());
    let project = Project {
        model: "MC-707".into(),
        name: None,
        tracks: vec![TrackDump {
            track_number: 1,
            settings: vec![0x22; 31],
            track_tone: TonePreset::new("MC-707", None, vec![]),
            clips: vec![],
        }],
        system_header: vec![],
        system_block: vec![],
    };
    let error = project.load_into(&c, false).unwrap_err();
    assert!(error.to_string().contains("settings"));
    assert_eq!(
        probe.snapshot(mc_programmer::addresses::TRK_SETTINGS_BASE_ADDR[0], 1),
        vec![0xFF]
    );
}
/// Verify an unrecognized but 7-bit step scale is restored without normalization.
#[test]
fn project_preserves_unknown_7bit_step_scale() {
    let transport = loopback();
    let probe = transport.clone();
    let c = MCClient::new(transport, ModelId::Mc707.value());
    let project = Project {
        model: "MC-707".into(),
        name: None,
        tracks: vec![TrackDump {
            track_number: 1,
            settings: vec![0x11; 0x20],
            track_tone: TonePreset::new("MC-707", None, vec![]),
            clips: vec![ClipDump {
                clip_number: 1,
                step_scale: 6,
                step_count: 1,
                note_steps: vec![],
                motion_steps: vec![],
                pitchbend: vec![],
                tone: TonePreset::new("MC-707", None, vec![]),
            }],
        }],
        system_header: vec![],
        system_block: vec![],
    };
    project.load_into(&c, false).unwrap();
    assert_eq!(
        probe.snapshot(c.addresses.clip_params_address(0, 0).unwrap(), 1),
        vec![6]
    );
}

/// Verify a non-MIDI step scale is rejected before track settings are written.
#[test]
fn project_rejects_high_bit_step_scale_before_writing() {
    let transport = loopback();
    let probe = transport.clone();
    let c = MCClient::new(transport, ModelId::Mc707.value());
    let project = Project {
        model: "MC-707".into(),
        name: None,
        tracks: vec![TrackDump {
            track_number: 1,
            settings: vec![0x11; 0x20],
            track_tone: TonePreset::new("MC-707", None, vec![]),
            clips: vec![ClipDump {
                clip_number: 1,
                step_scale: 0x80,
                step_count: 1,
                note_steps: vec![],
                motion_steps: vec![],
                pitchbend: vec![],
                tone: TonePreset::new("MC-707", None, vec![]),
            }],
        }],
        system_header: vec![],
        system_block: vec![],
    };
    let error = project.load_into(&c, false).unwrap_err();
    assert!(error.to_string().contains("step_scale"));
    assert_eq!(
        probe.snapshot(mc_programmer::addresses::TRK_SETTINGS_BASE_ADDR[0], 1),
        vec![0xFF]
    );
}

#[test]
fn project_rejects_partial_step_records_before_writing() {
    let transport = loopback();
    let probe = transport.clone();
    let c = MCClient::new(transport, ModelId::Mc707.value());
    let project = Project {
        model: "MC-707".into(),
        name: None,
        tracks: vec![TrackDump {
            track_number: 1,
            settings: vec![0x11; 0x20],
            track_tone: TonePreset::new("MC-707", None, vec![]),
            clips: vec![ClipDump {
                clip_number: 1,
                step_scale: 1,
                step_count: 1,
                note_steps: vec![0; 7],
                motion_steps: vec![],
                pitchbend: vec![],
                tone: TonePreset::new("MC-707", None, vec![]),
            }],
        }],
        system_header: vec![],
        system_block: vec![],
    };
    let error = project.load_into(&c, false).unwrap_err();
    assert!(error.to_string().contains("note_steps"));
    assert_eq!(
        probe.snapshot(mc_programmer::addresses::TRK_SETTINGS_BASE_ADDR[0], 1),
        vec![0xFF]
    );
}

/// Verify a tone region beyond its slot prevents all project writes.
#[test]
fn project_rejects_tone_region_outside_descriptor_before_writing() {
    let transport = loopback();
    let probe = transport.clone();
    let c = MCClient::new(transport, ModelId::Mc707.value());
    let project = Project {
        model: "MC-707".into(),
        name: None,
        tracks: vec![TrackDump {
            track_number: 1,
            settings: vec![0x11; 0x20],
            track_tone: TonePreset::new(
                "MC-707",
                None,
                vec![ToneRegion {
                    offset: mc_programmer::addresses::ZENCORE_TONE_ALIGNMENT,
                    label: "outside_slot".into(),
                    data: vec![0x22],
                }],
            ),
            clips: vec![],
        }],
        system_header: vec![],
        system_block: vec![],
    };
    let error = project.load_into(&c, false).unwrap_err();
    assert!(error.to_string().contains("tone region"));
    assert_eq!(
        probe.snapshot(mc_programmer::addresses::TRK_SETTINGS_BASE_ADDR[0], 1),
        vec![0xFF]
    );
}

/// Verify forced cross-model restore also bypasses nested tone model tags.
#[test]
fn force_overrides_nested_tone_model_tags() {
    let transport = LoopbackTransport::new(ModelId::Mc101, 0xFF);
    let probe = transport.clone();
    let c = MCClient::new(transport, ModelId::Mc101.value());
    let project = Project {
        model: "MC-707".into(),
        name: None,
        tracks: vec![TrackDump {
            track_number: 1,
            settings: vec![0x11; 0x20],
            track_tone: TonePreset::new(
                "MC-707",
                None,
                vec![ToneRegion {
                    offset: 0,
                    label: "tone_common".into(),
                    data: vec![0x33],
                }],
            ),
            clips: vec![],
        }],
        system_header: vec![],
        system_block: vec![],
    };
    project.load_into(&c, true).unwrap();
    assert_eq!(
        probe.snapshot(c.addresses.zencore_tone_address(0, None).unwrap(), 1),
        vec![0x33]
    );
}

/// Verify an oversized system block leaves system memory untouched.
#[test]
fn oversized_system_block_is_rejected_before_writing() {
    let transport = loopback();
    let probe = transport.clone();
    let c = MCClient::new(transport, ModelId::Mc707.value());
    let mut project = Project::new("MC-707", None, vec![]);
    project.system_block = vec![0; mc_programmer::project::SYSTEM_BLOCK_SIZE + 1];
    let error = project.load_into(&c, false).unwrap_err();
    assert!(error.to_string().contains("system_block"));
    assert_eq!(
        probe.snapshot(mc_programmer::project::SYSTEM_BLOCK_ADDRESS, 1),
        vec![0xFF]
    );
}

#[test]
fn short_rq1_without_checksum_is_ignored() {
    let transport = loopback();
    let mut request = mc_programmer::build_rq1(ModelId::Mc707.value(), 0x2000_0000, 1).unwrap();
    request.remove(request.len() - 2);
    assert!(
        mc_programmer::Transport::transfer(&transport, &request, 0.1)
            .unwrap()
            .is_empty()
    );
}

/// Verify every project payload category rejects high-bit data before writing track settings.
#[test]
fn project_rejects_high_bit_payloads_before_any_write() {
    let tone = || {
        TonePreset::new(
            "MC-707",
            None,
            vec![ToneRegion {
                offset: 0,
                label: "region".into(),
                data: vec![0x01],
            }],
        )
    };
    let base = Project {
        model: "MC-707".into(),
        name: None,
        tracks: vec![TrackDump {
            track_number: 1,
            settings: vec![0; 0x20],
            track_tone: tone(),
            clips: vec![ClipDump {
                clip_number: 1,
                step_scale: 0,
                step_count: 1,
                note_steps: vec![0; 256],
                motion_steps: vec![0; 256],
                pitchbend: vec![0; 256],
                tone: tone(),
            }],
        }],
        system_header: vec![0],
        system_block: vec![0],
    };
    let mut cases = Vec::new();
    let mut invalid = base.clone();
    invalid.tracks[0].settings[0] = 0x80;
    cases.push(("track settings", invalid));
    let mut invalid = base.clone();
    invalid.tracks[0].track_tone.regions[0].data[0] = 0x80;
    cases.push(("track tone", invalid));
    let mut invalid = base.clone();
    invalid.tracks[0].clips[0].note_steps[0] = 0x80;
    cases.push(("note steps", invalid));
    let mut invalid = base.clone();
    invalid.tracks[0].clips[0].motion_steps[0] = 0x80;
    cases.push(("motion steps", invalid));
    let mut invalid = base.clone();
    invalid.tracks[0].clips[0].pitchbend[0] = 0x80;
    cases.push(("pitchbend", invalid));
    let mut invalid = base.clone();
    invalid.tracks[0].clips[0].tone.regions[0].data[0] = 0x80;
    cases.push(("clip tone", invalid));
    let mut invalid = base.clone();
    invalid.system_header[0] = 0x80;
    cases.push(("system header", invalid));
    let mut invalid = base;
    invalid.system_block[0] = 0x80;
    cases.push(("system block", invalid));

    for (field, project) in cases {
        let transport = loopback();
        let probe = transport.clone();
        let c = MCClient::new(transport, ModelId::Mc707.value());
        let error = project.load_into(&c, false).unwrap_err();
        assert!(error.to_string().contains("0x80"), "{field}: {error}");
        assert_eq!(
            probe.snapshot(mc_programmer::addresses::TRK_SETTINGS_BASE_ADDR[0], 1),
            vec![0xFF],
            "{field} validation happened after a write"
        );
    }
}

/// Verify invalid data in a later chunk prevents the first chunk from being written.
#[test]
fn chunked_write_rejects_high_bit_bytes_before_first_chunk() {
    let transport = loopback();
    let probe = transport.clone();
    let c = MCClient::new(transport, ModelId::Mc707.value());
    let mut data = vec![0x40; MCClient::<LoopbackTransport>::CHUNK_SIZE + 1];
    data[MCClient::<LoopbackTransport>::CHUNK_SIZE] = 0x80;
    let error = c.write_chunked(0x2000_0000, &data).unwrap_err();
    assert!(error.to_string().contains("128"));
    assert_eq!(probe.snapshot(0x2000_0000, 1), vec![0xFF]);
}

/// Verify a later region outside the tone slot leaves both destination and adjacent slot untouched.
#[test]
fn standalone_tone_load_rejects_out_of_slot_region_before_writing() {
    let transport = loopback();
    let probe = transport.clone();
    let c = MCClient::new(transport, ModelId::Mc707.value());
    let base = c.addresses.zencore_tone_address(0, None).unwrap();
    let adjacent =
        mc_programmer::addresses::add_roland_address(base, mc_programmer::addresses::ZENCORE_TONE_ALIGNMENT).unwrap();
    let preset = TonePreset::new(
        "MC-707",
        None,
        vec![
            ToneRegion {
                offset: 0,
                label: "valid".into(),
                data: vec![0x33],
            },
            ToneRegion {
                offset: mc_programmer::addresses::ZENCORE_TONE_ALIGNMENT,
                label: "outside".into(),
                data: vec![0x44],
            },
        ],
    );
    let error = c.load_tone(1, None, &preset).unwrap_err();
    assert!(error.to_string().contains("descriptor slot"));
    assert_eq!(probe.snapshot(base, 1), vec![0xFF]);
    assert_eq!(probe.snapshot(adjacent, 1), vec![0xFF]);
}

/// Verify high-bit data in a later tone region prevents earlier valid regions from being written.
#[test]
fn standalone_tone_load_rejects_later_invalid_region_before_writing() {
    let transport = loopback();
    let probe = transport.clone();
    let c = MCClient::new(transport, ModelId::Mc707.value());
    let base = c.addresses.zencore_tone_address(0, None).unwrap();
    let preset = TonePreset::new(
        "MC-707",
        None,
        vec![
            ToneRegion {
                offset: 0,
                label: "valid".into(),
                data: vec![0x33],
            },
            ToneRegion {
                offset: 0x0100,
                label: "invalid".into(),
                data: vec![0x80],
            },
        ],
    );
    let error = c.load_tone(1, None, &preset).unwrap_err();
    assert!(error.to_string().contains("0x80"));
    assert_eq!(probe.snapshot(base, 1), vec![0xFF]);
}

#[test]
fn client_rejects_invalid_model_id_before_transport_use() {
    let c = MCClient::new(NullTransport::new(), 0xD5);
    let error = c.read(0x2000_0000, 1).unwrap_err();
    assert!(error.to_string().contains("model ID"));
}

#[test]
fn read_param_rejects_width_larger_than_u32() {
    let reply = build_dt1(ModelId::Mc707.value(), 0x3000_0000, &[0; 9]).unwrap();
    let transport = NullTransport::with_responses(vec![reply]);
    let c = MCClient::new(transport, ModelId::Mc707.value());
    let error = c.read_param(0x3000_0000, 9).unwrap_err();
    assert!(error.to_string().contains("size"));
}

/// Verify a nine-byte parameter width is rejected without altering device memory.
#[test]
fn write_param_rejects_width_larger_than_u32_before_writing() {
    let transport = loopback();
    let probe = transport.clone();
    let c = MCClient::new(transport, ModelId::Mc707.value());
    let error = c.write_param(0x2000_0000, 0x1234, 9).unwrap_err();
    assert!(error.to_string().contains("size"));
    assert_eq!(probe.snapshot(0x2000_0000, 1), vec![0xFF]);
}

#[test]
fn read_param_rejects_oversized_response_before_shifting() {
    let reply = build_dt1(ModelId::Mc707.value(), 0x3000_0000, &[0; 9]).unwrap();
    let transport = NullTransport::with_responses(vec![reply]);
    let c = MCClient::new(transport, ModelId::Mc707.value());
    let error = c.read_param(0x3000_0000, 8).unwrap_err();
    assert!(error.to_string().contains("response size"));
}
