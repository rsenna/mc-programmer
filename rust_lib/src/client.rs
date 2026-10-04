//! High-level client that ties a [`Transport`] to the rest of the library.
//!
//! Typical entry point:
//!
//! ```
//! use mc_programmer::{MCClient, LoopbackTransport, ModelId};
//!
//! let transport = LoopbackTransport::new(ModelId::Mc707, 0xFF);
//! let client = MCClient::new(transport, ModelId::Mc707.value());
//!
//! let mut step = client.read_note_step(0, 0, 4).unwrap();
//! step.components[0].note = 60; // C3 (Roland convention)
//! client.write_note_step(0, 0, 4, &step).unwrap();
//! ```
//!
//! The methods on this class are deliberately thin — they compute the right
//! address via [`AddressMap`], build a Sys-Ex message via [`crate::protocol`],
//! and call `transport.transfer`.

use crate::addresses::{
    add_roland_address, add_roland_offset, AddressMap, StepScale, STEP_SIZE, TONE_COMMON_COARSE_TUNE_OFFS,
};
use crate::error::{Error, Result};
use crate::notes::Pitch;
use crate::preset::{region_size, tone_region_specs, TonePreset, ToneRegion};
use crate::protocol::{
    build_dt1, build_rq1, model_name, parse_dt1_payload, universal_device_inquiry, validate_7bit_data,
};
use crate::sequencer::{decode_note_step, encode_note_step, NoteStep};
use crate::transport::Transport;
use crate::zencore::{self, lookup, substructure_offset, ParamGroup, ZCParam, ZCValue};

/// One clip's step-time parameters (scale + count).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ClipStepParams {
    pub scale: StepScale,
    pub step_count: usize,
}

pub struct MCClient<T: Transport> {
    pub transport: T,
    pub addresses: AddressMap,
    pub model_id: u8,
}

impl<T: Transport> MCClient<T> {
    /// Create a client with the standard address map and supplied model ID.
    /// Construction performs no device inquiry; SysEx operations validate the ID.
    pub fn new(transport: T, model_id: u8) -> Self {
        Self::with_addresses(transport, model_id, AddressMap::new())
    }

    /// Create a client from a transport, model ID, and precomputed address map.
    pub fn with_addresses(transport: T, model_id: u8, addresses: AddressMap) -> Self {
        MCClient {
            transport,
            addresses,
            model_id,
        }
    }

    /// Build a client and auto-detect the model ID via universal device inquiry.
    pub fn detect(transport: T) -> Result<Self> {
        let model_id = Self::detect_model_id(&transport)?;
        Ok(Self::new(transport, model_id))
    }

    /// Detect a model ID via universal device inquiry. Errors if no reply.
    pub fn detect_model_id(transport: &T) -> Result<u8> {
        let reply = transport.transfer(&universal_device_inquiry(), 0.2)?;
        if reply.len() < 7 {
            return Err(Error::NoDeviceResponse);
        }
        Ok(reply[6])
    }

    // ---- low-level RQ1 / DT1 ----------------------------------------------

    /// Request `count` bytes at a packed Roland address and return the DT1 payload.
    /// An empty transport reply yields an empty vector; framing and transport errors propagate.
    pub fn read(&self, address: u32, count: u32) -> Result<Vec<u8>> {
        let msg = build_rq1(self.model_id, address, count)?;
        if count == 0 {
            return Ok(Vec::new());
        }
        let reply = self.transport.transfer(&msg, 0.1)?;
        if reply.is_empty() {
            return Ok(Vec::new());
        }
        let response = parse_dt1_payload(&reply)?;
        if reply[6] != self.model_id {
            return Err(Error::Midi(format!(
                "DT1 model ID does not match request: expected 0x{:02X}, got 0x{:02X}",
                self.model_id, reply[6]
            )));
        }
        if response.address != address {
            return Err(Error::Midi(format!(
                "DT1 address mismatch: requested 0x{address:08X}, got 0x{:08X}",
                response.address
            )));
        }
        if response.data.len() != count as usize {
            return Err(Error::Midi(format!(
                "DT1 payload length mismatch at 0x{address:08X}: requested {count}, got {}",
                response.data.len()
            )));
        }
        Ok(response.data)
    }

