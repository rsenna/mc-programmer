"""Smoke tests for the mc_programmer Python library.

Run with:  python -m unittest python_lib/tests/test_mc_programmer.py
"""
import os
import sys
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.abspath(os.path.join(HERE, "..")))

from mc_programmer import (  # noqa: E402
    AddressMap,
    EMPTY_NOTE,
    MCClient,
    ModelID,
    NoteComponent,
    NoteStep,
    NullTransport,
    PARTIAL_BASE_OFFS,
    PARTIAL_STRIDE,
    ParamGroup,
    SubStep,
    TONE_COMMON_OFFS,
    TONE_COMMON_PARAMS,
    ZCParam,
    build_dt1,
    build_rq1,
    decode_note_step,
    encode_note_step,
    lookup_param,
    parse_dt1_payload,
    partial_offset,
    roland_checksum,
    substructure_offset,
)


class ProtocolTests(unittest.TestCase):
    def test_checksum_complement_to_128(self):
        payload = [0x20, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01]
        self.assertEqual(roland_checksum(payload), (128 - sum(payload) % 128) & 0x7F)

    def test_rq1_framing(self):
        msg = build_rq1(ModelID.MC_707, 0x20000000, 0x10)
        self.assertEqual(msg[0], 0xF0)
        self.assertEqual(msg[-1], 0xF7)
        self.assertEqual(msg[1], 0x41)        # Roland
        self.assertEqual(msg[6], 0x5D)        # MC-707
        self.assertEqual(msg[7], 0x11)        # RQ1

    def test_dt1_round_trip(self):
        msg = build_dt1(ModelID.MC_101, 0x30000018, [0x40])
        parsed = parse_dt1_payload(msg)
        self.assertEqual(parsed.address, 0x30000018)
        self.assertEqual(parsed.data, [0x40])


class AddressMapTests(unittest.TestCase):
    def test_no_clip_address_has_high_msb(self):
        amap = AddressMap.build()
        for trk in range(8):
            for clip in range(16):
                addr = amap.clip_base(trk, clip)
                self.assertEqual(addr & 0x80808080, 0,
                                 f"clip {trk},{clip} addr 0x{addr:08X} has high MSB")

    def test_zencore_track_tone(self):
        amap = AddressMap.build()
        # Track tone of track 0 = base + alignment * 16
        self.assertEqual(amap.zencore_tone_address(0, None),
                         0x30000000 + 0x20000 * 16)
        self.assertEqual(amap.zencore_tone_address(0, 0), 0x30000000)


class NoteStepTests(unittest.TestCase):
    def test_round_trip_complex_step(self):
        step = NoteStep()
        step.components[0] = NoteComponent(note=60, velocity=123, start_cents=-10, end_cents=100)
        step.components[1] = NoteComponent(note=67, velocity=100, start_cents=6,   end_cents=56)
        step.substep = SubStep.TRIPLE

        encoded = encode_note_step(step)
        self.assertEqual(len(encoded), 0x100)

        decoded = decode_note_step(encoded)
        self.assertEqual(decoded.components[0].note, 60)
        self.assertEqual(decoded.components[0].velocity, 123)
        self.assertEqual(decoded.components[0].start_cents, -10)
        self.assertEqual(decoded.components[0].end_cents, 100)
        self.assertTrue(decoded.components[0].ties_to_next)
        self.assertEqual(decoded.components[1].note, 67)
        self.assertEqual(decoded.components[1].start_cents, 6)
        self.assertEqual(decoded.substep, SubStep.TRIPLE)
        for i in range(2, 16):
            self.assertEqual(decoded.components[i].note, EMPTY_NOTE)

    def test_empty_note_sentinel(self):
        encoded = encode_note_step(NoteStep())
        # bytes 0x10 / 0x11 are the first note's nibble pair → 0x0F 0x0F
        self.assertEqual(encoded[0x10], 0x0F)
        self.assertEqual(encoded[0x11], 0x0F)

    def test_signed_start_cents_roundtrip_extremes(self):
        for cents in (-50, -1, 0, 1, 50, 99):
            step = NoteStep()
            step.components[0] = NoteComponent(note=60, start_cents=cents)
            decoded = decode_note_step(encode_note_step(step))
            self.assertEqual(decoded.components[0].start_cents, cents)


