"""MC-101 / MC-707 / MV-1 address-map constants and address arithmetic.

All values come straight from the test scripts (``trk_clip_tools.py``,
``zen_core_tools.py``) and the two video transcripts in the parent repo.

Address-space rules
-------------------

* Addresses are 28 useful bits laid out across four bytes; **no byte may have
  its MSB set** (the ``0x80808080`` mask in the dump routines), since the
  bytes have to fit through the 7-bit MIDI pipe. The
  ``next_valid_base_address`` helper jumps over any candidate that would
  violate that rule.

* The sequencer area starts at ``0x20000000``. Each track is offset by
  ``0x01080000`` from the previous one and each clip within a track is
  offset by ``0x00080000`` from the previous (subject to the 0x80 rule).

* Within a clip, four sub-areas matter:
    - ``0x00000`` clip header (name + misc)
    - ``0x10000`` sound-control parameters (volume / filter / resonance / …)
    - ``0x20000`` step-time + step-count parameters
    - ``0x30000`` 128 note-step records (each ``0x100`` bytes)
    - ``0x40000`` 128 motion-step records (each ``0x100`` bytes)
    - ``0x50000`` recorded pitch-bend stream
"""
from __future__ import annotations

from dataclasses import dataclass
from enum import IntEnum
from typing import List


# --- Sequencer constants -----------------------------------------------------

SEQ_BASE_ADDRESS = 0x20000000
SEQ_PARAMS_OFFS = 0x20000
SEQ_NOTE_OFFS = 0x30000
SEQ_MOTION_OFFS = 0x40000
SEQ_PITCHBEND_OFFS = 0x50000

STEP_SIZE = 0x100  # bytes per note step or motion step
SEQ_TRK_OFFSET = 0x01080000
SEQ_CLIP_OFFSET = 0x00080000
NUM_TRACKS = 8
CLIPS_PER_TRACK = 16
STEPS_PER_CLIP = 128

# Per the test scripts, clip 0 of each track lives at one of these bases:
CLIP1_BASE_ADDR: List[int] = [
    0x20000000,
    0x21080000,
    0x22100000,
    0x23180000,
    0x24200000,
    0x25280000,
    0x26300000,
    0x27380000,
]

# Track sound (not clip sound) base addresses, per trk_clip_tools.
TRK_BASE_ADDR: List[int] = [
    0x21000000,
    0x22080000,
    0x23100000,
    0x24180000,
    0x25200000,
    0x26280000,
    0x27300000,
    0x28380000,
]

# Per-track settings block (color, sound source, etc.)
TRK_SETTINGS_BASE_ADDR: List[int] = [
    0x10000800,
    0x10000A00,
    0x10000C00,
    0x10000E00,
    0x10001000,
    0x10001200,
    0x10001400,
    0x10001600,
]
SND_SRC_CFG_OFFS = 0x04  # 0 -> track is sound source, 1 -> clip is
TRK_COLOR_OFFS = 0x19
TRK_TYPE_OFFS = 0x14  # 0=TONE, 1=DRUM, 0x7F=OFF


# --- ZEN-Core descriptor constants ------------------------------------------

# Per the ZC video transcript: each tone-track block has 17 ZEN-Core
# descriptors — 16 clip tones + 1 track tone — aligned every 0x20000 bytes.
ZENCORE_TONE_ALIGNMENT = 0x20000
ZENCORE_TRACK_BASE: List[int] = [
    0x30000000,
    0x30220000,
    0x30440000,
    0x30660000,
    0x31080000,
    0x312A0000,
    0x314C0000,
    0x316E0000,
]

# Selected ZEN-Core tone offsets demonstrated in the ZC programming video.
TONE_COMMON_COARSE_TUNE_OFFS = 0x0018  # 1 byte, valid 16..112 (center 64)


# --- Note-step parameter offsets (within one 0x100-byte step record) --------

NOTES_OFFS = 0x10  # 16 notes × 2 bytes (4-bit nibble pairs)
VELOCITY_OFFS = 0x30  # 16 velocities × 1 byte
START_OFFS = 0x40  # 16 start times × 2 bytes (cents, two's complement)
END_OFFS = 0x60  # 16 end times × 1 byte (0..100; 100 ties to next step)
SUBSTEP_OFFS = 0x70  # 1 byte, 0..4 = none/2/3/4/flam
NOTES_PER_STEP = 16


# --- Motion lane constants --------------------------------------------------

