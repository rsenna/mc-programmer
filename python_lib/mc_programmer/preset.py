"""TonePreset — capture and restore one ZEN-Core tone from an MC.

This is the per-tone layer. It dumps every byte of the conventional
ZEN-Core Tone substructures (Common + PMT + Partial 1-4 + Partial Pitch
Env × 4 + Partial Filter Env × 4 + Partial Amp Env × 4 + Partial LFO × 4
+ Partial EQ × 4) for one track or clip tone, and can write the same
bytes back to any other track or clip slot — even on a different MC.

Example::

    # Capture the active tone of track 1.
    preset = client.dump_tone(track=1)
    preset.save("my_bass.tone")

    # Later, load it onto track 3, clip 2.
    preset = TonePreset.load("my_bass.tone")
    client.load_tone(track=3, clip=2, preset=preset)

The file format is plain JSON with hex byte strings. It's human-readable,
diffable, and tagged with the model ID so you can't accidentally flash a
707 tone onto a 101.

For full-project dump/restore (every track + every clip + every pattern
in one file), see :class:`mc_programmer.project.Project`.
"""
from __future__ import annotations

import json
from dataclasses import dataclass, field
from typing import List, Optional

from .zencore_data import SECTION_TOTAL_SIZE


PRESET_FORMAT = "mc_programmer.tone.v1"


@dataclass
class ToneRegion:
    """One contiguous chunk of a tone descriptor.

    ``offset`` is relative to the tone descriptor's base address. ``label``
    is the substructure name (``"tone_common"``, ``"partial_1"``, …) and
    is informational only — what matters for round-tripping is ``offset``
    and ``data``.
    """

    offset: int
    label: str
    data: bytes

    def to_dict(self) -> dict:
        return {
            "offset": f"0x{self.offset:04X}",
            "label": self.label,
            "data": " ".join(f"{b:02X}" for b in self.data),
        }

    @classmethod
    def from_dict(cls, d: dict) -> "ToneRegion":
        return cls(
            offset=int(d["offset"], 0),
            label=d["label"],
            data=bytes(int(b, 16) for b in d["data"].split()),
        )


@dataclass
class TonePreset:
    """A captured ZEN-Core tone — every substructure as a byte blob.

    Round-trip safe: ``dump_tone`` → ``save`` → ``load`` → ``load_tone``
    rewrites the device to byte-for-byte the same state.
    """

    model: str                          # e.g. "MC-707" — used to refuse cross-model loads
    name: Optional[str] = None          # human-readable, defaults to the device's tone name
    regions: List[ToneRegion] = field(default_factory=list)

    def to_json(self) -> str:
        return json.dumps(
            {
                "format": PRESET_FORMAT,
                "model": self.model,
                "name": self.name,
                "regions": [r.to_dict() for r in self.regions],
            },
            indent=2,
        )

    @classmethod
    def from_json(cls, raw: str) -> "TonePreset":
        d = json.loads(raw)
        if d.get("format") != PRESET_FORMAT:
            raise ValueError(
                f"not an mc_programmer tone preset (format={d.get('format')!r})"
            )
        return cls(
            model=d["model"],
            name=d.get("name"),
            regions=[ToneRegion.from_dict(r) for r in d["regions"]],
        )

    def save(self, path: str) -> None:
        with open(path, "w") as f:
            f.write(self.to_json())

    @classmethod
    def load(cls, path: str) -> "TonePreset":
        with open(path) as f:
            return cls.from_json(f.read())


# ---------------------------------------------------------------------------
# Region table — describes which substructures get dumped, and where each
# one lives within the tone descriptor (relative to the tone base address).
#
# These offsets come from the [ZCore] address map in Roland's Jupiter-X/Xm
# MIDI Implementation v1.06, page 8. The MC family uses the conventional
# ZEN-Core Tone path so the same offsets apply (the ZC programming-video
# transcript states this explicitly).
# ---------------------------------------------------------------------------

# (label, offset_within_tone, size, total_size_key)
# size is taken from the auto-generated SECTION_TOTAL_SIZE table where
# possible, and hard-coded as 0x100 for Tone Partial because that's the
# fixed Roland-defined record size.

_TONE_REGIONS_SPEC = [
    # 1 Tone Common at 0x0000
    ("tone_common",       0x0000, "TONE_COMMON",       0x0100),
    # 1 Tone MFX at 0x0100 (272 bytes — full Sys-Ex MFX block)
    ("tone_mfx",          0x0100, "TONE_MFX",          0x0200),
    # 1 Tone PMT at 0x1000
    ("tone_pmt",          0x1000, "TONE_PMT",          0x0100),
]


def _build_per_partial_specs() -> list:
    """Generate the per-partial region specs (4 partials × 5 substructures)."""
    out = []
    for n in range(1, 5):
        out.append((f"partial_{n}",            0x2000 + (n-1)*0x100, "TONE_PARTIAL",       0x0100))
    for n in range(1, 5):
        out.append((f"partial_pitch_env_{n}",  0x2400 + (n-1)*0x100, "PARTIAL_PITCH_ENV",  0x0100))
    for n in range(1, 5):
        out.append((f"partial_filter_env_{n}", 0x2800 + (n-1)*0x100, "PARTIAL_FILTER_ENV", 0x0100))
    for n in range(1, 5):
        out.append((f"partial_amp_env_{n}",    0x2C00 + (n-1)*0x100, "PARTIAL_AMP_ENV",    0x0100))
    # Partial LFO has a larger stride (0x200) because the struct is 0x11E bytes.
    for n in range(1, 5):
        out.append((f"partial_lfo_{n}",        0x3000 + (n-1)*0x200, "PARTIAL_LFO",        0x0200))
    for n in range(1, 5):
        out.append((f"partial_eq_{n}",         0x3800 + (n-1)*0x100, "PARTIAL_EQ",         0x0100))

    # SuperNATURAL synth half of the ZCore tone descriptor — the MC's
    # OSC type / structure / cross-mod / unison parameters live here.
    out.append(("tone_synth_common",           0x3C00, "TONE_SYNTH_COMMON", 0x0100))
    out.append(("tone_synth_pmt",              0x3D00, "TONE_SYNTH_PMT",    0x0100))
    for n in range(1, 5):
        out.append((f"tone_synth_partial_{n}", 0x3E00 + (n-1)*0x100, "TONE_SYNTH_PARTIAL", 0x0100))
    return out


TONE_REGIONS_SPEC = _TONE_REGIONS_SPEC + _build_per_partial_specs()


def region_size(section_key: str) -> int:
    """Return the size in bytes of one substructure (from the auto-generated table)."""
    return SECTION_TOTAL_SIZE[section_key]
