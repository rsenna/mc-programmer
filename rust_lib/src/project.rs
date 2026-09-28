//! [`Project`] — full-state dump and restore for an entire MC.
//!
//! A `Project` is the everything-snapshot of an MC: every clip's note steps,
//! motion lanes, and pitch-bend recordings; every track-tone and clip-tone;
//! per-track settings; and the device's system block.
//!
//! It layers above [`TonePreset`] — that one captures just one tone, this one
//! captures the whole groovebox. Both use a JSON file format with hex byte
//! strings, so files diff cleanly and can be edited or merged.

use std::fs;

use serde_json::{json, Value};

use crate::addresses::{
    add_roland_address, SEQ_MOTION_OFFS, SEQ_NOTE_OFFS, SEQ_PARAMS_OFFS, SEQ_PITCHBEND_OFFS, STEP_SIZE,
    TRK_SETTINGS_BASE_ADDR,
};
use crate::client::MCClient;
use crate::error::{Error, Result};
use crate::preset::{bytes_to_hex, hex_to_bytes, TonePreset};
use crate::protocol::{model_name, validate_7bit_data};
use crate::transport::Transport;

pub const PROJECT_FORMAT: &str = "mc_programmer.project.v1";

/// Tiny system header. `hack_tools.py` is known to read this range.
pub const SYSTEM_HEADER_ADDRESS: u32 = 0x00000000;
pub const SYSTEM_HEADER_SIZE: usize = 0x30;

/// System / setup block, verified by `hack_tools.dump_settings(basic=False)`.
pub const SYSTEM_BLOCK_ADDRESS: u32 = 0x10000000;
pub const SYSTEM_BLOCK_SIZE: usize = 0x4000;

/// Per-track settings block size (the reference scripts use 32 bytes).
pub const TRACK_SETTINGS_SIZE: usize = 0x20;

/// Captured state of one clip.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClipDump {
    /// 1-based.
    pub clip_number: u32,
    /// Raw scale index (0..5).
    pub step_scale: u8,
    /// 1..128.
    pub step_count: usize,
    pub note_steps: Vec<u8>,
    pub motion_steps: Vec<u8>,
    pub pitchbend: Vec<u8>,
    pub tone: TonePreset,
}

/// Captured state of one track (settings + track tone + 16 clips).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TrackDump {
    /// 1-based.
    pub track_number: u32,
    pub settings: Vec<u8>,
    pub track_tone: TonePreset,
    pub clips: Vec<ClipDump>,
}

/// Snapshot of an entire MC's state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Project {
    /// `"MC-707"`, `"MC-101"`, or `"MV-1"`.
    pub model: String,
    /// Human-readable label.
    pub name: Option<String>,
    pub tracks: Vec<TrackDump>,
    /// Opaque bytes from `0x00000000..0x00000030`.
    pub system_header: Vec<u8>,
    /// Opaque bytes from `0x10000000..0x10004000`.
    pub system_block: Vec<u8>,
}

/// Serialize a nested tone using the tone preset JSON format.
fn tone_value(t: &TonePreset) -> Value {
    t.to_value()
}

/// Parse a nested tone with the same format checks as a standalone preset.
fn tone_from_value(v: &Value) -> Result<TonePreset> {
    TonePreset::from_value(v)
}
/// Require an object field and include its context in missing-field errors.
fn required_field<'a>(value: &'a Value, field: &str, context: &str) -> Result<&'a Value> {
    if !value.is_object() {
        return Err(Error::Json(format!("{context} must be a JSON object")));
    }
    value
        .get(field)
        .ok_or_else(|| Error::Json(format!("{context}.{field} is required")))
}

/// Read a required JSON string into an owned value, rejecting other types.
fn required_string(value: &Value, field: &str, context: &str) -> Result<String> {
    required_field(value, field, context)?
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| Error::Json(format!("{context}.{field} must be a string")))
}

/// Require a field containing a string or null; missing fields remain errors.
fn required_optional_string(value: &Value, field: &str, context: &str) -> Result<Option<String>> {
    match required_field(value, field, context)? {
        Value::Null => Ok(None),
        Value::String(text) => Ok(Some(text.clone())),
        _ => Err(Error::Json(format!(
            "{context}.{field} must be a string or null"
        ))),
    }
}

