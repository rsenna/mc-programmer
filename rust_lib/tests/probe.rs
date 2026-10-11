//! Tests for the address-space probe.

use mc_programmer::addresses::{roland_address_from_linear, roland_address_to_linear};
use mc_programmer::probe::{self, ProbeOptions, ScanCoverage, Snapshot};
use mc_programmer::{LoopbackTransport, MCClient, ModelId, NullTransport};

/// A device whose first mapped byte is at `first_mapped` and which answers
/// every request with the next mapped run at or after it — the behaviour that
/// makes a masked reply collapse onto the requested address.
struct FirstMappedAt {
    first_mapped: u32,
}

impl mc_programmer::Transport for FirstMappedAt {
    fn transfer(&self, message: &[u8], _timeout: f64) -> mc_programmer::Result<Vec<u8>> {
        let packed = u32::from_be_bytes([message[8], message[9], message[10], message[11]]);
        let base = mc_programmer::addresses::roland_address_to_linear(packed).unwrap();
        let count = ((u32::from(message[12]) << 21)
            | (u32::from(message[13]) << 14)
            | (u32::from(message[14]) << 7)
            | u32::from(message[15])) as usize;
        let start = base.max(self.first_mapped);
        let data: Vec<u8> = (0..count).map(|i| 0x20 + (i as u8 % 0x40)).collect();
        Ok(mc_programmer::build_dt1(ModelId::Mc101.value(), start & 0x7f7f_7f7f, &data).unwrap())
    }
}

fn mc101() -> (LoopbackTransport, MCClient<LoopbackTransport>) {
    // Hardware-faithful reply addresses: the probe's whole job is mapping
    // replies back to addresses, so a fake that echoes the request address
    // would not exercise it.
    let transport = LoopbackTransport::new(ModelId::Mc101, 0xff).with_hardware_reply_addresses();
    let client = MCClient::new(transport.clone(), ModelId::Mc101.value());
    (transport, client)
}

fn full_coverage() -> Option<ScanCoverage> {
    Some(ScanCoverage {
        start_linear: 0,
        end_linear: 0x1000_0000,
        chunk: 0x80,
        step: 0x80,
    })
}
#[test]
fn walk_records_every_answered_byte() {
    let (transport, client) = mc101();
    transport.preload(0x20000000, &[0x10, 0x11, 0x12, 0x13]);

    let snapshot = probe::walk(&client, 0x20000000, 0x20000004, &ProbeOptions::gapless(4)).unwrap();

    assert_eq!(snapshot.model, "MC-101");
    assert_eq!(snapshot.len(), 4);
    assert_eq!(snapshot.chunks_probed, 1);
    assert_eq!(snapshot.chunks_answered, 1);
    assert_eq!(snapshot.bytes[&0x20000002], 0x12);
}

#[test]
fn walk_trims_the_last_chunk_to_the_end_address() {
    let (_transport, client) = mc101();

    // 20 bytes requested with a 16-byte chunk: 16 then 4, never past the end.
    let snapshot = probe::walk(&client, 0x20000000, 0x20000014, &ProbeOptions::default()).unwrap();

    assert_eq!(snapshot.chunks_probed, 2);
    assert_eq!(snapshot.len(), 20);
    assert!(snapshot.bytes.contains_key(&0x20000013));
    assert!(!snapshot.bytes.contains_key(&0x20000014));
}

#[test]
fn walk_only_ever_produces_valid_7bit_addresses() {
    let (_transport, client) = mc101();

    // Straddles the 0x7F -> 0x0100 packed discontinuity.
    let snapshot = probe::walk(&client, 0x20000070, 0x20000110, &ProbeOptions::default()).unwrap();

    assert!(!snapshot.is_empty());
    for &address in snapshot.bytes.keys() {
        assert_eq!(
            address & 0x8080_8080,
            0,
            "address 0x{address:08X} has a high MSB"
        );
    }
    // 0x7F is immediately followed by 0x0100 in linear order, with no 0x80.
    assert!(snapshot.bytes.contains_key(&0x2000007f));
    assert!(snapshot.bytes.contains_key(&0x20000100));
    assert!(!snapshot.bytes.contains_key(&0x20000080));
}

#[test]
fn walk_skips_addresses_that_do_not_answer() {
    // NullTransport never replies, which is how the real device reports an
    // unmapped region.
    let client = MCClient::new(NullTransport::new(), ModelId::Mc101.value());

    let snapshot = probe::walk(&client, 0x01000000, 0x01000040, &ProbeOptions::default()).unwrap();

    assert_eq!(snapshot.chunks_probed, 4);
    assert_eq!(snapshot.chunks_answered, 0);
    assert!(snapshot.is_empty());
}

#[test]
fn walk_with_a_step_larger_than_the_chunk_samples_sparsely() {
    let (_transport, client) = mc101();
    let opts = ProbeOptions {
        chunk: 0x10,
        step: 0x40,
        timeout: 0.1,
    };

    // 0x20000200 is linear +0x100, so four 0x40 strides fit: linear 0x00,
    // 0x40, 0x80, 0xC0 -> packed 0x...0000, 0x...0040, 0x...0100, 0x...0140.
    let snapshot = probe::walk(&client, 0x20000000, 0x20000200, &opts).unwrap();

    assert_eq!(snapshot.chunks_probed, 4);
    assert_eq!(snapshot.len(), 4 * 0x10);
    assert!(snapshot.bytes.contains_key(&0x20000040));
    assert!(snapshot.bytes.contains_key(&0x20000100));
    assert!(
        !snapshot.bytes.contains_key(&0x20000020),
        "0x20 is inside a skipped gap"
    );
}

#[test]
fn empty_range_is_allowed() {
    let (_transport, client) = mc101();
    let snapshot = probe::walk(&client, 0x20000000, 0x20000000, &ProbeOptions::default()).unwrap();
    assert_eq!(snapshot.chunks_probed, 0);
    assert!(snapshot.is_empty());
}