    /// Send 7-bit data to a packed Roland address in a DT1 message.
    /// Invalid model IDs, addresses, or payload bytes fail before transport use.
    pub fn write(&self, address: u32, data: &[u8]) -> Result<()> {
        let msg = build_dt1(self.model_id, address, data)?;
        self.transport.transfer(&msg, 0.05)?;
        Ok(())
    }

    /// Read a parameter; single-byte values are raw 7-bit, wider values are
    /// MSB-first nibble-packed into the returned `u32`.
    pub fn read_param(&self, address: u32, size: usize) -> Result<u32> {
        if size > 8 {
            return Err(Error::Value(format!(
                "parameter size {size} exceeds the 8-byte u32 packing limit"
            )));
        }
        let data = self.read(address, size as u32)?;

        if size == 1 {
            return Ok(data.first().copied().map(u32::from).unwrap_or(0));
        }
        let mut value = 0u32;
        for (i, &b) in data.iter().enumerate() {
            value |= u32::from(b & 0x0f) << (4 * (data.len() - i - 1));
        }
        Ok(value)
    }

    /// Write a value using up to eight wire bytes.
    /// A size of one writes the low seven bits; other sizes write the low
    /// `size` nibbles, most significant first. Sizes above eight return an error.
    pub fn write_param(&self, address: u32, value: u32, size: usize) -> Result<()> {
        if size > 8 {
            return Err(Error::Value(format!(
                "parameter size {size} exceeds the 8-byte u32 packing limit"
            )));
        }
        let fits = if size == 1 {
            value <= 0x7f
        } else if size < 8 {
            value < (1u32 << (4 * size))
        } else {
            true
        };
        if !fits {
            return Err(Error::Value(format!(
                "value 0x{value:X} does not fit in {size} parameter bytes"
            )));
        }
        if size == 1 {
            return self.write(address, &[value as u8]);
        }
        let nibbles: Vec<u8> = (0..size)
            .map(|i| ((value >> (4 * (size - i - 1))) & 0x0f) as u8)
            .collect();
        self.write(address, &nibbles)
    }

    // ---- sequencer ---------------------------------------------------------

    /// Read and decode a note record using zero-based track, clip, and step indices.
    pub fn read_note_step(&self, trk_idx: usize, clip_idx: usize, step_idx: usize) -> Result<NoteStep> {
        let addr = self
            .addresses
            .note_step_address(trk_idx, clip_idx, step_idx)?;
        let raw = self.read(addr, STEP_SIZE)?;
        decode_note_step(&raw)
    }

    /// Encode and write a note record using zero-based track, clip, and step indices.
    pub fn write_note_step(&self, trk_idx: usize, clip_idx: usize, step_idx: usize, step: &NoteStep) -> Result<()> {
        let addr = self
            .addresses
            .note_step_address(trk_idx, clip_idx, step_idx)?;
        self.write(addr, &encode_note_step(step))
    }

    /// Returns the clip's step scale and step count. Per the MC video the
    /// step length is encoded as two nibble bytes — e.g. `0x08 0x00` → 128.
    pub fn read_clip_step_params(&self, trk_idx: usize, clip_idx: usize) -> Result<ClipStepParams> {
        let addr = self.addresses.clip_params_address(trk_idx, clip_idx)?;
        let data = self.read(addr, 4)?;
        if data.len() < 3 {
            return Ok(ClipStepParams {
                scale: StepScale::Sixteenth,
                step_count: 16,
            });
        }
        Ok(ClipStepParams {
            scale: StepScale::from_u8(data[0]),
            step_count: ((data[1] as usize) << 4) | data[2] as usize,
        })
    }

