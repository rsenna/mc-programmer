//! Boundary and failure cases for the Rust port, using pure codecs and scripted I/O.
use mc_programmer::addresses::{
    add_roland_address, add_roland_offset, roland_address_from_linear, roland_address_to_linear,
};
use mc_programmer::project::{SYSTEM_HEADER_SIZE, TRACK_SETTINGS_SIZE};
use mc_programmer::*;
use serde_json::json;

fn client(responses: Vec<Vec<u8>>) -> MCClient<NullTransport> {
    MCClient::new(
        NullTransport::with_responses(responses),
        ModelId::Mc707.value(),
    )
}

fn reply_at(address: u32, data: &[u8]) -> Vec<u8> {
    build_dt1(0x5d, address, data).unwrap()
}

fn reply(data: &[u8]) -> Vec<u8> {
    reply_at(0x3000_0000, data)
}
#[test]
fn loopback_snapshot_matches_the_7bit_view_of_rq1_reads() {
    let transport = LoopbackTransport::new(ModelId::Mc707, 0xff);
    let client = MCClient::new(transport.clone(), ModelId::Mc707.value());
    transport.preload(0, &[0xc5]);

    let read = client.read(0, 2).unwrap();
    assert_eq!(read, vec![0x45, 0x7f]);
    assert_eq!(transport.snapshot(0, 2), read);
}

#[test]
fn linear_addresses_carry_across_every_seven_bit_digit() {
    for (linear, packed) in [
        (0, 0),
        (0x7f, 0x7f),
        (0x80, 0x100),
        (0x3fff, 0x7f7f),
        (0x4000, 0x10000),
        (0x1fffff, 0x7f7f7f),
        (0x200000, 0x1000000),
        (0x0fffffff, 0x7f7f7f7f),
    ] {
        assert_eq!(roland_address_from_linear(linear).unwrap(), packed);
        assert_eq!(roland_address_to_linear(packed).unwrap(), linear);
    }
    for (address, next) in [(0x7f, 0x100), (0x7f7f, 0x10000), (0x7f7f7f, 0x1000000)] {
        assert_eq!(add_roland_offset(address, 1).unwrap(), next);
    }
}

#[test]
fn address_addition_distinguishes_packed_offsets_from_byte_counts() {
    assert_eq!(add_roland_address(0x307f_7f7f, 0x100).unwrap(), 0x3100_007f);
    assert_eq!(add_roland_offset(0x307f_7f7f, 0x100).unwrap(), 0x3100_017f);
}

#[test]
fn address_arithmetic_rejects_high_bits_and_overflow() {
    for invalid in [0x80, 0x8000, 0x800000, 0x80000000] {
        assert!(matches!(
            roland_address_to_linear(invalid),
            Err(Error::Value(_))
        ));
        assert!(add_roland_offset(invalid, 0).is_err());
        assert!(add_roland_address(0, invalid).is_err());
    }
    assert!(roland_address_from_linear(0x10000000).is_err());
    assert!(add_roland_offset(0x7f7f7f7f, 1).is_err());
    assert!(add_roland_offset(1, u32::MAX).is_err());
}

#[test]
fn motion_addresses_cover_last_lane_and_last_step_without_aliasing() {
    let map = AddressMap::new();
    assert_eq!(map.motion_step_address(0, 0, 0, 7).unwrap(), 0x2004_0070);
    assert_eq!(map.motion_step_address(0, 0, 127, 7).unwrap(), 0x2005_7e70);
    for (track, clip, step, lane) in [(8, 0, 0, 0), (0, 16, 0, 0), (0, 0, 128, 0), (0, 0, 0, 8)] {
        assert!(matches!(
            map.motion_step_address(track, clip, step, lane),
            Err(Error::Index(_))
        ));
    }
}

#[test]
fn request_lengths_use_base_128_at_every_boundary() {
    for (count, bytes, checksum) in [
        (0, [0, 0, 0, 0], 0x04),
        (127, [0, 0, 0, 127], 0x05),
        (128, [0, 0, 1, 0], 0x03),
        (16384, [0, 1, 0, 0], 0x03),
        (2097152, [1, 0, 0, 0], 0x03),
        (0x0fffffff, [127; 4], 0x08),
    ] {
        let message = build_rq1(0x5f, 0x7f7f7f7f, count).unwrap();
        assert_eq!(
            &message[..12],
            &[0xf0, 0x41, 0x10, 0, 0, 0, 0x5f, 0x11, 127, 127, 127, 127]
        );
        assert_eq!(&message[12..16], &bytes);
        assert_eq!(message[16], checksum);
        assert_eq!(message[17], 0xf7);
    }
}