class MCClientTests(unittest.TestCase):
    def test_write_note_step_emits_sysex(self):
        transport = NullTransport()
        client = MCClient(transport, model_id=ModelID.MC_707)
        step = NoteStep()
        step.components[0] = NoteComponent(note=60)
        client.write_note_step(0, 0, 0, step)

        self.assertEqual(len(transport.sent), 1)
        sent = transport.sent[0]
        self.assertEqual(sent[0], 0xF0)
        self.assertEqual(sent[-1], 0xF7)
        self.assertEqual(sent[7], 0x12)  # DT1

    def test_read_note_step_decodes_dt1_reply(self):
        # Build a DT1 reply that the transport should hand back.
        empty_step = encode_note_step(NoteStep())
        reply = build_dt1(int(ModelID.MC_707), 0x20030000, list(empty_step))
        transport = NullTransport(responses=[reply])

        client = MCClient(transport, model_id=ModelID.MC_707)
        step = client.read_note_step(0, 0, 0)
        self.assertEqual(step.components[0].note, EMPTY_NOTE)


class ZenCoreTests(unittest.TestCase):
    def test_partial_offset_layout(self):
        # Conventional ZEN-Core: partial 1 at 0x2000, stride 0x100.
        self.assertEqual(partial_offset(1), PARTIAL_BASE_OFFS)
        self.assertEqual(partial_offset(2), PARTIAL_BASE_OFFS + PARTIAL_STRIDE)
        self.assertEqual(partial_offset(4), PARTIAL_BASE_OFFS + 3 * PARTIAL_STRIDE)
        self.assertEqual(PARTIAL_STRIDE, 0x0100)
        with self.assertRaises(ValueError):
            partial_offset(0)
        with self.assertRaises(ValueError):
            partial_offset(5)

    def test_substructure_offset_dispatch(self):
        from mc_programmer import (
            PARTIAL_LFO_BASE,
            PARTIAL_LFO_STRIDE,
        )
        self.assertEqual(substructure_offset(ParamGroup.COMMON), TONE_COMMON_OFFS)
        self.assertEqual(substructure_offset(ParamGroup.PARTIAL, 3),
                         PARTIAL_BASE_OFFS + 2 * PARTIAL_STRIDE)
        # Partial LFO has its own larger stride.
        self.assertEqual(substructure_offset(ParamGroup.PARTIAL_LFO, 3),
                         PARTIAL_LFO_BASE + 2 * PARTIAL_LFO_STRIDE)
        self.assertEqual(PARTIAL_LFO_STRIDE, 0x0200)

    def test_tone_coarse_tune_param_descriptor(self):
        param = lookup_param(ParamGroup.COMMON, "tone_coarse_tune")
        self.assertEqual(param.offset, 0x18)
        self.assertEqual(param.size, 1)
        self.assertEqual(param.encode(64), [64])
        self.assertEqual(param.decode([64]), 64)
        with self.assertRaises(ValueError):
            param.encode(0)    # below min (16)
        with self.assertRaises(ValueError):
            param.encode(200)  # above max (112)

    def test_tone_name_round_trip(self):
        # Roland names this field just "Name" → snake_case "name", 16 bytes ASCII.
        param = lookup_param(ParamGroup.COMMON, "name")
        self.assertEqual(param.size, 16)
        self.assertTrue(param.ascii)
        encoded = param.encode("INIT TONE")
        self.assertEqual(len(encoded), 16)
        self.assertEqual(encoded[-1], 0x20)            # right-padded with space
        self.assertEqual(param.decode(encoded), "INIT TONE")

    def test_nibble_packed_4_byte_value(self):
        # Real generated example: TVF Cutoff Frequency is a 4-byte 0..1023 value.
        param = lookup_param(ParamGroup.PARTIAL, "tvf_cutoff_frequency")
        self.assertEqual(param.size, 4)
        self.assertEqual(param.encode(0x123), [0x00, 0x01, 0x02, 0x03])
        self.assertEqual(param.decode([0x00, 0x01, 0x02, 0x03]), 0x123)

    def test_partial_pitch_env_lookup(self):
        param = lookup_param(ParamGroup.PARTIAL_PITCH_ENV, "pitch_env_time1")
        self.assertEqual(param.size, 4)
        self.assertEqual(param.min_value, 0)
        self.assertEqual(param.max_value, 1023)

    def test_partial_eq_lookup(self):
        param = lookup_param(ParamGroup.PARTIAL_EQ, "eq_low_gain")
        self.assertEqual(param.offset, 0x00)
        self.assertEqual(param.size, 4)
        self.assertEqual(param.min_value, 784)
        self.assertEqual(param.max_value, 1264)

    def test_client_read_tone_name_uses_correct_address(self):
        # Stage a fake DT1 reply for the Name field of clip 0/0.
        from mc_programmer import build_dt1
        param = lookup_param(ParamGroup.COMMON, "name")
        encoded = param.encode("BASS")
        reply = build_dt1(int(ModelID.MC_707), 0x30000000, encoded)

        transport = NullTransport(responses=[reply])
        client = MCClient(transport, model_id=ModelID.MC_707)
        name = client.read_tone_name(trk_idx=0, clip_idx=0)
        self.assertEqual(name, "BASS")

        self.assertEqual(len(transport.sent), 1)
        sent = transport.sent[0]
        self.assertEqual(sent[7], 0x11)  # RQ1
        addr = (sent[8] << 24) | (sent[9] << 16) | (sent[10] << 8) | sent[11]
        self.assertEqual(addr, 0x30000000)

    def test_client_writes_partial_filter_param_to_correct_address(self):
        # Per the [ZCore] address map, Tone Partial 2 is at offset 0x2100
        # within the tone descriptor; the Tone Coarse Tune of a partial is
        # at offset 0x01 inside [Tone Partial]. So writing to partial 2 of
        # the track-0 clip-0 tone (base 0x30000000) should target 0x30002101.
        transport = NullTransport()
        client = MCClient(transport, model_id=ModelID.MC_707)
        client.write_zc_param(0, 0, "coarse_tune", 70,
                              group=ParamGroup.PARTIAL, partial_idx=2)
        sent = transport.sent[0]
        self.assertEqual(sent[7], 0x12)  # DT1
        addr = (sent[8] << 24) | (sent[9] << 16) | (sent[10] << 8) | sent[11]
        self.assertEqual(addr, 0x30000000 + 0x2100 + 0x01)


