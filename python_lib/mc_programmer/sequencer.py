"""Note-step and motion-step encode / decode for the MC sequencer.

Layout (per the MC programming video transcript)
-------------------------------------------------

Each step record occupies 0x100 bytes inside its clip's note-sequence area.
The first 0x10 bytes are unknown / not decoded here. After that:

  offset 0x10  16 notes        — 2 bytes/note, packed as two 4-bit nibbles
                                  (e.g. C4 = 0x3C → 0x03 0x0C). An empty
                                  note slot is encoded as 0x0F 0x0F (i.e.
                                  0xFF / -1).
  offset 0x30  16 velocities   — 1 byte each (0..127)
  offset 0x40  16 start times  — 2 bytes each, signed cents-of-a-step
                                  (range −50..+99). Negative values are
                                  encoded in two's-complement nibble pairs:
                                  −10 → 0xF6 → bytes 0x0F 0x06.
  offset 0x60  16 end times    — 1 byte each, 0..100. 100 means *tied* to
                                  the next step; 0..99 means the note ends
                                  that many cents into the current step.
  offset 0x70  1 byte substep  — 0 (none) / 1 (×2) / 2 (×3) / 3 (×4) / 4 (flam)

Polyphony is represented by up to 16 *components* per step, indexed 0..15
("lane 0" through "lane 15"). The MC packs concurrent notes into the lowest
free lane and rebalances when notes are deleted, so editor code that wants
to round-trip cleanly must do the same.
"""
from __future__ import annotations

from dataclasses import dataclass, field
from enum import IntEnum
from typing import List, Sequence

from .addresses import (
    END_OFFS,
    MOTION_LANES_PER_STEP,
    MOTION_LANE_OFFSET,
    NOTES_OFFS,
    NOTES_PER_STEP,
    START_OFFS,
    STEP_SIZE,
    SUBSTEP_OFFS,
    VELOCITY_OFFS,
)

from .notes import note_name


EMPTY_NOTE = 0xFF  # sentinel byte value used for unused note slots
END_TIME_TIE = 100  # end-time value that ties the note into the next step
VELOCITY_DEFAULT = 100
END_TIME_DEFAULT = 0x50  # 80 cents — the device's default note length


class SubStep(IntEnum):
    NONE = 0
    DOUBLE = 1
    TRIPLE = 2
    QUADRUPLE = 3
    FLAM = 4


def pack_nibble_pair(value: int) -> List[int]:
    """Encode an unsigned byte (or signed via two's-complement) as a nibble pair.

    Used for both note numbers and signed start-time cents.
    """
    v = value & 0xFF
    return [(v >> 4) & 0x0F, v & 0x0F]


def unpack_nibble_pair(hi: int, lo: int) -> int:
    """Decode a nibble pair to an unsigned byte (call ``signed_byte`` for s8)."""
    return ((hi & 0x0F) << 4) | (lo & 0x0F)


def signed_byte(b: int) -> int:
    """Interpret an 8-bit value as two's-complement signed (-128..127)."""
    return b - 256 if b & 0x80 else b


def to_signed_byte(v: int) -> int:
    """Inverse of ``signed_byte``: clamp & encode -128..127 as an 8-bit value."""
    if not -128 <= v <= 127:
        raise ValueError(f"signed byte out of range: {v}")
    return v & 0xFF


@dataclass
class NoteComponent:
    """One concurrent note within a step (a "lane" entry)."""

    note: int = EMPTY_NOTE          # MIDI note 0..127, or EMPTY_NOTE
    velocity: int = VELOCITY_DEFAULT
    start_cents: int = 0            # signed; -50..+99 in practice
    end_cents: int = END_TIME_DEFAULT  # 0..99, or 100 to tie into next step
    processed: bool = False         # scratch flag used by tie-resolution code

    @property
    def is_empty(self) -> bool:
        return self.note == EMPTY_NOTE

    @property
    def ties_to_next(self) -> bool:
        return self.end_cents == END_TIME_TIE


@dataclass
class NoteStep:
    """A full step record — 16 components plus the substep flag."""

    components: List[NoteComponent] = field(
        default_factory=lambda: [NoteComponent() for _ in range(NOTES_PER_STEP)]
    )
    substep: SubStep = SubStep.NONE

    def __post_init__(self) -> None:
        if len(self.components) != NOTES_PER_STEP:
            raise ValueError(
                f"NoteStep needs exactly {NOTES_PER_STEP} components, "
                f"got {len(self.components)}"
            )

    def active_components(self) -> List[NoteComponent]:
        return [c for c in self.components if not c.is_empty]

    def compact(self) -> None:
        """Push all live notes down to the lowest lane indices.

        The MC firmware does this internally; if you build a step from
        scratch and want a stable round-trip, call this before encoding.
        """
        live = [c for c in self.components if not c.is_empty]
        empties = [NoteComponent() for _ in range(NOTES_PER_STEP - len(live))]
        self.components = live + empties


