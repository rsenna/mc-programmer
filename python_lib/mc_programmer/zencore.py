"""Conventional ZEN-Core Tone parameter framework for MC-101 / MC-707 / MV-1.

The MC family stores its synth patches as **conventional ZEN-Core Tone**
descriptors — the PCM/sample-based half of the ``[ZCore]`` address block in
Roland's Jupiter-X/Xm MIDI Implementation. Per the ZC programming-video
transcript, the parameter offsets within a ZCore tone are the same across
every Roland ZEN-Core instrument; only the *base address* of each tone
descriptor differs from one product to the next, and that lives in
``mc_programmer.addresses.AddressMap``.

The conventional ZEN-Core Tone is composed of these substructures (the
SuperNATURAL Synth sub-block at ``[Synth Partial]`` is **not** part of this
path and is intentionally excluded):

==================  ============  ====================================
Substructure        Offset        Description
==================  ============  ====================================
Tone Common         ``0x0000``    Tone Name, Level, Coarse/Fine Tune,
                                  Octave Shift, Mono/Poly, Portamento,
                                  Bend Range, etc. (24 params)
Tone MFX            ``0x0100``    Multi-effect parameters (not in this
                                  framework yet)
Tone PMT            ``0x1000``    Partial Mix Table — per-partial
                                  velocity / key range / fade settings
                                  (38 params)
Tone Partial 1..4   ``0x2000`` +  Per-partial OSC, filter, amplifier,
                    ``n*0x100``   modulation matrix, etc. (88 params)
Partial Pitch Env   ``0x2400`` +  4 × pitch envelope (14 params each)
                    ``n*0x100``
Partial Filter Env  ``0x2800`` +  4 × filter envelope (14 params each)
                    ``n*0x100``
Partial Amp Env     ``0x2C00`` +  4 × amp envelope (9 params each)
                    ``n*0x100``
Partial LFO         ``0x3000`` +  4 × { LFO 1 + LFO 2 } (97 params each
                    ``n*0x200``   — LFO struct is 0x11E bytes, so the
                                  stride between adjacent partials is
                                  0x200 not 0x100)
Partial EQ          ``0x3800`` +  4 × 3-band EQ (8 params each)
                    ``n*0x100``
==================  ============  ====================================

This module provides the framework. The actual parameter table is
auto-generated from the Jupiter-X PDF — see ``zencore_data.py`` and the
generator at ``tools/gen_zencore_params.py``.

Provenance
----------

* The ``Tone Coarse Tune`` parameter at offset ``0x18`` of Tone Common is
  **directly verified** by the parent repository's ``zen_core_tools.py``
  test script and demonstrated live in the ZC programming video.
* All other entries come straight from Roland's Jupiter-X/Xm MIDI
  Implementation v1.06 (Apr 2022), section ``[ZCore]``. The ZC video
  states that the parameter map is shared across Roland ZEN-Core
  instruments.

How to read or write a parameter
--------------------------------

Use the high-level helpers on :class:`mc_programmer.MCClient`::

    client.read_zc_param(trk_idx=1, clip_idx=None, name="tone_coarse_tune")
    client.write_zc_param(trk_idx=1, clip_idx=None,
                          name="tvf_cutoff_frequency", value=512,
                          group=ParamGroup.PARTIAL, partial_idx=1)
"""
from __future__ import annotations

from dataclasses import dataclass
from typing import Dict, List, Sequence


# ---------------------------------------------------------------------------
# Substructure offsets within a single conventional ZEN-Core Tone descriptor
# ---------------------------------------------------------------------------

#: Tone Common — verified at offset 0 by the parent repo's test scripts.
TONE_COMMON_OFFS = 0x0000

#: Tone MFX (multi-effect). From Jupiter-X [ZCore] map; not yet exposed.
TONE_MFX_OFFS = 0x0100

#: Tone PMT (Partial Mix Table). From Jupiter-X [ZCore] map.
TONE_PMT_OFFS = 0x1000

#: Per-substructure base addresses + strides within a tone descriptor.
#: Each entry is ``(base_offset, stride_per_partial)``.
PARTIAL_BASE_OFFS = 0x2000
PARTIAL_STRIDE = 0x0100

PARTIAL_PITCH_ENV_BASE = 0x2400
PARTIAL_PITCH_ENV_STRIDE = 0x0100