class NoteParserTests(unittest.TestCase):
    """The note-name parser is the friendly entry point a musician hits first."""

    def test_passthrough_int(self):
        from mc_programmer import note
        self.assertEqual(note(60), 60)
        self.assertEqual(note(0), 0)
        self.assertEqual(note(127), 127)

    def test_int_out_of_range(self):
        from mc_programmer import note
        with self.assertRaises(ValueError):
            note(-1)
        with self.assertRaises(ValueError):
            note(128)

    def test_roland_octave_convention(self):
        # Roland MC: middle C = C3 = 60 (kick = C1 = 36).
        from mc_programmer import note
        self.assertEqual(note("C3"), 60)
        self.assertEqual(note("C1"), 36)         # kick
        self.assertEqual(note("D1"), 38)         # snare
        self.assertEqual(note("F#1"), 42)        # closed hat
        self.assertEqual(note("A#1"), 46)        # open hat

    def test_accidentals_both_forms(self):
        from mc_programmer import note
        self.assertEqual(note("C#3"), 61)
        self.assertEqual(note("Db3"), 61)
        self.assertEqual(note("Bb3"), 70)
        self.assertEqual(note("A#3"), 70)

    def test_case_insensitive(self):
        from mc_programmer import note
        self.assertEqual(note("c3"), 60)
        self.assertEqual(note("eb2"), 51)

    def test_negative_octave(self):
        from mc_programmer import note
        self.assertEqual(note("C-2"), 0)
        self.assertEqual(note("C-1"), 12)

    def test_default_octave(self):
        # No octave digit → assume octave 3 (Roland middle).
        from mc_programmer import note
        self.assertEqual(note("C"), 60)

    def test_invalid_name_message(self):
        from mc_programmer import note
        with self.assertRaises(ValueError) as cm:
            note("Hello")
        self.assertIn("unrecognised note name", str(cm.exception))

    def test_drum_constants(self):
        from mc_programmer import KICK, SNARE, HAT_CLOSED, HAT_OPEN
        self.assertEqual(KICK, 36)
        self.assertEqual(SNARE, 38)
        self.assertEqual(HAT_CLOSED, 42)
        self.assertEqual(HAT_OPEN, 46)

    def test_note_name_round_trip(self):
        from mc_programmer import note, note_name
        for n in (0, 1, 36, 60, 69, 127):
            self.assertEqual(note(note_name(n)), n)