#[test]
fn packet_builders_validate_model_and_every_payload_position() {
    for model in [0x80, 0xff] {
        assert!(matches!(build_rq1(model, 0, 1), Err(Error::Value(_))));
        assert!(matches!(build_dt1(model, 0, &[]), Err(Error::Value(_))));
    }
    for position in 0..3 {
        let mut data = [0x7f; 3];
        data[position] = 0x80;
        assert!(matches!(build_dt1(0x7f, 0, &data), Err(Error::Value(_))));
    }
    assert!(build_dt1(0x7f, 0x7f7f7f7f, &[0, 0x7f]).is_ok());
}

#[test]
fn dt1_parser_accepts_empty_payload_and_rejects_truncated_frames() {
    let message = reply(&[]);
    assert_eq!(parse_dt1_payload(&message).unwrap().data, Vec::<u8>::new());
    for size in 0..14 {
        assert!(matches!(parse_dt1_payload(&message[..size]), Err(Error::MessageTooShort(n)) if n == size));
    }
}

#[test]
fn dt1_parser_distinguishes_bad_framing_and_wrong_command() {
    for index in [0, 14] {
        let mut message = reply(&[42]);
        message[index] = 0;
        assert!(matches!(
            parse_dt1_payload(&message),
            Err(Error::MissingFraming)
        ));
    }
    let request = build_rq1(0x5d, 0, 1).unwrap();
    assert!(matches!(
        parse_dt1_payload(&request),
        Err(Error::WrongCommand(0x11))
    ));
}

#[test]
fn note_and_motion_constructors_require_exact_slot_counts() {
    for count in [0, 15, 17] {
        assert!(matches!(
            NoteStep::new(vec![NoteComponent::empty(); count], SubStep::None),
            Err(Error::Value(_))
        ));
    }
    for count in [0, 7, 9] {
        assert!(matches!(
            MotionStep::new(vec![None; count]),
            Err(Error::Value(_))
        ));
    }
    let lanes = vec![Some(vec![0, 1, 2, 3, 4, 5, 6, 127]); 8];
    assert_eq!(MotionStep::new(lanes.clone()).unwrap().lanes, lanes);
}

#[test]
fn compact_preserves_live_order_and_metadata_and_resets_empty_lanes() {
    let mut step = NoteStep {
        substep: SubStep::Flam,
        ..NoteStep::default()
    };
    step.components[2] = NoteComponent {
        note: 0,
        velocity: 1,
        start_cents: -50,
        end_cents: 100,
        processed: true,
    };
    step.components[15] = NoteComponent {
        note: 127,
        velocity: 127,
        start_cents: 99,
        end_cents: 0,
        processed: false,
    };
    step.components[0].velocity = 0;
    let live = step.active_components();
    step.compact();
    assert_eq!(&step.components[..2], live.as_slice());
    assert!(step.components[2..]
        .iter()
        .all(|c| *c == NoteComponent::empty()));
    assert_eq!(step.substep, SubStep::Flam);
    let compacted = step.clone();
    step.compact();
    assert_eq!(step, compacted);
}

#[test]
fn sequencer_wire_layout_covers_last_lane_and_scratch_flag() {
    let mut step = NoteStep {
        substep: SubStep::Quadruple,
        ..NoteStep::default()
    };
    step.components[15] = NoteComponent {
        note: 127,
        velocity: 127,
        start_cents: -50,
        end_cents: 255,
        processed: true,
    };
    let raw = encode_note_step(&step);
    assert_eq!(raw.len(), 256);
    assert_eq!(&raw[..16], &[0; 16]);
    assert_eq!(&raw[0x2e..0x30], &[7, 15]);
    assert_eq!(raw[0x3f], 127);
    assert_eq!(&raw[0x5e..0x60], &[12, 14]);
    assert_eq!(raw[0x6f], 100);
    assert_eq!(raw[0x70], 3);
    assert!(raw[0x71..].iter().all(|&b| b == 0));
    let decoded = decode_note_step(&raw).unwrap();
    assert!(!decoded.components[15].processed);
    assert!(decoded.components[15].ties_to_next());
}

