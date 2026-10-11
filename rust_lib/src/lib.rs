//! `mc_programmer` — Rust library for programming Roland MC-101, MC-707 and
//! MV-1 grooveboxes via MIDI System Exclusive messages.
//!
//! This is the Rust counterpart to the Python (`python_lib/mc_programmer`) and
//! Swift (`swift_lib/Sources/MCProgrammer`) libraries in this repository. The
//! three ports share the same data model, parameter table, and test coverage.
//!
//! The library covers:
//!
//! * the Roland RQ1 / DT1 Sys-Ex protocol (with checksum)
//! * the MC address map (tracks, clips, sequencer, motion, ZEN-Core tones)
//! * note-step encoding / decoding (16-lane "components", start/end cents, ties)
//! * motion lane addressing
//! * an address-space probe for locating undocumented blocks ([`probe`])
//! * the ZEN-Core parameter framework (auto-generated ~389-entry table)
//! * a high-level client that ties a pluggable [`Transport`] to all of the above
//!
//! # Example
//!
//! ```
//! use mc_programmer::{MCClient, LoopbackTransport, ModelId};
//!
//! let transport = LoopbackTransport::new(ModelId::Mc707, 0xFF);
//! let client = MCClient::new(transport, ModelId::Mc707.value());
//!
//! client.write_coarse_tune(1, None, 70).unwrap();
//! assert_eq!(client.read_coarse_tune(1, None).unwrap(), 70);
//! ```

pub mod addresses;
pub mod client;
pub mod error;
pub mod loopback;
#[cfg(feature = "midir")]
pub mod midir_transport;
pub mod notes;
pub mod preset;
pub mod probe;
pub mod project;
pub mod protocol;
pub mod sequencer;
pub mod transport;
pub mod zencore;
mod zencore_data;

pub use addresses::{
    reply_address_candidates, resolve_reply_location, AddressMap, ReplyLocation, StepScale, CLIPS_PER_TRACK,
    NUM_TRACKS, STEPS_PER_CLIP, STEP_SIZE,
};
pub use client::{ClipStepParams, MCClient};
pub use error::{Error, Result};
pub use loopback::LoopbackTransport;
#[cfg(feature = "midir")]
pub use midir_transport::{MidiPortNames, MidirTransport};
pub use notes::{
    note, note_name, Pitch, CLAP, CRASH, HAT_CLOSED, HAT_OPEN, HAT_PEDAL, KICK, RIDE, SIDE_STICK, SNARE, SNARE_RIM,
    TOM_HIGH, TOM_HIGH_MID, TOM_LOW, TOM_LOW_MID, TOM_MID,
};
pub use preset::{TonePreset, ToneRegion, TONE_PRESET_FORMAT};
pub use probe::{ByteDiff, DiffRun, ProbeOptions, Run, Snapshot, SNAPSHOT_FORMAT};
pub use project::{ClipDump, Project, TrackDump, PROJECT_FORMAT};
pub use protocol::{
    build_dt1, build_rq1, model_name, parse_dt1_payload, roland_checksum, universal_device_inquiry, ModelId,
};
pub use sequencer::{
    decode_note_step, encode_note_step, format_note_step, pack_nibble_pair, unpack_nibble_pair, MotionStep,
    NoteComponent, NoteStep, SubStep, EMPTY_NOTE, END_TIME_TIE,
};
pub use transport::{NullTransport, Transport};
pub use zencore::{
    group_table, lookup, partial_offset, substructure_offset, ParamGroup, ZCParam, ZCValue, NUM_PARTIALS,
    PARTIAL_AMP_ENV_BASE, PARTIAL_BASE_OFFS, PARTIAL_EQ_BASE, PARTIAL_FILTER_ENV_BASE, PARTIAL_LFO_BASE,
    PARTIAL_LFO_STRIDE, PARTIAL_PITCH_ENV_BASE, PARTIAL_STRIDE, TONE_COMMON_OFFS, TONE_MFX_OFFS, TONE_PMT_OFFS,
    TONE_SYNTH_COMMON_OFFS, TONE_SYNTH_PARTIAL_BASE, TONE_SYNTH_PMT_OFFS,
};