class HighLevelHelperTests(unittest.TestCase):
    """The musician-friendly layer on MCClient — 1-based, accepts note names."""

    def _client(self):
        from mc_programmer import LoopbackTransport, MCClient, ModelID
        return MCClient(LoopbackTransport(), model_id=ModelID.MC_707)

    def test_place_note_with_name(self):
        client = self._client()
        # Place a kick on step 1 of clip 1, track 1 — front-panel numbering.
        client.place_note(track=1, clip=1, step=1, pitch="C1", velocity=120)
        # Read back via the low-level (0-based) primitive to verify.
        step = client.read_note_step(0, 0, 0)
        self.assertEqual(step.components[0].note, 36)
        self.assertEqual(step.components[0].velocity, 120)

    def test_place_note_with_midi_number(self):
        client = self._client()
        client.place_note(track=1, clip=1, step=1, pitch=60)
        step = client.read_note_step(0, 0, 0)
        self.assertEqual(step.components[0].note, 60)

    def test_place_note_packs_into_lowest_lane(self):
        client = self._client()
        client.place_note(track=1, clip=1, step=1, pitch="C1")
        client.place_note(track=1, clip=1, step=1, pitch="E1")
        client.place_note(track=1, clip=1, step=1, pitch="G1")
        step = client.read_note_step(0, 0, 0)
        notes = [c.note for c in step.components if not c.is_empty]
        self.assertEqual(sorted(notes), [36, 40, 43])

    def test_place_note_long_ties_across_steps(self):
        client = self._client()
        # 2.5 step note → step 1 ties to step 2 ties to half of step 3.
        client.place_note(track=1, clip=1, step=1, pitch="C3", length_steps=2.5)
        s1 = client.read_note_step(0, 0, 0)
        s2 = client.read_note_step(0, 0, 1)
        s3 = client.read_note_step(0, 0, 2)
        self.assertEqual(s1.components[0].end_cents, 100)  # tie
        self.assertEqual(s2.components[0].end_cents, 100)  # tie
        # Third step ends partway through (50 cents).
        self.assertEqual(s3.components[0].end_cents, 50)

    def test_clear_clip_empties_every_step(self):
        client = self._client()
        client.set_pattern(track=1, clip=1, notes={"C1": [1, 5, 9, 13]})
        client.clear_clip(track=1, clip=1)
        for s in range(16):
            step = client.read_note_step(0, 0, s)
            self.assertEqual(step.active_components(), [])

    def test_set_pattern_kick_and_hat(self):
        client = self._client()
        client.set_pattern(track=1, clip=1, notes={
            "C1":  [1, 5, 9, 13],     # kick on downbeats
            "F#1": [3, 7, 11, 15],    # hat on offbeats
        })
        kicks = []
        hats = []
        for s in range(16):
            step = client.read_note_step(0, 0, s)
            for c in step.active_components():
                if c.note == 36:
                    kicks.append(s + 1)   # convert back to 1-based for the assertion
                if c.note == 42:
                    hats.append(s + 1)
        self.assertEqual(sorted(kicks), [1, 5, 9, 13])
        self.assertEqual(sorted(hats), [3, 7, 11, 15])

    def test_track_out_of_range_message_is_friendly(self):
        client = self._client()
        with self.assertRaises(ValueError) as cm:
            client.place_note(track=9, clip=1, step=1, pitch="C1")
        self.assertIn("tracks 1..8", str(cm.exception))

    def test_coarse_tune_in_semitones(self):
        client = self._client()
        client.write_coarse_tune_semitones(track=2, clip=None, semitones=+5)
        self.assertEqual(client.read_coarse_tune_semitones(track=2, clip=None), 5)
        # Underlying wire value is 64 + 5 = 69.
        self.assertEqual(client.read_coarse_tune(trk_idx=1, clip_idx=None), 69)

    def test_coarse_tune_semitones_range_error(self):
        client = self._client()
        with self.assertRaises(ValueError) as cm:
            client.write_coarse_tune_semitones(track=2, clip=None, semitones=+50)
        self.assertIn("-48..+48", str(cm.exception))

    def test_did_you_mean_for_typo_in_param_name(self):
        client = self._client()
        with self.assertRaises(KeyError) as cm:
            client.read_zc_param(0, 0, "tone_coarse_tunes")  # typo
        self.assertIn("tone_coarse_tune", str(cm.exception))
        self.assertIn("Did you mean", str(cm.exception))

    def test_list_parameters_filtered_by_group(self):
        from mc_programmer import ParamGroup
        client = self._client()
        common = client.list_parameters(group=ParamGroup.COMMON)
        self.assertIn("tone_coarse_tune", common)
        self.assertIn("name", common)
        self.assertNotIn("tvf_cutoff_frequency", common)  # that's in PARTIAL

    def test_describe_parameter(self):
        client = self._client()
        s = client.describe_parameter("tone_coarse_tune")
        self.assertIn("Tone Coarse Tune", s)
        self.assertIn("0x0018", s)


