//! Note-step and motion-step encode / decode for the MC sequencer.
//!
//! Layout (per the MC programming-video transcript)
//! -------------------------------------------------
//!
//! Each step record occupies `0x100` bytes inside its clip's note-sequence
//! area. The first `0x10` bytes are unknown / not decoded here. After that:
//!
//! * offset `0x10` — 16 notes, 2 bytes/note, packed as two 4-bit nibbles
//!   (e.g. C4 = `0x3C` → `0x03 0x0C`). An empty slot is `0x0F 0x0F`.
//! * offset `0x30` — 16 velocities, 1 byte each (0..127)
//! * offset `0x40` — 16 start times, 2 bytes each, signed cents-of-a-step
//!   (range −50..+99). Negative values are two's-complement nibble pairs
//!   (−10 → `0xF6` → bytes `0x0F 0x06`).
//! * offset `0x60` — 16 end times, 1 byte each, 0..100. 100 ties to the next
//!   step; 0..99 ends that many cents into the current step.
//! * offset `0x70` — 1 byte substep: 0 (none) / 1 (×2) / 2 (×3) / 3 (×4) /
//!   4 (flam)
//!
//! Polyphony is represented by up to 16 *components* per step, indexed 0..15.
//! The MC packs concurrent notes into the lowest free lane and rebalances when
//! notes are deleted, so editor code that wants a stable round-trip should call
//! [`NoteStep::compact`] before encoding.

use crate::addresses::{
    END_OFFS, MOTION_LANES_PER_STEP, NOTES_OFFS, NOTES_PER_STEP, START_OFFS, STEP_SIZE, SUBSTEP_OFFS, VELOCITY_OFFS,
};
use crate::error::{Error, Result};

/// Sentinel byte value used for unused note slots.
pub const EMPTY_NOTE: u8 = 0xFF;
/// End-time value that ties the note into the next step.
pub const END_TIME_TIE: u8 = 100;
pub const VELOCITY_DEFAULT: u8 = 100;
/// The device's default note length (80 cents).
pub const END_TIME_DEFAULT: u8 = 0x50;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum SubStep {
    None = 0,
    Double = 1,
    Triple = 2,
    Quadruple = 3,
    Flam = 4,
}

impl SubStep {
    pub fn from_u8(value: u8) -> SubStep {
        match value {
            0 => SubStep::None,
            1 => SubStep::Double,
            2 => SubStep::Triple,
            3 => SubStep::Quadruple,
            4 => SubStep::Flam,
            _ => SubStep::None,
        }
    }
}

/// One concurrent note within a step (a "lane" entry).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NoteComponent {
    /// MIDI note 0..127, or [`EMPTY_NOTE`].
    pub note: u8,
    /// 0..127.
    pub velocity: u8,
    /// Signed; -50..+99 in practice.
    pub start_cents: i8,
    /// 0..99, or 100 to tie into the next step.
    pub end_cents: u8,
    /// Scratch flag used by tie-resolution code.
    pub processed: bool,
}

impl NoteComponent {
    pub const fn empty() -> Self {
        NoteComponent {
            note: EMPTY_NOTE,
            velocity: VELOCITY_DEFAULT,
            start_cents: 0,
            end_cents: END_TIME_DEFAULT,
            processed: false,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.note == EMPTY_NOTE
    }

    pub fn ties_to_next(&self) -> bool {
        self.end_cents == END_TIME_TIE
    }
}

impl Default for NoteComponent {
    fn default() -> Self {
        Self::empty()
    }
}

/// A full step record — 16 components plus the substep flag.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NoteStep {
    pub components: Vec<NoteComponent>,
    pub substep: SubStep,
}

impl NoteStep {
    /// Build a step from exactly [`NOTES_PER_STEP`] components.
    pub fn new(components: Vec<NoteComponent>, substep: SubStep) -> Result<Self> {
        if components.len() != NOTES_PER_STEP {
            return Err(Error::Value(format!(
                "NoteStep needs exactly {NOTES_PER_STEP} components, got {}",
                components.len()
            )));
        }
        Ok(NoteStep {
            components,
            substep,
        })
    }

    pub fn active_components(&self) -> Vec<NoteComponent> {
        self.components
            .iter()
            .copied()
            .filter(|c| !c.is_empty())
            .collect()
    }

    /// Push all live notes down to the lowest lane indices, the way the MC
    /// firmware does internally.
    pub fn compact(&mut self) {
        let mut live: Vec<NoteComponent> = self
            .components
            .iter()
            .copied()
            .filter(|c| !c.is_empty())
            .collect();
        let pad = NOTES_PER_STEP - live.len();
        live.extend(std::iter::repeat(NoteComponent::empty()).take(pad));
        self.components = live;
    }
}

impl Default for NoteStep {
    fn default() -> Self {
        NoteStep {
            components: vec![NoteComponent::empty(); NOTES_PER_STEP],
            substep: SubStep::None,
        }
    }
}

/// Encode an unsigned byte (or signed via two's-complement) as a nibble pair.
pub fn pack_nibble_pair(value: u8) -> (u8, u8) {
    ((value >> 4) & 0x0F, value & 0x0F)
}

/// Decode a nibble pair to an unsigned byte (call [`signed_byte`] for s8).
pub fn unpack_nibble_pair(hi: u8, lo: u8) -> u8 {
    ((hi & 0x0F) << 4) | (lo & 0x0F)
}

/// Interpret an 8-bit value as two's-complement signed (-128..127).
pub fn signed_byte(b: u8) -> i8 {
    b as i8
}