def encode_note_step(step: NoteStep) -> bytes:
    """Encode a ``NoteStep`` into the 0x100-byte on-device step record.

    The first 0x10 bytes (unknown header) are left as zeros — that matches
    what writing a freshly-initialised step looks like in the test scripts.
    """
    buf = bytearray(STEP_SIZE)

    for i, comp in enumerate(step.components):
        # notes (nibble-packed) at NOTES_OFFS
        nb = NOTES_OFFS + i * 2
        if comp.is_empty:
            buf[nb] = 0x0F
            buf[nb + 1] = 0x0F
        else:
            hi, lo = pack_nibble_pair(comp.note & 0x7F)
            buf[nb] = hi
            buf[nb + 1] = lo

        # velocity at VELOCITY_OFFS
        buf[VELOCITY_OFFS + i] = max(0, min(127, comp.velocity)) & 0x7F

        # start time (signed) — two nibble-packed bytes
        sb = START_OFFS + i * 2
        s_byte = to_signed_byte(comp.start_cents)
        sb_hi, sb_lo = pack_nibble_pair(s_byte)
        buf[sb] = sb_hi
        buf[sb + 1] = sb_lo

        # end time (0..100)
        buf[END_OFFS + i] = max(0, min(END_TIME_TIE, comp.end_cents))

    buf[SUBSTEP_OFFS] = int(step.substep) & 0x7F
    return bytes(buf)


def decode_note_step(raw: Sequence[int]) -> NoteStep:
    """Inverse of :func:`encode_note_step`."""
    if len(raw) < SUBSTEP_OFFS + 1:
        raise ValueError(f"step buffer too short: {len(raw)}")

    components: List[NoteComponent] = []
    for i in range(NOTES_PER_STEP):
        nb = NOTES_OFFS + i * 2
        note_byte = unpack_nibble_pair(raw[nb], raw[nb + 1])

        vel = raw[VELOCITY_OFFS + i] & 0x7F

        sb = START_OFFS + i * 2
        start_byte = unpack_nibble_pair(raw[sb], raw[sb + 1])
        start_cents = signed_byte(start_byte)

        end_cents = raw[END_OFFS + i]

        components.append(
            NoteComponent(
                note=note_byte if note_byte != EMPTY_NOTE else EMPTY_NOTE,
                velocity=vel,
                start_cents=start_cents,
                end_cents=end_cents,
            )
        )

    substep_byte = raw[SUBSTEP_OFFS] & 0x7F
    try:
        substep = SubStep(substep_byte)
    except ValueError:
        substep = SubStep.NONE

    return NoteStep(components=components, substep=substep)


def format_note_step(raw: Sequence[int]) -> str:
    """Format one raw 256-byte note step as a hex dump and decoded lane list.

    The raw dump preserves unknown bytes. The decoded section names valid MIDI
    pitches using Roland's octave convention and reports malformed pitches
    without hiding the original byte values.
    """
    if len(raw) != STEP_SIZE:
        raise ValueError(f"step record must be exactly {STEP_SIZE} bytes, got {len(raw)}")

    for byte in raw:
        if not isinstance(byte, int) or not 0 <= byte <= 0xFF:
            raise ValueError("step record must contain byte values in 0..255")

    step = decode_note_step(raw)
    lines = [f"MC note step ({STEP_SIZE} bytes)"]
    for offset in range(0, STEP_SIZE, 16):
        chunk = " ".join(f"{byte:02X}" for byte in raw[offset : offset + 16])
        lines.append(f"{offset:04X}: {chunk}")

    lines.append("Decoded lanes:")
    for lane, component in enumerate(step.components):
        if component.is_empty:
            pitch = "empty"
        elif component.note <= 127:
            pitch = f"{note_name(component.note)} ({component.note})"
        else:
            pitch = f"invalid 0x{component.note:02X}"

        end = "tie" if component.ties_to_next else f"{component.end_cents}c"
        lines.append(
            f"{lane:02}: note={pitch}, velocity={component.velocity}, "
            f"start={component.start_cents:+d}c, end={end}"
        )

    lines.append(f"Substep: {step.substep.name.lower()} (raw 0x{raw[SUBSTEP_OFFS]:02X})")
    return "\n".join(lines)


# --- Motion lanes -----------------------------------------------------------


@dataclass
class MotionStep:
    """A motion-step record holds up to 8 lanes (C1..C4 + 4 extras).

    Each lane has 8 substeps for finer-than-step time resolution. We
    represent that as a flat list of 8 ints per lane; lanes that aren't
    used can be ``None``.
    """

    lanes: List[List[int] | None] = field(
        default_factory=lambda: [None] * MOTION_LANES_PER_STEP
    )

    def __post_init__(self) -> None:
        if len(self.lanes) != MOTION_LANES_PER_STEP:
            raise ValueError("MotionStep requires 8 lane slots")
