"""mc_programmer — Python library for programming Roland MC-101, MC-707, and MV-1
via MIDI System Exclusive messages.

Derived from the test scripts and video transcripts in the parent repository.
The library covers:

  * the Roland RQ1 / DT1 Sys-Ex protocol (with checksum)
  * the MC address map (tracks, clips, sequencer, motion, ZEN-Core tones)
  * note-step encoding / decoding (16-lane "components", start/end cents, ties)
  * motion lane addressing
  * a small high-level client that ties a pluggable Transport to all of the
    above
"""

from .protocol import (
    ModelID,
    build_rq1,
    build_dt1,
    parse_dt1_payload,
    roland_checksum,
)
from .addresses import (
    AddressMap,
    StepScale,
    SEQ_BASE_ADDRESS,
    SEQ_PARAMS_OFFS,
    SEQ_NOTE_OFFS,
    SEQ_MOTION_OFFS,
    SEQ_PITCHBEND_OFFS,
    STEP_SIZE,
    SEQ_TRK_OFFSET,
    SEQ_CLIP_OFFSET,
    ZENCORE_TRACK_BASE,
    ZENCORE_TONE_ALIGNMENT,
)
from .sequencer import (
    NoteComponent,
    NoteStep,
    MotionStep,
    SubStep,
    EMPTY_NOTE,
    encode_note_step,
    decode_note_step,
    format_note_step,
    pack_nibble_pair,
    unpack_nibble_pair,
)
from .transport import Transport, NullTransport
from .loopback import LoopbackTransport
from .client import MCClient
from .preset import TonePreset, ToneRegion, TONE_REGIONS_SPEC
from .project import Project, TrackDump, ClipDump
from .notes import (
    note,
    note_name,
    KICK, SNARE, CLAP, HAT_CLOSED, HAT_OPEN, RIDE, CRASH,
    TOM_LOW, TOM_LOW_MID, TOM_MID, TOM_HIGH_MID, TOM_HIGH,
    SIDE_STICK, SNARE_RIM, HAT_PEDAL,
)
from .zencore import (
    ZCParam,
    ParamGroup,
    TONE_COMMON_PARAMS,
    TONE_PMT_PARAMS,
    TONE_PARTIAL_PARAMS,
    PARTIAL_PITCH_ENV_PARAMS,
    PARTIAL_FILTER_ENV_PARAMS,
    PARTIAL_AMP_ENV_PARAMS,
    PARTIAL_LFO_PARAMS,
    PARTIAL_EQ_PARAMS,
    TONE_SYNTH_COMMON_PARAMS,
    TONE_SYNTH_PMT_PARAMS,
    TONE_SYNTH_PARTIAL_PARAMS,
    TONE_MFX_PARAMS,
    TONE_COMMON_OFFS,
    TONE_MFX_OFFS,
    TONE_PMT_OFFS,
    PARTIAL_BASE_OFFS,
    PARTIAL_STRIDE,
    PARTIAL_PITCH_ENV_BASE,
    PARTIAL_FILTER_ENV_BASE,
    PARTIAL_AMP_ENV_BASE,
    PARTIAL_LFO_BASE,
    PARTIAL_LFO_STRIDE,
    PARTIAL_EQ_BASE,
    TONE_SYNTH_COMMON_OFFS,
    TONE_SYNTH_PMT_OFFS,
    TONE_SYNTH_PARTIAL_BASE,
    TONE_SYNTH_PARTIAL_STRIDE,
    NUM_PARTIALS,
    partial_offset,
    lookup_param,
    substructure_offset,
)

__all__ = [
    "ModelID",
    "build_rq1",
    "build_dt1",
    "parse_dt1_payload",
    "roland_checksum",
    "AddressMap",
    "StepScale",
    "SEQ_BASE_ADDRESS",
    "SEQ_PARAMS_OFFS",
    "SEQ_NOTE_OFFS",
    "SEQ_MOTION_OFFS",
    "SEQ_PITCHBEND_OFFS",
    "STEP_SIZE",
    "SEQ_TRK_OFFSET",
    "SEQ_CLIP_OFFSET",
    "ZENCORE_TRACK_BASE",
    "ZENCORE_TONE_ALIGNMENT",
    "NoteComponent",
    "NoteStep",
    "MotionStep",
    "SubStep",
    "EMPTY_NOTE",
    "encode_note_step",
    "decode_note_step",
    "format_note_step",
    "pack_nibble_pair",
    "unpack_nibble_pair",
    "Transport",
    "NullTransport",
    "LoopbackTransport",
    "MCClient",
    "TonePreset",
    "ToneRegion",
    "TONE_REGIONS_SPEC",
    "Project",
    "TrackDump",
    "ClipDump",
    "note",
    "note_name",
    "KICK", "SNARE", "CLAP", "HAT_CLOSED", "HAT_OPEN", "RIDE", "CRASH",
    "TOM_LOW", "TOM_LOW_MID", "TOM_MID", "TOM_HIGH_MID", "TOM_HIGH",
    "SIDE_STICK", "SNARE_RIM", "HAT_PEDAL",
    "ZCParam",
    "ParamGroup",
    "TONE_COMMON_PARAMS",
    "TONE_PMT_PARAMS",
    "TONE_PARTIAL_PARAMS",
    "PARTIAL_PITCH_ENV_PARAMS",
    "PARTIAL_FILTER_ENV_PARAMS",
    "PARTIAL_AMP_ENV_PARAMS",
    "PARTIAL_LFO_PARAMS",
    "PARTIAL_EQ_PARAMS",
    "TONE_COMMON_OFFS",
    "TONE_MFX_OFFS",
    "TONE_PMT_OFFS",
    "PARTIAL_BASE_OFFS",
    "PARTIAL_STRIDE",
    "PARTIAL_PITCH_ENV_BASE",
    "PARTIAL_FILTER_ENV_BASE",
    "PARTIAL_AMP_ENV_BASE",
    "PARTIAL_LFO_BASE",
    "PARTIAL_LFO_STRIDE",
    "PARTIAL_EQ_BASE",
    "NUM_PARTIALS",
    "partial_offset",
    "lookup_param",
    "substructure_offset",
]