#[test]
fn sequencer_decoder_checks_minimum_length_and_unknown_substeps() {
    let mut raw = encode_note_step(&NoteStep::default());
    for size in [0, 16, 112] {
        assert!(matches!(
            decode_note_step(&raw[..size]),
            Err(Error::Value(_))
        ));
    }
    assert_eq!(decode_note_step(&raw[..113]).unwrap(), NoteStep::default());
    for value in [5, 127] {
        raw[0x70] = value;
        assert_eq!(decode_note_step(&raw).unwrap().substep, SubStep::None);
    }
}

#[test]
fn zencore_numeric_codecs_have_explicit_wire_vectors() {
    for (size, max, value, bytes) in [
        (1, 127, 127, vec![127]),
        (2, 255, 0xa5, vec![10, 5]),
        (4, 65535, 0xabcd, vec![10, 11, 12, 13]),
    ] {
        let param = ZCParam::new("value", 0, size, 0, max, "Value", "");
        assert_eq!(param.encode(&value.into()).unwrap(), bytes);
        assert_eq!(param.decode(&bytes).unwrap(), ZCValue::Int(value));
        for boundary in [0, max] {
            assert_eq!(
                param
                    .decode(&param.encode(&boundary.into()).unwrap())
                    .unwrap(),
                boundary.into()
            );
        }
        for invalid in [-1, max + 1] {
            assert!(matches!(
                param.encode(&invalid.into()),
                Err(Error::Value(_))
            ));
        }
        assert!(param.encode(&"wrong type".into()).is_err());
        assert!(param.decode(&bytes[..bytes.len() - 1]).is_err());
    }
}

#[test]
fn zencore_rejects_unsupported_numeric_widths() {
    for size in [0, 3, 5, 8] {
        let param = ZCParam::new("value", 0, size, 0, 127, "Value", "");
        assert!(matches!(param.encode(&0.into()), Err(Error::Value(_))));
        assert!(matches!(
            param.decode(&vec![0; size as usize]),
            Err(Error::Value(_))
        ));
    }
}

#[test]
fn tone_name_padding_truncation_and_control_characters() {
    let param = lookup(ParamGroup::Common, "name").unwrap();
    assert_eq!(param.encode(&"".into()).unwrap(), vec![b' '; 16]);
    assert_eq!(
        param.encode(&"0123456789abcdefEXTRA".into()).unwrap(),
        b"0123456789abcdef"
    );
    assert_eq!(param.encode(&"A\nB".into()).unwrap(), b"A?B             ");
    assert_eq!(
        param.decode(b" A\0B            ignored").unwrap(),
        ZCValue::from(" A B")
    );
    assert!(matches!(param.encode(&42.into()), Err(Error::Value(_))));
    assert!(param.decode(b"short").is_err());
}

#[test]
fn every_partial_group_rejects_invalid_partial_indexes() {
    for group in [
        ParamGroup::Partial,
        ParamGroup::PartialPitchEnv,
        ParamGroup::PartialFilterEnv,
        ParamGroup::PartialAmpEnv,
        ParamGroup::PartialLfo,
        ParamGroup::PartialEq,
        ParamGroup::SynthPartial,
    ] {
        for index in [0, 5, usize::MAX] {
            assert!(
                matches!(substructure_offset(group, index), Err(Error::Index(_))),
                "{group:?} {index}"
            );
        }
        assert!(matches!(
            lookup(group, "missing_parameter"),
            Err(Error::Index(_))
        ));
    }
}

#[test]
fn null_transport_clones_share_fifo_without_raw_messages_consuming_replies() {
    let transport = NullTransport::with_responses(vec![vec![1], vec![2]]);
    let clone = transport.clone();
    clone.send_raw(&[0xc0, 1]).unwrap();
    assert_eq!(transport.transfer(&[3], 0.0).unwrap(), [1]);
    assert_eq!(clone.transfer(&[4], 0.0).unwrap(), [2]);
    assert!(transport.transfer(&[5], 0.0).unwrap().is_empty());
    assert_eq!(clone.sent(), vec![vec![0xc0, 1], vec![3], vec![4], vec![5]]);
}

