//! Conventional ZEN-Core Tone parameter framework for the MC family.
//!
//! The MC family stores its synth patches as **conventional ZEN-Core Tone**
//! descriptors — the PCM/sample-based half of the `[ZCore]` address block in
//! Roland's Jupiter-X/Xm MIDI Implementation. Per the ZC programming-video
//! transcript, the parameter offsets within a ZCore tone are the same across
//! every Roland ZEN-Core instrument; only the *base address* of each tone
//! descriptor differs from one product to the next, and that lives in
//! [`crate::addresses::AddressMap`].
//!
//! Substructure layout within a single tone descriptor:
//!
//! | Substructure        | Offset  | Description                              |
//! |---------------------|---------|------------------------------------------|
//! | Tone Common         | `0x0000`| Name, Level, Coarse/Fine Tune, Mono/Poly |
//! | Tone MFX            | `0x0100`| Multi-effect (per-tone)                  |
//! | Tone PMT            | `0x1000`| Partial Mix Table                        |
//! | Tone Partial 1..4   | `0x2000`+ `n*0x100` | OSC, filter, amp, modulation     |
//! | Partial Pitch Env   | `0x2400`+ `n*0x100` | 4 × pitch envelope              |
//! | Partial Filter Env  | `0x2800`+ `n*0x100` | 4 × filter envelope             |
//! | Partial Amp Env     | `0x2C00`+ `n*0x100` | 4 × amp envelope                |
//! | Partial LFO         | `0x3000`+ `n*0x200` | 4 × { LFO 1 + LFO 2 }          |
//! | Partial EQ          | `0x3800`+ `n*0x100` | 4 × 3-band EQ                  |
//! | Tone Synth Common   | `0x3C00`| SuperNATURAL synth half                 |
//! | Tone Synth PMT      | `0x3D00`| Synth partial mix / structure             |
//! | Tone Synth Partial  | `0x3E00`+ `n*0x100` | Synth OSC per partial          |
//!
//! The actual parameter table (~500 entries) is auto-generated from Roland's
//! Jupiter-X PDF — see `src/zencore_data.rs` and
//! `tools/gen_rust_zencore.py`.

use crate::error::{Error, Result};
use crate::zencore_data;

// ---------------------------------------------------------------------------
// Substructure offsets within a single conventional ZEN-Core Tone descriptor
// ---------------------------------------------------------------------------

/// Tone Common — verified at offset 0 by the parent repo's test scripts.
pub const TONE_COMMON_OFFS: u32 = 0x0000;
/// Tone MFX (multi-effect). From Jupiter-X `[ZCore]` map.
pub const TONE_MFX_OFFS: u32 = 0x0100;
/// Tone PMT (Partial Mix Table). From Jupiter-X `[ZCore]` map.
pub const TONE_PMT_OFFS: u32 = 0x1000;

pub const PARTIAL_BASE_OFFS: u32 = 0x2000;
pub const PARTIAL_STRIDE: u32 = 0x0100;

pub const PARTIAL_PITCH_ENV_BASE: u32 = 0x2400;
pub const PARTIAL_PITCH_ENV_STRIDE: u32 = 0x0100;

pub const PARTIAL_FILTER_ENV_BASE: u32 = 0x2800;
pub const PARTIAL_FILTER_ENV_STRIDE: u32 = 0x0100;

pub const PARTIAL_AMP_ENV_BASE: u32 = 0x2C00;
pub const PARTIAL_AMP_ENV_STRIDE: u32 = 0x0100;

/// Partial LFO has a *bigger* stride because the LFO struct is 0x11E bytes
/// (carries both LFO 1 and LFO 2 in one block).
pub const PARTIAL_LFO_BASE: u32 = 0x3000;
pub const PARTIAL_LFO_STRIDE: u32 = 0x0200;

pub const PARTIAL_EQ_BASE: u32 = 0x3800;
pub const PARTIAL_EQ_STRIDE: u32 = 0x0100;

/// SuperNATURAL synth half of the ZCore tone descriptor — the MC's OSC type /
/// structure / cross-mod / unison parameters live here.
pub const TONE_SYNTH_COMMON_OFFS: u32 = 0x3C00;
pub const TONE_SYNTH_PMT_OFFS: u32 = 0x3D00;
pub const TONE_SYNTH_PARTIAL_BASE: u32 = 0x3E00;
pub const TONE_SYNTH_PARTIAL_STRIDE: u32 = 0x0100;

/// Number of partials in a conventional ZEN-Core Tone (Roland standard).
pub const NUM_PARTIALS: usize = 4;

