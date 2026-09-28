//! [`TonePreset`] — capture and restore one ZEN-Core tone from an MC.
//!
//! This is the per-tone layer. It dumps every byte of the conventional
//! ZEN-Core Tone substructures (Common + MFX + PMT + Partial 1-4 + Pitch/
//! Filter/Amp Envs × 4 + LFO × 4 + EQ × 4 + Synth Common/PMT/Partial) for one
//! track or clip tone, and can write the same bytes back to any other track or
//! clip slot — even on a different MC.
//!
//! The file format is plain JSON with hex byte strings, so it's human-readable,
//! diffable, and tagged with the model ID so you can't accidentally flash a
//! 707 tone onto a 101.

use std::fs;

use serde_json::{json, Value};

use crate::error::{Error, Result};
use crate::zencore_data::SECTION_TOTAL_SIZE;

pub const TONE_PRESET_FORMAT: &str = "mc_programmer.tone.v1";

pub(crate) fn bytes_to_hex(data: &[u8]) -> String {
    data.iter()
        .map(|b| format!("{b:02X}"))
        .collect::<Vec<_>>()
        .join(" ")
}

pub(crate) fn hex_to_bytes(hex: &str) -> Result<Vec<u8>> {
    hex.split_whitespace()
        .enumerate()
        .map(|(index, token)| {
            u8::from_str_radix(token, 16).map_err(|_| {
                Error::Json(format!(
                    "invalid hex byte {token:?} at position {}",
                    index + 1
                ))
            })
        })
        .collect()
}

fn required_field<'a>(value: &'a Value, field: &str, context: &str) -> Result<&'a Value> {
    if !value.is_object() {
        return Err(Error::Json(format!("{context} must be a JSON object")));
    }
    value
        .get(field)
        .ok_or_else(|| Error::Json(format!("{context}.{field} is required")))
}

fn required_string(value: &Value, field: &str, context: &str) -> Result<String> {
    required_field(value, field, context)?
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| Error::Json(format!("{context}.{field} must be a string")))
}

fn required_optional_string(value: &Value, field: &str, context: &str) -> Result<Option<String>> {
    match required_field(value, field, context)? {
        Value::Null => Ok(None),
        Value::String(text) => Ok(Some(text.clone())),
        _ => Err(Error::Json(format!(
            "{context}.{field} must be a string or null"
        ))),
    }
}

fn required_array<'a>(value: &'a Value, field: &str, context: &str) -> Result<&'a [Value]> {
    required_field(value, field, context)?
        .as_array()
        .map(Vec::as_slice)
        .ok_or_else(|| Error::Json(format!("{context}.{field} must be an array")))
}

/// One contiguous chunk of a tone descriptor.
///
/// `offset` is relative to the tone descriptor's base address. `label` is the
/// substructure name (`"tone_common"`, `"partial_1"`, …) and is informational
/// only — what matters for round-tripping is `offset` and `data`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ToneRegion {
    pub offset: u32,
    pub label: String,
    pub data: Vec<u8>,
}

/// A captured ZEN-Core tone — every substructure as a byte blob.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TonePreset {
    /// e.g. `"MC-707"` — used to refuse cross-model loads.
    pub model: String,
    /// Human-readable; defaults to the device's tone name.
    pub name: Option<String>,
    pub regions: Vec<ToneRegion>,
}

impl TonePreset {
    pub fn new(model: impl Into<String>, name: Option<String>, regions: Vec<ToneRegion>) -> Self {
        TonePreset {
            model: model.into(),
            name,
            regions,
        }
    }

    /// The full JSON object for this preset (used directly for file output and
    /// nested inside a [`crate::project::Project`]).
    pub fn to_value(&self) -> Value {
        let regions: Vec<Value> = self
            .regions
            .iter()
            .map(|r| {
                json!({
                    "offset": format!("0x{:04X}", r.offset),
                    "label": r.label,
                    "data": bytes_to_hex(&r.data),
                })
            })
            .collect();
        let name = match &self.name {
            Some(n) => json!(n),
            None => Value::Null,
        };
        json!({
            "format": TONE_PRESET_FORMAT,
            "model": self.model,
            "name": name,
            "regions": regions,
        })
    }

    pub fn from_value(v: &Value) -> Result<Self> {
        let format = required_string(v, "format", "tone preset")?;
        if format != TONE_PRESET_FORMAT {
            return Err(Error::Json(format!(
                "not an mc_programmer tone preset (format={format:?})"
            )));
        }
        let model = required_string(v, "model", "tone preset")?;
        let name = required_optional_string(v, "name", "tone preset")?;
        let regions = required_array(v, "regions", "tone preset")?
            .iter()
            .enumerate()
            .map(|(index, region)| parse_region(region, index))
            .collect::<Result<Vec<_>>>()?;

        Ok(TonePreset {
            model,
            name,
            regions,
        })
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(&self.to_value()).expect("serializing a serde_json::Value cannot fail")
    }