#[test]
fn walk_rejects_a_backwards_range() {
    let (_transport, client) = mc101();
    let err = probe::walk(&client, 0x20000010, 0x20000000, &ProbeOptions::default()).unwrap_err();
    assert!(err.to_string().contains("below start"), "{err}");
}

#[test]
fn probe_options_are_validated() {
    let (_transport, client) = mc101();
    let bad = [
        ProbeOptions {
            chunk: 0,
            step: 0x10,
            timeout: 0.1,
        },
        ProbeOptions {
            chunk: 0x81,
            step: 0x81,
            timeout: 0.1,
        },
        ProbeOptions {
            chunk: 0x30,
            step: 0x30,
            timeout: 0.1,
        }, // does not divide 0x80
        ProbeOptions {
            chunk: 0x10,
            step: 0x08,
            timeout: 0.1,
        }, // step < chunk
        ProbeOptions {
            chunk: 0x10,
            step: 0x10,
            timeout: 0.0,
        },
        ProbeOptions {
            chunk: 0x10,
            step: 0x10,
            timeout: f64::INFINITY,
        },
        ProbeOptions {
            chunk: 0x10,
            step: 0x10,
            timeout: f64::MAX,
        },
        ProbeOptions {
            chunk: 0x10,
            step: 0x10,
            timeout: f64::NAN,
        },
    ];
    for opts in bad {
        assert!(
            probe::walk(&client, 0x20000000, 0x20000010, &opts).is_err(),
            "expected {opts:?} to be rejected"
        );
    }
}

#[test]
fn a_garbled_reply_is_recorded_and_the_sweep_continues() {
    // Answers every request with a frame that is not a parseable DT1. The real
    // device does this occasionally; it must not abort a multi-minute sweep.
    struct Garbled;
    impl mc_programmer::Transport for Garbled {
        fn transfer(&self, _message: &[u8], _timeout: f64) -> mc_programmer::Result<Vec<u8>> {
            Ok(vec![0xf0, 0x41, 0x10, 0x5e, 0x12, 0xf7]) // too short for DT1
        }
    }

    let client = MCClient::new(Garbled, ModelId::Mc101.value());
    let snapshot = probe::walk(&client, 0x20000000, 0x20000030, &ProbeOptions::default()).unwrap();

    assert_eq!(snapshot.chunks_probed, 3, "the sweep ran to the end");
    assert_eq!(snapshot.chunks_failed, 3);
    assert_eq!(snapshot.chunks_answered, 0);
    assert!(snapshot.is_empty());
    assert_eq!(snapshot.failures.len(), 3);
    assert_eq!(snapshot.failures[0].0, 0x20000000);
}

#[test]
fn a_sweep_keeps_the_bytes_it_got_before_a_failure() {
    // Good reply, then garbage, then good again: the good bytes survive.
    // Garbles one ADDRESS rather than one call index: the confirmation
    // follow-up adds exchanges, so a call counter would move the fault around.
    struct GarblesOneAddress;
    impl mc_programmer::Transport for GarblesOneAddress {
        fn transfer(&self, message: &[u8], _timeout: f64) -> mc_programmer::Result<Vec<u8>> {
            let packed = u32::from_be_bytes([message[8], message[9], message[10], message[11]]);
            let base = mc_programmer::addresses::roland_address_to_linear(packed).unwrap();
            if base == 0x0400_0010 {
                return Ok(vec![0xf0, 0xf7]); // unparseable
            }
            Ok(mc_programmer::build_dt1(ModelId::Mc101.value(), base & 0x7f7f_7f7f, &[0x2b; 0x10]).unwrap())
        }
    }

    let client = MCClient::new(GarblesOneAddress, ModelId::Mc101.value());
    let snapshot = probe::walk(&client, 0x20000000, 0x20000030, &ProbeOptions::default()).unwrap();

    assert_eq!(snapshot.chunks_probed, 3);
    assert_eq!(snapshot.chunks_failed, 1);
    assert_eq!(snapshot.chunks_answered, 2);
    assert_eq!(snapshot.len(), 0x20, "two good chunks of 16 bytes kept");
    assert!(snapshot.bytes.contains_key(&0x20000000));
    assert!(
        !snapshot.bytes.contains_key(&0x20000010),
        "the failed chunk left a hole"
    );
    assert!(snapshot.bytes.contains_key(&0x20000020));
}

#[test]
fn failures_round_trip_through_json() {
    let mut snapshot = Snapshot {
        model: "MC-101".to_string(),
        label: None,
        bytes: Default::default(),
        coverage: full_coverage(),
        chunks_probed: 2,
        chunks_answered: 1,
        chunks_failed: 1,
        failures: vec![(0x20000010, "DT1 message too short: 2 bytes".to_string())],
        relocated_replies: 0,
        unplaceable_replies: 0,
        subdivisions: 0,
        truncated_placements: 0,
        unprovable_placements: 0,
        confirmations: 0,
        confirmation_exchanges: 0,
        unconfirmed_placements: 0,
    };
    snapshot.bytes.insert(0x20000000, 0x7f);

    let restored = Snapshot::from_json(&snapshot.to_json()).unwrap();

    assert_eq!(restored.chunks_failed, 1);
    assert_eq!(restored.failures, snapshot.failures);
    assert_eq!(restored.bytes, snapshot.bytes);
}