MOTION_LANE_NAMES = [
    "C1/FILTER",
    "C2/MOD",
    "C3/FX",
    "C4/SOUND",
    "Ext 0x40",
    "Ext 0x50",
    "Ext 0x60",
    "Ext 0x70",
]
MOTION_LANE_OFFSET = 0x10  # bytes between lanes inside a motion step
MOTION_LANES_PER_STEP = 8
MOTION_SUBSTEPS = 8


class StepScale(IntEnum):
    """Step-time lookup table — index 0 is the slowest (eighth notes)."""

    EIGHTH = 0
    SIXTEENTH = 1
    THIRTY_SECOND = 2
    QUARTER_TRIPLET = 3
    EIGHTH_TRIPLET = 4
    SIXTEENTH_TRIPLET = 5

    @property
    def label(self) -> str:
        return [
            "1/8",
            "1/16",
            "1/32",
            "1/4T",
            "1/8T",
            "1/16T",
        ][int(self)]


def _next_valid_base_address(previous: int, offset: int = SEQ_CLIP_OFFSET) -> int:
    """Add ``offset`` and skip any address that would have a high MSB byte."""
    nxt = previous + offset
    while nxt & 0x80808080:
        nxt += offset
    return nxt


@dataclass(frozen=True)
class AddressMap:
    """Computes addresses for tracks, clips and steps in the MC address map.

    Construct once and reuse — the per-track lists of clip base addresses are
    cached at construction time so per-step lookups are pure arithmetic.
    """

    clip_base_addresses: List[List[int]]

    @classmethod
    def build(cls) -> "AddressMap":
        clip_addrs: List[List[int]] = []
        for trk_idx in range(NUM_TRACKS):
            row = [CLIP1_BASE_ADDR[trk_idx]]
            for _ in range(CLIPS_PER_TRACK - 1):
                row.append(_next_valid_base_address(row[-1]))
            clip_addrs.append(row)
        return cls(clip_base_addresses=clip_addrs)

    # ------- track / clip ---------------------------------------------------

    def track_base(self, trk_idx: int) -> int:
        return TRK_BASE_ADDR[self._t(trk_idx)]

    def track_settings_base(self, trk_idx: int) -> int:
        return TRK_SETTINGS_BASE_ADDR[self._t(trk_idx)]

    def clip_base(self, trk_idx: int, clip_idx: int) -> int:
        return self.clip_base_addresses[self._t(trk_idx)][self._c(clip_idx)]

    # ------- sequencer step addresses --------------------------------------

    def clip_params_address(self, trk_idx: int, clip_idx: int) -> int:
        return self.clip_base(trk_idx, clip_idx) + SEQ_PARAMS_OFFS

    def note_step_address(self, trk_idx: int, clip_idx: int, step_idx: int) -> int:
        return (
            self.clip_base(trk_idx, clip_idx)
            + SEQ_NOTE_OFFS
            + self._s(step_idx) * STEP_SIZE
        )

    def motion_step_address(
        self, trk_idx: int, clip_idx: int, step_idx: int, lane_idx: int = 0
    ) -> int:
        if not 0 <= lane_idx < MOTION_LANES_PER_STEP:
            raise ValueError(f"motion lane out of range: {lane_idx}")
        return (
            self.clip_base(trk_idx, clip_idx)
            + SEQ_MOTION_OFFS
            + self._s(step_idx) * STEP_SIZE
            + lane_idx * MOTION_LANE_OFFSET
        )

    # ------- ZEN-Core tone descriptors --------------------------------------

    def zencore_tone_address(self, trk_idx: int, clip_idx: int | None = None) -> int:
        """Address of a ZEN-Core tone descriptor.

        Pass ``clip_idx=None`` for the *track* tone (loaded when the track's
        sound source is set to TRACK), or 0..15 for one of the clip tones.
        """
        base = ZENCORE_TRACK_BASE[self._t(trk_idx)]
        if clip_idx is None:
            return base + ZENCORE_TONE_ALIGNMENT * CLIPS_PER_TRACK
        return base + ZENCORE_TONE_ALIGNMENT * self._c(clip_idx)

    # ------- bounds helpers -------------------------------------------------

    @staticmethod
    def _t(i: int) -> int:
        if not 0 <= i < NUM_TRACKS:
            raise ValueError(f"track index out of range: {i}")
        return i

    @staticmethod
    def _c(i: int) -> int:
        if not 0 <= i < CLIPS_PER_TRACK:
            raise ValueError(f"clip index out of range: {i}")
        return i

    @staticmethod
    def _s(i: int) -> int:
        if not 0 <= i < STEPS_PER_CLIP:
            raise ValueError(f"step index out of range: {i}")
        return i