/// Borrow a required JSON array, rejecting missing fields and other types.
fn required_array<'a>(value: &'a Value, field: &str, context: &str) -> Result<&'a [Value]> {
    required_field(value, field, context)?
        .as_array()
        .map(Vec::as_slice)
        .ok_or_else(|| Error::Json(format!("{context}.{field} must be an array")))
}

/// Read a required unsigned integer that fits `u32` and the inclusive bounds.
fn required_u32(value: &Value, field: &str, context: &str, min: u32, max: u32) -> Result<u32> {
    let raw = required_field(value, field, context)?
        .as_u64()
        .ok_or_else(|| Error::Json(format!("{context}.{field} must be an unsigned integer")))?;
    let number = u32::try_from(raw).map_err(|_| Error::Json(format!("{context}.{field} is too large")))?;
    if !(min..=max).contains(&number) {
        return Err(Error::Json(format!(
            "{context}.{field}={number} out of range ({min}..={max})"
        )));
    }
    Ok(number)
}

/// Read a required unsigned integer that fits `usize` and the inclusive bounds.
fn required_usize(value: &Value, field: &str, context: &str, min: usize, max: usize) -> Result<usize> {
    let raw = required_field(value, field, context)?
        .as_u64()
        .ok_or_else(|| Error::Json(format!("{context}.{field} must be an unsigned integer")))?;
    let number = usize::try_from(raw).map_err(|_| Error::Json(format!("{context}.{field} is too large")))?;
    if !(min..=max).contains(&number) {
        return Err(Error::Json(format!(
            "{context}.{field}={number} out of range ({min}..={max})"
        )));
    }
    Ok(number)
}

/// Decode a required hex string and attach the field context to parse errors.
fn required_hex(value: &Value, field: &str, context: &str) -> Result<Vec<u8>> {
    let text = required_string(value, field, context)?;
    hex_to_bytes(&text).map_err(|error| Error::Json(format!("{context}.{field}: {error}")))
}

/// Serialize clip timing, sequencer bytes, and the nested tone to JSON.
fn clip_value(c: &ClipDump) -> Value {
    json!({
        "clip_number": c.clip_number,
        "step_scale": c.step_scale,
        "step_count": c.step_count,
        "note_steps": bytes_to_hex(&c.note_steps),
        "motion_steps": bytes_to_hex(&c.motion_steps),
        "pitchbend": bytes_to_hex(&c.pitchbend),
        "tone": tone_value(&c.tone),
    })
}

/// Parse clip fields and their numeric bounds, using track and clip indices for diagnostics.
fn clip_from_value(v: &Value, track_index: usize, clip_index: usize) -> Result<ClipDump> {
    let context = format!("project.tracks[{track_index}].clips[{clip_index}]");
    Ok(ClipDump {
        clip_number: required_u32(v, "clip_number", &context, 1, 16)?,
        step_scale: required_u32(v, "step_scale", &context, 0, u8::MAX as u32)? as u8,
        step_count: required_usize(v, "step_count", &context, 1, 128)?,
        note_steps: required_hex(v, "note_steps", &context)?,
        motion_steps: required_hex(v, "motion_steps", &context)?,
        pitchbend: required_hex(v, "pitchbend", &context)?,
        tone: tone_from_value(required_field(v, "tone", &context)?)?,
    })
}

/// Serialize a track with its settings, track tone, and captured clips.
fn track_value(t: &TrackDump) -> Value {
    let clips: Vec<Value> = t.clips.iter().map(clip_value).collect();
    json!({
        "track_number": t.track_number,
        "settings": bytes_to_hex(&t.settings),
        "track_tone": tone_value(&t.track_tone),
        "clips": clips,
    })
}

/// Parse a numbered track and its nested clips, reporting its index on field errors.
fn track_from_value(v: &Value, track_index: usize) -> Result<TrackDump> {
    let context = format!("project.tracks[{track_index}]");
    let clips = required_array(v, "clips", &context)?
        .iter()
        .enumerate()
        .map(|(clip_index, clip)| clip_from_value(clip, track_index, clip_index))
        .collect::<Result<Vec<_>>>()?;
    Ok(TrackDump {
        track_number: required_u32(
            v,
            "track_number",
            &context,
            1,
            TRK_SETTINGS_BASE_ADDR.len() as u32,
        )?,
        settings: required_hex(v, "settings", &context)?,
        track_tone: tone_from_value(required_field(v, "track_tone", &context)?)?,
        clips,
    })
}