    // ---- ZEN-Core ----------------------------------------------------------

    /// Read raw coarse tune, where 64 is centered; an empty reply yields zero.
    /// Indices are zero-based; `None` selects the track tone.
    pub fn read_coarse_tune(&self, trk_idx: usize, clip_idx: Option<usize>) -> Result<u8> {
        let base = self.addresses.zencore_tone_address(trk_idx, clip_idx)?;
        let addr = add_roland_address(base, TONE_COMMON_COARSE_TUNE_OFFS)?;
        let data = self.read(addr, 1)?;
        Ok(data.first().copied().unwrap_or(0))
    }

    /// Clamp raw coarse tune to `16..=112` and write it to the selected tone.
    /// Indices are zero-based; `None` selects the track tone.
    pub fn write_coarse_tune(&self, trk_idx: usize, clip_idx: Option<usize>, value: u8) -> Result<()> {
        // MC range is 16..112 with 64 = center, per ZC video / zen_core_tools.
        let clamped = value.clamp(16, 112);
        let base = self.addresses.zencore_tone_address(trk_idx, clip_idx)?;
        let addr = add_roland_address(base, TONE_COMMON_COARSE_TUNE_OFFS)?;
        self.write(addr, &[clamped])
    }

    /// Read-modify-write helper matching `coarse_tune_rmw` in the scripts.
    pub fn nudge_coarse_tune(&self, trk_idx: usize, clip_idx: Option<usize>, step: i32) -> Result<u8> {
        let current = self.read_coarse_tune(trk_idx, clip_idx)? as i32;
        let next = (current + step).clamp(16, 112) as u8;
        self.write_coarse_tune(trk_idx, clip_idx, next)?;
        Ok(next)
    }

    // ---- ZEN-Core: generic parameter access -------------------------------

    /// Combine the tone, substructure, and parameter offsets into a packed address.
    /// Track and clip indices are zero-based; partial numbers are `1..=4`.
    fn zc_param_address(
        &self,
        trk_idx: usize,
        clip_idx: Option<usize>,
        group: ParamGroup,
        param: &ZCParam,
        partial_idx: usize,
    ) -> Result<u32> {
        let tone = self.addresses.zencore_tone_address(trk_idx, clip_idx)?;
        let substructure = add_roland_address(tone, substructure_offset(group, partial_idx)?)?;
        add_roland_address(substructure, param.offset)
    }

    /// Read a registered ZEN-Core parameter by name.
    ///
    /// Returns `None` for an empty transport reply. Partial groups require
    /// `partial_idx` in 1..4; malformed replies return an error.
    pub fn read_zc_param(
        &self,
        trk_idx: usize,
        clip_idx: Option<usize>,
        name: &str,
        group: ParamGroup,
        partial_idx: usize,
    ) -> Result<Option<ZCValue>> {
        let param = lookup(group, name)?;
        let addr = self.zc_param_address(trk_idx, clip_idx, group, &param, partial_idx)?;
        let data = self.read(addr, param.size as u32)?;
        if data.is_empty() {
            Ok(None)
        } else {
            Ok(Some(param.decode(&data)?))
        }
    }

    /// Write any registered ZEN-Core parameter by name.
    pub fn write_zc_param(
        &self,
        trk_idx: usize,
        clip_idx: Option<usize>,
        name: &str,
        value: impl Into<ZCValue>,
        group: ParamGroup,
        partial_idx: usize,
    ) -> Result<()> {
        let param = lookup(group, name)?;
        let addr = self.zc_param_address(trk_idx, clip_idx, group, &param, partial_idx)?;
        let data = param.encode(&value.into())?;
        self.write(addr, &data)
    }

    // Convenience helpers built on the generic accessors.

    /// Read the tone name, or return an empty string when no value is received.
    /// Track and clip indices are zero-based; `None` selects the track tone.
    pub fn read_tone_name(&self, trk_idx: usize, clip_idx: Option<usize>) -> Result<String> {
        // Roland labels this field just "Name" → snake_case "name".
        match self.read_zc_param(trk_idx, clip_idx, "name", ParamGroup::Common, 1)? {
            Some(ZCValue::Str(s)) => Ok(s),
            _ => Ok(String::new()),
        }
    }

