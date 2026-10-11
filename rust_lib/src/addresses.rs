//! MC-101 / MC-707 / MV-1 address-map constants and address arithmetic.
//!
//! All values come straight from the reference scripts (`trk_clip_tools.py`,
//! `zen_core_tools.py`) and the two video transcripts in the parent repo.
//!
//! Address-space rules:
//!
//! * Addresses are 28 useful bits laid out across four bytes; **no byte may
//!   have its MSB set** (the `0x80808080` mask in the dump routines) because
//!   the bytes have to fit through the 7-bit MIDI pipe. The
//!   `next_valid_base_address` helper jumps over any candidate that would
//!   violate that rule.
//! * The sequencer area starts at `0x20000000`. Each track is offset by
//!   `0x01080000` from the previous one, and each clip within a track is
//!   offset by `0x00080000` from the previous (subject to the 0x80 rule).

use crate::error::{Error, Result};

const ADDRESS_MSB_MASK: u32 = 0x8080_8080;
const MAX_LINEAR_ADDRESS: u32 = 0x0fff_ffff;

/// Convert a four-byte Roland 7-bit address into a linear byte index.
pub fn roland_address_to_linear(address: u32) -> Result<u32> {
    if address & ADDRESS_MSB_MASK != 0 {
        return Err(Error::Value(format!(
            "invalid Roland address 0x{address:08X}: every byte must be 7-bit"
        )));
    }
    Ok(((address >> 24) & 0x7f) << 21
        | ((address >> 16) & 0x7f) << 14
        | ((address >> 8) & 0x7f) << 7
        | (address & 0x7f))
}

/// Convert a linear byte index into a four-byte Roland 7-bit address.
pub fn roland_address_from_linear(linear: u32) -> Result<u32> {
    if linear > MAX_LINEAR_ADDRESS {
        return Err(Error::Value(format!(
            "linear Roland address 0x{linear:X} exceeds the 28-bit address space"
        )));
    }
    Ok(((linear >> 21) & 0x7f) << 24 | ((linear >> 14) & 0x7f) << 16 | ((linear >> 7) & 0x7f) << 8 | (linear & 0x7f))
}

/// Add a linear byte count to a Roland 7-bit address.
pub fn add_roland_offset(address: u32, byte_offset: u32) -> Result<u32> {
    let linear = roland_address_to_linear(address)?
        .checked_add(byte_offset)
        .ok_or_else(|| Error::Value("Roland address addition overflowed".to_string()))?;
    roland_address_from_linear(linear)
}

/// Candidate linear addresses a DT1 reply could be reporting.
///
/// The MC reports the returned data's **linear** address, but pushes it
/// through the 7-bit MIDI pipe with every byte masked to `0x7F` — so up to 16
/// distinct linear addresses arrive as the same four bytes. (Verified on an
/// MC-101: requests for packed `0x40000080`, `0x40000100` and `0x40020000`,
/// whose linear forms are `0x08000000`, `0x08000080` and `0x08008000`, all
/// reply `08 00 00 00`. A Roland Clan forum report of request `32 10 00 00`
/// answering `06 44 00 00` is the same rule — `0x06440000` is that address's
/// linear form.)
///
/// Each returned byte is therefore either itself or itself plus `0x80`, which
/// this expands into every combination that fits the 28-bit space.
pub fn reply_address_candidates(reply_address: u32) -> Vec<u32> {
    let bytes = [
        (reply_address >> 24) & 0xff,
        (reply_address >> 16) & 0xff,
        (reply_address >> 8) & 0xff,
        reply_address & 0xff,
    ];

    let mut candidates = Vec::with_capacity(16);
    for combination in 0u32..16 {
        let mut value = 0u32;
        for (index, byte) in bytes.iter().enumerate() {
            let high = (combination >> (3 - index)) & 1;
            value = (value << 8) | (byte | (high << 7));
        }
        if value <= MAX_LINEAR_ADDRESS {
            candidates.push(value);
        }
    }
    candidates.sort_unstable();
    candidates.dedup();
    candidates
}

/// Where a DT1 reply's data actually lives, resolved against the request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReplyLocation {
    /// Exactly one candidate falls inside the requested span. `linear` is the
    /// data's linear start address; `relocated` is true when the device moved
    /// forward from the address asked for, which it does to skip unmapped
    /// bytes.
    Resolved { linear: u32, relocated: bool },
    /// Several candidates fit and no rule picks between them. Not currently
    /// produced — [`resolve_reply_location`] takes the nearest candidate at or
    /// after the request — but kept so a stricter caller can distinguish
    /// "resolved by a rule" from "resolved uniquely".
    Ambiguous(Vec<u32>),
    /// No candidate falls inside the requested span. Either the reply belongs
    /// to a different request, or the device did something unmodelled.
    Unexplained,
}