#[test]
fn detection_accepts_all_supported_models_and_rejects_short_replies() {
    for model in [ModelId::Mc707, ModelId::Mc101, ModelId::Mv1] {
        let transport = NullTransport::with_responses(vec![vec![
            0xf0,
            0x7e,
            0x10,
            6,
            2,
            0x41,
            model.value(),
            0xf7,
        ]]);
        let detected = MCClient::detect(transport.clone()).unwrap();
        assert_eq!(detected.model_id, model.value());
        assert_eq!(transport.sent(), vec![vec![0xf0, 0x7e, 0x7f, 6, 1, 0xf7]]);
    }
    for size in 0..7 {
        assert!(matches!(
            MCClient::detect(NullTransport::with_responses(vec![vec![0; size]])),
            Err(Error::NoDeviceResponse)
        ));
    }
}

#[test]
fn empty_chunk_operations_do_not_contact_transport() {
    let client = client(vec![]);
    assert!(client.read_chunked(0x30000000, 0).unwrap().is_empty());
    client.write_chunked(0x30000000, &[]).unwrap();
    assert!(client.transport.sent().is_empty());
}

#[test]
fn chunked_write_splits_payload_and_carries_address_at_exact_boundary() {
    for length in [1, 127, 128, 129, 256, 257] {
        let client = client(vec![]);
        let data: Vec<u8> = (0..length).map(|i| (i % 128) as u8).collect();
        client.write_chunked(0x3000_007f, &data).unwrap();
        let messages = client.transport.sent();
        assert_eq!(messages.len(), (length + 127) / 128);
        for (index, (message, chunk)) in messages.iter().zip(data.chunks(128)).enumerate() {
            let decoded = parse_dt1_payload(message).unwrap();
            assert_eq!(decoded.address, [0x3000007f, 0x3000017f, 0x3000027f][index]);
            assert_eq!(decoded.data, chunk);
        }
    }
}

#[test]
fn chunked_read_rejects_missing_reply_and_preserves_short_tail() {
    let missing = client(vec![reply_at(0x3000_007f, &[7; 128]), vec![]]);
    assert!(missing.read_chunked(0x3000_007f, 257).is_err());
    assert_eq!(missing.transport.sent().len(), 2);

    let client = client(vec![
        reply_at(0x3000_007f, &[7; 128]),
        reply_at(0x3000_017f, &[8; 128]),
        reply_at(0x3000_027f, &[9]),
    ]);
    let data = client.read_chunked(0x3000_007f, 257).unwrap();
    assert_eq!(&data[..128], &[7; 128]);
    assert_eq!(&data[128..256], &[8; 128]);
    assert_eq!(data[256], 9);

    let messages = client.transport.sent();
    assert_eq!(messages.len(), 3);
    for (index, message) in messages.iter().enumerate() {
        assert_eq!(
            &message[8..12],
            &[[0x30, 0, 0, 127], [0x30, 0, 1, 127], [0x30, 0, 2, 127]][index]
        );
        assert_eq!(
            &message[12..16],
            if index == 2 {
                &[0, 0, 0, 1]
            } else {
                &[0, 0, 1, 0]
            }
        );
    }
}

#[test]
fn client_propagates_transport_failures() {
    struct FailingTransport;
    impl Transport for FailingTransport {
        fn transfer(&self, _: &[u8], _: f64) -> Result<Vec<u8>> {
            Err(Error::Midi("disconnected".into()))
        }
        fn send_raw(&self, _: &[u8]) -> Result<()> {
            Err(Error::Midi("disconnected".into()))
        }
    }
    let client = MCClient::new(FailingTransport, 0x5d);
    assert!(matches!(client.read_chunked(0, 129), Err(Error::Midi(_))));
    assert!(matches!(
        client.write_chunked(0, &[0; 129]),
        Err(Error::Midi(_))
    ));
    assert!(matches!(client.select_scene(1, 1), Err(Error::Midi(_))));
}