    /// Write a name through the fixed-width ASCII parameter codec.
    /// Track and clip indices are zero-based; `None` selects the track tone.
    pub fn write_tone_name(&self, trk_idx: usize, clip_idx: Option<usize>, name: &str) -> Result<()> {
        self.write_zc_param(trk_idx, clip_idx, "name", name, ParamGroup::Common, 1)
    }

    // =====================================================================
    // High-level helpers — front-panel numbering
    // =====================================================================
    //
    // The methods below accept `track` / `clip` / `step` parameters that match
    // the **MC's front-panel display exactly**: tracks 1..8, clips 1..16,
    // steps 1..128. The lower-level primitives above (`read_note_step`,
    // `write_zc_param`, …) take `trk_idx` / `clip_idx` / `step_idx` arguments
    // that are 0-based. `u32` (front-panel) vs `usize` (`_idx`) is the
    // discriminator.

    /// Number of physical tracks on the connected device (4 or 8).
    pub fn num_tracks(&self) -> usize {
        match self.model_id {
            0x5e => 4, // MC-101
            _ => 8,    // MC-707 / MV-1 / unknown
        }
    }

    /// Validate a front-panel track number against this model and convert it to zero-based.
    fn track_to_idx(&self, track: u32) -> Result<usize> {
        let max = self.num_tracks() as u32;
        if !(1..=max).contains(&track) {
            return Err(Error::Index(format!(
                "track={track} doesn't exist — {} has tracks 1..{max} (matching the front-panel labels)",
                model_name(self.model_id)
            )));
        }
        Ok((track - 1) as usize)
    }

    /// Convert a front-panel clip number in `1..=16` to a zero-based index.
    fn clip_to_idx(clip: u32) -> Result<usize> {
        if !(1..=16).contains(&clip) {
            return Err(Error::Index(format!(
                "clip={clip} doesn't exist — each track has clips 1..16"
            )));
        }
        Ok((clip - 1) as usize)
    }

    /// Convert a front-panel step number in `1..=128` to a zero-based index.
    fn step_to_idx(step: u32) -> Result<usize> {
        if !(1..=128).contains(&step) {
            return Err(Error::Index(format!(
                "step={step} doesn't exist — each clip has steps 1..128"
            )));
        }
        Ok((step - 1) as usize)
    }

    // ---- pattern programming ---------------------------------------------

    /// Add one note to a step, packing it into the lowest free lane.
    ///
    /// `track`, `clip` and `step` are 1-based. `pitch` accepts a MIDI number
    /// (via [`Pitch::Midi`]) or a note name (via [`Pitch::Note`] / `.into()`).
    ///
    /// `length_steps` is the duration in step units: `0.8` is the default
    /// 80-cent length, `1.0` a full step; values > 1.0 tie the note across the
    /// appropriate number of following steps.
    pub fn place_note(
        &self,
        track: u32,
        clip: u32,
        step: u32,
        pitch: impl Into<Pitch>,
        velocity: u8,
        length_steps: f64,
    ) -> Result<()> {
        let ti = self.track_to_idx(track)?;
        let ci = Self::clip_to_idx(clip)?;
        let si = Self::step_to_idx(step)?;
        let midi = pitch.into().resolve()?;

        let mut existing = self.read_note_step(ti, ci, si)?;
        for comp in existing.components.iter_mut() {
            if comp.is_empty() {
                comp.note = midi;
                comp.velocity = velocity.min(127);
                comp.start_cents = 0;
                comp.end_cents = if length_steps > 1.0 {
                    crate::sequencer::END_TIME_TIE
                } else {
                    (length_steps * 100.0).round().clamp(0.0, 100.0) as u8
                };
                self.write_note_step(ti, ci, si, &existing)?;
                // Note placed; continue ties.
                self.tie_following(ti, ci, si, midi, velocity, length_steps)?;
                return Ok(());
            }
        }
        Ok(()) // all 16 lanes full — silently drop, like the hardware does
    }