/// Resolve a DT1 reply's masked address against the request that provoked it.
///
/// An RQ1 behaves like "return the next mapped run at or after this address",
/// with the count as a hint rather than a bound: an MC-101 asked about packed
/// `0x40000020` for 16 bytes answered for `0x40000026`, and asked for 8 bytes
/// from `0x40000020` answered for `0x4000002E` — past the end of the span.
/// So a candidate is accepted when it is at or after the requested address,
/// and the nearest one forward wins, since the device returns the *next*
/// mapped bytes rather than an arbitrary later run.
///
/// Anything with no candidate at or after the request is reported rather than
/// guessed: placing a payload at the wrong address produces an address map
/// that looks plausible and is wrong.
pub fn resolve_reply_location(reply_address: u32, request_linear: u32, _count: u32) -> ReplyLocation {
    match reply_address_candidates(reply_address)
        .into_iter()
        .filter(|&candidate| candidate >= request_linear)
        .min()
    {
        Some(linear) => ReplyLocation::Resolved {
            linear,
            relocated: linear != request_linear,
        },
        None => ReplyLocation::Unexplained,
    }
}

/// Add one packed Roland address value to another.
pub fn add_roland_address(address: u32, packed_offset: u32) -> Result<u32> {
    add_roland_offset(address, roland_address_to_linear(packed_offset)?)
}

// --- Sequencer constants -----------------------------------------------------

pub const SEQ_BASE_ADDRESS: u32 = 0x20000000;
pub const SEQ_PARAMS_OFFS: u32 = 0x0002_0000;
pub const SEQ_NOTE_OFFS: u32 = 0x0003_0000;
pub const SEQ_MOTION_OFFS: u32 = 0x0004_0000;
pub const SEQ_PITCHBEND_OFFS: u32 = 0x0005_0000;

pub const STEP_SIZE: u32 = 0x100; // bytes per note step or motion step
pub const SEQ_TRK_OFFSET: u32 = 0x0108_0000;
pub const SEQ_CLIP_OFFSET: u32 = 0x0008_0000;
pub const NUM_TRACKS: usize = 8;
pub const CLIPS_PER_TRACK: usize = 16;
pub const STEPS_PER_CLIP: usize = 128;

/// Clip 0 of each track lives at one of these bases (per the test scripts).
pub const CLIP1_BASE_ADDR: [u32; NUM_TRACKS] = [
    0x20000000, 0x21080000, 0x22100000, 0x23180000, 0x24200000, 0x25280000, 0x26300000, 0x27380000,
];

/// Track sound (not clip sound) base addresses, per `trk_clip_tools`.
pub const TRK_BASE_ADDR: [u32; NUM_TRACKS] = [
    0x21000000, 0x22080000, 0x23100000, 0x24180000, 0x25200000, 0x26280000, 0x27300000, 0x28380000,
];

/// Per-track settings block (color, sound source, etc.).
pub const TRK_SETTINGS_BASE_ADDR: [u32; NUM_TRACKS] = [
    0x10000800, 0x10000a00, 0x10000c00, 0x10000e00, 0x10001000, 0x10001200, 0x10001400, 0x10001600,
];
pub const SND_SRC_CFG_OFFS: u32 = 0x04; // 0 -> track is sound source, 1 -> clip is
pub const TRK_COLOR_OFFS: u32 = 0x19;
pub const TRK_TYPE_OFFS: u32 = 0x14; // 0=TONE, 1=DRUM, 0x7F=OFF

// --- ZEN-Core descriptor constants ------------------------------------------

/// Each tone-track block has 17 ZEN-Core descriptors — 16 clip tones + 1
/// track tone — aligned every `0x20000` bytes.
pub const ZENCORE_TONE_ALIGNMENT: u32 = 0x0002_0000;
pub const ZENCORE_TRACK_BASE: [u32; NUM_TRACKS] = [
    0x30000000, 0x30220000, 0x30440000, 0x30660000, 0x31080000, 0x312a0000, 0x314c0000, 0x316e0000,
];

/// Selected ZEN-Core tone offsets demonstrated in the ZC programming video.
pub const TONE_COMMON_COARSE_TUNE_OFFS: u32 = 0x0018; // 1 byte, valid 16..112

// --- Note-step parameter offsets (within one 0x100-byte step record) --------

pub const NOTES_OFFS: u32 = 0x10; // 16 notes × 2 bytes (4-bit nibble pairs)
pub const VELOCITY_OFFS: u32 = 0x30; // 16 velocities × 1 byte
pub const START_OFFS: u32 = 0x40; // 16 start times × 2 bytes (signed cents)
pub const END_OFFS: u32 = 0x60; // 16 end times × 1 byte (0..100; 100 ties)
pub const SUBSTEP_OFFS: u32 = 0x70; // 1 byte
pub const NOTES_PER_STEP: usize = 16;