/// Offset of partial *N* (1..4) within a tone descriptor.
///
/// By default returns the offset of `Tone Partial N` itself; pass an explicit
/// `base` and `stride` to compute the address of one of the per-partial
/// sub-substructures (Pitch Env, Filter Env, Amp Env, LFO, EQ).
pub fn partial_offset(partial_idx: usize, base: u32, stride: u32) -> Result<u32> {
    if !(1..=NUM_PARTIALS).contains(&partial_idx) {
        return Err(Error::Index(format!(
            "partial index out of range: {partial_idx}"
        )));
    }
    Ok(base + ((partial_idx - 1) as u32) * stride)
}

// ---------------------------------------------------------------------------
// ZCValue / ZCParam — descriptor + codec for one parameter
// ---------------------------------------------------------------------------

/// A decoded/encoded ZEN-Core parameter value: an integer or an ASCII string.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ZCValue {
    Int(i32),
    Str(String),
}

impl ZCValue {
    pub fn as_int(&self) -> Option<i32> {
        match self {
            ZCValue::Int(v) => Some(*v),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            ZCValue::Str(s) => Some(s.as_str()),
            _ => None,
        }
    }
}

impl From<i32> for ZCValue {
    fn from(v: i32) -> Self {
        ZCValue::Int(v)
    }
}

impl From<&str> for ZCValue {
    fn from(s: &str) -> Self {
        ZCValue::Str(s.to_string())
    }
}

impl From<String> for ZCValue {
    fn from(s: String) -> Self {
        ZCValue::Str(s)
    }
}

/// Description of a single ZEN-Core parameter.
///
/// `size` is the number of *wire* bytes in RQ1/DT1 messages, not the
/// parameter's logical width: `1` is a single 7-bit byte, `2` is an 8-bit
/// value packed across two nibble bytes, `4` is a 16-bit value packed across
/// four nibble bytes, and an N-byte ASCII field (Tone Name) uses N character
/// bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ZCParam {
    /// Machine name (snake_case).
    pub name: &'static str,
    /// Offset within the containing substructure.
    pub offset: u32,
    /// Bytes on the wire.
    pub size: u8,
    pub min_value: i32,
    pub max_value: i32,
    pub default: i32,
    /// Human-readable name from the Roland doc.
    pub label: &'static str,
    /// True for variable-length ASCII fields.
    pub ascii: bool,
    /// True for signed two's-complement payloads.
    pub signed: bool,
    /// Original Roland row text, kept for display.
    pub notes: &'static str,
}

impl ZCParam {
    pub const fn new(
        name: &'static str,
        offset: u32,
        size: u8,
        min_value: i32,
        max_value: i32,
        label: &'static str,
        notes: &'static str,
    ) -> Self {
        // Heuristic shared with the Python / Swift ports: any 8+ byte field
        // whose Roland range is 32..127 is an ASCII name string.
        let ascii = size >= 8 && min_value == 32 && max_value == 127;
        ZCParam {
            name,
            offset,
            size,
            min_value,
            max_value,
            default: 0,
            label,
            ascii,
            signed: false,
            notes,
        }
    }

    /// Encode `value` into the byte list that goes inside a DT1 packet.
    pub fn encode(&self, value: &ZCValue) -> Result<Vec<u8>> {
        if self.ascii {
            let s = value
                .as_str()
                .ok_or_else(|| Error::Value(format!("{}: expected string", self.name)))?;
            return Ok(self.encode_ascii(s));
        }

        let v = value
            .as_int()
            .ok_or_else(|| Error::Value(format!("{}: expected int", self.name)))?;

        if !(self.min_value..=self.max_value).contains(&v) {
            return Err(Error::Value(format!(
                "{}: value {v} out of range ({}..{})",
                self.name, self.min_value, self.max_value
            )));
        }

        Ok(match self.size {
            1 => vec![(v & 0x7F) as u8],
            2 => vec![((v >> 4) & 0x0F) as u8, (v & 0x0F) as u8],
            4 => vec![
                ((v >> 12) & 0x0F) as u8,
                ((v >> 8) & 0x0F) as u8,
                ((v >> 4) & 0x0F) as u8,
                (v & 0x0F) as u8,
            ],
            _ => {
                return Err(Error::Value(format!(
                    "{}: unsupported wire size {}",
                    self.name, self.size
                )))
            }
        })
    }