class SceneClipSelectionTests(unittest.TestCase):
    """Scene/Clip selection on MC-707 / MC-101 happens via Program Change.

    No Bank Select, no Sys-Ex, no tone selection of any other kind.
    """

    def _client(self):
        from mc_programmer import LoopbackTransport, MCClient, ModelID
        return MCClient(LoopbackTransport(), model_id=ModelID.MC_707)

    def test_select_scene_emits_program_change_on_control_channel(self):
        client = self._client()
        client.select_scene(scene_number=5, control_channel=1)
        sent = client.transport.sent_raw
        self.assertEqual(len(sent), 1)
        # 0xC0 = Program Change on channel 1; program 4 = scene 5 (1-based)
        self.assertEqual(sent[0], [0xC0, 4])

    def test_select_scene_uses_configured_control_channel(self):
        client = self._client()
        client.select_scene(scene_number=128, control_channel=16)
        # 0xCF = Program Change on channel 16
        self.assertEqual(client.transport.sent_raw[0], [0xCF, 127])

    def test_select_scene_out_of_range(self):
        client = self._client()
        with self.assertRaises(ValueError):
            client.select_scene(scene_number=0)
        with self.assertRaises(ValueError):
            client.select_scene(scene_number=129)
        with self.assertRaises(ValueError):
            client.select_scene(scene_number=1, control_channel=17)

    def test_select_clip_defaults_to_track_channel(self):
        client = self._client()
        client.select_clip(track=3, clip_number=12)
        # Default mapping: track 3 → channel 3 → status 0xC2; clip 12 → program 11
        self.assertEqual(client.transport.sent_raw[0], [0xC2, 11])

    def test_select_clip_with_explicit_channel(self):
        client = self._client()
        client.select_clip(track=1, clip_number=4, track_channel=10)
        # Channel 10 → status 0xC9; clip 4 → program 3
        self.assertEqual(client.transport.sent_raw[0], [0xC9, 3])

    def test_select_clip_out_of_range(self):
        client = self._client()
        with self.assertRaises(ValueError):
            client.select_clip(track=1, clip_number=17)
        with self.assertRaises(ValueError):
            client.select_clip(track=9, clip_number=1)

    def test_loopback_records_active_scene_and_clip(self):
        client = self._client()
        client.select_scene(scene_number=7)             # PC on channel 1
        client.select_clip(track=2, clip_number=15)     # PC on channel 2
        self.assertEqual(client.transport.active_scene, 15)
        self.assertEqual(client.transport.active_clip_per_channel[1], 7)
        self.assertEqual(client.transport.active_clip_per_channel[2], 15)