// --- Motion lane constants --------------------------------------------------

pub const MOTION_LANE_NAMES: [&str; 8] = [
    "C1/FILTER",
    "C2/MOD",
    "C3/FX",
    "C4/SOUND",
    "Ext 0x40",
    "Ext 0x50",
    "Ext 0x60",
    "Ext 0x70",
];
pub const MOTION_LANE_OFFSET: u32 = 0x10; // bytes between lanes inside a motion step
pub const MOTION_LANES_PER_STEP: usize = 8;
pub const MOTION_SUBSTEPS: usize = 8;

/// Step-time lookup table — index 0 is the slowest (eighth notes).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum StepScale {
    Eighth = 0,
    Sixteenth = 1,
    ThirtySecond = 2,
    QuarterTriplet = 3,
    EighthTriplet = 4,
    SixteenthTriplet = 5,
}

impl StepScale {
    /// Return the musical duration label, using `T` for triplet scales.
    pub fn label(self) -> &'static str {
        match self {
            StepScale::Eighth => "1/8",
            StepScale::Sixteenth => "1/16",
            StepScale::ThirtySecond => "1/32",
            StepScale::QuarterTriplet => "1/4T",
            StepScale::EighthTriplet => "1/8T",
            StepScale::SixteenthTriplet => "1/16T",
        }
    }

    /// Decode a raw scale byte; unknown values fall back to sixteenth.
    pub fn from_u8(value: u8) -> StepScale {
        match value {
            0 => StepScale::Eighth,
            1 => StepScale::Sixteenth,
            2 => StepScale::ThirtySecond,
            3 => StepScale::QuarterTriplet,
            4 => StepScale::EighthTriplet,
            5 => StepScale::SixteenthTriplet,
            _ => StepScale::Sixteenth,
        }
    }
}

/// Add `offset` and skip any address that would have a high MSB byte.
fn next_valid_base_address(previous: u32, offset: u32) -> u32 {
    let mut next = previous + offset;
    while next & 0x80808080 != 0 {
        next += offset;
    }
    next
}

/// Computes addresses for tracks, clips and steps in the MC address map.
///
/// Construct once (`AddressMap::new`) and reuse — the per-track lists of clip
/// base addresses are cached at construction time so per-step lookups are pure
/// arithmetic.
#[derive(Clone, Debug)]
pub struct AddressMap {
    clip_base_addresses: [[u32; CLIPS_PER_TRACK]; NUM_TRACKS],
}

impl Default for AddressMap {
    /// Build the standard MC address map with cached clip bases.
    fn default() -> Self {
        Self::new()
    }
}

impl AddressMap {
    /// Precompute all eight tracks and sixteen clip bases, skipping invalid MIDI addresses.
    pub fn new() -> Self {
        let mut rows = [[0u32; CLIPS_PER_TRACK]; NUM_TRACKS];
        for trk in 0..NUM_TRACKS {
            let mut row = [0u32; CLIPS_PER_TRACK];
            row[0] = CLIP1_BASE_ADDR[trk];
            for clip in 1..CLIPS_PER_TRACK {
                row[clip] = next_valid_base_address(row[clip - 1], SEQ_CLIP_OFFSET);
            }
            rows[trk] = row;
        }
        AddressMap {
            clip_base_addresses: rows,
        }
    }

    // ------- track / clip ---------------------------------------------------

    /// Return the track sound base for a track index in `0..8`; reject other indices.
    pub fn track_base(&self, trk_idx: usize) -> Result<u32> {
        Ok(TRK_BASE_ADDR[Self::t(trk_idx)?])
    }

    /// Return the settings block address for a track index in `0..8`.
    pub fn track_settings_base(&self, trk_idx: usize) -> Result<u32> {
        Ok(TRK_SETTINGS_BASE_ADDR[Self::t(trk_idx)?])
    }

    /// Return the clip base after checking track `0..8` and clip `0..16` indices.
    pub fn clip_base(&self, trk_idx: usize, clip_idx: usize) -> Result<u32> {
        let t = Self::t(trk_idx)?;
        let c = Self::c(clip_idx)?;
        Ok(self.clip_base_addresses[t][c])
    }

    // ------- sequencer step addresses --------------------------------------

    /// Locate clip timing parameters using zero-based track and clip indices.
    pub fn clip_params_address(&self, trk_idx: usize, clip_idx: usize) -> Result<u32> {
        add_roland_address(self.clip_base(trk_idx, clip_idx)?, SEQ_PARAMS_OFFS)
    }