    /// Decode the data bytes from a DT1 reply back into a value.
    pub fn decode(&self, raw: &[u8]) -> Result<ZCValue> {
        if raw.len() < self.size as usize {
            return Err(Error::Value(format!(
                "{}: expected {} bytes, received {}",
                self.name,
                self.size,
                raw.len()
            )));
        }
        if self.ascii {
            return Ok(ZCValue::Str(self.decode_ascii(raw)));
        }

        let v = match self.size {
            1 => (raw[0] & 0x7F) as i32,
            2 => (((raw[0] & 0x0F) as i32) << 4) | (raw[1] & 0x0F) as i32,
            4 => {
                (((raw[0] & 0x0F) as i32) << 12)
                    | (((raw[1] & 0x0F) as i32) << 8)
                    | (((raw[2] & 0x0F) as i32) << 4)
                    | (raw[3] & 0x0F) as i32
            }
            _ => {
                return Err(Error::Value(format!(
                    "{}: unsupported wire size {}",
                    self.name, self.size
                )))
            }
        };
        Ok(ZCValue::Int(v))
    }

    fn encode_ascii(&self, value: &str) -> Vec<u8> {
        let size = self.size as usize;
        let mut out: Vec<u8> = value
            .chars()
            .take(size)
            .map(|c| (c as u32 & 0x7F) as u8)
            .collect();
        out.resize(size, 0x20); // right-pad with spaces
        out.into_iter()
            .map(|b| {
                if (0x20..=0x7E).contains(&b) {
                    b
                } else {
                    b'?'
                }
            })
            .collect()
    }

    fn decode_ascii(&self, raw: &[u8]) -> String {
        let mut s = String::new();
        for &b in raw.iter().take(self.size as usize) {
            s.push(if (0x20..=0x7E).contains(&b) {
                b as char
            } else {
                ' '
            });
        }
        s.trim_end().to_string()
    }
}

// ---------------------------------------------------------------------------
// Group lookup (parameters live in `zencore_data`)
// ---------------------------------------------------------------------------

/// Identifies which substructure a parameter lives in.
///
/// For partial groups, also pass `partial_idx` (1..4) to the lookup helpers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ParamGroup {
    Common,
    Pmt,
    Partial,
    PartialPitchEnv,
    PartialFilterEnv,
    PartialAmpEnv,
    PartialLfo,
    /// Named `PartialEq` after the `[Partial EQ]` section — qualified use
    /// (`ParamGroup::PartialEq`) avoids confusion with the `PartialEq` trait.
    PartialEq,
    SynthCommon,
    SynthPmt,
    SynthPartial,
    Mfx,
}

impl ParamGroup {
    pub fn as_str(self) -> &'static str {
        match self {
            ParamGroup::Common => "common",
            ParamGroup::Pmt => "pmt",
            ParamGroup::Partial => "partial",
            ParamGroup::PartialPitchEnv => "partial_pitch_env",
            ParamGroup::PartialFilterEnv => "partial_filter_env",
            ParamGroup::PartialAmpEnv => "partial_amp_env",
            ParamGroup::PartialLfo => "partial_lfo",
            ParamGroup::PartialEq => "partial_eq",
            ParamGroup::SynthCommon => "synth_common",
            ParamGroup::SynthPmt => "synth_pmt",
            ParamGroup::SynthPartial => "synth_partial",
            ParamGroup::Mfx => "mfx",
        }
    }
}

/// The parameter table for one group.
pub fn group_table(group: ParamGroup) -> &'static [ZCParam] {
    match group {
        ParamGroup::Common => zencore_data::TONE_COMMON,
        ParamGroup::Pmt => zencore_data::TONE_PMT,
        ParamGroup::Partial => zencore_data::TONE_PARTIAL,
        ParamGroup::PartialPitchEnv => zencore_data::PARTIAL_PITCH_ENV,
        ParamGroup::PartialFilterEnv => zencore_data::PARTIAL_FILTER_ENV,
        ParamGroup::PartialAmpEnv => zencore_data::PARTIAL_AMP_ENV,
        ParamGroup::PartialLfo => zencore_data::PARTIAL_LFO,
        ParamGroup::PartialEq => zencore_data::PARTIAL_EQ,
        ParamGroup::SynthCommon => zencore_data::TONE_SYNTH_COMMON,
        ParamGroup::SynthPmt => zencore_data::TONE_SYNTH_PMT,
        ParamGroup::SynthPartial => zencore_data::TONE_SYNTH_PARTIAL,
        ParamGroup::Mfx => zencore_data::TONE_MFX,
    }
}