#[test]
fn front_panel_validation_happens_before_transport_use() {
    let client = client(vec![]);
    for (track, clip, step) in [
        (0, 1, 1),
        (9, 1, 1),
        (1, 0, 1),
        (1, 17, 1),
        (1, 1, 0),
        (1, 1, 129),
    ] {
        assert!(matches!(
            client.place_note(track, clip, step, 60u8, 100, 0.8),
            Err(Error::Index(_))
        ));
    }
    for (scene, channel) in [(0, 1), (129, 1), (1, 0), (1, 17)] {
        assert!(client.select_scene(scene, channel).is_err());
    }
    for (track, clip, channel) in [
        (0, 1, 1),
        (9, 1, 1),
        (1, 0, 1),
        (1, 17, 1),
        (1, 1, 0),
        (1, 1, 17),
    ] {
        assert!(client.select_clip(track, clip, Some(channel)).is_err());
    }
    assert!(client.transport.sent().is_empty());
}

#[test]
fn program_change_upper_boundaries_use_channel_sixteen() {
    let client = client(vec![]);
    client.select_scene(128, 16).unwrap();
    client.select_clip(8, 16, Some(16)).unwrap();
    assert_eq!(
        client.transport.sent(),
        vec![vec![0xcf, 127], vec![0xcf, 15]]
    );
}

#[test]
fn full_note_step_drops_new_note_without_writing() {
    let full = NoteStep::new(
        vec![
            NoteComponent {
                note: 60,
                ..NoteComponent::empty()
            };
            16
        ],
        SubStep::None,
    )
    .unwrap();
    let client = client(vec![reply_at(0x2003_0000, &encode_note_step(&full))]);
    client.place_note(1, 1, 1, 72u8, 100, 2.5).unwrap();
    let sent = client.transport.sent();
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0][7], 0x11);
}

#[test]
fn note_ties_stop_at_last_clip_step_and_velocity_is_clamped() {
    let address = AddressMap::new().note_step_address(7, 15, 127).unwrap();
    let client = client(vec![reply_at(
        address,
        &encode_note_step(&NoteStep::default()),
    )]);
    client.place_note(8, 16, 128, 127u8, 255, 3.5).unwrap();
    let sent = client.transport.sent();
    assert_eq!(sent.len(), 2);
    let message = parse_dt1_payload(&sent[1]).unwrap();
    assert_eq!(message.address, address);
    let step = decode_note_step(&message.data).unwrap();
    assert_eq!(
        step.active_components(),
        vec![NoteComponent {
            note: 127,
            velocity: 127,
            start_cents: 0,
            end_cents: 100,
            processed: false
        }]
    );
}

#[test]
fn zencore_client_no_reply_and_invalid_values() {
    let client = client(vec![]);
    assert_eq!(
        client
            .read_zc_param(0, None, "tone_coarse_tune", ParamGroup::Common, 1)
            .unwrap(),
        None
    );
    assert_eq!(client.transport.sent().len(), 1);
    for value in [15, 113] {
        assert!(client
            .write_zc_param(0, None, "tone_coarse_tune", value, ParamGroup::Common, 1)
            .is_err());
    }
    assert!(client
        .write_zc_param(0, None, "missing", 64, ParamGroup::Common, 1)
        .is_err());
    assert_eq!(client.transport.sent().len(), 1);
}

fn preset() -> TonePreset {
    TonePreset::new(
        "MC-707",
        Some("Boundary tone".into()),
        vec![ToneRegion {
            offset: 0,
            label: "common".into(),
            data: vec![0, 127],
        }],
    )
}

#[test]
fn preset_json_requires_all_fields_and_correct_types() {
    for field in ["format", "model", "name", "regions"] {
        let mut value = preset().to_value();
        value.as_object_mut().unwrap().remove(field);
        assert!(
            matches!(TonePreset::from_value(&value), Err(Error::Json(_))),
            "missing {field}"
        );
        let mut value = preset().to_value();
        value[field] = json!(42);
        assert!(
            matches!(TonePreset::from_value(&value), Err(Error::Json(_))),
            "type of {field}"
        );
    }
    for field in ["offset", "label", "data"] {
        let mut value = preset().to_value();
        value["regions"][0].as_object_mut().unwrap().remove(field);
        assert!(
            matches!(TonePreset::from_value(&value), Err(Error::Json(_))),
            "region {field}"
        );
    }
}

