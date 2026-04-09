"""Project — full-state dump and restore for an entire MC.

A :class:`Project` is the everything-snapshot of an MC: every clip's
note steps, motion lanes, and pitch-bend recordings; every track-tone
and clip-tone; per-track settings; and the device's system block.

It's a layer above :class:`mc_programmer.preset.TonePreset` — that one captures
just one tone, this one captures the whole groovebox. Both use a JSON
file format with hex byte strings, so files diff cleanly and can be
edited or merged.

Example::

    # Snapshot the live device:
    project = Project.dump_from(client, name="my live set, 2026-04")
    project.save("liveset.mctproject")

    # Later (or on a different device of the same model):
    project = Project.load("liveset.mctproject")
    project.load_into(client)

What the project contains
-------------------------

Per **track** (8 of them):
  * Track settings block (color, sound source, tone type, …)
  * Track-wide ZEN-Core tone (the one loaded when the track sound source
    is set to TRACK)

Per **clip** (16 per track, 128 total):
  * Clip params (step scale + step count)
  * Note step records — only as many as the clip's ``step_count``
  * Motion lane records — same trim
  * Pitch-bend recording — same trim
  * Clip-bound ZEN-Core tone (loaded when sound source is CLIP)

Optional (``include_system=True``, default on):
  * The tiny system header at ``0x00000000..0x00000030``, dumped as a
    blob. ``hack_tools.py`` is known to read this range successfully on
    real MC hardware.
  * The system / setup block at ``0x10000000..0x10004000``, dumped as
    an opaque blob. ``hack_tools.dump_settings(basic=False)`` is the
    reference for this address range — it's verified to respond on
    real MC hardware. Covers Master EQ, Master Comp, the mixer, and
    the device's global config.

Not yet supported
-----------------

* **Scene block.** No verified MC address. Roland's Jupiter-X MIDI
  Implementation puts scenes at ``0x01000000``, but the MC family
  hasn't been confirmed to use the same layout, and no reference script
  in this repo touches that range. Until someone runs ``hack_tools.py``
  on real MC hardware and finds the scene block, this library refuses
  to guess at it.
* Per-arpeggio user pattern data — same situation.

Anything you need that isn't covered: dump it manually with
``client.read(addr, count)`` and add the bytes to your own copy of the
Project file. The format is just JSON.
"""
from __future__ import annotations

import json
from dataclasses import dataclass, field
from typing import List, Optional

from .addresses import (
    SEQ_MOTION_OFFS,
    SEQ_NOTE_OFFS,
    SEQ_PARAMS_OFFS,
    SEQ_PITCHBEND_OFFS,
    STEP_SIZE,
    TRK_SETTINGS_BASE_ADDR,
)
from .preset import TonePreset


PROJECT_FORMAT = "mc_programmer.project.v1"

#: Tiny system header. ``hack_tools.py`` is known to read this range.
SYSTEM_HEADER_ADDRESS = 0x00000000
SYSTEM_HEADER_SIZE = 0x30          # 48 bytes

#: System / setup block. ``hack_tools.dump_settings(basic=False)`` is
#: the reference for this range — verified to respond on real MC hardware.
SYSTEM_BLOCK_ADDRESS = 0x10000000
SYSTEM_BLOCK_SIZE = 0x4000         # 16 KB

#: Per-track settings block size (the reference scripts use 32 bytes).
TRACK_SETTINGS_SIZE = 0x20


def _bytes_to_hex(data) -> str:
    return " ".join(f"{b:02X}" for b in data)


def _hex_to_bytes(hex_str: str) -> bytes:
    if not hex_str:
        return b""
    return bytes(int(b, 16) for b in hex_str.split())


@dataclass
class ClipDump:
    """Captured state of one clip."""

    clip_number: int                  # 1-based
    step_scale: int                   # raw scale index (0..5)
    step_count: int                   # 1..128
    note_steps: bytes                 # step_count * STEP_SIZE bytes
    motion_steps: bytes               # step_count * STEP_SIZE bytes
    pitchbend: bytes                  # step_count * STEP_SIZE bytes
    tone: TonePreset                  # the clip-bound tone

    def to_dict(self) -> dict:
        return {
            "clip_number": self.clip_number,
            "step_scale": self.step_scale,
            "step_count": self.step_count,
            "note_steps": _bytes_to_hex(self.note_steps),
            "motion_steps": _bytes_to_hex(self.motion_steps),
            "pitchbend": _bytes_to_hex(self.pitchbend),
            "tone": json.loads(self.tone.to_json()),
        }

    @classmethod
    def from_dict(cls, d: dict) -> "ClipDump":
        return cls(
            clip_number=d["clip_number"],
            step_scale=d["step_scale"],
            step_count=d["step_count"],
            note_steps=_hex_to_bytes(d["note_steps"]),
            motion_steps=_hex_to_bytes(d["motion_steps"]),
            pitchbend=_hex_to_bytes(d["pitchbend"]),
            tone=TonePreset.from_json(json.dumps(d["tone"])),
        )