    /// Fill available lanes in following steps for the remaining note duration.
    /// Indices are zero-based; continuation stops at the clip boundary and skips full steps.
    fn tie_following(&self, ti: usize, ci: usize, si: usize, midi: u8, velocity: u8, length_steps: f64) -> Result<()> {
        let mut remaining = length_steps - 1.0;
        let mut cur = si + 1;
        while remaining > 0.0 && cur < crate::addresses::STEPS_PER_CLIP {
            let mut tail = self.read_note_step(ti, ci, cur)?;
            for comp in tail.components.iter_mut() {
                if comp.is_empty() {
                    comp.note = midi;
                    comp.velocity = velocity.min(127);
                    comp.start_cents = 0;
                    comp.end_cents = if remaining > 1.0 {
                        crate::sequencer::END_TIME_TIE
                    } else {
                        (remaining * 100.0).round().clamp(0.0, 100.0) as u8
                    };
                    self.write_note_step(ti, ci, cur, &tail)?;
                    break;
                }
            }
            remaining -= 1.0;
            cur += 1;
        }
        Ok(())
    }

    /// Erase every note from one step. 1-based `track`/`clip`/`step`.
    pub fn clear_step(&self, track: u32, clip: u32, step: u32) -> Result<()> {
        let ti = self.track_to_idx(track)?;
        let ci = Self::clip_to_idx(clip)?;
        let si = Self::step_to_idx(step)?;
        self.write_note_step(ti, ci, si, &NoteStep::default())
    }

    /// Erase every step of a clip. 1-based `track`/`clip`.
    pub fn clear_clip(&self, track: u32, clip: u32) -> Result<()> {
        let ti = self.track_to_idx(track)?;
        let ci = Self::clip_to_idx(clip)?;
        let empty = NoteStep::default();
        for s in 0..crate::addresses::STEPS_PER_CLIP {
            self.write_note_step(ti, ci, s, &empty)?;
        }
        Ok(())
    }

    /// Program a whole pattern in one call.
    ///
    /// `track` / `clip` are 1-based; `notes` maps pitches (names or MIDI
    /// numbers) to lists of **1-based** step positions. If `clear_first` is
    /// true (default) the clip is wiped before the new notes are written.
    pub fn set_pattern(
        &self,
        track: u32,
        clip: u32,
        notes: &[(Pitch, Vec<u32>)],
        velocity: u8,
        clear_first: bool,
    ) -> Result<()> {
        if clear_first {
            self.clear_clip(track, clip)?;
        }
        for (pitch, steps) in notes {
            for &step in steps {
                self.place_note(track, clip, step, pitch.clone(), velocity, 0.8)?;
            }
        }
        Ok(())
    }

    // ---- coarse tune in semitones (musician units) -----------------------

    /// Read coarse tune relative to center in semitones using front-panel numbering.
    /// Pass `None` for the track tone; an empty device reply maps to -64.
    pub fn read_coarse_tune_semitones(&self, track: u32, clip: Option<u32>) -> Result<i32> {
        let ti = self.track_to_idx(track)?;
        let ci = clip.map(Self::clip_to_idx).transpose()?;
        Ok(self.read_coarse_tune(ti, ci)? as i32 - 64)
    }

    /// Write coarse tune in `-48..=48` semitones using front-panel numbering.
    /// Pass `None` for the track tone; out-of-range tuning returns an error.
    pub fn write_coarse_tune_semitones(&self, track: u32, clip: Option<u32>, semitones: i32) -> Result<()> {
        if !(-48..=48).contains(&semitones) {
            return Err(Error::Value(format!(
                "semitones={semitones} out of range — must be -48..+48 (your MC's hardware limit)"
            )));
        }
        let ti = self.track_to_idx(track)?;
        let ci = clip.map(Self::clip_to_idx).transpose()?;
        self.write_coarse_tune(ti, ci, (64 + semitones) as u8)
    }