#[test]
fn preset_json_rejects_bad_hex_offsets_and_bytes() {
    for offset in ["10", "0x", "0xG0", "0x100000000", "0x-1"] {
        let mut value = preset().to_value();
        value["regions"][0]["offset"] = json!(offset);
        assert!(
            matches!(TonePreset::from_value(&value), Err(Error::Json(_))),
            "{offset}"
        );
    }
    for data in ["GG", "100", "-1", "00 7F ZZ"] {
        let mut value = preset().to_value();
        value["regions"][0]["data"] = json!(data);
        assert!(
            matches!(TonePreset::from_value(&value), Err(Error::Json(_))),
            "{data}"
        );
    }
}

#[test]
fn preset_json_accepts_uppercase_prefix_whitespace_and_null_name() {
    let mut value = preset().to_value();
    value["name"] = json!(null);
    value["regions"][0]["offset"] = json!("0X007f");
    value["regions"][0]["data"] = json!(" 00\t7f\n0a ");
    let tone = TonePreset::from_value(&value).unwrap();
    assert_eq!(tone.name, None);
    assert_eq!(tone.regions[0].offset, 127);
    assert_eq!(tone.regions[0].data, [0, 127, 10]);
    assert_eq!(TonePreset::from_json(&tone.to_json()).unwrap(), tone);
}

#[test]
fn tone_restore_accepts_exact_slot_end_but_rejects_one_byte_overrun() {
    let mut tone = preset();
    tone.model.clear(); // Untagged presets may be loaded onto any supported model.
    tone.regions[0].offset = 0x0001_7f7f;
    tone.regions[0].data = vec![42];
    let client = client(vec![]);
    client.load_tone(1, Some(1), &tone).unwrap();
    let messages = client.transport.sent();
    assert_eq!(messages.len(), 1);
    assert_eq!(
        parse_dt1_payload(&messages[0]).unwrap().address,
        0x3001_7f7f
    );
    tone.regions[0].data.push(43);
    assert!(matches!(
        client.load_tone(1, Some(1), &tone),
        Err(Error::Value(_))
    ));
    assert_eq!(client.transport.sent().len(), 1);
}

fn project() -> Project {
    Project::new(
        "MC-707",
        None,
        vec![TrackDump {
            track_number: 1,
            settings: vec![0; TRACK_SETTINGS_SIZE],
            track_tone: preset(),
            clips: vec![ClipDump {
                clip_number: 1,
                step_scale: 1,
                step_count: 1,
                note_steps: vec![],
                motion_steps: vec![],
                pitchbend: vec![],
                tone: preset(),
            }],
        }],
    )
}

#[test]
fn project_json_rejects_invalid_step_counts_and_nested_types() {
    for count in [
        json!(0),
        json!(129),
        json!(-1),
        json!(1.5),
        json!("1"),
        json!(null),
    ] {
        let mut value = project().to_value();
        value["tracks"][0]["clips"][0]["step_count"] = count.clone();
        assert!(
            matches!(Project::from_value(&value), Err(Error::Json(_))),
            "{count}"
        );
    }
    for path in ["tracks", "name", "system_header", "system_block"] {
        let mut value = project().to_value();
        value[path] = json!(42);
        assert!(
            matches!(Project::from_value(&value), Err(Error::Json(_))),
            "{path}"
        );
    }
}

#[test]
fn project_preflight_rejects_late_invalid_clip_even_when_forced() {
    for force in [false, true] {
        for step_count in [0, 129] {
            let mut project = project();
            let mut later = project.tracks[0].clone();
            later.track_number = 2;
            later.clips[0].step_count = step_count;
            project.tracks.push(later);
            let client = client(vec![]);
            assert!(matches!(
                project.load_into(&client, force),
                Err(Error::Value(_))
            ));
            assert!(client.transport.sent().is_empty());
        }
    }
}

#[test]
fn project_preflight_rejects_oversized_header_before_track_writes() {
    let mut project = project();
    project.system_header = vec![0; SYSTEM_HEADER_SIZE + 1];
    let client = client(vec![]);
    assert!(matches!(
        project.load_into(&client, false),
        Err(Error::Value(_))
    ));
    assert!(client.transport.sent().is_empty());
}

#[test]
fn project_force_does_not_bypass_nested_tone_payload_validation() {
    let mut project = project();
    project.model = "MC-101".into();
    project.tracks[0].clips[0].tone.regions[0].data = vec![0x80];
    let client = client(vec![]);
    assert!(matches!(
        project.load_into(&client, true),
        Err(Error::Value(_))
    ));
    assert!(client.transport.sent().is_empty());
}