#[test]
fn a_relocated_reply_is_narrowed_until_only_confirmed_bytes_are_placed() {
    // Real MC-101 behaviour: asked about packed 0x40000024 it answers for
    // 0x40000026, having skipped forward past unmapped bytes.
    //
    // A relocated reply can never satisfy the coverage gate — if it starts
    // later than the request it cannot also end within it — so the sweep
    // narrows the request instead of trusting the payload wholesale, and only
    // the byte the device actually reported an address for is recorded.
    // Coherent model: nothing mapped below linear 0x08000026, and every
    // request answered with the next mapped run at or after it — including the
    // confirmation follow-up, which a fixed-reply fake could not answer.

    let client = MCClient::new(
        FirstMappedAt {
            first_mapped: 0x0800_0026,
        },
        ModelId::Mc101.value(),
    );
    let snapshot = probe::walk(&client, 0x40000024, 0x40000028, &ProbeOptions::gapless(4)).unwrap();

    assert!(
        snapshot.subdivisions > 0,
        "a relocated reply must be narrowed, not trusted"
    );
    assert_eq!(snapshot.unplaceable_replies, 0);
    // Only the first byte of the payload is confirmed to live at 0x40000026.
    assert_eq!(snapshot.bytes.get(&0x40000026), Some(&0x20));
    // 0x...27 is legitimate: narrowing reaches a request the device answers in
    // full, so those bytes are confirmed rather than assumed.
    assert!(
        !snapshot.bytes.contains_key(&0x40000025),
        "bytes below the first mapped address must stay absent"
    );
    assert!(
        !snapshot.bytes.contains_key(&0x40000024),
        "the requested address must stay empty when the device answered elsewhere"
    );
}

#[test]
fn an_unresolvable_reply_is_discarded_not_guessed() {
    // A reply naming an address outside the requested span cannot be placed.
    // Discarding it is the only safe move: a guess becomes a wrong entry in
    // the address map that looks entirely plausible.
    struct RepliesOutOfRange;
    impl mc_programmer::Transport for RepliesOutOfRange {
        fn transfer(&self, _message: &[u8], _timeout: f64) -> mc_programmer::Result<Vec<u8>> {
            Ok(mc_programmer::build_dt1(ModelId::Mc101.value(), 0x00000001, &[0x7f; 4]).unwrap())
        }
    }

    let client = MCClient::new(RepliesOutOfRange, ModelId::Mc101.value());
    let snapshot = probe::walk(
        &client,
        0x40000000,
        0x40000010,
        &ProbeOptions::gapless(0x10),
    )
    .unwrap();

    assert_eq!(snapshot.unplaceable_replies, 1);
    assert!(
        snapshot.is_empty(),
        "nothing may be recorded from an unplaceable reply"
    );
    assert_eq!(snapshot.failures.len(), 1);
    assert!(
        snapshot.failures[0].1.contains("resolves to nothing"),
        "{:?}",
        snapshot.failures[0]
    );
}

#[test]
fn a_reply_for_the_requested_address_is_not_counted_as_relocated() {
    let (transport, client) = mc101();
    transport.preload(0x20000000, &[1, 2, 3, 4]);

    let snapshot = probe::walk(&client, 0x20000000, 0x20000004, &ProbeOptions::gapless(4)).unwrap();

    assert_eq!(snapshot.relocated_replies, 0);
    assert_eq!(snapshot.bytes[&0x20000000], 1);
}

// --- diffing -----------------------------------------------------------------

#[test]
fn diff_finds_a_single_changed_byte() {
    let (transport, client) = mc101();
    transport.preload(0x20000000, &[0x01, 0x02, 0x03, 0x04]);
    let before = probe::walk(&client, 0x20000000, 0x20000004, &ProbeOptions::gapless(4)).unwrap();

    transport.preload(0x20000002, &[0x33]);
    let after = probe::walk(&client, 0x20000000, 0x20000004, &ProbeOptions::gapless(4)).unwrap();

    let diffs = probe::diff(&before, &after).unwrap();
    assert_eq!(diffs.len(), 1);
    assert_eq!(diffs[0].address, 0x20000002);
    assert_eq!(diffs[0].before, Some(0x03));
    assert_eq!(diffs[0].after, Some(0x33));
}

#[test]
fn identical_snapshots_have_no_diff() {
    let (transport, client) = mc101();
    transport.preload(0x20000000, &[0x01, 0x02]);
    let a = probe::walk(&client, 0x20000000, 0x20000002, &ProbeOptions::gapless(2)).unwrap();
    let b = probe::walk(&client, 0x20000000, 0x20000002, &ProbeOptions::gapless(2)).unwrap();

    assert!(probe::diff(&a, &b).unwrap().is_empty());
    assert_eq!(probe::format_diff(&[]), "no differences\n");
}

#[test]
fn diff_ignores_addresses_skipped_by_one_sampling_pattern() {
    let (transport, client) = mc101();
    transport.preload(0x20000000, &[0x00; 0x20]);
    let before = probe::walk(
        &client,
        0x20000000,
        0x20000040,
        &ProbeOptions {
            chunk: 0x08,
            step: 0x10,
            timeout: 0.1,
        },
    )
    .unwrap();

    transport.preload(0x20000000, &[0x11]);
    transport.preload(0x20000008, &[0x22]);
    let after = probe::walk(
        &client,
        0x20000000,
        0x20000040,
        &ProbeOptions {
            chunk: 0x08,
            step: 0x08,
            timeout: 0.1,
        },
    )
    .unwrap();

    let diffs = probe::diff(&before, &after).unwrap();
    assert_eq!(diffs.len(), 1);
    assert_eq!(diffs[0].address, 0x20000000);
    assert_eq!((diffs[0].before, diffs[0].after), (Some(0x00), Some(0x11)));
}

#[test]
fn diff_only_reports_bytes_covered_by_both_sweeps() {
    let (transport, client) = mc101();
    transport.preload(0x20000000, &[0x00; 0x18]);
    let before = probe::walk(
        &client,
        0x20000000,
        0x20000020,
        &ProbeOptions::gapless(0x10),
    )
    .unwrap();

    transport.preload(0x20000010, &[0x2a]);
    transport.preload(0x20000020, &[0xbb]);
    let after = probe::walk(
        &client,
        0x20000010,
        0x20000030,
        &ProbeOptions::gapless(0x10),
    )
    .unwrap();

    let diffs = probe::diff(&before, &after).unwrap();
    assert_eq!(diffs.len(), 1);
    assert_eq!(diffs[0].address, 0x20000010);
    assert_eq!((diffs[0].before, diffs[0].after), (Some(0x00), Some(0x2a)));
}