impl Project {
    /// Create a project from captured tracks with empty system header and system block.
    pub fn new(model: impl Into<String>, name: Option<String>, tracks: Vec<TrackDump>) -> Self {
        Project {
            model: model.into(),
            name,
            tracks,
            system_header: Vec::new(),
            system_block: Vec::new(),
        }
    }

    /// Serialize the project with its format tag, nested tracks, and hexadecimal system data.
    pub fn to_value(&self) -> Value {
        let tracks: Vec<Value> = self.tracks.iter().map(track_value).collect();
        let name = match &self.name {
            Some(n) => json!(n),
            None => Value::Null,
        };
        json!({
            "format": PROJECT_FORMAT,
            "model": self.model,
            "name": name,
            "tracks": tracks,
            "system_header": bytes_to_hex(&self.system_header),
            "system_block": bytes_to_hex(&self.system_block),
        })
    }

    /// Parse a project object, validating its format tag, required fields, and numeric bounds.
    /// Device-specific compatibility and payload sizes are checked when restoring.
    pub fn from_value(v: &Value) -> Result<Self> {
        let format = required_string(v, "format", "project")?;
        if format != PROJECT_FORMAT {
            return Err(Error::Json(format!(
                "not an mc_programmer project file (format={format:?})"
            )));
        }
        let tracks = required_array(v, "tracks", "project")?
            .iter()
            .enumerate()
            .map(|(index, track)| track_from_value(track, index))
            .collect::<Result<Vec<_>>>()?;

        Ok(Project {
            model: required_string(v, "model", "project")?,
            name: required_optional_string(v, "name", "project")?,
            tracks,
            system_header: required_hex(v, "system_header", "project")?,
            system_block: required_hex(v, "system_block", "project")?,
        })
    }

    /// Preflight all project indices, payloads, and tone regions before device writes.
    /// `force` bypasses model matching only; size, address, and 7-bit checks still apply.
    fn validate_for_load<T: Transport>(&self, client: &MCClient<T>, force: bool) -> Result<()> {
        let expected = model_name(client.model_id);
        if !force && !self.model.is_empty() && self.model != expected {
            return Err(Error::Value(format!(
                "project model {:?} doesn't match the client's model {expected:?}. Pass force: true to override.",
                self.model
            )));
        }
        if self.system_header.len() > SYSTEM_HEADER_SIZE {
            return Err(Error::Value(format!(
                "project system_header length={} exceeds {SYSTEM_HEADER_SIZE} bytes",
                self.system_header.len()
            )));
        }
        if self.system_block.len() > SYSTEM_BLOCK_SIZE {
            return Err(Error::Value(format!(
                "project system_block length={} exceeds {SYSTEM_BLOCK_SIZE} bytes",
                self.system_block.len()
            )));
        }
        validate_7bit_data(&self.system_header, "project system_header")?;
        validate_7bit_data(&self.system_block, "project system_block")?;

        // Validate the entire in-memory project before the first DT1 write.
        // A late bounds error would otherwise leave the device partially restored.
        let max_tracks = client.num_tracks() as u32;
        for track in &self.tracks {
            if !(1..=max_tracks).contains(&track.track_number) {
                return Err(Error::Value(format!(
                    "project track_number={} out of range (1..={max_tracks}) for {expected}",
                    track.track_number
                )));
            }
            for clip in &track.clips {
                if !(1..=16).contains(&clip.clip_number) {
                    return Err(Error::Value(format!(
                        "project track {} clip_number={} out of range (1..=16)",
                        track.track_number, clip.clip_number
                    )));
                }
            }
        }

        for track in &self.tracks {
            if track.settings.len() != TRACK_SETTINGS_SIZE {
                return Err(Error::Value(format!(
                    "project track {} settings length={} expected {TRACK_SETTINGS_SIZE} bytes",
                    track.track_number,
                    track.settings.len()
                )));
            }
            validate_7bit_data(&track.settings, "project track settings")?;
            for clip in &track.clips {
                if !(1..=128).contains(&clip.step_count) {
                    return Err(Error::Value(format!(
                        "project track {} clip {} step_count={} out of range (1..=128)",
                        track.track_number, clip.clip_number, clip.step_count
                    )));
                }
                if clip.step_scale > 0x7F {
                    return Err(Error::Value(format!(
                        "project track {} clip {} step_scale={} is not a 7-bit MIDI value",
                        track.track_number, clip.clip_number, clip.step_scale
                    )));
                }
                let expected_bytes = clip.step_count * STEP_SIZE as usize;
                for (name, data) in [
                    ("note_steps", &clip.note_steps),
                    ("motion_steps", &clip.motion_steps),
                    ("pitchbend", &clip.pitchbend),
                ] {
                    if !data.is_empty() && data.len() != expected_bytes {
                        return Err(Error::Value(format!(
                            "project track {} clip {} {name} length={} expected {expected_bytes} bytes",
                            track.track_number,
                            clip.clip_number,
                            data.len()
                        )));
                    }
                    validate_7bit_data(
                        data,
                        &format!(
                            "project track {} clip {} {name}",
                            track.track_number, clip.clip_number
                        ),
                    )?;
                }
            }
        }

        for track in &self.tracks {
            let track_idx = (track.track_number - 1) as usize;
            let track_tone_base = client.addresses.zencore_tone_address(track_idx, None)?;
            Self::validate_tone(
                &track.track_tone,
                &expected,
                track.track_number,
                None,
                track_tone_base,
                force,
            )?;
            for clip in &track.clips {
                let clip_idx = (clip.clip_number - 1) as usize;
                let clip_tone_base = client
                    .addresses
                    .zencore_tone_address(track_idx, Some(clip_idx))?;
                Self::validate_tone(
                    &clip.tone,
                    &expected,
                    track.track_number,
                    Some(clip.clip_number),
                    clip_tone_base,
                    force,
                )?;
            }
        }
        Ok(())
    }