    pub fn from_json(raw: &str) -> Result<Self> {
        let v: Value =
            serde_json::from_str(raw).map_err(|e| Error::Json(format!("invalid JSON in tone preset: {e}")))?;
        Self::from_value(&v)
    }

    pub fn save(&self, path: &str) -> Result<()> {
        fs::write(path, self.to_json())?;
        Ok(())
    }

    pub fn load(path: &str) -> Result<Self> {
        let raw = fs::read_to_string(path)?;
        Self::from_json(&raw)
    }
}

fn parse_region(v: &Value, index: usize) -> Result<ToneRegion> {
    let context = format!("tone preset.regions[{index}]");
    let off = required_string(v, "offset", &context)?;
    let hex = off
        .strip_prefix("0x")
        .or_else(|| off.strip_prefix("0X"))
        .ok_or_else(|| Error::Json(format!("{context}.offset must start with 0x")))?;
    let offset = u32::from_str_radix(hex, 16)
        .map_err(|_| Error::Json(format!("{context}.offset is not a valid u32 hex value")))?;
    let label = required_string(v, "label", &context)?;
    let data_text = required_string(v, "data", &context)?;
    let data = hex_to_bytes(&data_text).map_err(|error| Error::Json(format!("{context}.data: {error}")))?;
    Ok(ToneRegion {
        offset,
        label,
        data,
    })
}

// ---------------------------------------------------------------------------
// Region table — which substructures get dumped, and where each one lives
// within the tone descriptor (relative to the tone base address).
// ---------------------------------------------------------------------------

/// One entry of the tone dump region table.
#[derive(Clone, Debug)]
pub struct ToneRegionSpec {
    pub label: String,
    pub offset: u32,
    pub section_key: &'static str,
}

/// The full region spec list, built the same way as the Python / Swift ports.
///
/// Offsets come from the `[ZCore]` address map in Roland's Jupiter-X/Xm MIDI
/// Implementation v1.06; sizes come from the auto-generated `SECTION_TOTAL_SIZE`.
pub fn tone_region_specs() -> Vec<ToneRegionSpec> {
    fn push(out: &mut Vec<ToneRegionSpec>, label: &str, offset: u32, key: &'static str) {
        out.push(ToneRegionSpec {
            label: label.to_string(),
            offset,
            section_key: key,
        });
    }

    let mut out = Vec::with_capacity(26);
    push(&mut out, "tone_common", 0x0000, "TONE_COMMON");
    push(&mut out, "tone_mfx", 0x0100, "TONE_MFX");
    push(&mut out, "tone_pmt", 0x1000, "TONE_PMT");

    for n in 1..=4usize {
        push(
            &mut out,
            &format!("partial_{n}"),
            0x2000 + (n - 1) as u32 * 0x100,
            "TONE_PARTIAL",
        );
    }
    for n in 1..=4usize {
        push(
            &mut out,
            &format!("partial_pitch_env_{n}"),
            0x2400 + (n - 1) as u32 * 0x100,
            "PARTIAL_PITCH_ENV",
        );
    }
    for n in 1..=4usize {
        push(
            &mut out,
            &format!("partial_filter_env_{n}"),
            0x2800 + (n - 1) as u32 * 0x100,
            "PARTIAL_FILTER_ENV",
        );
    }
    for n in 1..=4usize {
        push(
            &mut out,
            &format!("partial_amp_env_{n}"),
            0x2C00 + (n - 1) as u32 * 0x100,
            "PARTIAL_AMP_ENV",
        );
    }
    // Partial LFO has a larger stride (0x200) — the struct is 0x11E bytes.
    for n in 1..=4usize {
        push(
            &mut out,
            &format!("partial_lfo_{n}"),
            0x3000 + (n - 1) as u32 * 0x200,
            "PARTIAL_LFO",
        );
    }
    for n in 1..=4usize {
        push(
            &mut out,
            &format!("partial_eq_{n}"),
            0x3800 + (n - 1) as u32 * 0x100,
            "PARTIAL_EQ",
        );
    }

    push(&mut out, "tone_synth_common", 0x3C00, "TONE_SYNTH_COMMON");
    push(&mut out, "tone_synth_pmt", 0x3D00, "TONE_SYNTH_PMT");
    for n in 1..=4usize {
        push(
            &mut out,
            &format!("tone_synth_partial_{n}"),
            0x3E00 + (n - 1) as u32 * 0x100,
            "TONE_SYNTH_PARTIAL",
        );
    }
    out
}

/// Size in bytes of one substructure (from the auto-generated table).
pub fn region_size(section_key: &str) -> u32 {
    SECTION_TOTAL_SIZE
        .iter()
        .find(|(k, _)| *k == section_key)
        .map(|(_, s)| *s)
        .unwrap_or(0)
}