    /// Locate a note record using zero-based track, clip, and step indices.
    /// Reject indices outside eight tracks, sixteen clips, or 128 steps.
    pub fn note_step_address(&self, trk_idx: usize, clip_idx: usize, step_idx: usize) -> Result<u32> {
        let s = Self::s(step_idx)? as u32;
        let notes = add_roland_address(self.clip_base(trk_idx, clip_idx)?, SEQ_NOTE_OFFS)?;
        add_roland_offset(notes, s * STEP_SIZE)
    }

    /// Locate a motion lane using zero-based track, clip, step, and lane indices.
    /// Return an error if any index is out of range; each step has eight lanes.
    pub fn motion_step_address(
        &self,
        trk_idx: usize,
        clip_idx: usize,
        step_idx: usize,
        lane_idx: usize,
    ) -> Result<u32> {
        if lane_idx >= MOTION_LANES_PER_STEP {
            return Err(Error::Index(format!(
                "motion lane out of range: {lane_idx}"
            )));
        }
        let s = Self::s(step_idx)? as u32;
        let motion = add_roland_address(self.clip_base(trk_idx, clip_idx)?, SEQ_MOTION_OFFS)?;
        let step_bytes = s * STEP_SIZE;
        let lane_bytes = lane_idx as u32 * roland_address_to_linear(MOTION_LANE_OFFSET)?;
        add_roland_offset(motion, step_bytes + lane_bytes)
    }

    // ------- ZEN-Core tone descriptors --------------------------------------

    /// Address of a ZEN-Core tone descriptor.
    ///
    /// Pass `clip_idx = None` for the *track* tone (loaded when the track's
    /// sound source is set to TRACK), or `Some(0..15)` for one of the clip
    /// tones.
    pub fn zencore_tone_address(&self, trk_idx: usize, clip_idx: Option<usize>) -> Result<u32> {
        let t = Self::t(trk_idx)?;
        let slot = match clip_idx {
            Some(clip) => Self::c(clip)?,
            None => CLIPS_PER_TRACK,
        } as u32;
        let slot_size = roland_address_to_linear(ZENCORE_TONE_ALIGNMENT)?;
        add_roland_offset(ZENCORE_TRACK_BASE[t], slot * slot_size)
    }

    // ------- bounds helpers -------------------------------------------------

    /// Validate and return a zero-based track index in `0..8`.
    fn t(i: usize) -> Result<usize> {
        if i < NUM_TRACKS {
            Ok(i)
        } else {
            Err(Error::Index(format!("track index out of range: {i}")))
        }
    }

    /// Validate and return a zero-based clip index in `0..16`.
    fn c(i: usize) -> Result<usize> {
        if i < CLIPS_PER_TRACK {
            Ok(i)
        } else {
            Err(Error::Index(format!("clip index out of range: {i}")))
        }
    }

    /// Validate and return a zero-based step index in `0..128`.
    fn s(i: usize) -> Result<usize> {
        if i < STEPS_PER_CLIP {
            Ok(i)
        } else {
            Err(Error::Index(format!("step index out of range: {i}")))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verify every cached clip base fits in four 7-bit address bytes.
    #[test]
    fn no_clip_address_has_high_msb() {
        let map = AddressMap::new();
        for trk in 0..NUM_TRACKS {
            for clip in 0..CLIPS_PER_TRACK {
                let base = map.clip_base(trk, clip).unwrap();
                assert_eq!(base & 0x80808080, 0, "0x{base:08X}");
            }
        }
    }

    /// Verify the first clip tone and track tone occupy the expected descriptor slots.
    #[test]
    fn zencore_tone_address() {
        let map = AddressMap::new();
        assert_eq!(map.zencore_tone_address(0, Some(0)).unwrap(), 0x30000000);
        assert_eq!(
            map.zencore_tone_address(0, None).unwrap(),
            0x30000000 + 16 * ZENCORE_TONE_ALIGNMENT
        );
    }

    /// Verify step addresses advance by linear record sizes across packed address boundaries.
    #[test]
    fn note_step_address_arithmetic() {
        let map = AddressMap::new();
        let clip0 = map.clip_base(0, 0).unwrap();
        assert_eq!(
            map.note_step_address(0, 0, 0).unwrap(),
            clip0 + SEQ_NOTE_OFFS
        );
        assert_eq!(
            map.note_step_address(0, 0, 4).unwrap(),
            add_roland_offset(map.note_step_address(0, 0, 0).unwrap(), 4 * STEP_SIZE).unwrap()
        );
    }

    /// Verify out-of-range track, clip, and step indices return errors.
    #[test]
    fn bounds_checked() {
        let map = AddressMap::new();
        assert!(map.clip_base(8, 0).is_err());
        assert!(map.clip_base(0, 16).is_err());
        assert!(map.note_step_address(0, 0, 128).is_err());
    }
}