    /// Validate a nested tone for its destination, including track and clip context in errors.
    fn validate_tone(
        tone: &TonePreset,
        expected: &str,
        track_number: u32,
        clip_number: Option<u32>,
        base_address: u32,
        force: bool,
    ) -> Result<()> {
        let location = match clip_number {
            Some(clip) => format!("project track {track_number} clip {clip}"),
            None => format!("project track {track_number}"),
        };
        tone.validate_for_write(expected, force, base_address, &location)
    }

    /// Serialize the project as indented JSON with hexadecimal byte blocks.
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(&self.to_value()).expect("serializing a serde_json::Value cannot fail")
    }

    /// Parse JSON text as a project, returning JSON errors for malformed input.
    pub fn from_json(raw: &str) -> Result<Self> {
        let v: Value = serde_json::from_str(raw).map_err(|e| Error::Json(format!("invalid JSON in project: {e}")))?;
        Self::from_value(&v)
    }

    /// Write the project as indented JSON, replacing an existing file at `path`.
    pub fn save(&self, path: &str) -> Result<()> {
        fs::write(path, self.to_json())?;
        Ok(())
    }

    /// Read and parse a project file, propagating I/O and JSON errors.
    pub fn load(path: &str) -> Result<Self> {
        let raw = fs::read_to_string(path)?;
        Self::from_json(&raw)
    }

    // ---- dump (device → memory) ------------------------------------------

    /// Read every track and clip off the device into a `Project`.
    pub fn dump_from<T: Transport>(
        client: &MCClient<T>,
        name: Option<String>,
        include_system: bool,
    ) -> Result<Project> {
        let model = model_name(client.model_id);
        let mut project = Project::new(model, name, Vec::new());

        // Walk only the tracks the connected device actually has.
        for (track_idx, &settings_addr) in TRK_SETTINGS_BASE_ADDR
            .iter()
            .enumerate()
            .take(client.num_tracks())
        {
            let settings = client.read_chunked(settings_addr, TRACK_SETTINGS_SIZE)?;

            let track_tone = client.dump_tone((track_idx + 1) as u32, None, None)?;

            let mut clips: Vec<ClipDump> = Vec::with_capacity(16);
            for clip_idx in 0..16usize {
                let clip_base = client.addresses.clip_base(track_idx, clip_idx)?;

                let params = client.read_chunked(add_roland_address(clip_base, SEQ_PARAMS_OFFS)?, 4)?;
                let step_scale = params.first().copied().unwrap_or(1) & 0x7F;
                let mut step_count = if params.len() >= 3 {
                    ((params[1] as usize) << 4) | params[2] as usize
                } else {
                    16
                };
                step_count = step_count.clamp(1, 128);

                let length = step_count * STEP_SIZE as usize;
                let note_steps = client.read_chunked(add_roland_address(clip_base, SEQ_NOTE_OFFS)?, length)?;
                let motion_steps = client.read_chunked(add_roland_address(clip_base, SEQ_MOTION_OFFS)?, length)?;
                let pitchbend = client.read_chunked(add_roland_address(clip_base, SEQ_PITCHBEND_OFFS)?, length)?;

                let clip_tone = client.dump_tone((track_idx + 1) as u32, Some((clip_idx + 1) as u32), None)?;

                clips.push(ClipDump {
                    clip_number: (clip_idx + 1) as u32,
                    step_scale,
                    step_count,
                    note_steps,
                    motion_steps,
                    pitchbend,
                    tone: clip_tone,
                });
            }

            project.tracks.push(TrackDump {
                track_number: (track_idx + 1) as u32,
                settings,
                track_tone,
                clips,
            });
        }

        if include_system {
            project.system_header = client.read_chunked(SYSTEM_HEADER_ADDRESS, SYSTEM_HEADER_SIZE)?;
            project.system_block = client.read_chunked(SYSTEM_BLOCK_ADDRESS, SYSTEM_BLOCK_SIZE)?;
        }
        Ok(project)
    }

    // ---- restore (memory → device) ---------------------------------------

    /// Write every captured byte back to the device.
    ///
    /// Refuses to load a project whose model doesn't match the client's device
    /// unless `force` is set.
    pub fn load_into<T: Transport>(&self, client: &MCClient<T>, force: bool) -> Result<()> {
        self.validate_for_load(client, force)?;

        for track_dump in &self.tracks {
            let track_idx = (track_dump.track_number - 1) as usize;
            client.write_chunked(TRK_SETTINGS_BASE_ADDR[track_idx], &track_dump.settings)?;
            client.load_tone_with_force(track_dump.track_number, None, &track_dump.track_tone, force)?;

            for clip_dump in &track_dump.clips {
                let clip_idx = (clip_dump.clip_number - 1) as usize;
                let clip_base = client.addresses.clip_base(track_idx, clip_idx)?;

                client.write_chunked(
                    add_roland_address(clip_base, SEQ_PARAMS_OFFS)?,
                    &[
                        clip_dump.step_scale & 0x7F,
                        (clip_dump.step_count >> 4) as u8 & 0x0F,
                        clip_dump.step_count as u8 & 0x0F,
                        0x00,
                    ],
                )?;
                if !clip_dump.note_steps.is_empty() {
                    client.write_chunked(
                        add_roland_address(clip_base, SEQ_NOTE_OFFS)?,
                        &clip_dump.note_steps,
                    )?;
                }
                if !clip_dump.motion_steps.is_empty() {
                    client.write_chunked(
                        add_roland_address(clip_base, SEQ_MOTION_OFFS)?,
                        &clip_dump.motion_steps,
                    )?;
                }
                if !clip_dump.pitchbend.is_empty() {
                    client.write_chunked(
                        add_roland_address(clip_base, SEQ_PITCHBEND_OFFS)?,
                        &clip_dump.pitchbend,
                    )?;
                }

                client.load_tone_with_force(
                    track_dump.track_number,
                    Some(clip_dump.clip_number),
                    &clip_dump.tone,
                    force,
                )?;
            }
        }

        if !self.system_header.is_empty() {
            client.write_chunked(SYSTEM_HEADER_ADDRESS, &self.system_header)?;
        }
        if !self.system_block.is_empty() {
            client.write_chunked(SYSTEM_BLOCK_ADDRESS, &self.system_block)?;
        }
        Ok(())
    }
}
