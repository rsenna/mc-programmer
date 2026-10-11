//! Decoding the MC's masked DT1 reply address.
//!
//! Every expectation here is a frame captured from a real MC-101, or the
//! Roland Clan forum report of the same behaviour on one.

use mc_programmer::addresses::{
    reply_address_candidates, resolve_reply_location, roland_address_from_linear, roland_address_to_linear,
};
use mc_programmer::ReplyLocation;

/// Reply bytes seen on the wire for a given packed request, from raw Sys-Ex
/// captures: (requested packed address, reply address field).
///
/// (A capture for "0x40000080" was discarded: that is not a valid packed
/// address — 0x80 has its MSB set — and `build_rq1` masks it, so the device was
/// really asked about 0x40000000.)
const CAPTURES: &[(u32, u32)] = &[
    (0x40000000, 0x08000000),
    (0x40010000, 0x08004000),
    (0x20000000, 0x04000000),
    (0x10000000, 0x02000000),
    (0x00000000, 0x00000000),
    // Linear forms that are not 7-bit-safe, so the reply arrives masked and
    // reports `08 00 00 00` just as 0x40000000 does.
    (0x40000100, 0x08000000),
    (0x40020000, 0x08000000),
    // Single byte that answered on its own.
    (0x4000002e, 0x0800002e),
];

#[test]
fn every_capture_resolves_to_the_requested_address() {
    for &(packed, reply) in CAPTURES {
        let request_linear = roland_address_to_linear(packed).unwrap();
        match resolve_reply_location(reply, request_linear) {
            ReplyLocation::Resolved {
                linear, relocated, ..
            } => {
                assert_eq!(
                    linear, request_linear,
                    "packed 0x{packed:08X}: reply 0x{reply:08X} resolved to 0x{linear:08X}"
                );
                assert!(
                    !relocated,
                    "packed 0x{packed:08X} should resolve to the request"
                );
            }
            other => panic!("packed 0x{packed:08X}: expected a resolution, got {other:?}"),
        }
    }
}

#[test]
fn masking_collapses_distinct_addresses_onto_one_reply() {
    let linears: Vec<u32> = [0x40000000u32, 0x40000100, 0x40020000]
        .iter()
        .map(|&p| roland_address_to_linear(p).unwrap())
        .collect();
    assert_eq!(linears, vec![0x08000000, 0x08000080, 0x08008000]);
    for linear in &linears {
        assert_eq!(
            linear & 0x7f7f_7f7f,
            0x08000000,
            "all three mask to the same reply"
        );
    }
    for &linear in &linears {
        assert!(reply_address_candidates(0x08000000).contains(&linear));
    }
}

#[test]
fn the_forum_report_follows_the_same_rule() {
    // Roland Clan: a request for `32 10 00 00` came back labelled
    // `06 44 00 00`, described as the device encoding the address
    // "incorrectly". It is the linear form of that packed address.
    let request_linear = roland_address_to_linear(0x32100000).unwrap();
    assert_eq!(request_linear, 0x06440000);
    assert!(matches!(
        resolve_reply_location(0x06440000, request_linear),
        ReplyLocation::Resolved {
            linear: 0x06440000,
            ..
        }
    ));
}

#[test]
fn a_relocated_reply_resolves_forward() {
    // Asked at packed 0x40000020 for 16 bytes, the device answered
    // `08 00 00 26`: it skipped unmapped bytes and reported the real start.
    let request_linear = roland_address_to_linear(0x40000020).unwrap();
    match resolve_reply_location(0x0800_0026, request_linear) {
        ReplyLocation::Resolved {
            linear, relocated, ..
        } => {
            assert_eq!(roland_address_from_linear(linear).unwrap(), 0x40000026);
            assert!(relocated);
        }
        other => panic!("expected a relocated resolution, got {other:?}"),
    }
}

#[test]
fn a_reply_relocating_past_the_requested_span_still_resolves() {
    // Measured: asked about packed 0x40000020 for 8 bytes, the device answered
    // for 0x4000002E — beyond the span. The count is a hint, not a bound,
    // which is why it is not a parameter of the resolver at all.
    let request_linear = roland_address_to_linear(0x40000020).unwrap();
    match resolve_reply_location(0x0800_002e, request_linear) {
        ReplyLocation::Resolved {
            linear, relocated, ..
        } => {
            assert_eq!(roland_address_from_linear(linear).unwrap(), 0x4000002e);
            assert!(relocated);
        }
        other => panic!("expected resolution past the span, got {other:?}"),
    }
}

#[test]
fn candidates_cover_every_masked_byte_and_stay_in_the_address_space() {
    let candidates = reply_address_candidates(0x00000000);
    // 8, not 16: a linear address is bounded by 28 bits, so its top byte can
    // never carry bit 7, and those 8 combinations are discarded.
    assert_eq!(candidates.len(), 8, "{candidates:?}");
    assert!(candidates.contains(&0x00000000));
    assert!(candidates.contains(&0x00000080));
    assert!(candidates.contains(&0x00008000));
    assert!(candidates.contains(&0x00808080));
    for candidate in &candidates {
        assert!(
            *candidate <= 0x0fff_ffff,
            "0x{candidate:08X} is outside the space"
        );
    }
}

#[test]
fn a_reply_before_the_request_is_unexplained() {
    let request_linear = roland_address_to_linear(0x40000000).unwrap();
    assert_eq!(
        resolve_reply_location(0x00000001, request_linear),
        ReplyLocation::Unexplained
    );
}

#[test]
fn an_unprovable_resolution_is_reported_as_such() {
    // The case the masking cannot rule out: asked at linear 0x100, a device
    // that truly answered for 0x180 sends exactly the same four bytes as one
    // that answered for 0x100. The nearest candidate is the leading
    // hypothesis, but it must not be presented as certain.
    let masked = 0x180u32 & 0x7f7f_7f7f;
    assert_eq!(
        masked, 0x100,
        "0x180 and 0x100 are indistinguishable once masked"
    );

    match resolve_reply_location(masked, 0x100) {
        ReplyLocation::Resolved {
            linear, provable, ..
        } => {
            assert_eq!(linear, 0x100, "the nearest candidate is the hypothesis");
            assert!(!provable, "but it cannot be proven, and must say so");
        }
        other => panic!("expected an unprovable resolution, got {other:?}"),
    }
}

#[test]
fn a_forced_resolution_is_marked_provable() {
    // When the request is already the highest candidate — every low byte
    // carrying bit 7 — no higher one exists, so the answer is forced and can
    // be trusted outright.
    let linear = 0x0fff_ffffu32;
    match resolve_reply_location(linear & 0x7f7f_7f7f, linear) {
        ReplyLocation::Resolved { provable, .. } => assert!(provable),
        other => panic!("expected a provable resolution, got {other:?}"),
    }
}