PARTIAL_FILTER_ENV_BASE = 0x2800
PARTIAL_FILTER_ENV_STRIDE = 0x0100

PARTIAL_AMP_ENV_BASE = 0x2C00
PARTIAL_AMP_ENV_STRIDE = 0x0100

#: Partial LFO has a *bigger* stride because the LFO struct is 0x11E bytes
#: (almost 286 bytes) — it carries both LFO 1 and LFO 2 in one block.
PARTIAL_LFO_BASE = 0x3000
PARTIAL_LFO_STRIDE = 0x0200

PARTIAL_EQ_BASE = 0x3800
PARTIAL_EQ_STRIDE = 0x0100

#: SuperNATURAL synth half of the ZCore tone descriptor. The MC-707
#: Reference Manual page 62–63 (STRUCTURE / OSC MOD COM / UNISON / OSC
#: categories) documents these as part of the Z-Core Tone, so they're
#: present and active alongside the conventional Tone Common/PMT/Partial
#: substructures above.
TONE_SYNTH_COMMON_OFFS = 0x3C00
TONE_SYNTH_PMT_OFFS = 0x3D00
TONE_SYNTH_PARTIAL_BASE = 0x3E00
TONE_SYNTH_PARTIAL_STRIDE = 0x0100

#: Number of partials in a conventional ZEN-Core Tone (Roland standard).
NUM_PARTIALS = 4


def partial_offset(
    partial_idx: int,
    base: int = PARTIAL_BASE_OFFS,
    stride: int = PARTIAL_STRIDE,
) -> int:
    """Offset of partial *N* (1..4) within a tone descriptor.

    By default returns the offset of ``Tone Partial N`` itself; pass an
    explicit ``base`` and ``stride`` to compute the address of one of the
    per-partial sub-substructures (Pitch Env, Filter Env, Amp Env, LFO, EQ).
    """
    if not 1 <= partial_idx <= NUM_PARTIALS:
        raise ValueError(f"partial index out of range: {partial_idx}")
    return base + (partial_idx - 1) * stride


# ---------------------------------------------------------------------------
# ZCParam — describes one parameter and how to encode / decode it
# ---------------------------------------------------------------------------

@dataclass(frozen=True)
class ZCParam:
    """Description of a single ZEN-Core parameter.

    ``size`` is the number of *wire* bytes in RQ1/DT1 messages, not the
    parameter's logical width:

    * ``size == 1``  — a single 7-bit byte (0..127), used for most simple
      parameters such as Tone Level, Coarse Tune, Mono/Poly, etc.
    * ``size == 2``  — an 8-bit value packed across two bytes, four bits
      each: value ``0xAB`` → wire bytes ``0x0A 0x0B``. Roland uses this
      for many ``-100..+100`` style signed offsets, encoded as the
      unsigned wire range ``28..228``.
    * ``size == 4``  — a 16-bit value packed across four bytes, four bits
      each: value ``0x1234`` → wire bytes ``0x01 0x02 0x03 0x04``. Used
      for 0..1023 envelope-time values, 0..16383 wave numbers, etc.

    For ASCII fields like Tone Name, ``size`` is the number of characters
    (e.g. 16) and the runtime treats every byte as one ASCII character.
    """

    name: str               # machine name (snake_case)
    offset: int             # offset within the containing substructure
    size: int               # bytes on the wire
    min_value: int = 0
    max_value: int = 127
    default: int = 0
    label: str = ""         # human-readable name from the Roland doc
    ascii: bool = False     # True for variable-length ASCII fields
    signed: bool = False    # True for signed two's-complement payloads
    notes: str = ""         # original Roland row text, kept for display

    def __post_init__(self):
        # Heuristic: any 12+ byte field whose range is 32..127 is an
        # ASCII string (Tone Name etc.).
        if self.size >= 8 and self.min_value == 32 and self.max_value == 127 and not self.ascii:
            object.__setattr__(self, "ascii", True)

    # ---- encoding ---------------------------------------------------------

    def encode(self, value) -> List[int]:
        """Encode ``value`` into the byte list that goes inside a DT1 packet."""
        if self.ascii:
            return self._encode_ascii(value)

        v = int(value)
        self._check(v)

        if self.size == 1:
            return [v & 0x7F]
        if self.size == 2:
            return [(v >> 4) & 0x0F, v & 0x0F]
        if self.size == 4:
            return [
                (v >> 12) & 0x0F,
                (v >> 8) & 0x0F,
                (v >> 4) & 0x0F,
                v & 0x0F,
            ]
        raise ValueError(f"unsupported ZCParam wire size: {self.size}")

    def decode(self, raw: Sequence[int]):
        """Decode ``raw`` (the data bytes from a DT1 reply) back into a value."""
        if self.ascii:
            return self._decode_ascii(raw)

        if self.size == 1:
            return raw[0] & 0x7F
        if self.size == 2:
            return ((raw[0] & 0x0F) << 4) | (raw[1] & 0x0F)
        if self.size == 4:
            return (
                ((raw[0] & 0x0F) << 12)
                | ((raw[1] & 0x0F) << 8)
                | ((raw[2] & 0x0F) << 4)
                | (raw[3] & 0x0F)
            )
        raise ValueError(f"unsupported ZCParam wire size: {self.size}")

    # ---- ASCII helpers ----------------------------------------------------

    def _encode_ascii(self, value: str) -> List[int]:
        if not isinstance(value, str):
            raise TypeError(f"{self.name}: expected str, got {type(value).__name__}")
        encoded = [ord(c) & 0x7F for c in value[: self.size]]
        encoded += [0x20] * (self.size - len(encoded))   # right-pad with spaces
        return [b if 0x20 <= b < 0x7F else ord("?") for b in encoded]

    def _decode_ascii(self, raw: Sequence[int]) -> str:
        chars = []
        for b in list(raw)[: self.size]:
            chars.append(chr(b) if 0x20 <= b < 0x7F else " ")
        return "".join(chars).rstrip()

    def _check(self, v: int) -> None:
        if not self.min_value <= v <= self.max_value:
            raise ValueError(
                f"{self.name}: value {v} out of range "
                f"({self.min_value}..{self.max_value})"
            )