/// Inverse of [`signed_byte`]: re-encode an `i8` as its raw 8-bit pattern.
pub fn to_signed_byte(v: i8) -> u8 {
    v as u8
}

/// Encode a [`NoteStep`] into the `0x100`-byte on-device step record.
///
/// The first `0x10` bytes (unknown header) are left as zeros — matching what
/// writing a freshly-initialised step looks like in the test scripts.
pub fn encode_note_step(step: &NoteStep) -> Vec<u8> {
    let mut buf = vec![0u8; STEP_SIZE as usize];

    for (i, comp) in step.components.iter().enumerate() {
        // notes (nibble-packed) at NOTES_OFFS
        let nb = NOTES_OFFS as usize + i * 2;
        if comp.is_empty() {
            buf[nb] = 0x0F;
            buf[nb + 1] = 0x0F;
        } else {
            let (hi, lo) = pack_nibble_pair(comp.note & 0x7F);
            buf[nb] = hi;
            buf[nb + 1] = lo;
        }

        // velocity (clamped to 0..127)
        buf[VELOCITY_OFFS as usize + i] = comp.velocity & 0x7F;

        // signed start cents — two's-complement nibble pair
        let sb = START_OFFS as usize + i * 2;
        let (shi, slo) = pack_nibble_pair(to_signed_byte(comp.start_cents));
        buf[sb] = shi;
        buf[sb + 1] = slo;

        // end time (clamped to 0..100)
        buf[END_OFFS as usize + i] = comp.end_cents.min(END_TIME_TIE);
    }

    buf[SUBSTEP_OFFS as usize] = step.substep as u8 & 0x7F;
    buf
}

/// Inverse of [`encode_note_step`].
pub fn decode_note_step(raw: &[u8]) -> Result<NoteStep> {
    let needed = SUBSTEP_OFFS as usize + 1;
    if raw.len() < needed {
        return Err(Error::Value(format!(
            "step buffer too short: {}",
            raw.len()
        )));
    }

    let mut components = Vec::with_capacity(NOTES_PER_STEP);
    for i in 0..NOTES_PER_STEP {
        let nb = NOTES_OFFS as usize + i * 2;
        let note_byte = unpack_nibble_pair(raw[nb], raw[nb + 1]);

        let vel = raw[VELOCITY_OFFS as usize + i] & 0x7F;

        let sb = START_OFFS as usize + i * 2;
        let start_byte = unpack_nibble_pair(raw[sb], raw[sb + 1]);
        let start_cents = signed_byte(start_byte);

        let end_cents = raw[END_OFFS as usize + i];

        components.push(NoteComponent {
            note: if note_byte != EMPTY_NOTE {
                note_byte
            } else {
                EMPTY_NOTE
            },
            velocity: vel,
            start_cents,
            end_cents,
            processed: false,
        });
    }

    let substep = SubStep::from_u8(raw[SUBSTEP_OFFS as usize] & 0x7F);
    Ok(NoteStep {
        components,
        substep,
    })
}

// --- Motion lanes -----------------------------------------------------------

/// A motion-step record holds up to 8 lanes (C1..C4 + 4 extras).
///
/// Each lane has 8 substeps for finer-than-step time resolution, represented
/// as a flat `Vec<u8>` per lane; lanes that aren't used are `None`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MotionStep {
    pub lanes: Vec<Option<Vec<u8>>>,
}

impl MotionStep {
    pub fn new(lanes: Vec<Option<Vec<u8>>>) -> Result<Self> {
        if lanes.len() != MOTION_LANES_PER_STEP {
            return Err(Error::Value(format!(
                "MotionStep requires {MOTION_LANES_PER_STEP} lane slots"
            )));
        }
        Ok(MotionStep { lanes })
    }
}

impl Default for MotionStep {
    fn default() -> Self {
        MotionStep {
            lanes: vec![None; MOTION_LANES_PER_STEP],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::notes::note;

    #[test]
    fn round_trip_complex_step() {
        let mut comps = vec![NoteComponent::empty(); NOTES_PER_STEP];
        comps[0] = NoteComponent {
            note: note("C4").unwrap(),
            velocity: 100,
            start_cents: 0,
            end_cents: END_TIME_TIE,
            processed: false,
        };
        comps[1] = NoteComponent {
            note: note("C#1").unwrap(),
            velocity: 7,
            start_cents: -10,
            end_cents: 80,
            processed: false,
        };
        let step = NoteStep::new(comps, SubStep::Triple).unwrap();

        let raw = encode_note_step(&step);
        assert_eq!(raw.len(), STEP_SIZE as usize);

        let decoded = decode_note_step(&raw).unwrap();
        assert_eq!(decoded, step);
    }

    #[test]
    fn empty_slot_round_trips() {
        let step = NoteStep::default();
        let raw = encode_note_step(&step);
        assert_eq!(raw[NOTES_OFFS as usize], 0x0F);
        assert_eq!(raw[NOTES_OFFS as usize + 1], 0x0F);
        assert_eq!(decode_note_step(&raw).unwrap(), step);
    }

    #[test]
    fn signed_start_cents_are_two_complement() {
        let mut step = NoteStep::default();
        step.components[0].note = 60;
        step.components[0].start_cents = -10;
        let raw = encode_note_step(&step);
        // -10 → 0xF6 → nibbles 0x0F 0x06
        assert_eq!(raw[START_OFFS as usize], 0x0F);
        assert_eq!(raw[START_OFFS as usize + 1], 0x06);
        assert_eq!(
            decode_note_step(&raw).unwrap().components[0].start_cents,
            -10
        );
    }
}
