//! Decoding the MC's masked DT1 reply address.
//!
//! Every expectation here is a frame captured from a real MC-101, or the
//! Roland Clan forum report of the same behaviour on one.

use mc_programmer::addresses::{
    reply_address_candidates, resolve_reply_location, roland_address_from_linear, roland_address_to_linear,
};
use mc_programmer::ReplyLocation;

/// Reply bytes seen on the wire for a given packed request, from raw Sys-Ex
/// captures: (requested packed address, count, reply address field).
const CAPTURES: &[(u32, u32, u32)] = &[
    (0x40000000, 10, 0x08000000),
    (0x40010000, 4, 0x08004000),
    (0x20000000, 4, 0x04000000),
    (0x10000000, 4, 0x02000000),
    (0x00000000, 4, 0x00000000),
    // Linear form is not 7-bit-safe, so the reply arrives masked and these
    // report `08 00 00 00` just like 0x40000000 does.
    //
    // (A capture for "0x40000080" was discarded: that is not a valid packed
    // address — 0x80 has its MSB set — and `build_rq1` masks it, so the device
    // was really asked about 0x40000000.)
    (0x40000100, 16, 0x08000000),
    (0x40020000, 16, 0x08000000),
    // Single byte that answered on its own.
    (0x4000002e, 1, 0x0800002e),
];

#[test]
fn every_capture_resolves_to_the_requested_address() {
    for &(packed, count, reply) in CAPTURES {
        let request_linear = roland_address_to_linear(packed).unwrap();
        match resolve_reply_location(reply, request_linear, count) {
            ReplyLocation::Resolved { linear, relocated } => {
                assert_eq!(
                    linear, request_linear,
                    "packed 0x{packed:08X}: reply 0x{reply:08X} resolved to linear 0x{linear:08X}, \
                     expected 0x{request_linear:08X}"
                );
                assert!(
                    !relocated,
                    "packed 0x{packed:08X} should resolve to the requested address"
                );
            }
            other => panic!("packed 0x{packed:08X}: expected a resolved address, got {other:?}"),
        }
    }
}

#[test]
fn masking_collapses_distinct_addresses_onto_one_reply() {
    // The three captures that share reply `08 00 00 00` have different linear
    // addresses; only the request distinguishes them.
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
    // And the decoder recovers each one from that single reply value.
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
        resolve_reply_location(0x06440000, request_linear, 16),
        ReplyLocation::Resolved {
            linear: 0x06440000,
            relocated: false
        }
    ));
}

#[test]
fn a_relocated_reply_resolves_forward_within_the_request() {
    // Asked at packed 0x40000020 for 16 bytes, the device answered
    // `08 00 00 26`: it skipped unmapped bytes and reported the real start.
    let request_linear = roland_address_to_linear(0x40000020).unwrap();
    match resolve_reply_location(0x0800_0026, request_linear, 16) {
        ReplyLocation::Resolved { linear, relocated } => {
            assert_eq!(roland_address_from_linear(linear).unwrap(), 0x40000026);
            assert!(relocated);
        }
        other => panic!("expected a resolved, relocated address, got {other:?}"),
    }
}

#[test]
fn candidates_cover_every_masked_byte_and_stay_in_the_address_space() {
    let candidates = reply_address_candidates(0x00000000);
    assert!(candidates.contains(&0x00000000));
    assert!(candidates.contains(&0x00000080));
    assert!(candidates.contains(&0x00008000));
    assert!(candidates.contains(&0x00808080));
    for candidate in &candidates {
        assert!(
            *candidate <= 0x0fff_ffff,
            "0x{candidate:08X} is outside the 28-bit space"
        );
    }
}

#[test]
fn a_reply_outside_the_request_is_unexplained() {
    let request_linear = roland_address_to_linear(0x40000000).unwrap();
    assert_eq!(
        resolve_reply_location(0x00000001, request_linear, 16),
        ReplyLocation::Unexplained
    );
}

#[test]
fn the_nearest_candidate_at_or_after_the_request_is_chosen() {
    // The MC returns the NEXT mapped bytes at or after the requested address,
    // so when masking leaves several candidates the nearest forward one is the
    // device's meaning. Here 0x08000000 and 0x08000080 both fit a wide span.
    assert!(matches!(
        resolve_reply_location(0x08000000, 0x08000000, 0x200),
        ReplyLocation::Resolved {
            linear: 0x08000000,
            relocated: false
        }
    ));
    // Asked from just past the first candidate, the second is chosen.
    assert!(matches!(
        resolve_reply_location(0x08000000, 0x08000001, 0x200),
        ReplyLocation::Resolved {
            linear: 0x08000080,
            relocated: true
        }
    ));
}

#[test]
fn a_reply_relocating_past_the_requested_span_still_resolves() {
    // Measured: asked about packed 0x40000020 for 8 bytes, the device answered
    // for 0x4000002E — beyond the span. The count is a hint, not a bound.
    let request_linear = roland_address_to_linear(0x40000020).unwrap();
    match resolve_reply_location(0x0800_002e, request_linear, 8) {
        ReplyLocation::Resolved { linear, relocated } => {
            assert_eq!(roland_address_from_linear(linear).unwrap(), 0x4000002e);
            assert!(relocated);
        }
        other => panic!("expected resolution past the span, got {other:?}"),
    }
}