/// Every group table, in a stable order — used for discoverability.
pub static ALL_TABLES: &[(ParamGroup, &[ZCParam])] = &[
    (ParamGroup::Common, zencore_data::TONE_COMMON),
    (ParamGroup::Pmt, zencore_data::TONE_PMT),
    (ParamGroup::Partial, zencore_data::TONE_PARTIAL),
    (ParamGroup::PartialPitchEnv, zencore_data::PARTIAL_PITCH_ENV),
    (
        ParamGroup::PartialFilterEnv,
        zencore_data::PARTIAL_FILTER_ENV,
    ),
    (ParamGroup::PartialAmpEnv, zencore_data::PARTIAL_AMP_ENV),
    (ParamGroup::PartialLfo, zencore_data::PARTIAL_LFO),
    (ParamGroup::PartialEq, zencore_data::PARTIAL_EQ),
    (ParamGroup::SynthCommon, zencore_data::TONE_SYNTH_COMMON),
    (ParamGroup::SynthPmt, zencore_data::TONE_SYNTH_PMT),
    (ParamGroup::SynthPartial, zencore_data::TONE_SYNTH_PARTIAL),
    (ParamGroup::Mfx, zencore_data::TONE_MFX),
];

/// Find a parameter by group + machine name.
pub fn lookup(group: ParamGroup, name: &str) -> Result<ZCParam> {
    group_table(group)
        .iter()
        .find(|p| p.name == name)
        .copied()
        .ok_or_else(|| Error::Index(format!("unknown {} parameter: {name}", group.as_str())))
}

/// Offset of a substructure relative to the tone descriptor base.
pub fn substructure_offset(group: ParamGroup, partial_idx: usize) -> Result<u32> {
    match group {
        ParamGroup::Common => Ok(TONE_COMMON_OFFS),
        ParamGroup::Pmt => Ok(TONE_PMT_OFFS),
        ParamGroup::Mfx => Ok(TONE_MFX_OFFS),
        ParamGroup::SynthCommon => Ok(TONE_SYNTH_COMMON_OFFS),
        ParamGroup::SynthPmt => Ok(TONE_SYNTH_PMT_OFFS),
        ParamGroup::Partial => partial_offset(partial_idx, PARTIAL_BASE_OFFS, PARTIAL_STRIDE),
        ParamGroup::PartialPitchEnv => partial_offset(
            partial_idx,
            PARTIAL_PITCH_ENV_BASE,
            PARTIAL_PITCH_ENV_STRIDE,
        ),
        ParamGroup::PartialFilterEnv => partial_offset(
            partial_idx,
            PARTIAL_FILTER_ENV_BASE,
            PARTIAL_FILTER_ENV_STRIDE,
        ),
        ParamGroup::PartialAmpEnv => partial_offset(partial_idx, PARTIAL_AMP_ENV_BASE, PARTIAL_AMP_ENV_STRIDE),
        ParamGroup::PartialLfo => partial_offset(partial_idx, PARTIAL_LFO_BASE, PARTIAL_LFO_STRIDE),
        ParamGroup::PartialEq => partial_offset(partial_idx, PARTIAL_EQ_BASE, PARTIAL_EQ_STRIDE),
        ParamGroup::SynthPartial => partial_offset(
            partial_idx,
            TONE_SYNTH_PARTIAL_BASE,
            TONE_SYNTH_PARTIAL_STRIDE,
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn partial_offset_layout() {
        assert_eq!(
            partial_offset(1, PARTIAL_BASE_OFFS, PARTIAL_STRIDE).unwrap(),
            0x2000
        );
        assert_eq!(
            partial_offset(4, PARTIAL_BASE_OFFS, PARTIAL_STRIDE).unwrap(),
            0x2300
        );
        assert_eq!(
            partial_offset(1, PARTIAL_LFO_BASE, PARTIAL_LFO_STRIDE).unwrap(),
            0x3000
        );
        assert_eq!(
            partial_offset(2, PARTIAL_LFO_BASE, PARTIAL_LFO_STRIDE).unwrap(),
            0x3200
        );
        assert!(partial_offset(0, PARTIAL_BASE_OFFS, PARTIAL_STRIDE).is_err());
        assert!(partial_offset(5, PARTIAL_BASE_OFFS, PARTIAL_STRIDE).is_err());
    }

    #[test]
    fn name_field_is_ascii() {
        let p = lookup(ParamGroup::Common, "name").unwrap();
        assert!(p.ascii);
        assert_eq!(p.size, 16);
    }

    #[test]
    fn coarse_tune_round_trip() {
        let p = lookup(ParamGroup::Common, "tone_coarse_tune").unwrap();
        assert_eq!(p.size, 1);
        let encoded = p.encode(&ZCValue::Int(70)).unwrap();
        assert_eq!(encoded, vec![70]);
        assert_eq!(p.decode(&encoded).unwrap(), ZCValue::Int(70));
    }
}