# ---------------------------------------------------------------------------
# Parameter table loader
# ---------------------------------------------------------------------------

# The actual tables live in zencore_data.py (auto-generated). Imported here
# at module load time so callers can use ``mc_programmer.TONE_COMMON_PARAMS`` etc.
# directly without needing to know about the data file.
from . import zencore_data as _data  # noqa: E402

TONE_COMMON_PARAMS:        Dict[str, ZCParam] = _data.TONE_COMMON
TONE_PMT_PARAMS:           Dict[str, ZCParam] = _data.TONE_PMT
TONE_PARTIAL_PARAMS:       Dict[str, ZCParam] = _data.TONE_PARTIAL
PARTIAL_PITCH_ENV_PARAMS:  Dict[str, ZCParam] = _data.PARTIAL_PITCH_ENV
PARTIAL_FILTER_ENV_PARAMS: Dict[str, ZCParam] = _data.PARTIAL_FILTER_ENV
PARTIAL_AMP_ENV_PARAMS:    Dict[str, ZCParam] = _data.PARTIAL_AMP_ENV
PARTIAL_LFO_PARAMS:        Dict[str, ZCParam] = _data.PARTIAL_LFO
PARTIAL_EQ_PARAMS:         Dict[str, ZCParam] = _data.PARTIAL_EQ
TONE_SYNTH_COMMON_PARAMS:  Dict[str, ZCParam] = _data.TONE_SYNTH_COMMON
TONE_SYNTH_PMT_PARAMS:     Dict[str, ZCParam] = _data.TONE_SYNTH_PMT
TONE_SYNTH_PARTIAL_PARAMS: Dict[str, ZCParam] = _data.TONE_SYNTH_PARTIAL
TONE_MFX_PARAMS:           Dict[str, ZCParam] = _data.TONE_MFX


# ---------------------------------------------------------------------------
# Group lookup
# ---------------------------------------------------------------------------

class ParamGroup:
    """Identifies which substructure a parameter lives in.

    For partial groups, also pass ``partial_idx`` (1..4) to the lookup
    helpers.
    """

    COMMON = "common"
    PMT = "pmt"
    PARTIAL = "partial"
    PARTIAL_PITCH_ENV = "partial_pitch_env"
    PARTIAL_FILTER_ENV = "partial_filter_env"
    PARTIAL_AMP_ENV = "partial_amp_env"
    PARTIAL_LFO = "partial_lfo"
    PARTIAL_EQ = "partial_eq"
    SYNTH_COMMON = "synth_common"
    SYNTH_PMT = "synth_pmt"
    SYNTH_PARTIAL = "synth_partial"
    MFX = "mfx"