#[test]
fn diff_rejects_snapshots_from_different_models() {
    let (_transport, client) = mc101();
    let before = probe::walk(
        &client,
        0x20000000,
        0x20000010,
        &ProbeOptions::gapless(0x10),
    )
    .unwrap();
    let mut after = before.clone();
    after.model = "MC-707".to_string();

    let error = probe::diff(&before, &after).unwrap_err();
    assert!(error.to_string().contains("different models"), "{error}");
}

#[test]
fn legacy_snapshots_without_coverage_cannot_be_diffed() {
    let (_transport, client) = mc101();
    let snapshot = probe::walk(
        &client,
        0x20000000,
        0x20000010,
        &ProbeOptions::gapless(0x10),
    )
    .unwrap();
    let mut legacy = snapshot.to_value();
    legacy.as_object_mut().unwrap().remove("coverage");
    let legacy = Snapshot::from_value(&legacy).unwrap();

    let error = probe::diff(&legacy, &snapshot).unwrap_err();
    assert!(error.to_string().contains("coverage"), "{error}");
}

#[test]
fn diff_reports_addresses_present_on_only_one_side() {
    let mut before = Snapshot {
        model: "MC-101".to_string(),
        label: None,
        bytes: Default::default(),
        coverage: full_coverage(),
        chunks_probed: 1,
        chunks_answered: 1,
        chunks_failed: 0,
        failures: Vec::new(),
        relocated_replies: 0,
        unplaceable_replies: 0,
        subdivisions: 0,
        truncated_placements: 0,
        unprovable_placements: 0,
        confirmations: 0,
        confirmation_exchanges: 0,
        unconfirmed_placements: 0,
    };
    before.bytes.insert(0x20000000, 0xaa);

    let mut after = before.clone();
    after.bytes.remove(&0x20000000);
    after.bytes.insert(0x20000001, 0xbb);

    let diffs = probe::diff(&before, &after).unwrap();
    assert_eq!(diffs.len(), 2);
    assert_eq!((diffs[0].before, diffs[0].after), (Some(0xaa), None));
    assert_eq!((diffs[1].before, diffs[1].after), (None, Some(0xbb)));
}

#[test]
fn diff_runs_group_contiguous_changes_and_split_at_gaps() {
    let (transport, client) = mc101();
    transport.preload(0x20000000, &[0; 16]);
    let before = probe::walk(&client, 0x20000000, 0x20000010, &ProbeOptions::default()).unwrap();

    // Two separate fields: a two-byte one at +0x02 and a one-byte one at +0x08.
    transport.preload(0x20000002, &[0x11, 0x22]);
    transport.preload(0x20000008, &[0x33]);
    let after = probe::walk(&client, 0x20000000, 0x20000010, &ProbeOptions::default()).unwrap();

    let runs = probe::diff_runs(&probe::diff(&before, &after).unwrap());
    assert_eq!(runs.len(), 2);
    assert_eq!(runs[0].address, 0x20000002);
    assert_eq!(runs[0].len(), 2);
    assert_eq!(runs[1].address, 0x20000008);
    assert_eq!(runs[1].len(), 1);

    let report = probe::format_diff(&runs);
    assert!(
        report.contains("2 changed run(s), 3 byte(s) total"),
        "{report}"
    );
    assert!(report.contains("20000002"), "{report}");
    assert!(report.contains("11 22"), "{report}");
}

#[test]
fn diff_runs_join_across_the_packed_address_discontinuity() {
    let mut before = Snapshot {
        model: "MC-101".to_string(),
        label: None,
        bytes: Default::default(),
        coverage: full_coverage(),
        chunks_probed: 1,
        chunks_answered: 1,
        chunks_failed: 0,
        failures: Vec::new(),
        relocated_replies: 0,
        unplaceable_replies: 0,
        subdivisions: 0,
        truncated_placements: 0,
        unprovable_placements: 0,
        confirmations: 0,
        confirmation_exchanges: 0,
        unconfirmed_placements: 0,
    };
    before.bytes.insert(0x2000007f, 0x00);
    before.bytes.insert(0x20000100, 0x00);

    let mut after = before.clone();
    after.bytes.insert(0x2000007f, 0x01);
    after.bytes.insert(0x20000100, 0x02);

    // 0x7F and 0x0100 are adjacent in linear space, so this is one field.
    let runs = probe::diff_runs(&probe::diff(&before, &after).unwrap());
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].len(), 2);
    assert_eq!(runs[0].address, 0x2000007f);
}

// --- snapshot serialization --------------------------------------------------

#[test]
fn snapshot_json_round_trips() {
    let (transport, client) = mc101();
    transport.preload(0x20000000, &[0xde, 0xad, 0x7e, 0x7f]);
    let snapshot = probe::walk(&client, 0x20000000, 0x20000004, &ProbeOptions::gapless(4))
        .unwrap()
        .labelled("scene 1 selected");

    let restored = Snapshot::from_json(&snapshot.to_json()).unwrap();

    assert_eq!(restored.bytes, snapshot.bytes);
    assert_eq!(restored.coverage, snapshot.coverage);
    assert_eq!(restored.model, "MC-101");
    assert_eq!(restored.label.as_deref(), Some("scene 1 selected"));
    assert_eq!(restored.chunks_probed, snapshot.chunks_probed);
}

#[test]
fn snapshot_save_replaces_an_existing_file() {
    let (_transport, client) = mc101();
    let original = probe::walk(
        &client,
        0x20000000,
        0x20000010,
        &ProbeOptions::gapless(0x10),
    )
    .unwrap();
    let path = std::env::temp_dir().join(format!("mc-probe-save-{}.json", std::process::id()));
    let path = path.to_string_lossy().into_owned();

    original.save(&path).unwrap();
    let replacement = original.clone().labelled("replacement");
    replacement.save(&path).unwrap();

    assert_eq!(Snapshot::load(&path).unwrap(), replacement);
    std::fs::remove_file(&path).unwrap();
}

#[test]
fn snapshot_json_round_trips_across_the_packed_discontinuity() {
    let (_transport, client) = mc101();
    let snapshot = probe::walk(&client, 0x20000078, 0x20000108, &ProbeOptions::default()).unwrap();

    let restored = Snapshot::from_json(&snapshot.to_json()).unwrap();

    assert_eq!(restored.bytes, snapshot.bytes);
    for &address in restored.bytes.keys() {
        assert_eq!(address & 0x8080_8080, 0);
    }
}

