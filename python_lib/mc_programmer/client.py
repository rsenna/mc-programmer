"""High-level client that ties a :class:`Transport` to the rest of the library.

This is the typical entry point for application code:

    from mc_programmer import MCClient, ModelID
    client = MCClient(my_transport, model_id=ModelID.MC_707)
    step = client.read_note_step(trk_idx=0, clip_idx=0, step_idx=4)
    step.components[0].note = 60  # C4
    client.write_note_step(0, 0, 4, step)

The methods on this class are deliberately thin — they just compute the
right address via :class:`AddressMap`, build a Sys-Ex message via
``protocol``, and call ``transport.transfer``.
"""
from __future__ import annotations

from typing import List, Optional, Sequence

from .addresses import (
    AddressMap,
    StepScale,
    TONE_COMMON_COARSE_TUNE_OFFS,
    STEP_SIZE,
)
from .protocol import (
    ModelID,
    build_dt1,
    build_rq1,
    parse_dt1_payload,
    universal_device_inquiry,
)
from .sequencer import (
    NoteComponent,
    NoteStep,
    decode_note_step,
    encode_note_step,
)
from .transport import Transport
from .zencore import (
    _GROUP_TABLES,
    ParamGroup,
    ZCParam,
    lookup_param,
    substructure_offset,
)
from .notes import note as _parse_note, note_name


def _suggest(name: str, candidates) -> str:
    import difflib
    matches = difflib.get_close_matches(name, list(candidates), n=3, cutoff=0.5)
    if not matches:
        return ""
    if len(matches) == 1:
        return f" Did you mean {matches[0]!r}?"
    return f" Did you mean one of: {', '.join(repr(m) for m in matches)}?"