class SynthAndMFXSubstructureTests(unittest.TestCase):
    """The MC's tone uses both the conventional and Synth halves of the
    ZCore tone descriptor, plus its own MFX block. Verify the framework
    addresses each substructure at the right offset and that the table
    contains the parameters the MC-707 Reference Manual documents.
    """

    def _client(self):
        from mc_programmer import LoopbackTransport, MCClient, ModelID
        return MCClient(LoopbackTransport(), model_id=ModelID.MC_707)

    def test_synth_common_unison_parameters_present(self):
        from mc_programmer import ParamGroup, lookup_param
        # MC-707 manual page 63 "UNISON" → maps to [Tone Synth Common].
        unison_switch = lookup_param(ParamGroup.SYNTH_COMMON, "unison_switch")
        unison_size   = lookup_param(ParamGroup.SYNTH_COMMON, "unison_size")
        unison_detune = lookup_param(ParamGroup.SYNTH_COMMON, "unison_detune")
        self.assertEqual(unison_switch.size, 1)
        self.assertEqual(unison_size.min_value, 2)
        self.assertEqual(unison_size.max_value, 8)
        self.assertEqual(unison_detune.max_value, 100)

    def test_synth_pmt_structure_parameters_present(self):
        from mc_programmer import ParamGroup, lookup_param
        # MC-707 manual page 62 "STRUCTURE" → [Tone Synth PMT]
        s12 = lookup_param(ParamGroup.SYNTH_PMT, "structure1_2")
        s34 = lookup_param(ParamGroup.SYNTH_PMT, "structure3_4")
        ring12 = lookup_param(ParamGroup.SYNTH_PMT, "ring1_2_level")
        cross12 = lookup_param(ParamGroup.SYNTH_PMT, "crossmod1_2_depth")
        self.assertEqual(s12.max_value, 4)   # OFF, SYNC, RING, XMOD, XMOD2
        self.assertEqual(s34.max_value, 4)
        self.assertEqual(ring12.max_value, 127)
        self.assertEqual(cross12.size, 4)    # 4-byte nibble-packed 0..10800

    def test_synth_partial_osc_type_parameter_present(self):
        from mc_programmer import ParamGroup, lookup_param
        # MC-707 manual: per-partial OSC Type (PCM/VA/PCM-Sync/SuperSAW/Noise)
        # → [Tone Synth Partial]
        osc_type = lookup_param(ParamGroup.SYNTH_PARTIAL, "osc_type")
        self.assertEqual(osc_type.offset, 0x00)
        self.assertEqual(osc_type.max_value, 4)  # 5 values: 0..4

    def test_mfx_table_present(self):
        from mc_programmer import ParamGroup, lookup_param
        mfx_type = lookup_param(ParamGroup.MFX, "type")
        switch   = lookup_param(ParamGroup.MFX, "mfx_switch")
        chorus   = lookup_param(ParamGroup.MFX, "mfx_chorus_send_level")
        self.assertEqual(mfx_type.offset, 0x00)
        self.assertEqual(switch.offset, 0x01)
        self.assertEqual(chorus.offset, 0x02)
        # Total Size 0x110 = 272 bytes (32 numeric MFX param slots + control matrix)
        from mc_programmer.zencore_data import SECTION_TOTAL_SIZE
        self.assertEqual(SECTION_TOTAL_SIZE["TONE_MFX"], 0x110)

    def test_substructure_offsets_match_address_map(self):
        from mc_programmer import (
            ParamGroup,
            substructure_offset,
            TONE_COMMON_OFFS,
            TONE_MFX_OFFS,
            TONE_PMT_OFFS,
        )
        self.assertEqual(substructure_offset(ParamGroup.COMMON), TONE_COMMON_OFFS)
        self.assertEqual(substructure_offset(ParamGroup.MFX), TONE_MFX_OFFS)
        self.assertEqual(substructure_offset(ParamGroup.PMT), TONE_PMT_OFFS)
        # Tone Synth substructures (per the [ZCore] address map):
        self.assertEqual(substructure_offset(ParamGroup.SYNTH_COMMON), 0x3C00)
        self.assertEqual(substructure_offset(ParamGroup.SYNTH_PMT), 0x3D00)
        self.assertEqual(substructure_offset(ParamGroup.SYNTH_PARTIAL, 1), 0x3E00)
        self.assertEqual(substructure_offset(ParamGroup.SYNTH_PARTIAL, 4), 0x4100)

    def test_client_writes_synth_partial_to_correct_address(self):
        # Per the [ZCore] address map, Synth Partial 2 is at 0x3F00.
        # OSC Type is at offset 0x00 inside Synth Partial.
        client = self._client()
        client.write_zc_param(0, 0, "osc_type", 1,    # PCM
                              group=ParamGroup.SYNTH_PARTIAL, partial_idx=2)
        # Verify by reading back through the framework
        self.assertEqual(
            client.read_zc_param(0, 0, "osc_type",
                                 group=ParamGroup.SYNTH_PARTIAL, partial_idx=2),
            1,
        )
        # And verify the underlying address by checking what got written
        # to the loopback memory at the expected location.
        addr = 0x30000000 + 0x3F00 + 0x00
        self.assertEqual(client.transport.snapshot(addr, 1), [1])

    def test_dump_tone_captures_synth_half_and_mfx(self):
        # Set distinguishable bytes in each new region, then dump and
        # verify they survive a round-trip.
        from mc_programmer import ParamGroup
        client = self._client()
        client.write_zc_param(0, None, "unison_switch", 1, group=ParamGroup.SYNTH_COMMON)
        client.write_zc_param(0, None, "structure1_2", 3, group=ParamGroup.SYNTH_PMT)
        client.write_zc_param(0, None, "osc_type", 2, group=ParamGroup.SYNTH_PARTIAL, partial_idx=1)
        client.write_zc_param(0, None, "mfx_switch", 1, group=ParamGroup.MFX)

        preset = client.dump_tone(track=1, clip=None)

        # Restore onto a different track and verify each new region survived.
        client.load_tone(track=5, clip=None, preset=preset)
        self.assertEqual(client.read_zc_param(4, None, "unison_switch", group=ParamGroup.SYNTH_COMMON), 1)
        self.assertEqual(client.read_zc_param(4, None, "structure1_2", group=ParamGroup.SYNTH_PMT), 3)
        self.assertEqual(client.read_zc_param(4, None, "osc_type", group=ParamGroup.SYNTH_PARTIAL, partial_idx=1), 2)
        self.assertEqual(client.read_zc_param(4, None, "mfx_switch", group=ParamGroup.MFX), 1)