#[test]
fn loading_a_foreign_json_file_is_refused() {
    let err = Snapshot::from_json(r#"{"format":"mc_programmer.tone.v1"}"#).unwrap_err();
    assert!(err.to_string().contains("not a probe snapshot"), "{err}");
}

#[test]
fn runs_collapse_contiguous_bytes_and_split_at_gaps() {
    let mut snapshot = Snapshot {
        model: "MC-101".to_string(),
        label: None,
        bytes: Default::default(),
        coverage: full_coverage(),
        chunks_probed: 2,
        chunks_answered: 2,
        chunks_failed: 0,
        failures: Vec::new(),
        relocated_replies: 0,
        unplaceable_replies: 0,
        subdivisions: 0,
        truncated_placements: 0,
        unprovable_placements: 0,
        confirmations: 0,
        confirmation_exchanges: 0,
        unconfirmed_placements: 0,
    };
    snapshot.bytes.insert(0x20000000, 1);
    snapshot.bytes.insert(0x20000001, 2);
    snapshot.bytes.insert(0x20000010, 3);

    let runs = snapshot.runs();
    assert_eq!(runs.len(), 2);
    assert_eq!(runs[0].bytes, vec![1, 2]);
    assert_eq!(runs[1].address, 0x20000010);
}

#[test]
fn hex_dump_and_csv_label_each_row_with_its_own_address() {
    let (transport, client) = mc101();
    transport.preload(0x20000000, b"DM-101 SCENE 01x");
    transport.preload(0x20000010, &[0x41; 16]);
    let snapshot = probe::walk(&client, 0x20000000, 0x20000020, &ProbeOptions::default()).unwrap();

    let dump = snapshot.hex_dump();
    assert!(dump.contains("20000000"), "{dump}");
    assert!(dump.contains("|DM-101 SCENE 01x|"), "{dump}");
    assert!(dump.contains("20000010"), "{dump}");

    let csv = snapshot.to_csv();
    assert!(csv.starts_with("address,length,hex,ascii\n"), "{csv}");
    assert_eq!(
        csv.lines().count(),
        3,
        "header plus two 16-byte rows:\n{csv}"
    );
}

#[test]
fn probe_progress_reports_each_chunk() {
    let (_transport, client) = mc101();
    let mut seen: Vec<(u32, usize)> = Vec::new();

    probe::walk_with_progress(
        &client,
        0x20000000,
        0x20000030,
        &ProbeOptions::default(),
        &mut |address, len| seen.push((address, len)),
    )
    .unwrap();

    assert_eq!(seen.len(), 3);
    assert_eq!(seen[0], (0x20000000, 0x10));
    assert_eq!(seen[2].0, 0x20000020);
}

#[test]
fn linear_round_trip_matches_the_address_helpers() {
    // Guards the assumption the walker rests on: stepping linearly and
    // converting back is exactly the device's byte order.
    for linear in [0u32, 1, 0x7f, 0x80, 0x3fff, 0x4000] {
        let packed = roland_address_from_linear(linear).unwrap();
        assert_eq!(roland_address_to_linear(packed).unwrap(), linear);
        assert_eq!(packed & 0x8080_8080, 0);
    }
}

#[test]
fn a_short_reply_is_split_rather_than_laid_down_contiguously() {
    // Models the measured MC behaviour: a span containing an unmapped hole
    // answers a wide request with only the mapped bytes, concatenated. Laying
    // those down contiguously silently shifts everything past the hole, which
    // is what made sweeps at different chunk sizes disagree on hardware. The
    // sweep must subdivide until each reply covers its request exactly.
    //
    // Map, as linear offsets from 0x20000000: 0..4 mapped, 4..8 unmapped,
    // 8..12 mapped, 12..16 unmapped.
    const BASE: u32 = 0x0400_0000; // linear of packed 0x20000000
    fn mapped(offset: u32) -> bool {
        let i = offset - BASE;
        i < 4 || (8..12).contains(&i)
    }

    struct HoleInTheMiddle;
    impl mc_programmer::Transport for HoleInTheMiddle {
        fn transfer(&self, message: &[u8], _timeout: f64) -> mc_programmer::Result<Vec<u8>> {
            let packed = u32::from_be_bytes([message[8], message[9], message[10], message[11]]);
            let base = mc_programmer::addresses::roland_address_to_linear(packed).unwrap();
            let count = u32::from(message[15]).max(1);

            let offsets: Vec<u32> = (base..base + count).filter(|&o| mapped(o)).collect();
            let Some(&first) = offsets.first() else {
                return Ok(Vec::new());
            };
            let data: Vec<u8> = offsets.iter().map(|&o| 0x10 + (o - BASE) as u8).collect();
            Ok(mc_programmer::build_dt1(ModelId::Mc101.value(), first & 0x7f7f_7f7f, &data).unwrap())
        }
    }

    let client = MCClient::new(HoleInTheMiddle, ModelId::Mc101.value());
    let snapshot = probe::walk(
        &client,
        0x20000000,
        0x20000010,
        &ProbeOptions::gapless(0x10),
    )
    .unwrap();

    assert!(
        snapshot.subdivisions > 0,
        "a short reply must trigger a split"
    );
    for i in 0..16u32 {
        let at = snapshot.bytes.get(&(0x20000000 + i));
        if mapped(BASE + i) {
            assert_eq!(
                at,
                Some(&(0x10 + i as u8)),
                "offset {i} misplaced or missing"
            );
        } else {
            assert_eq!(at, None, "offset {i} is unmapped and must stay absent");
        }
    }
}

#[test]
fn a_full_length_reply_that_ends_past_the_request_is_not_trusted() {
    // Pins the gate's SECOND condition. Remove `reach <= start_linear + count`
    // from probe.rs and this test fails: the device returns exactly `count`
    // bytes but starting two bytes into the span, so the payload ends past the
    // request and the last two bytes belong to addresses outside it.
    // First mapped byte sits two into the span, so a full-length reply starts
    // late and therefore ends past the request.

    let client = MCClient::new(
        FirstMappedAt {
            first_mapped: 0x0400_0002,
        },
        ModelId::Mc101.value(),
    );
    let snapshot = probe::walk(&client, 0x20000000, 0x20000004, &ProbeOptions::gapless(4)).unwrap();

    // Trusting length alone would place 4 bytes at linear +2..+6, i.e. two of
    // them past the request. The gate must refuse, narrow, and end up placing
    // only bytes the device gave an address for.
    assert!(
        snapshot.subdivisions > 0,
        "an over-long reach must force a split"
    );
    assert!(
        !snapshot.bytes.contains_key(&0x20000004),
        "no byte may be placed past the requested range"
    );
    assert!(
        !snapshot.bytes.contains_key(&0x20000005),
        "no byte may be placed past the requested range"
    );
}

#[test]
fn an_empty_dt1_payload_does_not_abort_the_sweep() {
    // `read_dt1` skips the payload-length check `read` applies, so a
    // well-formed DT1 with no payload reaches the walker. It used to panic on
    // `&data[..1]`, destroying a sweep that had taken minutes.
    struct EmptyPayload;
    impl mc_programmer::Transport for EmptyPayload {
        fn transfer(&self, message: &[u8], _timeout: f64) -> mc_programmer::Result<Vec<u8>> {
            let packed = u32::from_be_bytes([message[8], message[9], message[10], message[11]]);
            let base = mc_programmer::addresses::roland_address_to_linear(packed).unwrap();
            Ok(mc_programmer::build_dt1(ModelId::Mc101.value(), base & 0x7f7f_7f7f, &[]).unwrap())
        }
    }

    let client = MCClient::new(EmptyPayload, ModelId::Mc101.value());
    // chunk 1 reaches the one-byte arm directly; subdivision would too.
    let snapshot = probe::walk(&client, 0x20000000, 0x20000004, &ProbeOptions::gapless(1)).unwrap();

    assert!(snapshot.is_empty());
    assert_eq!(
        snapshot.chunks_failed, 4,
        "each empty reply is recorded, not fatal"
    );
    assert!(
        snapshot.failures[0].1.contains("empty payload"),
        "{:?}",
        snapshot.failures[0]
    );
}

#[test]
fn a_reply_beyond_the_request_is_not_subdivided_to_death() {
    // The device reporting a base past the span has already said the span
    // holds nothing. Re-asking in halves costs 2*chunk-1 round trips to learn
    // what one reply said, and inflates every counter by the same factor.
    // Nothing mapped anywhere near the swept range.

    let client = MCClient::new(
        FirstMappedAt {
            first_mapped: 0x0400_1000,
        },
        ModelId::Mc101.value(),
    );
    let snapshot = probe::walk(
        &client,
        0x20000000,
        0x20000100,
        &ProbeOptions::gapless(0x80),
    )
    .unwrap();

    assert_eq!(snapshot.chunks_probed, 1, "one request, not 2*chunk-1");
    assert_eq!(
        snapshot.subdivisions, 0,
        "the span is known empty; splitting it is wasted I/O"
    );
    // The address the device volunteered lies outside the swept range, so the
    // byte is dropped rather than widening the snapshot past what was asked.
    assert!(snapshot.is_empty());
    assert_eq!(snapshot.truncated_placements, 1);
}

#[test]
fn a_collapsed_reply_candidates_are_visible_as_unconfirmed() {
    // The device maps bytes from linear 0x180, while the masked reply also
    // names 0x100. Keep the evidence, but never count the lower candidate as
    // confirmed unless a unique follow-up proves it.
    let client = MCClient::new(
        FirstMappedAt {
            first_mapped: 0x180,
        },
        ModelId::Mc101.value(),
    );

    for chunk in [0x10u32, 0x80] {
        let snapshot = probe::walk(
            &client,
            0x00000200,
            0x00000310,
            &ProbeOptions::gapless(chunk),
        )
        .unwrap();
        assert!(snapshot.bytes.len() >= 0x10, "chunk 0x{chunk:X}");
        assert_eq!(snapshot.bytes.get(&0x00000300), Some(&0x20));
        assert_eq!(snapshot.bytes.get(&0x0000030f), Some(&0x2f));
        assert!(snapshot.unconfirmed_placements > 0, "chunk 0x{chunk:X}");
        assert_eq!(snapshot.confirmations, 0, "chunk 0x{chunk:X}");
    }
}

#[test]
fn a_confirmed_resolution_is_placed_and_counted() {
    // This address's original reply is ambiguous, but the next-byte reply is
    // unique at the top of the candidate set and proves the first byte.
    const BASE: u32 = 0x0080_807f;
    let start = roland_address_from_linear(BASE).unwrap();
    let end = roland_address_from_linear(BASE + 2).unwrap();
    let client = MCClient::new(
        MapsRange {
            first: BASE,
            last_exclusive: BASE + 2,
        },
        ModelId::Mc101.value(),
    );
    let snapshot = probe::walk(&client, start, end, &ProbeOptions::gapless(2)).unwrap();

    assert_eq!(snapshot.bytes.get(&start), Some(&0x30));
    assert_eq!(snapshot.confirmations, 1);
    assert_eq!(snapshot.unconfirmed_placements, 0);
    assert_eq!(snapshot.unprovable_placements, 0);
}

#[test]
fn a_later_mapped_run_does_not_confirm_the_original_payload_address() {
    use std::sync::atomic::{AtomicUsize, Ordering};

    const BASE: u32 = 0x0400_0000;
    struct TwoReplyRuns {
        calls: AtomicUsize,
    }
    impl mc_programmer::Transport for TwoReplyRuns {
        fn transfer(&self, _request: &[u8], _timeout: f64) -> mc_programmer::Result<Vec<u8>> {
            let call = self.calls.fetch_add(1, Ordering::SeqCst);
            let (address, value) = match call {
                0 => (BASE, 0x11),
                1 => (BASE + 0x20, 0x22),
                _ => return Ok(Vec::new()),
            };
            let data = [value; 0x10];
            Ok(mc_programmer::build_dt1(ModelId::Mc101.value(), address & 0x7f7f_7f7f, &data).unwrap())
        }
    }

    let client = MCClient::new(
        TwoReplyRuns {
            calls: AtomicUsize::new(0),
        },
        ModelId::Mc101.value(),
    );
    let snapshot = probe::walk(
        &client,
        roland_address_from_linear(BASE).unwrap(),
        roland_address_from_linear(BASE + 0x40).unwrap(),
        &ProbeOptions::gapless(0x10),
    )
    .unwrap();

    let first = roland_address_from_linear(BASE).unwrap();
    let later = roland_address_from_linear(BASE + 0x20).unwrap();
    assert_eq!(snapshot.bytes.len(), 0x10);
    assert_eq!(snapshot.bytes.get(&first), Some(&0x11));
    assert!(!snapshot.bytes.contains_key(&later));
    assert!(snapshot.unconfirmed_placements > 0);
    assert_eq!(snapshot.confirmations, 0);
}

#[test]
fn an_ambiguous_follow_up_address_does_not_confirm_the_lower_candidate() {
    use std::sync::atomic::{AtomicUsize, Ordering};

    const BASE: u32 = 0x0400_0000;
    struct AmbiguousFollowUp {
        calls: AtomicUsize,
    }
    impl mc_programmer::Transport for AmbiguousFollowUp {
        fn transfer(&self, _request: &[u8], _timeout: f64) -> mc_programmer::Result<Vec<u8>> {
            let call = self.calls.fetch_add(1, Ordering::SeqCst);
            let (address, value) = match call {
                0 => (BASE, 0x11),
                1 => (BASE + 1, 0x22),
                _ => return Ok(Vec::new()),
            };
            let data = [value; 0x10];
            Ok(mc_programmer::build_dt1(ModelId::Mc101.value(), address & 0x7f7f_7f7f, &data).unwrap())
        }
    }

    let client = MCClient::new(
        AmbiguousFollowUp {
            calls: AtomicUsize::new(0),
        },
        ModelId::Mc101.value(),
    );
    let snapshot = probe::walk(
        &client,
        roland_address_from_linear(BASE).unwrap(),
        roland_address_from_linear(BASE + 0x20).unwrap(),
        &ProbeOptions::gapless(0x10),
    )
    .unwrap();

    let first = roland_address_from_linear(BASE).unwrap();
    assert_eq!(snapshot.bytes.get(&first), Some(&0x11));
    assert_eq!(snapshot.confirmations, 0);
    assert_eq!(snapshot.unconfirmed_placements, 1);
}
#[test]
fn a_confirmation_error_is_recorded_and_the_sweep_continues() {
    use std::sync::atomic::{AtomicUsize, Ordering};

    const BASE: u32 = 0x0400_0000;
    struct FailsOnFollowUp {
        calls: AtomicUsize,
    }
    impl mc_programmer::Transport for FailsOnFollowUp {
        fn transfer(&self, _request: &[u8], _timeout: f64) -> mc_programmer::Result<Vec<u8>> {
            match self.calls.fetch_add(1, Ordering::SeqCst) {
                0 => Ok(mc_programmer::build_dt1(ModelId::Mc101.value(), BASE, &[0x11; 0x10]).unwrap()),
                1 => Err(mc_programmer::Error::Midi(
                    "confirmation failed".to_string(),
                )),
                _ => Ok(Vec::new()),
            }
        }
    }

    let client = MCClient::new(
        FailsOnFollowUp {
            calls: AtomicUsize::new(0),
        },
        ModelId::Mc101.value(),
    );
    let snapshot = probe::walk(
        &client,
        roland_address_from_linear(BASE).unwrap(),
        roland_address_from_linear(BASE + 0x20).unwrap(),
        &ProbeOptions::gapless(0x10),
    )
    .unwrap();

    let first = roland_address_from_linear(BASE).unwrap();
    assert_eq!(snapshot.bytes.len(), 0x10);
    assert_eq!(snapshot.bytes.get(&first), Some(&0x11));
    assert_eq!(snapshot.unconfirmed_placements, 1);
    assert!(snapshot
        .failures
        .iter()
        .any(|(_, message)| message.contains("confirmation failed")));
    assert_eq!(snapshot.chunks_probed, 2);
}

#[test]
fn an_empty_reply_is_not_counted_as_answered() {
    struct EmptyPayload;
    impl mc_programmer::Transport for EmptyPayload {
        fn transfer(&self, message: &[u8], _timeout: f64) -> mc_programmer::Result<Vec<u8>> {
            let packed = u32::from_be_bytes([message[8], message[9], message[10], message[11]]);
            let base = mc_programmer::addresses::roland_address_to_linear(packed).unwrap();
            Ok(mc_programmer::build_dt1(ModelId::Mc101.value(), base & 0x7f7f_7f7f, &[]).unwrap())
        }
    }

    let client = MCClient::new(EmptyPayload, ModelId::Mc101.value());
    let snapshot = probe::walk(&client, 0x20000000, 0x20000004, &ProbeOptions::gapless(1)).unwrap();

    // A chunk cannot be both answered and failed: the dump summary would
    // print the same chunks as a success and a warning.
    assert_eq!(snapshot.chunks_failed, 4);
    assert_eq!(
        snapshot.chunks_answered, 0,
        "an unusable reply is not an answer"
    );
    assert!(snapshot.chunks_probed >= snapshot.chunks_answered + snapshot.chunks_failed);
}

/// A device mapping exactly `[first, last_exclusive)` and nothing else.
struct MapsRange {
    first: u32,
    last_exclusive: u32,
}

impl mc_programmer::Transport for MapsRange {
    fn transfer(&self, message: &[u8], _timeout: f64) -> mc_programmer::Result<Vec<u8>> {
        let packed = u32::from_be_bytes([message[8], message[9], message[10], message[11]]);
        let base = mc_programmer::addresses::roland_address_to_linear(packed).unwrap();
        let count = u32::from(message[15]).max(1);
        let start = base.max(self.first);
        if start >= self.last_exclusive {
            return Ok(Vec::new());
        }
        let len = count.min(self.last_exclusive - start);
        let data: Vec<u8> = (0..len)
            .map(|i| 0x30 + (start + i - self.first) as u8)
            .collect();
        Ok(mc_programmer::build_dt1(ModelId::Mc101.value(), start & 0x7f7f_7f7f, &data).unwrap())
    }
}

#[test]
fn the_last_byte_of_a_mapped_region_is_not_dropped() {
    // The byte at a region's top edge has an unmapped successor, so the
    // confirmation follow-up gets silence. Silence is evidence, not ignorance:
    // had the run really started 0x80 further on, that run is mapped and above
    // the follow-up's address, so the device would have answered. Treating it
    // as unknown discarded the byte — and a one-byte field at a region's end
    // is exactly what a scene-block diff needs to see.
    const BASE: u32 = 0x0400_0000; // linear of packed 0x20000000
    let client = MCClient::new(
        MapsRange {
            first: BASE,
            last_exclusive: BASE + 5,
        },
        ModelId::Mc101.value(),
    );

    for chunk in [1u32, 2, 4, 8, 0x10] {
        let snapshot = probe::walk(
            &client,
            0x20000000,
            0x20000010,
            &ProbeOptions::gapless(chunk),
        )
        .unwrap();
        assert_eq!(
            snapshot.bytes.len(),
            5,
            "chunk 0x{chunk:X}: expected all 5 mapped bytes, got {:?}",
            snapshot.bytes
        );
        assert!(
            snapshot.bytes.contains_key(&0x20000004),
            "chunk 0x{chunk:X}: the region's last byte was dropped"
        );
        assert_eq!(snapshot.unprovable_placements, 0, "chunk 0x{chunk:X}");
    }
}

#[test]
fn confirmation_exchanges_are_counted_so_real_traffic_is_visible() {
    // Ambiguous replies still require follow-ups, even when those reads cannot
    // prove an address; account for their real MIDI traffic.
    const BASE: u32 = 0x0400_0000;
    let client = MCClient::new(
        MapsRange {
            first: BASE,
            last_exclusive: BASE + 0x40,
        },
        ModelId::Mc101.value(),
    );

    let snapshot = probe::walk(
        &client,
        0x20000000,
        0x20000040,
        &ProbeOptions::gapless(0x10),
    )
    .unwrap();

    assert_eq!(snapshot.bytes.len(), 0x40);
    assert_eq!(snapshot.confirmations, 0);
    assert!(snapshot.unconfirmed_placements > 0);
    assert!(snapshot.confirmation_exchanges > 0);
}

#[test]
fn a_dropped_follow_up_frame_keeps_mapped_bytes_unconfirmed() {
    // An empty follow-up may be a dropped frame, so it cannot confirm the
    // candidate. The sweep must still retain the mapped bytes from the primary
    // read and mark the placement unconfirmed rather than counting it certain.
    struct DropsFollowUps {
        first: u32,
    }
    impl mc_programmer::Transport for DropsFollowUps {
        fn transfer(&self, message: &[u8], _timeout: f64) -> mc_programmer::Result<Vec<u8>> {
            let packed = u32::from_be_bytes([message[8], message[9], message[10], message[11]]);
            let base = mc_programmer::addresses::roland_address_to_linear(packed).unwrap();
            if base > self.first {
                return Ok(Vec::new());
            }
            let start = base.max(self.first);
            let data: Vec<u8> = (0..4).map(|i| 0x50 + i as u8).collect();
            Ok(mc_programmer::build_dt1(ModelId::Mc101.value(), start & 0x7f7f_7f7f, &data).unwrap())
        }
    }

    let client = MCClient::new(
        DropsFollowUps { first: 0x0400_0080 },
        ModelId::Mc101.value(),
    );
    let snapshot = probe::walk(
        &client,
        0x20000000,
        0x20000110,
        &ProbeOptions::gapless(0x10),
    )
    .unwrap();

    assert_eq!(snapshot.bytes.get(&0x20000100), Some(&0x50));
    assert!(snapshot.unconfirmed_placements > 0);
    assert_eq!(snapshot.confirmations, 0);
}

#[test]
fn a_run_behind_a_masked_address_remains_unconfirmed() {
    // Later replies can share a masked address without proving where the
    // original payload started. Keep candidate data but never mark it certain.
    const BASE: u32 = 0x0400_0000;
    let client = MCClient::new(
        MapsRange {
            first: BASE + 0x81,
            last_exclusive: BASE + 0x90,
        },
        ModelId::Mc101.value(),
    );

    let snapshot = probe::walk(
        &client,
        0x20000000,
        0x20000110,
        &ProbeOptions::gapless(0x10),
    )
    .unwrap();

    assert_eq!(snapshot.confirmations, 0);
    assert!(snapshot.unconfirmed_placements > 0);
    assert_eq!(snapshot.unprovable_placements, 0);
    assert!(
        !snapshot.bytes.is_empty(),
        "unconfirmed bytes remain visible"
    );
    assert!(snapshot.confirmation_exchanges > 0);
}

#[test]
fn relocation_is_counted_once_per_reply() {
    // `relocated_replies` is a per-reply reliability signal, so it must never
    // exceed the number of replies — it was incremented both in the confirm
    // arm and in the arm below it.
    const BASE: u32 = 0x0400_0000;
    let client = MCClient::new(
        MapsRange {
            first: BASE + 0x81,
            last_exclusive: BASE + 0x85,
        },
        ModelId::Mc101.value(),
    );

    let snapshot = probe::walk(
        &client,
        0x20000000,
        0x20000110,
        &ProbeOptions::gapless(0x10),
    )
    .unwrap();

    assert!(
        snapshot.relocated_replies <= snapshot.chunks_answered,
        "{} relocations reported for {} replies",
        snapshot.relocated_replies,
        snapshot.chunks_answered
    );
}