class MCClient:
    def __init__(
        self,
        transport: Transport,
        model_id: Optional[int] = None,
        addresses: Optional[AddressMap] = None,
    ) -> None:
        self.transport = transport
        self.addresses = addresses or AddressMap.build()
        self.model_id = int(model_id) if model_id is not None else self.detect_model_id()

    # ---- low-level RQ1 / DT1 ----------------------------------------------

    def detect_model_id(self) -> int:
        reply = self.transport.transfer(universal_device_inquiry(), timeout=0.2)
        if not reply or len(reply) < 7:
            raise RuntimeError("no response to universal device inquiry")
        return reply[6]

    def read(self, address: int, count: int, timeout: float = 0.1) -> List[int]:
        msg = build_rq1(self.model_id, address, count)
        reply = self.transport.transfer(msg, timeout=timeout)
        if not reply:
            return []
        return parse_dt1_payload(reply).data

    def write(self, address: int, data: Sequence[int]) -> None:
        msg = build_dt1(self.model_id, address, data)
        self.transport.transfer(msg, timeout=0.05)

    def read_param(self, address: int, size: int, timeout: float = 0.1) -> int:
        """Read a multi-byte parameter, MSB-first nibble-packed."""
        data = self.read(address, size, timeout=timeout)
        value = 0
        for i, b in enumerate(data):
            value |= (b & 0x0F) << (4 * (len(data) - i - 1))
        return value

    def write_param(self, address: int, value: int, size: int) -> None:
        if size == 1:
            self.write(address, [value & 0x7F])
            return
        nibbles = [(value >> (4 * (size - i - 1))) & 0x0F for i in range(size)]
        self.write(address, nibbles)

    # ---- sequencer ---------------------------------------------------------

    def read_note_step(self, trk_idx: int, clip_idx: int, step_idx: int) -> NoteStep:
        addr = self.addresses.note_step_address(trk_idx, clip_idx, step_idx)
        raw = self.read(addr, STEP_SIZE)
        return decode_note_step(raw)

    def write_note_step(
        self, trk_idx: int, clip_idx: int, step_idx: int, step: NoteStep
    ) -> None:
        addr = self.addresses.note_step_address(trk_idx, clip_idx, step_idx)
        self.write(addr, list(encode_note_step(step)))

    def read_clip_step_params(self, trk_idx: int, clip_idx: int) -> tuple[StepScale, int]:
        """Return ``(step_scale, step_count)`` for the given clip.

        Per the MC video, step length is encoded as two nibble bytes — e.g.
        ``0x08 0x00`` decodes to 0x80 = 128.
        """
        addr = self.addresses.clip_params_address(trk_idx, clip_idx)
        data = self.read(addr, 4)
        if len(data) < 3:
            return StepScale.SIXTEENTH, 16
        return StepScale(data[0]), (data[1] << 4) | data[2]

    # ---- ZEN-Core ----------------------------------------------------------

    def read_coarse_tune(self, trk_idx: int, clip_idx: Optional[int] = None) -> int:
        addr = self.addresses.zencore_tone_address(trk_idx, clip_idx) + TONE_COMMON_COARSE_TUNE_OFFS
        data = self.read(addr, 1)
        return data[0] if data else 0

    def write_coarse_tune(
        self, trk_idx: int, clip_idx: Optional[int], value: int
    ) -> None:
        # MC range is 16..112 with 64 = center, per ZC video / zen_core_tools.
        value = max(16, min(112, value))
        addr = self.addresses.zencore_tone_address(trk_idx, clip_idx) + TONE_COMMON_COARSE_TUNE_OFFS
        self.write(addr, [value])

    def nudge_coarse_tune(
        self, trk_idx: int, clip_idx: Optional[int], step: int
    ) -> int:
        """Read-modify-write helper matching ``coarse_tune_rmw`` from the scripts."""
        new_value = self.read_coarse_tune(trk_idx, clip_idx) + step
        self.write_coarse_tune(trk_idx, clip_idx, new_value)
        return new_value

    # ---- ZEN-Core: generic parameter access -------------------------------

    def _zc_param_address(
        self,
        trk_idx: int,
        clip_idx: Optional[int],
        group: str,
        param: ZCParam,
        partial_idx: int = 1,
    ) -> int:
        return (
            self.addresses.zencore_tone_address(trk_idx, clip_idx)
            + substructure_offset(group, partial_idx)
            + param.offset
        )

    def read_zc_param(
        self,
        trk_idx: int,
        clip_idx: Optional[int],
        name: str,
        group: str = ParamGroup.COMMON,
        partial_idx: int = 1,
    ):
        """Read any registered ZEN-Core parameter by name.

        ``group`` is one of ``ParamGroup.COMMON``, ``ParamGroup.PMT`` or
        ``ParamGroup.PARTIAL``. For ``PARTIAL``, also pass ``partial_idx``
        in 1..4. Returns whatever the parameter's ``decode`` method
        produces — usually an int, or a string for ASCII fields.
        """
        param = lookup_param(group, name)
        addr = self._zc_param_address(trk_idx, clip_idx, group, param, partial_idx)
        data = self.read(addr, param.size)
        if not data:
            return None
        return param.decode(data)

    def write_zc_param(
        self,
        trk_idx: int,
        clip_idx: Optional[int],
        name: str,
        value,
        group: str = ParamGroup.COMMON,
        partial_idx: int = 1,
    ) -> None:
        """Write any registered ZEN-Core parameter by name."""
        param = lookup_param(group, name)
        addr = self._zc_param_address(trk_idx, clip_idx, group, param, partial_idx)
        self.write(addr, param.encode(value))

    # convenience helpers built on the generic accessors
    def read_tone_name(self, trk_idx: int, clip_idx: Optional[int] = None) -> str:
        return self.read_zc_param(trk_idx, clip_idx, "name")

    def write_tone_name(
        self, trk_idx: int, clip_idx: Optional[int], name: str
    ) -> None:
        self.write_zc_param(trk_idx, clip_idx, "name", name)

    # =====================================================================
    # HIGH-LEVEL HELPERS — front-panel numbering
    # =====================================================================
    #
    # The methods in this section accept ``track`` / ``clip`` / ``step``
    # parameters that match the **MC's front-panel display exactly**:
    # tracks 1..8, clips 1..16, steps 1..128. So:
    #
    #     client.place_note(track=1, clip=1, step=1, pitch="C1")
    #
    # places a kick on the first step of the first clip on the first track,
    # which is what the device shows as "TRACK 1 / CLIP 1 / STEP 1".
    #
    # The lower-level primitives above (``read_note_step``, ``write_zc_param``,
    # …) still take ``trk_idx`` / ``clip_idx`` / ``step_idx`` arguments that
    # are 0-based — the ``_idx`` suffix is the discriminator. Use whichever
    # layer matches how you're thinking about the problem.
    #
    # Notes can be passed as either MIDI numbers (``60``) or names
    # (``"C3"``, ``"F#1"``); the parser at :func:`mc_programmer.notes.note` handles
    # both transparently. The MC family follows the Roland octave
    # convention where **C3 = MIDI 60**, so the kick drum is at **C1 = 36**.
    #
    # Coarse tune is exposed in semitones (-48..+48). 0 semitones = the
    # device's center value of 64.

    #: How many tracks each model exposes on its front panel.
    #: MC-101 has 4 tracks; MC-707 / MV-1 have 8.
    _MODEL_TRACK_COUNT = {
        0x5D: 8,   # MC-707
        0x5E: 4,   # MC-101
        0x5F: 8,   # MV-1
    }

    @property
    def num_tracks(self) -> int:
        """Number of physical tracks on the connected device (4 or 8)."""
        return self._MODEL_TRACK_COUNT.get(self.model_id, 8)

    def _track_to_idx(self, track: int) -> int:
        max_track = self.num_tracks
        if not 1 <= track <= max_track:
            model_str = {0x5D: "MC-707", 0x5E: "MC-101", 0x5F: "MV-1"}.get(
                self.model_id, "your device"
            )
            raise ValueError(
                f"track={track} doesn't exist — {model_str} has tracks 1..{max_track} "
                f"(matching the front-panel labels)"
            )
        return track - 1

    @staticmethod
    def _clip_to_idx(clip: int) -> int:
        if not 1 <= clip <= 16:
            raise ValueError(
                f"clip={clip} doesn't exist — each track has clips 1..16"
            )
        return clip - 1

    @staticmethod
    def _step_to_idx(step: int) -> int:
        if not 1 <= step <= 128:
            raise ValueError(
                f"step={step} doesn't exist — each clip has steps 1..128"
            )
        return step - 1

    # ---- pattern programming ---------------------------------------------

    def place_note(
        self,
        track: int,
        clip: int,
        step: int,
        pitch,
        velocity: int = 100,
        length_steps: float = 0.8,
    ) -> None:
        """Add one note to a step, packing it into the lowest free lane.

        ``track``, ``clip`` and ``step`` are **1-based**, matching the
        labels on the MC's screen.

        ``pitch`` accepts a MIDI number (``60``) or a note name (``"C3"``,
        ``"F#1"``). The Roland MC family puts the kick drum at **C1 = 36**.

        ``length_steps`` is the note duration in *step units*. ``0.8`` is
        the default 80-cent length the MC writes when you punch in a note
        from the front panel. ``1.0`` is a full step. Values greater than
        1.0 automatically tie the note across the appropriate number of
        following steps.

        If the step already has 16 notes, the new one is silently dropped
        — the MC hardware itself only stores 16 concurrent notes per step.
        """
        ti = self._track_to_idx(track)
        ci = self._clip_to_idx(clip)
        si = self._step_to_idx(step)
        midi = _parse_note(pitch)

        existing = self.read_note_step(ti, ci, si)
        for comp in existing.components:
            if comp.is_empty:
                comp.note = midi
                comp.velocity = max(0, min(127, velocity))
                comp.start_cents = 0
                comp.end_cents = 100 if length_steps > 1.0 else int(round(length_steps * 100))
                self.write_note_step(ti, ci, si, existing)
                break
        else:
            return  # all 16 lanes full — silently drop, like the hardware does

        # If the note ties into following steps, lay down the continuations.
        remaining = length_steps - 1.0
        cur = si + 1
        while remaining > 0 and cur < 128:
            tail = self.read_note_step(ti, ci, cur)
            for comp in tail.components:
                if comp.is_empty:
                    comp.note = midi
                    comp.velocity = max(0, min(127, velocity))
                    comp.start_cents = 0
                    comp.end_cents = 100 if remaining > 1.0 else int(round(remaining * 100))
                    self.write_note_step(ti, ci, cur, tail)
                    break
            remaining -= 1.0
            cur += 1

    def clear_step(self, track: int, clip: int, step: int) -> None:
        """Erase every note from one step. ``track``/``clip``/``step`` are 1-based."""
        ti = self._track_to_idx(track)
        ci = self._clip_to_idx(clip)
        si = self._step_to_idx(step)
        self.write_note_step(ti, ci, si, NoteStep())

    def clear_clip(self, track: int, clip: int) -> None:
        """Erase every step of a clip. ``track``/``clip`` are 1-based."""
        ti = self._track_to_idx(track)
        ci = self._clip_to_idx(clip)
        empty = NoteStep()
        for s in range(128):
            self.write_note_step(ti, ci, s, empty)

    def set_pattern(
        self,
        track: int,
        clip: int,
        notes,
        velocity: int = 100,
        clear_first: bool = True,
    ) -> None:
        """Program a whole pattern in one call.

        ``track`` and ``clip`` are **1-based**. ``notes`` is a mapping
        from a pitch (name or MIDI number) to a list of **1-based** step
        positions. ``velocity`` defaults to 100. If ``clear_first`` is
        True (the default) the clip is wiped before the new notes are
        written, so this is the safe "set this exact pattern" call.

        Example — a 4-on-the-floor kick + offbeat hat in 16 steps::

            client.set_pattern(track=1, clip=1, notes={
                "C1":  [1, 5, 9, 13],   # kick on every downbeat
                "F#1": [3, 7, 11, 15],  # hi-hat on the offbeats
            })
        """
        if clear_first:
            self.clear_clip(track, clip)
        for pitch, steps in notes.items():
            for step in steps:
                self.place_note(track, clip, step, pitch, velocity=velocity)

    # ---- coarse tune in semitones (musician units) -----------------------

    def read_coarse_tune_semitones(
        self, track: int, clip: Optional[int] = None
    ) -> int:
        """Coarse tune in semitones (-48..+48). 0 = no transposition.

        ``track``/``clip`` are 1-based. Pass ``clip=None`` for the
        track-wide tone (loaded when the track's sound source is set to
        TRACK), or 1..16 for one of the clip tones.
        """
        ti = self._track_to_idx(track)
        ci = self._clip_to_idx(clip) if clip is not None else None
        return self.read_coarse_tune(ti, ci) - 64

    def write_coarse_tune_semitones(
        self, track: int, clip: Optional[int], semitones: int
    ) -> None:
        """Set coarse tune in semitones (-48..+48). 0 = no transposition.

        ``track``/``clip`` are 1-based.
        """
        if not -48 <= semitones <= 48:
            raise ValueError(
                f"semitones={semitones} out of range — must be -48..+48 "
                f"(your MC's hardware limit)"
            )
        ti = self._track_to_idx(track)
        ci = self._clip_to_idx(clip) if clip is not None else None
        self.write_coarse_tune(ti, ci, 64 + semitones)

    # ---- discoverability --------------------------------------------------

    def list_parameters(self, group: Optional[str] = None) -> List[str]:
        """Return the names of every ZEN-Core parameter we know how to set.

        Pass ``group`` to filter (e.g. ``ParamGroup.COMMON`` for tone-wide
        params, or ``ParamGroup.PARTIAL`` for per-partial). With no
        argument, returns every name across every group.
        """
        if group is not None:
            if group not in _GROUP_TABLES:
                groups = ", ".join(sorted(_GROUP_TABLES))
                raise KeyError(f"unknown parameter group {group!r}. Known: {groups}")
            return sorted(_GROUP_TABLES[group].keys())
        seen = set()
        for table in _GROUP_TABLES.values():
            seen.update(table.keys())
        return sorted(seen)

    # ---- Scene / Clip selection via Program Change -----------------------
    #
    # Per the MC-707 / MC-101 MIDI Implementation Chart:
    #
    #   "Program Change (0-127) on the control channel selects a Scene.
    #    Program Change (0-15) on a track channel selects that track's Clip."
    #
    # The MC family does *not* use Bank Select for any kind of tone or
    # patch selection — only the front-panel scene/clip change is exposed
    # over MIDI. There is no MIDI message that loads an arbitrary preset
    # tone into a track.

    def select_scene(self, scene_number: int, control_channel: int = 1) -> None:
        """Switch the MC to a stored scene via Program Change.

        ``scene_number`` is 1-based (1..128). ``control_channel`` is the
        MIDI channel that the MC's system menu has set as the *control*
        channel — usually 1, but you can change it on the device. Pass
        whatever your MC is configured to.
        """
        if not 1 <= scene_number <= 128:
            raise ValueError(
                f"scene_number={scene_number} out of range — must be 1..128"
            )
        if not 1 <= control_channel <= 16:
            raise ValueError(
                f"control_channel={control_channel} out of range — must be 1..16"
            )
        status = 0xC0 | ((control_channel - 1) & 0x0F)
        self.transport.send_raw([status, scene_number - 1])

    def select_clip(
        self,
        track: int,
        clip_number: int,
        track_channel: Optional[int] = None,
    ) -> None:
        """Trigger a stored clip on a track via Program Change.

        ``track`` and ``clip_number`` are 1-based (1..8 and 1..16). The
        message goes out on ``track_channel`` if you pass it; otherwise
        the convention "track N → MIDI channel N" is used (which is the
        MC's default mapping — change it in the MC's system menu and
        pass an explicit channel here if you've remapped).
        """
        if not 1 <= clip_number <= 16:
            raise ValueError(
                f"clip_number={clip_number} out of range — must be 1..16"
            )
        ti = self._track_to_idx(track)
        channel = track_channel if track_channel is not None else (ti + 1)
        if not 1 <= channel <= 16:
            raise ValueError(
                f"track_channel={channel} out of range — must be 1..16"
            )
        status = 0xC0 | ((channel - 1) & 0x0F)
        self.transport.send_raw([status, clip_number - 1])

    # =====================================================================
    # Tone preset dump / restore
    # =====================================================================
    #
    # ``dump_tone`` reads every substructure of one ZEN-Core tone descriptor
    # (Common + PMT + 4 × Partial + 4 × Pitch/Filter/Amp Env + 4 × LFO + 4 × EQ)
    # and packages the bytes into a :class:`mc_programmer.preset.TonePreset`.
    # ``load_tone`` is the inverse — it writes those bytes back to *any*
    # tone slot, even on a different MC.
    #
    # The MC's per-request size limit (~128 bytes for both reads and writes)
    # means anything larger than that gets transparently chunked.

    _CHUNK_SIZE = 0x80   # safe upper bound on a single RQ1/DT1 payload

    def _read_chunked(self, address: int, count: int) -> List[int]:
        out: List[int] = []
        offset = 0
        while offset < count:
            n = min(self._CHUNK_SIZE, count - offset)
            chunk = self.read(address + offset, n)
            if not chunk:
                # Device returned nothing — pad with zeros so the caller
                # gets a constant-length result and can detect missing data.
                chunk = [0] * n
            out.extend(chunk)
            offset += n
        return out

    def _write_chunked(self, address: int, data) -> None:
        data = list(data)
        offset = 0
        while offset < len(data):
            n = min(self._CHUNK_SIZE, len(data) - offset)
            self.write(address + offset, data[offset:offset + n])
            offset += n

    def dump_tone(self, track: int, clip: Optional[int] = None,
                  name: Optional[str] = None) -> "TonePreset":
        """Capture every byte of one ZEN-Core tone into a :class:`TonePreset`.

        ``track``/``clip`` are 1-based. Pass ``clip=None`` to dump the
        track-wide tone (loaded when the track's sound source is TRACK).

        Optionally pass ``name`` to label the preset; otherwise the
        device's own Tone Name is used.
        """
        from .preset import TonePreset, ToneRegion, TONE_REGIONS_SPEC, region_size

        ti = self._track_to_idx(track)
        ci = self._clip_to_idx(clip) if clip is not None else None
        base = self.addresses.zencore_tone_address(ti, ci)

        regions: List[ToneRegion] = []
        for label, offset, section_key, _slot in TONE_REGIONS_SPEC:
            size = region_size(section_key)
            data = self._read_chunked(base + offset, size)
            regions.append(ToneRegion(offset=offset, label=label, data=bytes(data)))

        if name is None:
            try:
                name = self.read_tone_name(ti, ci)
            except Exception:
                name = None

        # Map model id back to a friendly string for the file header.
        model_str = {0x5D: "MC-707", 0x5E: "MC-101", 0x5F: "MV-1"}.get(
            self.model_id, f"0x{self.model_id:02X}"
        )
        return TonePreset(model=model_str, name=name, regions=regions)

    def load_tone(self, track: int, clip: Optional[int], preset: "TonePreset") -> None:
        """Write a :class:`TonePreset` into a tone slot.

        ``track``/``clip`` are 1-based. Pass ``clip=None`` for the
        track-wide tone slot.

        The preset's ``model`` field is checked against this client's
        model ID; loading a 707 preset onto a 101 (or vice versa) raises
        a ``ValueError`` unless the preset has no model tag.
        """
        from .preset import TonePreset  # noqa: F401  (typing)

        expected = {0x5D: "MC-707", 0x5E: "MC-101", 0x5F: "MV-1"}.get(
            self.model_id, f"0x{self.model_id:02X}"
        )
        if preset.model and preset.model != expected:
            raise ValueError(
                f"preset model {preset.model!r} doesn't match this client's "
                f"model {expected!r}. Pass a matching client or capture the "
                f"preset on the same model."
            )

        ti = self._track_to_idx(track)
        ci = self._clip_to_idx(clip) if clip is not None else None
        base = self.addresses.zencore_tone_address(ti, ci)

        for region in preset.regions:
            self._write_chunked(base + region.offset, list(region.data))

    def describe_parameter(self, name: str) -> str:
        """Return a one-line human-readable description of a parameter."""
        for group, table in _GROUP_TABLES.items():
            if name in table:
                p = table[name]
                return (f"{p.label} ({group}, offset 0x{p.offset:04X}, "
                        f"size {p.size}, range {p.min_value}..{p.max_value})")
        all_names = [n for table in _GROUP_TABLES.values() for n in table]
        raise KeyError(f"unknown parameter {name!r}.{_suggest(name, all_names)}")