class MC101CompatibilityTests(unittest.TestCase):
    """Verify the framework drives an MC-101 the same way as an MC-707.

    The MC-101 uses the same ZEN-Core engine, the same SysEx address
    layout, the same parameter table, and the same MFX type catalog
    (per the MC-101 Reference Manual + 1.80 update). The only practical
    difference is that the MC-101 has 4 tracks instead of 8.
    """

    def _client_101(self):
        from mc_programmer import LoopbackTransport, MCClient, ModelID
        return MCClient(LoopbackTransport(model_id=ModelID.MC_101),
                        model_id=ModelID.MC_101)

    def test_num_tracks_is_4_on_mc101(self):
        from mc_programmer import LoopbackTransport, MCClient, ModelID
        client_101 = MCClient(LoopbackTransport(), model_id=ModelID.MC_101)
        client_707 = MCClient(LoopbackTransport(), model_id=ModelID.MC_707)
        self.assertEqual(client_101.num_tracks, 4)
        self.assertEqual(client_707.num_tracks, 8)

    def test_mc101_rejects_track_5_through_8(self):
        client = self._client_101()
        for t in (5, 6, 7, 8):
            with self.assertRaises(ValueError) as cm:
                client.place_note(track=t, clip=1, step=1, pitch="C1")
            self.assertIn("MC-101", str(cm.exception))
            self.assertIn("1..4", str(cm.exception))

    def test_mc101_accepts_tracks_1_through_4(self):
        client = self._client_101()
        for t in (1, 2, 3, 4):
            client.place_note(track=t, clip=1, step=1, pitch="C1")
            step = client.read_note_step(t - 1, 0, 0)
            self.assertEqual(step.components[0].note, 36)

    def test_mc101_uses_same_zen_core_param_table(self):
        # Same Tone Common 'tone_coarse_tune' parameter as the MC-707.
        from mc_programmer import ParamGroup, lookup_param
        p = lookup_param(ParamGroup.COMMON, "tone_coarse_tune")
        self.assertEqual(p.offset, 0x18)
        self.assertEqual(p.size, 1)

    def test_mc101_full_project_roundtrip(self):
        from mc_programmer import Project
        src = self._client_101()
        src.set_pattern(track=1, clip=1, notes={"C1": [1, 5, 9, 13]})
        src.write_zc_param(0, 0, "tone_coarse_tune", 70)

        project = Project.dump_from(src, name="mc101 snapshot")
        self.assertEqual(project.model, "MC-101")
        # MC-101 has 4 tracks; Project.dump_from respects client.num_tracks.
        self.assertEqual(len(project.tracks), 4)

        dst = self._client_101()
        project.load_into(dst)

        kicks = []
        for s in range(16):
            for c in dst.read_note_step(0, 0, s).active_components():
                if c.note == 36:
                    kicks.append(s + 1)
        self.assertEqual(sorted(kicks), [1, 5, 9, 13])
        self.assertEqual(dst.read_zc_param(0, 0, "tone_coarse_tune"), 70)

    def test_mc101_load_refuses_mc707_project(self):
        from mc_programmer import Project
        client = self._client_101()
        wrong = Project(model="MC-707", tracks=[])
        with self.assertRaises(ValueError) as cm:
            wrong.load_into(client)
        self.assertIn("MC-707", str(cm.exception))
        self.assertIn("MC-101", str(cm.exception))


class TonePresetTests(unittest.TestCase):
    """Round-trip a tone preset through dump → save → load → restore."""

    def _client(self):
        from mc_programmer import LoopbackTransport, MCClient, ModelID
        return MCClient(LoopbackTransport(), model_id=ModelID.MC_707)

    def test_dump_then_load_roundtrip(self):
        client = self._client()
        # Make the source tone non-trivial: write a few ZEN-Core params
        # and a recognisable tone name on track 1.
        client.write_zc_param(0, None, "tone_coarse_tune", 70)   # +6 semitones
        client.write_tone_name(0, None, "TEST")

        preset = client.dump_tone(track=1, clip=None)
        self.assertTrue(preset.regions)
        self.assertEqual(preset.model, "MC-707")

        # Load it onto track 5 and verify the params transferred.
        client.load_tone(track=5, clip=None, preset=preset)
        self.assertEqual(client.read_zc_param(4, None, "tone_coarse_tune"), 70)
        self.assertEqual(client.read_tone_name(4, None), "TEST")

    def test_save_load_file_roundtrip(self):
        import os
        import tempfile
        from mc_programmer import TonePreset

        client = self._client()
        client.write_tone_name(0, None, "BASS")

        with tempfile.NamedTemporaryFile("w", suffix=".tone", delete=False) as f:
            path = f.name
        try:
            client.dump_tone(track=1, clip=None).save(path)
            preset = TonePreset.load(path)
            self.assertEqual(preset.name, "BASS")
            client.load_tone(track=2, clip=None, preset=preset)
            self.assertEqual(client.read_tone_name(1, None), "BASS")
        finally:
            os.unlink(path)

    def test_load_refuses_wrong_model(self):
        from mc_programmer import TonePreset, ToneRegion
        client = self._client()
        wrong = TonePreset(model="MC-101", regions=[
            ToneRegion(offset=0x18, label="x", data=bytes([70]))
        ])
        with self.assertRaises(ValueError) as cm:
            client.load_tone(track=1, clip=None, preset=wrong)
        self.assertIn("MC-101", str(cm.exception))
        self.assertIn("MC-707", str(cm.exception))