_GROUP_TABLES: Dict[str, Dict[str, ZCParam]] = {
    ParamGroup.COMMON:             TONE_COMMON_PARAMS,
    ParamGroup.PMT:                TONE_PMT_PARAMS,
    ParamGroup.PARTIAL:            TONE_PARTIAL_PARAMS,
    ParamGroup.PARTIAL_PITCH_ENV:  PARTIAL_PITCH_ENV_PARAMS,
    ParamGroup.PARTIAL_FILTER_ENV: PARTIAL_FILTER_ENV_PARAMS,
    ParamGroup.PARTIAL_AMP_ENV:    PARTIAL_AMP_ENV_PARAMS,
    ParamGroup.PARTIAL_LFO:        PARTIAL_LFO_PARAMS,
    ParamGroup.PARTIAL_EQ:         PARTIAL_EQ_PARAMS,
    ParamGroup.SYNTH_COMMON:       TONE_SYNTH_COMMON_PARAMS,
    ParamGroup.SYNTH_PMT:          TONE_SYNTH_PMT_PARAMS,
    ParamGroup.SYNTH_PARTIAL:      TONE_SYNTH_PARTIAL_PARAMS,
    ParamGroup.MFX:                TONE_MFX_PARAMS,
}

# (base, stride) for each per-partial group.
_GROUP_PARTIAL_LAYOUT: Dict[str, tuple] = {
    ParamGroup.PARTIAL:            (PARTIAL_BASE_OFFS,         PARTIAL_STRIDE),
    ParamGroup.PARTIAL_PITCH_ENV:  (PARTIAL_PITCH_ENV_BASE,    PARTIAL_PITCH_ENV_STRIDE),
    ParamGroup.PARTIAL_FILTER_ENV: (PARTIAL_FILTER_ENV_BASE,   PARTIAL_FILTER_ENV_STRIDE),
    ParamGroup.PARTIAL_AMP_ENV:    (PARTIAL_AMP_ENV_BASE,      PARTIAL_AMP_ENV_STRIDE),
    ParamGroup.PARTIAL_LFO:        (PARTIAL_LFO_BASE,          PARTIAL_LFO_STRIDE),
    ParamGroup.PARTIAL_EQ:         (PARTIAL_EQ_BASE,           PARTIAL_EQ_STRIDE),
}


def lookup_param(group: str, name: str) -> ZCParam:
    """Find a parameter by group + machine name.

    On a missing name, suggests close matches via difflib so typos
    surface as actionable errors instead of opaque KeyErrors.
    """
    table = _GROUP_TABLES.get(group)
    if table is None:
        groups = ", ".join(sorted(_GROUP_TABLES))
        raise KeyError(
            f"unknown ZEN-Core parameter group: {group!r}. Known groups: {groups}"
        )
    if name not in table:
        import difflib
        suggestion = difflib.get_close_matches(name, list(table), n=3, cutoff=0.5)
        hint = ""
        if suggestion:
            hint = f" Did you mean {', '.join(repr(s) for s in suggestion)}?"
        raise KeyError(f"unknown {group} parameter: {name!r}.{hint}")
    return table[name]


def substructure_offset(group: str, partial_idx: int = 1) -> int:
    """Offset of a substructure relative to the tone descriptor base."""
    if group == ParamGroup.COMMON:
        return TONE_COMMON_OFFS
    if group == ParamGroup.PMT:
        return TONE_PMT_OFFS
    if group == ParamGroup.MFX:
        return TONE_MFX_OFFS
    if group == ParamGroup.SYNTH_COMMON:
        return TONE_SYNTH_COMMON_OFFS
    if group == ParamGroup.SYNTH_PMT:
        return TONE_SYNTH_PMT_OFFS
    if group == ParamGroup.SYNTH_PARTIAL:
        return partial_offset(
            partial_idx, base=TONE_SYNTH_PARTIAL_BASE,
            stride=TONE_SYNTH_PARTIAL_STRIDE,
        )
    layout = _GROUP_PARTIAL_LAYOUT.get(group)
    if layout is None:
        raise KeyError(f"unknown ZEN-Core parameter group: {group}")
    base, stride = layout
    return partial_offset(partial_idx, base=base, stride=stride)