    // ---- discoverability --------------------------------------------------

    /// Names of every ZEN-Core parameter the library knows how to set.
    pub fn list_parameters(&self, group: Option<ParamGroup>) -> Vec<&'static str> {
        let mut names: Vec<&'static str> = match group {
            Some(g) => zencore::group_table(g).iter().map(|p| p.name).collect(),
            None => {
                let mut set: Vec<&'static str> = Vec::new();
                for (_, table) in zencore::ALL_TABLES {
                    for p in *table {
                        if !set.contains(&p.name) {
                            set.push(p.name);
                        }
                    }
                }
                set
            }
        };
        names.sort_unstable();
        names
    }

    /// Describe the first matching parameter name with its group, offset, size, and range.
    /// Return an index error if no registered parameter has that name.
    pub fn describe_parameter(&self, name: &str) -> Result<String> {
        for (group, table) in zencore::ALL_TABLES {
            if let Some(p) = table.iter().find(|p| p.name == name) {
                return Ok(format!(
                    "{} ({}, offset 0x{:04X}, size {}, range {}..{})",
                    p.label,
                    group.as_str(),
                    p.offset,
                    p.size,
                    p.min_value,
                    p.max_value
                ));
            }
        }
        Err(Error::Index(format!("unknown parameter {name:?}")))
    }

    // ---- Scene / Clip selection via Program Change -----------------------
    //
    // "Program Change (0-127) on the control channel selects a Scene.
    //  Program Change (0-15) on a track channel selects that track's Clip."
    //
    // The MC family does **not** use Bank Select for tone/patch selection.

    /// Send Program Change for scene `1..=128` on control channel `1..=16`.
    pub fn select_scene(&self, scene_number: u32, control_channel: u32) -> Result<()> {
        if !(1..=128).contains(&scene_number) {
            return Err(Error::Index(format!(
                "scene_number={scene_number} out of range — must be 1..128"
            )));
        }
        if !(1..=16).contains(&control_channel) {
            return Err(Error::Index(format!(
                "control_channel={control_channel} out of range — must be 1..16"
            )));
        }
        let status = 0xc0 | ((control_channel - 1) & 0x0f) as u8;
        self.transport.send_raw(&[status, (scene_number - 1) as u8])
    }

    /// Select clip `1..=16` on a front-panel track using Program Change.
    /// The MIDI channel defaults to the track number; explicit channels must be `1..=16`.
    pub fn select_clip(&self, track: u32, clip_number: u32, track_channel: Option<u32>) -> Result<()> {
        if !(1..=16).contains(&clip_number) {
            return Err(Error::Index(format!(
                "clip_number={clip_number} out of range — must be 1..16"
            )));
        }
        let ti = self.track_to_idx(track)?;
        let channel = track_channel.unwrap_or((ti + 1) as u32);
        if !(1..=16).contains(&channel) {
            return Err(Error::Index(format!(
                "track_channel={channel} out of range — must be 1..16"
            )));
        }
        let status = 0xc0 | ((channel - 1) & 0x0f) as u8;
        self.transport.send_raw(&[status, (clip_number - 1) as u8])
    }

    // =====================================================================
    // Chunked I/O (multi-RQ1 / multi-DT1 for blocks > 128 bytes)
    // =====================================================================

    /// Conservative upper bound for one Sys-Ex payload.
    pub const CHUNK_SIZE: usize = 0x80;

    /// Read a block in requests of at most [`Self::CHUNK_SIZE`] bytes.
    /// Reject any chunk reply whose length differs from the requested size.
    pub fn read_chunked(&self, address: u32, count: usize) -> Result<Vec<u8>> {
        let mut out = Vec::with_capacity(count);
        let mut offset = 0usize;
        while offset < count {
            let n = Self::CHUNK_SIZE.min(count - offset);
            let chunk_address = add_roland_offset(
                address,
                u32::try_from(offset).map_err(|_| Error::Value("chunk offset exceeds u32".to_string()))?,
            )?;
            let chunk = self.read(chunk_address, n as u32)?;
            if chunk.len() != n {
                return Err(Error::Midi(format!(
                    "chunk at 0x{chunk_address:08X} has {} bytes; expected {n}",
                    chunk.len()
                )));
            }
            out.extend_from_slice(&chunk);
            offset += n;
        }
        Ok(out)
    }

    /// Write a block in chunks of at most [`Self::CHUNK_SIZE`] bytes.
    /// Validate all payload bytes as 7-bit before sending the first chunk.
    pub fn write_chunked(&self, address: u32, data: &[u8]) -> Result<()> {
        validate_7bit_data(data, "chunked DT1 payload")?;
        let mut offset = 0usize;
        while offset < data.len() {
            let n = Self::CHUNK_SIZE.min(data.len() - offset);
            let chunk_address = add_roland_offset(
                address,
                u32::try_from(offset).map_err(|_| Error::Value("chunk offset exceeds u32".to_string()))?,
            )?;
            self.write(chunk_address, &data[offset..offset + n])?;
            offset += n;
        }
        Ok(())
    }

    // =====================================================================
    // Tone preset dump / restore
    // =====================================================================

    /// Capture every byte of one ZEN-Core tone into a [`TonePreset`].
    ///
    /// `track` is 1-based; pass `clip = None` to dump the track-wide tone.
    pub fn dump_tone(&self, track: u32, clip: Option<u32>, name: Option<String>) -> Result<TonePreset> {
        let ti = self.track_to_idx(track)?;
        let ci = clip.map(Self::clip_to_idx).transpose()?;
        let base = self.addresses.zencore_tone_address(ti, ci)?;

        let mut regions: Vec<ToneRegion> = Vec::new();
        for spec in tone_region_specs() {
            let size = region_size(spec.section_key) as usize;
            let bytes = self.read_chunked(add_roland_address(base, spec.offset)?, size)?;
            regions.push(ToneRegion {
                offset: spec.offset,
                label: spec.label,
                data: bytes,
            });
        }

        let display_name = match name {
            Some(n) => Some(n),
            None => self.read_tone_name(ti, ci).ok(),
        };

        Ok(TonePreset::new(
            model_name(self.model_id),
            display_name,
            regions,
        ))
    }

    /// Write a [`TonePreset`] into a tone slot.
    ///
    /// The preset's `model` field is checked against this client's model ID;
    /// loading a 707 preset onto a 101 (or vice versa) errors unless the
    /// preset has no model tag.
    pub fn load_tone(&self, track: u32, clip: Option<u32>, preset: &TonePreset) -> Result<()> {
        self.load_tone_with_force(track, clip, preset, false)
    }

    /// Validate and restore a preset using front-panel track and clip numbers.
    /// `None` selects the track tone; `force` bypasses model matching only,
    /// while region bounds and payload validation still run before any writes.
    pub(crate) fn load_tone_with_force(
        &self,
        track: u32,
        clip: Option<u32>,
        preset: &TonePreset,
        force: bool,
    ) -> Result<()> {
        let expected = model_name(self.model_id);
        if !force && !preset.model.is_empty() && preset.model != expected {
            return Err(Error::Value(format!(
                "preset model {:?} doesn't match this client's model {expected:?}",
                preset.model
            )));
        }

        let ti = self.track_to_idx(track)?;
        let ci = clip.map(Self::clip_to_idx).transpose()?;
        let base = self.addresses.zencore_tone_address(ti, ci)?;
        preset.validate_for_write(&expected, force, base, "preset")?;

        for region in &preset.regions {
            self.write_chunked(add_roland_address(base, region.offset)?, &region.data)?;
        }
        Ok(())
    }
}