class ProjectTests(unittest.TestCase):
    """Full-state dump and restore for an entire MC."""

    def _client(self):
        from mc_programmer import LoopbackTransport, MCClient, ModelID
        return MCClient(LoopbackTransport(), model_id=ModelID.MC_707)

    def test_dump_then_load_into_blank_device(self):
        from mc_programmer import Project
        # Source device: a recognisable pattern + tone params on track 1.
        src = self._client()
        src.set_pattern(track=1, clip=1, notes={
            "C1":  [1, 5, 9, 13],
            "F#1": [3, 7, 11, 15],
        })
        src.write_zc_param(0, 0, "tone_coarse_tune", 72)   # +8 semitones
        src.write_tone_name(0, 0, "BIG BASS")

        project = Project.dump_from(src, name="snapshot")
        self.assertEqual(project.model, "MC-707")
        self.assertEqual(len(project.tracks), 8)
        self.assertEqual(len(project.tracks[0].clips), 16)

        # Target device: blank.
        dst = self._client()
        project.load_into(dst)

        # Verify pattern + tone restored.
        kicks = []
        for s in range(16):
            step = dst.read_note_step(0, 0, s)
            for c in step.active_components():
                if c.note == 36:
                    kicks.append(s + 1)
        self.assertEqual(sorted(kicks), [1, 5, 9, 13])
        self.assertEqual(dst.read_zc_param(0, 0, "tone_coarse_tune"), 72)
        self.assertEqual(dst.read_tone_name(0, 0), "BIG BASS")

    def test_save_load_project_file_roundtrip(self):
        import os
        import tempfile
        from mc_programmer import Project

        src = self._client()
        src.write_tone_name(0, 0, "PROJECT TEST")

        with tempfile.NamedTemporaryFile("w", suffix=".mctproject", delete=False) as f:
            path = f.name
        try:
            Project.dump_from(src, name="snapshot").save(path)
            project = Project.load(path)
            self.assertEqual(project.name, "snapshot")

            dst = self._client()
            project.load_into(dst)
            self.assertEqual(dst.read_tone_name(0, 0), "PROJECT TEST")
        finally:
            os.unlink(path)

    def test_load_refuses_wrong_model_unless_forced(self):
        from mc_programmer import Project
        client = self._client()
        wrong_model = Project(model="MC-101", tracks=[])
        with self.assertRaises(ValueError) as cm:
            wrong_model.load_into(client)
        self.assertIn("MC-101", str(cm.exception))
        # force=True should bypass the check
        wrong_model.load_into(client, force=True)


class CoreMIDIFramingTests(unittest.TestCase):
    """Drive ``RtMidiTransport._on_message`` directly to verify Sys-Ex
    framing without needing a real MIDI device.
    """

    def _make_transport(self):
        from mc_programmer.coremidi import RtMidiTransport

        # Bypass __init__ — we don't want to touch rtmidi at all.
        t = RtMidiTransport.__new__(RtMidiTransport)
        t._in = None
        t._out = None
        t._frames = []
        t._pending = []
        return t

    def test_complete_sysex_in_one_callback(self):
        t = self._make_transport()
        t._on_message(([0xF0, 0x41, 0x10, 0x00, 0xF7], 0.0))
        self.assertEqual(t._frames, [[0xF0, 0x41, 0x10, 0x00, 0xF7]])
        self.assertEqual(t._pending, [])

    def test_sysex_split_across_callbacks(self):
        t = self._make_transport()
        t._on_message(([0xF0, 0x41], 0.0))
        t._on_message(([0x10, 0x00], 0.0))
        t._on_message(([0xF7], 0.0))
        self.assertEqual(t._frames, [[0xF0, 0x41, 0x10, 0x00, 0xF7]])

    def test_strays_before_sysex_are_dropped(self):
        t = self._make_transport()
        # Active sensing 0xFE and clock 0xF8 between Sys-Ex frames are noise.
        t._on_message(([0xFE, 0xF8, 0xF0, 0x41, 0xF7], 0.0))
        self.assertEqual(t._frames, [[0xF0, 0x41, 0xF7]])


if __name__ == "__main__":
    unittest.main()