@dataclass
class TrackDump:
    """Captured state of one track (settings + track tone + 16 clips)."""

    track_number: int                 # 1-based
    settings: bytes                   # the track settings block
    track_tone: TonePreset            # the track-wide tone
    clips: List[ClipDump] = field(default_factory=list)

    def to_dict(self) -> dict:
        return {
            "track_number": self.track_number,
            "settings": _bytes_to_hex(self.settings),
            "track_tone": json.loads(self.track_tone.to_json()),
            "clips": [c.to_dict() for c in self.clips],
        }

    @classmethod
    def from_dict(cls, d: dict) -> "TrackDump":
        return cls(
            track_number=d["track_number"],
            settings=_hex_to_bytes(d["settings"]),
            track_tone=TonePreset.from_json(json.dumps(d["track_tone"])),
            clips=[ClipDump.from_dict(c) for c in d["clips"]],
        )


@dataclass
class Project:
    """Snapshot of an entire MC's state."""

    model: str                        # "MC-707", "MC-101", or "MV-1"
    name: Optional[str] = None        # human-readable label
    tracks: List[TrackDump] = field(default_factory=list)

    #: Tiny system header — opaque bytes from 0x00000000..0x00000030.
    system_header: bytes = b""

    #: System / setup block — opaque bytes from 0x10000000..0x10004000.
    system_block: bytes = b""

    # ---- serialisation ----------------------------------------------------

    def to_json(self) -> str:
        return json.dumps(
            {
                "format": PROJECT_FORMAT,
                "model": self.model,
                "name": self.name,
                "tracks": [t.to_dict() for t in self.tracks],
                "system_header": _bytes_to_hex(self.system_header),
                "system_block": _bytes_to_hex(self.system_block),
            },
            indent=2,
        )

    @classmethod
    def from_json(cls, raw: str) -> "Project":
        d = json.loads(raw)
        if d.get("format") != PROJECT_FORMAT:
            raise ValueError(
                f"not an mc_programmer project file (format={d.get('format')!r})"
            )
        return cls(
            model=d["model"],
            name=d.get("name"),
            tracks=[TrackDump.from_dict(t) for t in d["tracks"]],
            system_header=_hex_to_bytes(d.get("system_header", "")),
            system_block=_hex_to_bytes(d.get("system_block", "")),
        )

    def save(self, path: str) -> None:
        with open(path, "w") as f:
            f.write(self.to_json())

    @classmethod
    def load(cls, path: str) -> "Project":
        with open(path) as f:
            return cls.from_json(f.read())

    # ---- dump (device → memory) ------------------------------------------

    @classmethod
    def dump_from(
        cls,
        client,
        name: Optional[str] = None,
        include_system: bool = True,
    ) -> "Project":
        """Read every track and clip off the device into a Project.

        Set ``include_system=False`` to skip the 16 KB system/setup
        block (and the tiny system header at 0x00000000). The default
        captures everything that this repository's reference scripts
        are known to read successfully on real MC hardware.

        The scene block is *not* dumped — no MC-verified address yet.
        """
        model_str = {0x5D: "MC-707", 0x5E: "MC-101", 0x5F: "MV-1"}.get(
            client.model_id, f"0x{client.model_id:02X}"
        )
        project = cls(model=model_str, name=name)

        # Walk only the tracks the connected device actually has —
        # MC-101 has 4, MC-707/MV-1 have 8.
        for track_idx in range(client.num_tracks):
            settings_addr = TRK_SETTINGS_BASE_ADDR[track_idx]
            settings = bytes(client._read_chunked(settings_addr, TRACK_SETTINGS_SIZE))

            track_tone = client.dump_tone(track=track_idx + 1, clip=None)

            clips: List[ClipDump] = []
            for clip_idx in range(16):
                clip_base = client.addresses.clip_base(track_idx, clip_idx)

                # Clip params: scale + count.
                params = client._read_chunked(clip_base + SEQ_PARAMS_OFFS, 4)
                step_scale = params[0] if params else 1
                step_count = ((params[1] << 4) | params[2]) if len(params) >= 3 else 16
                step_count = max(1, min(128, step_count))

                # Notes / motion / pitch-bend, only as many records as the
                # clip actually uses.
                length = step_count * STEP_SIZE
                note_steps = bytes(
                    client._read_chunked(clip_base + SEQ_NOTE_OFFS, length)
                )
                motion_steps = bytes(
                    client._read_chunked(clip_base + SEQ_MOTION_OFFS, length)
                )
                pitchbend = bytes(
                    client._read_chunked(clip_base + SEQ_PITCHBEND_OFFS, length)
                )

                # Clip tone.
                clip_tone = client.dump_tone(track=track_idx + 1, clip=clip_idx + 1)

                clips.append(ClipDump(
                    clip_number=clip_idx + 1,
                    step_scale=step_scale,
                    step_count=step_count,
                    note_steps=note_steps,
                    motion_steps=motion_steps,
                    pitchbend=pitchbend,
                    tone=clip_tone,
                ))

            project.tracks.append(TrackDump(
                track_number=track_idx + 1,
                settings=settings,
                track_tone=track_tone,
                clips=clips,
            ))

        if include_system:
            project.system_header = bytes(
                client._read_chunked(SYSTEM_HEADER_ADDRESS, SYSTEM_HEADER_SIZE)
            )
            project.system_block = bytes(
                client._read_chunked(SYSTEM_BLOCK_ADDRESS, SYSTEM_BLOCK_SIZE)
            )

        return project

    # ---- restore (memory → device) ---------------------------------------

    def load_into(self, client, force: bool = False) -> None:
        """Write every captured byte back to the device.

        Refuses to load a project whose model doesn't match the client's
        connected device, unless ``force=True``. Set ``force=True`` only
        if you know the address layouts are compatible (e.g. MC-707 ↔
        MC-101 might work for the sequencer area but the system block
        and tone descriptors may differ).
        """
        expected = {0x5D: "MC-707", 0x5E: "MC-101", 0x5F: "MV-1"}.get(
            client.model_id, f"0x{client.model_id:02X}"
        )
        if not force and self.model and self.model != expected:
            raise ValueError(
                f"project model {self.model!r} doesn't match the client's "
                f"model {expected!r}. Pass force=True to override (only safe "
                f"between MC-707 and MC-101 for the sequencer area; tone and "
                f"system bytes are model-specific)."
            )

        for track_dump in self.tracks:
            track_idx = track_dump.track_number - 1
            client._write_chunked(
                TRK_SETTINGS_BASE_ADDR[track_idx], list(track_dump.settings)
            )
            client.load_tone(track=track_dump.track_number, clip=None,
                             preset=track_dump.track_tone)

            for clip_dump in track_dump.clips:
                clip_idx = clip_dump.clip_number - 1
                clip_base = client.addresses.clip_base(track_idx, clip_idx)

                # Restore clip params (scale + count).
                client._write_chunked(
                    clip_base + SEQ_PARAMS_OFFS,
                    [
                        clip_dump.step_scale & 0x7F,
                        (clip_dump.step_count >> 4) & 0x0F,
                        clip_dump.step_count & 0x0F,
                        0x00,
                    ],
                )
                if clip_dump.note_steps:
                    client._write_chunked(clip_base + SEQ_NOTE_OFFS,
                                          list(clip_dump.note_steps))
                if clip_dump.motion_steps:
                    client._write_chunked(clip_base + SEQ_MOTION_OFFS,
                                          list(clip_dump.motion_steps))
                if clip_dump.pitchbend:
                    client._write_chunked(clip_base + SEQ_PITCHBEND_OFFS,
                                          list(clip_dump.pitchbend))

                client.load_tone(track=track_dump.track_number,
                                 clip=clip_dump.clip_number,
                                 preset=clip_dump.tone)

        if self.system_header:
            client._write_chunked(SYSTEM_HEADER_ADDRESS, list(self.system_header))
        if self.system_block:
            client._write_chunked(SYSTEM_BLOCK_ADDRESS, list(self.system_block))
