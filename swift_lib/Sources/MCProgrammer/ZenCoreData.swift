// Auto-generated conventional ZEN-Core Tone parameter tables.
//
// DO NOT EDIT BY HAND. Regenerate with:
//
//     pdftotext -raw -f 76 -l 82 reference/JUPITER-X_Xm_MIDI_imple_eng06_W.pdf reference/tools_jp_zcore.txt
//     pdftotext -raw -f 85 -l 88 reference/JUPITER-X_Xm_MIDI_imple_eng06_W.pdf - \
//         | sed -n '/^* \[MFX\]/,/^* \[Reverb\]/p' >> reference/tools_jp_zcore.txt
//     python tools/gen_zencore_params.py
//
// Source: Roland Jupiter-X/Xm MIDI Implementation v1.06 (Apr 2022),
// [ZCore] address map. The MC family uses the *conventional* ZEN-Core
// Tone path (Tone Common / PMT / Partial / Pitch-Filter-Amp Envs / LFO / EQ)
// — *not* the SuperNATURAL Synth sub-section that lives at [Synth Partial].

import Foundation

public enum ZenCoreData {

    /// PARTIAL_EQ: 8 parameters, total size 0x1B bytes
    public static let PARTIAL_EQ: [String: ZCParam] = [
        "eq_low_gain": ZCParam(
            name: "eq_low_gain",
            offset: 0x0000,
            size: 4,
            minValue: 784, maxValue: 1264,
            defaultValue: 0,
            label: "EQ Low Gain",
            notes: "EQ Low Gain (784 - 1264)"
        ),
        "eq_mid_gain": ZCParam(
            name: "eq_mid_gain",
            offset: 0x0004,
            size: 4,
            minValue: 784, maxValue: 1264,
            defaultValue: 0,
            label: "EQ Mid Gain",
            notes: "EQ Mid Gain (784 - 1264)"
        ),
        "eq_high_gain": ZCParam(
            name: "eq_high_gain",
            offset: 0x0008,
            size: 4,
            minValue: 784, maxValue: 1264,
            defaultValue: 0,
            label: "EQ High Gain",
            notes: "EQ High Gain (784 - 1264)"
        ),
        "eq_low_frequency": ZCParam(
            name: "eq_low_frequency",
            offset: 0x000C,
            size: 4,
            minValue: 20, maxValue: 16000,
            defaultValue: 0,
            label: "EQ Low Frequency",
            notes: "EQ Low Frequency (20 - 16000)"
        ),
        "eq_mid_frequency": ZCParam(
            name: "eq_mid_frequency",
            offset: 0x0010,
            size: 4,
            minValue: 20, maxValue: 16000,
            defaultValue: 0,
            label: "EQ Mid Frequency",
            notes: "EQ Mid Frequency (20 - 16000)"
        ),
        "eq_high_frequency": ZCParam(
            name: "eq_high_frequency",
            offset: 0x0014,
            size: 4,
            minValue: 20, maxValue: 16000,
            defaultValue: 0,
            label: "EQ High Frequency",
            notes: "EQ High Frequency (20 - 16000)"
        ),
        "eq_mid_q": ZCParam(
            name: "eq_mid_q",
            offset: 0x0018,
            size: 2,
            minValue: 5, maxValue: 160,
            defaultValue: 0,
            label: "EQ Mid Q",
            notes: "EQ Mid Q (5 - 160)"
        ),
        "eq_switch": ZCParam(
            name: "eq_switch",
            offset: 0x001A,
            size: 1,
            minValue: 0, maxValue: 1,
            defaultValue: 0,
            label: "EQ Switch",
            notes: "EQ Switch (0 - 1)"
        ),
    ]

    /// PARTIAL_LFO: 98 parameters, total size 0x11E bytes
    public static let PARTIAL_LFO: [String: ZCParam] = [
        "waveform_1": ZCParam(
            name: "waveform_1",
            offset: 0x0000,
            size: 1,
            minValue: 0, maxValue: 10,
            defaultValue: 0,
            label: "Waveform 1",
            notes: "Waveform (0 - 10)"
        ),
        "rate_sync_1": ZCParam(
            name: "rate_sync_1",
            offset: 0x0001,
            size: 1,
            minValue: 0, maxValue: 1,
            defaultValue: 0,
            label: "Rate Sync 1",
            notes: "Rate Sync (0 - 1)"
        ),
        "rate_note_1": ZCParam(
            name: "rate_note_1",
            offset: 0x0002,
            size: 1,
            minValue: 0, maxValue: 22,
            defaultValue: 0,
            label: "Rate (note) 1",
            notes: "Rate (note) (0 - 22)"
        ),
        "step_size_1": ZCParam(
            name: "step_size_1",
            offset: 0x0003,
            size: 1,
            minValue: 0, maxValue: 15,
            defaultValue: 0,
            label: "Step Size 1",
            notes: "Step Size (0 - 15)"
        ),
        "rate_1": ZCParam(
            name: "rate_1",
            offset: 0x0004,
            size: 4,
            minValue: 0, maxValue: 1023,
            defaultValue: 0,
            label: "Rate 1",
            notes: "Rate (0 - 1023)"
        ),
        "offset_1": ZCParam(
            name: "offset_1",
            offset: 0x0008,
            size: 2,
            minValue: 28, maxValue: 228,
            defaultValue: 0,
            label: "Offset 1",
            notes: "Offset (28 - 228)"
        ),
        "rate_detune_1": ZCParam(
            name: "rate_detune_1",
            offset: 0x000A,
            size: 1,
            minValue: 0, maxValue: 127,
            defaultValue: 0,
            label: "Rate Detune 1",
            notes: "Rate Detune (0 - 127)"
        ),
        "delay_time_1": ZCParam(
            name: "delay_time_1",
            offset: 0x000B,
            size: 4,
            minValue: 0, maxValue: 1023,
            defaultValue: 0,
            label: "Delay Time 1",
            notes: "Delay Time (0 - 1023)"
        ),
        "delay_time_keyfolow_1": ZCParam(
            name: "delay_time_keyfolow_1",
            offset: 0x000F,
            size: 2,
            minValue: 28, maxValue: 228,
            defaultValue: 0,
            label: "Delay Time Keyfolow 1",
            notes: "Delay Time Keyfolow (28 - 228)"
        ),
        "fade_mode_1": ZCParam(
            name: "fade_mode_1",
            offset: 0x0011,
            size: 1,
            minValue: 0, maxValue: 3,
            defaultValue: 0,
            label: "Fade Mode 1",
            notes: "Fade Mode (0 - 3)"
        ),
        "fade_time_1": ZCParam(
            name: "fade_time_1",
            offset: 0x0012,
            size: 4,
            minValue: 0, maxValue: 1023,
            defaultValue: 0,
            label: "Fade Time 1",
            notes: "Fade Time (0 - 1023)"
        ),
        "key_trigger_1": ZCParam(
            name: "key_trigger_1",
            offset: 0x0016,
            size: 1,
            minValue: 0, maxValue: 1,
            defaultValue: 0,
            label: "Key Trigger 1",
            notes: "Key Trigger (0 - 1)"
        ),
        "pitch_depth_1": ZCParam(
            name: "pitch_depth_1",
            offset: 0x0017,
            size: 2,
            minValue: 28, maxValue: 228,
            defaultValue: 0,
            label: "Pitch Depth 1",
            notes: "Pitch Depth (28 - 228)"
        ),
        "tvf_depth_1": ZCParam(
            name: "tvf_depth_1",
            offset: 0x0019,
            size: 2,
            minValue: 28, maxValue: 228,
            defaultValue: 0,
            label: "TVF Depth 1",
            notes: "TVF Depth (28 - 228)"
        ),
        "tva_depth_1": ZCParam(
            name: "tva_depth_1",
            offset: 0x001B,
            size: 2,
            minValue: 28, maxValue: 228,
            defaultValue: 0,
            label: "TVA Depth 1",
            notes: "TVA Depth (28 - 228)"
        ),
        "pan_depth_1": ZCParam(
            name: "pan_depth_1",
            offset: 0x001D,
            size: 1,
            minValue: 1, maxValue: 127,
            defaultValue: 0,
            label: "PAN Depth 1",
            notes: "PAN Depth (1 - 127)"
        ),
        "phase_position_1": ZCParam(
            name: "phase_position_1",
            offset: 0x001E,
            size: 1,
            minValue: 0, maxValue: 3,
            defaultValue: 0,
            label: "Phase Position 1",
            notes: "Phase Position (0 - 3)"
        ),
        "step_1": ZCParam(
            name: "step_1",
            offset: 0x001F,
            size: 2,
            minValue: 56, maxValue: 200,
            defaultValue: 0,
            label: "Step 1",
            notes: "Step (56 - 200)"
        ),
        "step_2": ZCParam(
            name: "step_2",
            offset: 0x0021,
            size: 2,
            minValue: 56, maxValue: 200,
            defaultValue: 0,
            label: "Step 2",
            notes: "Step (56 - 200)"
        ),
        "step_3": ZCParam(
            name: "step_3",
            offset: 0x0023,
            size: 2,
            minValue: 56, maxValue: 200,
            defaultValue: 0,
            label: "Step 3",
            notes: "Step (56 - 200)"
        ),
        "step_4": ZCParam(
            name: "step_4",
            offset: 0x0025,
            size: 2,
            minValue: 56, maxValue: 200,
            defaultValue: 0,
            label: "Step 4",
            notes: "Step (56 - 200)"
        ),
        "step_5": ZCParam(
            name: "step_5",
            offset: 0x0027,
            size: 2,
            minValue: 56, maxValue: 200,
            defaultValue: 0,
            label: "Step 5",
            notes: "Step (56 - 200)"
        ),
        "step_6": ZCParam(
            name: "step_6",
            offset: 0x0029,
            size: 2,
            minValue: 56, maxValue: 200,
            defaultValue: 0,
            label: "Step 6",
            notes: "Step (56 - 200)"
        ),
        "step_7": ZCParam(
            name: "step_7",
            offset: 0x002B,
            size: 2,
            minValue: 56, maxValue: 200,
            defaultValue: 0,
            label: "Step 7",
            notes: "Step (56 - 200)"
        ),
        "step_8": ZCParam(
            name: "step_8",
            offset: 0x002D,
            size: 2,
            minValue: 56, maxValue: 200,
            defaultValue: 0,
            label: "Step 8",
            notes: "Step (56 - 200)"
        ),
        "step_9": ZCParam(
            name: "step_9",
            offset: 0x002F,
            size: 2,
            minValue: 56, maxValue: 200,
            defaultValue: 0,
            label: "Step 9",
            notes: "Step (56 - 200)"
        ),
        "step_10": ZCParam(
            name: "step_10",
            offset: 0x0031,
            size: 2,
            minValue: 56, maxValue: 200,
            defaultValue: 0,
            label: "Step 10",
            notes: "Step (56 - 200)"
        ),
        "step_11": ZCParam(
            name: "step_11",
            offset: 0x0033,
            size: 2,
            minValue: 56, maxValue: 200,
            defaultValue: 0,
            label: "Step 11",
            notes: "Step (56 - 200)"
        ),
        "step_12": ZCParam(
            name: "step_12",
            offset: 0x0035,
            size: 2,
            minValue: 56, maxValue: 200,
            defaultValue: 0,
            label: "Step 12",
            notes: "Step (56 - 200)"
        ),
        "step_13": ZCParam(
            name: "step_13",
            offset: 0x0037,
            size: 2,
            minValue: 56, maxValue: 200,
            defaultValue: 0,
            label: "Step 13",
            notes: "Step (56 - 200)"
        ),
        "step_14": ZCParam(
            name: "step_14",
            offset: 0x0039,
            size: 2,
            minValue: 56, maxValue: 200,
            defaultValue: 0,
            label: "Step 14",
            notes: "Step (56 - 200)"
        ),
        "step_15": ZCParam(
            name: "step_15",
            offset: 0x003B,
            size: 2,
            minValue: 56, maxValue: 200,
            defaultValue: 0,
            label: "Step 15",
            notes: "Step (56 - 200)"
        ),
        "step_16": ZCParam(
            name: "step_16",
            offset: 0x003D,
            size: 2,
            minValue: 56, maxValue: 200,
            defaultValue: 0,
            label: "Step 16",
            notes: "Step (56 - 200)"
        ),
        "step_curve_1": ZCParam(
            name: "step_curve_1",
            offset: 0x003F,
            size: 1,
            minValue: 0, maxValue: 36,
            defaultValue: 0,
            label: "Step Curve 1",
            notes: "Step Curve (0 - 36)"
        ),
        "step_curve_2": ZCParam(
            name: "step_curve_2",
            offset: 0x0040,
            size: 1,
            minValue: 0, maxValue: 36,
            defaultValue: 0,
            label: "Step Curve 2",
            notes: "Step Curve (0 - 36)"
        ),
        "step_curve_3": ZCParam(
            name: "step_curve_3",
            offset: 0x0041,
            size: 1,
            minValue: 0, maxValue: 36,
            defaultValue: 0,
            label: "Step Curve 3",
            notes: "Step Curve (0 - 36)"
        ),
        "step_curve_4": ZCParam(
            name: "step_curve_4",
            offset: 0x0042,
            size: 1,
            minValue: 0, maxValue: 36,
            defaultValue: 0,
            label: "Step Curve 4",
            notes: "Step Curve (0 - 36)"
        ),
        "step_curve_5": ZCParam(
            name: "step_curve_5",
            offset: 0x0043,
            size: 1,
            minValue: 0, maxValue: 36,
            defaultValue: 0,
            label: "Step Curve 5",
            notes: "Step Curve (0 - 36)"
        ),
        "step_curve_6": ZCParam(
            name: "step_curve_6",
            offset: 0x0044,
            size: 1,
            minValue: 0, maxValue: 36,
            defaultValue: 0,
            label: "Step Curve 6",
            notes: "Step Curve (0 - 36)"
        ),
        "step_curve_7": ZCParam(
            name: "step_curve_7",
            offset: 0x0045,
            size: 1,
            minValue: 0, maxValue: 36,
            defaultValue: 0,
            label: "Step Curve 7",
            notes: "Step Curve (0 - 36)"
        ),
        "step_curve_8": ZCParam(
            name: "step_curve_8",
            offset: 0x0046,
            size: 1,
            minValue: 0, maxValue: 36,
            defaultValue: 0,
            label: "Step Curve 8",
            notes: "Step Curve (0 - 36)"
        ),
        "step_curve_9": ZCParam(
            name: "step_curve_9",
            offset: 0x0047,
            size: 1,
            minValue: 0, maxValue: 36,
            defaultValue: 0,
            label: "Step Curve 9",
            notes: "Step Curve (0 - 36)"
        ),
        "step_curve_10": ZCParam(
            name: "step_curve_10",
            offset: 0x0048,
            size: 1,
            minValue: 0, maxValue: 36,
            defaultValue: 0,
            label: "Step Curve 10",
            notes: "Step Curve (0 - 36)"
        ),
        "step_curve_11": ZCParam(
            name: "step_curve_11",
            offset: 0x0049,
            size: 1,
            minValue: 0, maxValue: 36,
            defaultValue: 0,
            label: "Step Curve 11",
            notes: "Step Curve (0 - 36)"
        ),
        "step_curve_12": ZCParam(
            name: "step_curve_12",
            offset: 0x004A,
            size: 1,
            minValue: 0, maxValue: 36,
            defaultValue: 0,
            label: "Step Curve 12",
            notes: "Step Curve (0 - 36)"
        ),
        "step_curve_13": ZCParam(
            name: "step_curve_13",
            offset: 0x004B,
            size: 1,
            minValue: 0, maxValue: 36,
            defaultValue: 0,
            label: "Step Curve 13",
            notes: "Step Curve (0 - 36)"
        ),
        "step_curve_14": ZCParam(
            name: "step_curve_14",
            offset: 0x004C,
            size: 1,
            minValue: 0, maxValue: 36,
            defaultValue: 0,
            label: "Step Curve 14",
            notes: "Step Curve (0 - 36)"
        ),
        "step_curve_15": ZCParam(
            name: "step_curve_15",
            offset: 0x004D,
            size: 1,
            minValue: 0, maxValue: 36,
            defaultValue: 0,
            label: "Step Curve 15",
            notes: "Step Curve (0 - 36)"
        ),
        "step_curve_16": ZCParam(
            name: "step_curve_16",
            offset: 0x004E,
            size: 1,
            minValue: 0, maxValue: 36,
            defaultValue: 0,
            label: "Step Curve 16",
            notes: "Step Curve (0 - 36)"
        ),
        "waveform_2": ZCParam(
            name: "waveform_2",
            offset: 0x004F,
            size: 1,
            minValue: 0, maxValue: 10,
            defaultValue: 0,
            label: "Waveform 2",
            notes: "Waveform (0 - 10)"
        ),
        "rate_sync_2": ZCParam(
            name: "rate_sync_2",
            offset: 0x0050,
            size: 1,
            minValue: 0, maxValue: 1,
            defaultValue: 0,
            label: "Rate Sync 2",
            notes: "Rate Sync (0 - 1)"
        ),
        "rate_note_2": ZCParam(
            name: "rate_note_2",
            offset: 0x0051,
            size: 1,
            minValue: 0, maxValue: 22,
            defaultValue: 0,
            label: "Rate (note) 2",
            notes: "Rate (note) (0 - 22)"
        ),
        "step_size_2": ZCParam(
            name: "step_size_2",
            offset: 0x0052,
            size: 1,
            minValue: 0, maxValue: 15,
            defaultValue: 0,
            label: "Step Size 2",
            notes: "Step Size (0 - 15)"
        ),
        "rate_2": ZCParam(
            name: "rate_2",
            offset: 0x0053,
            size: 4,
            minValue: 0, maxValue: 1023,
            defaultValue: 0,
            label: "Rate 2",
            notes: "Rate (0 - 1023)"
        ),
        "offset_2": ZCParam(
            name: "offset_2",
            offset: 0x0057,
            size: 2,
            minValue: 28, maxValue: 228,
            defaultValue: 0,
            label: "Offset 2",
            notes: "Offset (28 - 228)"
        ),
        "rate_detune_2": ZCParam(
            name: "rate_detune_2",
            offset: 0x0059,
            size: 1,
            minValue: 0, maxValue: 127,
            defaultValue: 0,
            label: "Rate Detune 2",
            notes: "Rate Detune (0 - 127)"
        ),
        "delay_time_2": ZCParam(
            name: "delay_time_2",
            offset: 0x005A,
            size: 4,
            minValue: 0, maxValue: 1023,
            defaultValue: 0,
            label: "Delay Time 2",
            notes: "Delay Time (0 - 1023)"
        ),
        "delay_time_keyfolow_2": ZCParam(
            name: "delay_time_keyfolow_2",
            offset: 0x005E,
            size: 2,
            minValue: 28, maxValue: 228,
            defaultValue: 0,
            label: "Delay Time Keyfolow 2",
            notes: "Delay Time Keyfolow (28 - 228)"
        ),
        "fade_mode_2": ZCParam(
            name: "fade_mode_2",
            offset: 0x0060,
            size: 1,
            minValue: 0, maxValue: 3,
            defaultValue: 0,
            label: "Fade Mode 2",
            notes: "Fade Mode (0 - 3)"
        ),
        "fade_time_2": ZCParam(
            name: "fade_time_2",
            offset: 0x0061,
            size: 4,
            minValue: 0, maxValue: 1023,
            defaultValue: 0,
            label: "Fade Time 2",
            notes: "Fade Time (0 - 1023)"
        ),
        "key_trigger_2": ZCParam(
            name: "key_trigger_2",
            offset: 0x0065,
            size: 1,
            minValue: 0, maxValue: 1,
            defaultValue: 0,
            label: "Key Trigger 2",
            notes: "Key Trigger (0 - 1)"
        ),
        "pitch_depth_2": ZCParam(
            name: "pitch_depth_2",
            offset: 0x0066,
            size: 2,
            minValue: 28, maxValue: 228,
            defaultValue: 0,
            label: "Pitch Depth 2",
            notes: "Pitch Depth (28 - 228)"
        ),
        "tvf_depth_2": ZCParam(
            name: "tvf_depth_2",
            offset: 0x0068,
            size: 2,
            minValue: 28, maxValue: 228,
            defaultValue: 0,
            label: "TVF Depth 2",
            notes: "TVF Depth (28 - 228)"
        ),
        "tva_depth_2": ZCParam(
            name: "tva_depth_2",
            offset: 0x006A,
            size: 2,
            minValue: 28, maxValue: 228,
            defaultValue: 0,
            label: "TVA Depth 2",
            notes: "TVA Depth (28 - 228)"
        ),
        "pan_depth_2": ZCParam(
            name: "pan_depth_2",
            offset: 0x006C,
            size: 1,
            minValue: 1, maxValue: 127,
            defaultValue: 0,
            label: "PAN Depth 2",
            notes: "PAN Depth (1 - 127)"
        ),
        "phase_position_2": ZCParam(
            name: "phase_position_2",
            offset: 0x006D,
            size: 1,
            minValue: 0, maxValue: 3,
            defaultValue: 0,
            label: "Phase Position 2",
            notes: "Phase Position (0 - 3)"
        ),
        "step_17": ZCParam(
            name: "step_17",
            offset: 0x006E,
            size: 2,
            minValue: 56, maxValue: 200,
            defaultValue: 0,
            label: "Step 17",
            notes: "Step (56 - 200)"
        ),
        "step_18": ZCParam(
            name: "step_18",
            offset: 0x0070,
            size: 2,
            minValue: 56, maxValue: 200,
            defaultValue: 0,
            label: "Step 18",
            notes: "Step (56 - 200)"
        ),
        "step_19": ZCParam(
            name: "step_19",
            offset: 0x0072,
            size: 2,
            minValue: 56, maxValue: 200,
            defaultValue: 0,
            label: "Step 19",
            notes: "Step (56 - 200)"
        ),
        "step_20": ZCParam(
            name: "step_20",
            offset: 0x0074,
            size: 2,
            minValue: 56, maxValue: 200,
            defaultValue: 0,
            label: "Step 20",
            notes: "Step (56 - 200)"
        ),
        "step_21": ZCParam(
            name: "step_21",
            offset: 0x0076,
            size: 2,
            minValue: 56, maxValue: 200,
            defaultValue: 0,
            label: "Step 21",
            notes: "Step (56 - 200)"
        ),
        "step_22": ZCParam(
            name: "step_22",
            offset: 0x0078,
            size: 2,
            minValue: 56, maxValue: 200,
            defaultValue: 0,
            label: "Step 22",
            notes: "Step (56 - 200)"
        ),
        "step_23": ZCParam(
            name: "step_23",
            offset: 0x007A,
            size: 2,
            minValue: 56, maxValue: 200,
            defaultValue: 0,
            label: "Step 23",
            notes: "Step (56 - 200)"
        ),
        "step_24": ZCParam(
            name: "step_24",
            offset: 0x007C,
            size: 2,
            minValue: 56, maxValue: 200,
            defaultValue: 0,
            label: "Step 24",
            notes: "Step (56 - 200)"
        ),
        "step_25": ZCParam(
            name: "step_25",
            offset: 0x007E,
            size: 2,
            minValue: 56, maxValue: 200,
            defaultValue: 0,
            label: "Step 25",
            notes: "Step (56 - 200)"
        ),
        "step_26": ZCParam(
            name: "step_26",
            offset: 0x0100,
            size: 2,
            minValue: 56, maxValue: 200,
            defaultValue: 0,
            label: "Step 26",
            notes: "Step (56 - 200)"
        ),
        "step_27": ZCParam(
            name: "step_27",
            offset: 0x0102,
            size: 2,
            minValue: 56, maxValue: 200,
            defaultValue: 0,
            label: "Step 27",
            notes: "Step (56 - 200)"
        ),
        "step_28": ZCParam(
            name: "step_28",
            offset: 0x0104,
            size: 2,
            minValue: 56, maxValue: 200,
            defaultValue: 0,
            label: "Step 28",
            notes: "Step (56 - 200)"
        ),
        "step_29": ZCParam(
            name: "step_29",
            offset: 0x0106,
            size: 2,
            minValue: 56, maxValue: 200,
            defaultValue: 0,
            label: "Step 29",
            notes: "Step (56 - 200)"
        ),
        "step_30": ZCParam(
            name: "step_30",
            offset: 0x0108,
            size: 2,
            minValue: 56, maxValue: 200,
            defaultValue: 0,
            label: "Step 30",
            notes: "Step (56 - 200)"
        ),
        "step_31": ZCParam(
            name: "step_31",
            offset: 0x010A,
            size: 2,
            minValue: 56, maxValue: 200,
            defaultValue: 0,
            label: "Step 31",
            notes: "Step (56 - 200)"
        ),
        "step_32": ZCParam(
            name: "step_32",
            offset: 0x010C,
            size: 2,
            minValue: 56, maxValue: 200,
            defaultValue: 0,
            label: "Step 32",
            notes: "Step (56 - 200)"
        ),
        "step_curve_17": ZCParam(
            name: "step_curve_17",
            offset: 0x010E,
            size: 1,
            minValue: 0, maxValue: 36,
            defaultValue: 0,
            label: "Step Curve 17",
            notes: "Step Curve (0 - 36)"
        ),
        "step_curve_18": ZCParam(
            name: "step_curve_18",
            offset: 0x010F,
            size: 1,
            minValue: 0, maxValue: 36,
            defaultValue: 0,
            label: "Step Curve 18",
            notes: "Step Curve (0 - 36)"
        ),
        "step_curve_19": ZCParam(
            name: "step_curve_19",
            offset: 0x0110,
            size: 1,
            minValue: 0, maxValue: 36,
            defaultValue: 0,
            label: "Step Curve 19",
            notes: "Step Curve (0 - 36)"
        ),
        "step_curve_20": ZCParam(
            name: "step_curve_20",
            offset: 0x0111,
            size: 1,
            minValue: 0, maxValue: 36,
            defaultValue: 0,
            label: "Step Curve 20",
            notes: "Step Curve (0 - 36)"
        ),
        "step_curve_21": ZCParam(
            name: "step_curve_21",
            offset: 0x0112,
            size: 1,
            minValue: 0, maxValue: 36,
            defaultValue: 0,
            label: "Step Curve 21",
            notes: "Step Curve (0 - 36)"
        ),
        "step_curve_22": ZCParam(
            name: "step_curve_22",
            offset: 0x0113,
            size: 1,
            minValue: 0, maxValue: 36,
            defaultValue: 0,
            label: "Step Curve 22",
            notes: "Step Curve (0 - 36)"
        ),
        "step_curve_23": ZCParam(
            name: "step_curve_23",
            offset: 0x0114,
            size: 1,
            minValue: 0, maxValue: 36,
            defaultValue: 0,
            label: "Step Curve 23",
            notes: "Step Curve (0 - 36)"
        ),
        "step_curve_24": ZCParam(
            name: "step_curve_24",
            offset: 0x0115,
            size: 1,
            minValue: 0, maxValue: 36,
            defaultValue: 0,
            label: "Step Curve 24",
            notes: "Step Curve (0 - 36)"
        ),
        "step_curve_25": ZCParam(
            name: "step_curve_25",
            offset: 0x0116,
            size: 1,
            minValue: 0, maxValue: 36,
            defaultValue: 0,
            label: "Step Curve 25",
            notes: "Step Curve (0 - 36)"
        ),
        "step_curve_26": ZCParam(
            name: "step_curve_26",
            offset: 0x0117,
            size: 1,
            minValue: 0, maxValue: 36,
            defaultValue: 0,
            label: "Step Curve 26",
            notes: "Step Curve (0 - 36)"
        ),
        "step_curve_27": ZCParam(
            name: "step_curve_27",
            offset: 0x0118,
            size: 1,
            minValue: 0, maxValue: 36,
            defaultValue: 0,
            label: "Step Curve 27",
            notes: "Step Curve (0 - 36)"
        ),
        "step_curve_28": ZCParam(
            name: "step_curve_28",
            offset: 0x0119,
            size: 1,
            minValue: 0, maxValue: 36,
            defaultValue: 0,
            label: "Step Curve 28",
            notes: "Step Curve (0 - 36)"
        ),
        "step_curve_29": ZCParam(
            name: "step_curve_29",
            offset: 0x011A,
            size: 1,
            minValue: 0, maxValue: 36,
            defaultValue: 0,
            label: "Step Curve 29",
            notes: "Step Curve (0 - 36)"
        ),
        "step_curve_30": ZCParam(
            name: "step_curve_30",
            offset: 0x011B,
            size: 1,
            minValue: 0, maxValue: 36,
            defaultValue: 0,
            label: "Step Curve 30",
            notes: "Step Curve (0 - 36)"
        ),
        "step_curve_31": ZCParam(
            name: "step_curve_31",
            offset: 0x011C,
            size: 1,
            minValue: 0, maxValue: 36,
            defaultValue: 0,
            label: "Step Curve 31",
            notes: "Step Curve (0 - 36)"
        ),
        "step_curve_32": ZCParam(
            name: "step_curve_32",
            offset: 0x011D,
            size: 1,
            minValue: 0, maxValue: 36,
            defaultValue: 0,
            label: "Step Curve 32",
            notes: "Step Curve (0 - 36)"
        ),
    ]

    /// PARTIAL_PITCH_ENV: 14 parameters, total size 0x2D bytes
    public static let PARTIAL_PITCH_ENV: [String: ZCParam] = [
        "pitch_env_depth": ZCParam(
            name: "pitch_env_depth",
            offset: 0x0000,
            size: 2,
            minValue: 28, maxValue: 228,
            defaultValue: 0,
            label: "Pitch Env Depth",
            notes: "Pitch Env Depth (28 - 228)"
        ),
        "pitch_env_velocity_sens": ZCParam(
            name: "pitch_env_velocity_sens",
            offset: 0x0002,
            size: 2,
            minValue: 28, maxValue: 228,
            defaultValue: 0,
            label: "Pitch Env Velocity Sens",
            notes: "Pitch Env Velocity Sens (28 - 228)"
        ),
        "pitch_env_t1_velocity_sens": ZCParam(
            name: "pitch_env_t1_velocity_sens",
            offset: 0x0004,
            size: 2,
            minValue: 28, maxValue: 228,
            defaultValue: 0,
            label: "Pitch Env T1 Velocity Sens",
            notes: "Pitch Env T1 Velocity Sens (28 - 228)"
        ),
        "pitch_env_t4_velocity_sens": ZCParam(
            name: "pitch_env_t4_velocity_sens",
            offset: 0x0006,
            size: 2,
            minValue: 28, maxValue: 228,
            defaultValue: 0,
            label: "Pitch Env T4 Velocity Sens",
            notes: "Pitch Env T4 Velocity Sens (28 - 228)"
        ),
        "pitch_env_time1": ZCParam(
            name: "pitch_env_time1",
            offset: 0x0008,
            size: 4,
            minValue: 0, maxValue: 1023,
            defaultValue: 0,
            label: "Pitch Env Time1",
            notes: "Pitch Env Time1 (0 - 1023)"
        ),
        "pitch_env_time2": ZCParam(
            name: "pitch_env_time2",
            offset: 0x000C,
            size: 4,
            minValue: 0, maxValue: 1023,
            defaultValue: 0,
            label: "Pitch Env Time2",
            notes: "Pitch Env Time2 (0 - 1023)"
        ),
        "pitch_env_time3": ZCParam(
            name: "pitch_env_time3",
            offset: 0x0010,
            size: 4,
            minValue: 0, maxValue: 1023,
            defaultValue: 0,
            label: "Pitch Env Time3",
            notes: "Pitch Env Time3 (0 - 1023)"
        ),
        "pitch_env_time4": ZCParam(
            name: "pitch_env_time4",
            offset: 0x0014,
            size: 4,
            minValue: 0, maxValue: 1023,
            defaultValue: 0,
            label: "Pitch Env Time4",
            notes: "Pitch Env Time4 (0 - 1023)"
        ),
        "pitch_env_level0": ZCParam(
            name: "pitch_env_level0",
            offset: 0x0018,
            size: 4,
            minValue: 513, maxValue: 1535,
            defaultValue: 0,
            label: "Pitch Env Level0",
            notes: "Pitch Env Level0 (513 - 1535)"
        ),
        "pitch_env_level1": ZCParam(
            name: "pitch_env_level1",
            offset: 0x001C,
            size: 4,
            minValue: 513, maxValue: 1535,
            defaultValue: 0,
            label: "Pitch Env Level1",
            notes: "Pitch Env Level1 (513 - 1535)"
        ),
        "pitch_env_level2": ZCParam(
            name: "pitch_env_level2",
            offset: 0x0020,
            size: 4,
            minValue: 513, maxValue: 1535,
            defaultValue: 0,
            label: "Pitch Env Level2",
            notes: "Pitch Env Level2 (513 - 1535)"
        ),
        "pitch_env_level3": ZCParam(
            name: "pitch_env_level3",
            offset: 0x0024,
            size: 4,
            minValue: 513, maxValue: 1535,
            defaultValue: 0,
            label: "Pitch Env Level3",
            notes: "Pitch Env Level3 (513 - 1535)"
        ),
        "pitch_env_level4": ZCParam(
            name: "pitch_env_level4",
            offset: 0x0028,
            size: 4,
            minValue: 513, maxValue: 1535,
            defaultValue: 0,
            label: "Pitch Env Level4",
            notes: "Pitch Env Level4 (513 - 1535)"
        ),
        "pitch_env_velocity_curve": ZCParam(
            name: "pitch_env_velocity_curve",
            offset: 0x002C,
            size: 1,
            minValue: 0, maxValue: 7,
            defaultValue: 0,
            label: "Pitch Env Velocity Curve",
            notes: "Pitch Env Velocity Curve (0 - 7)"
        ),
    ]

    /// PARTIAL_FILTER_ENV: 14 parameters, total size 0x2C bytes
    public static let PARTIAL_FILTER_ENV: [String: ZCParam] = [
        "tvf_env_depth": ZCParam(
            name: "tvf_env_depth",
            offset: 0x0000,
            size: 1,
            minValue: 1, maxValue: 127,
            defaultValue: 0,
            label: "TVF Env Depth",
            notes: "TVF Env Depth (1 - 127)"
        ),
        "tvf_env_velocity_curve": ZCParam(
            name: "tvf_env_velocity_curve",
            offset: 0x0001,
            size: 1,
            minValue: 0, maxValue: 7,
            defaultValue: 0,
            label: "TVF Env Velocity Curve",
            notes: "TVF Env Velocity Curve (0 - 7)"
        ),
        "tvf_env_velocity_sens": ZCParam(
            name: "tvf_env_velocity_sens",
            offset: 0x0002,
            size: 2,
            minValue: 28, maxValue: 228,
            defaultValue: 0,
            label: "TVF Env Velocity Sens",
            notes: "TVF Env Velocity Sens (28 - 228)"
        ),
        "tvf_env_t1_velocity_sens": ZCParam(
            name: "tvf_env_t1_velocity_sens",
            offset: 0x0004,
            size: 2,
            minValue: 28, maxValue: 228,
            defaultValue: 0,
            label: "TVF Env T1 Velocity Sens",
            notes: "TVF Env T1 Velocity Sens (28 - 228)"
        ),
        "tvf_env_t4_velocity_sens": ZCParam(
            name: "tvf_env_t4_velocity_sens",
            offset: 0x0006,
            size: 2,
            minValue: 28, maxValue: 228,
            defaultValue: 0,
            label: "TVF Env T4 Velocity Sens",
            notes: "TVF Env T4 Velocity Sens (28 - 228)"
        ),
        "tvf_env_time1": ZCParam(
            name: "tvf_env_time1",
            offset: 0x0008,
            size: 4,
            minValue: 0, maxValue: 1023,
            defaultValue: 0,
            label: "TVF Env Time1",
            notes: "TVF Env Time1 (0 - 1023)"
        ),
        "tvf_env_time2": ZCParam(
            name: "tvf_env_time2",
            offset: 0x000C,
            size: 4,
            minValue: 0, maxValue: 1023,
            defaultValue: 0,
            label: "TVF Env Time2",
            notes: "TVF Env Time2 (0 - 1023)"
        ),
        "tvf_env_time3": ZCParam(
            name: "tvf_env_time3",
            offset: 0x0010,
            size: 4,
            minValue: 0, maxValue: 1023,
            defaultValue: 0,
            label: "TVF Env Time3",
            notes: "TVF Env Time3 (0 - 1023)"
        ),
        "tvf_env_time4": ZCParam(
            name: "tvf_env_time4",
            offset: 0x0014,
            size: 4,
            minValue: 0, maxValue: 1023,
            defaultValue: 0,
            label: "TVF Env Time4",
            notes: "TVF Env Time4 (0 - 1023)"
        ),
        "tvf_env_level0": ZCParam(
            name: "tvf_env_level0",
            offset: 0x0018,
            size: 4,
            minValue: 0, maxValue: 1023,
            defaultValue: 0,
            label: "TVF Env Level0",
            notes: "TVF Env Level0 (0 - 1023)"
        ),
        "tvf_env_level1": ZCParam(
            name: "tvf_env_level1",
            offset: 0x001C,
            size: 4,
            minValue: 0, maxValue: 1023,
            defaultValue: 0,
            label: "TVF Env Level1",
            notes: "TVF Env Level1 (0 - 1023)"
        ),
        "tvf_env_level2": ZCParam(
            name: "tvf_env_level2",
            offset: 0x0020,
            size: 4,
            minValue: 0, maxValue: 1023,
            defaultValue: 0,
            label: "TVF Env Level2",
            notes: "TVF Env Level2 (0 - 1023)"
        ),
        "tvf_env_level3": ZCParam(
            name: "tvf_env_level3",
            offset: 0x0024,
            size: 4,
            minValue: 0, maxValue: 1023,
            defaultValue: 0,
            label: "TVF Env Level3",
            notes: "TVF Env Level3 (0 - 1023)"
        ),
        "tvf_env_level4": ZCParam(
            name: "tvf_env_level4",
            offset: 0x0028,
            size: 4,
            minValue: 0, maxValue: 1023,
            defaultValue: 0,
            label: "TVF Env Level4",
            notes: "TVF Env Level4 (0 - 1023)"
        ),
    ]

    /// PARTIAL_AMP_ENV: 9 parameters, total size 0x20 bytes
    public static let PARTIAL_AMP_ENV: [String: ZCParam] = [
        "tva_env_time1_velocity_sens": ZCParam(
            name: "tva_env_time1_velocity_sens",
            offset: 0x0000,
            size: 2,
            minValue: 28, maxValue: 228,
            defaultValue: 0,
            label: "TVA Env Time1 Velocity Sens",
            notes: "TVA Env Time1 Velocity Sens (28 - 228)"
        ),
        "tva_env_time4_velocity_sens": ZCParam(
            name: "tva_env_time4_velocity_sens",
            offset: 0x0002,
            size: 2,
            minValue: 28, maxValue: 228,
            defaultValue: 0,
            label: "TVA Env Time4 Velocity Sens",
            notes: "TVA Env Time4 Velocity Sens (28 - 228)"
        ),
        "tva_env_time1": ZCParam(
            name: "tva_env_time1",
            offset: 0x0004,
            size: 4,
            minValue: 0, maxValue: 1023,
            defaultValue: 0,
            label: "TVA Env Time1",
            notes: "TVA Env Time1 (0 - 1023)"
        ),
        "tva_env_time2": ZCParam(
            name: "tva_env_time2",
            offset: 0x0008,
            size: 4,
            minValue: 0, maxValue: 1023,
            defaultValue: 0,
            label: "TVA Env Time2",
            notes: "TVA Env Time2 (0 - 1023)"
        ),
        "tva_env_time3": ZCParam(
            name: "tva_env_time3",
            offset: 0x000C,
            size: 4,
            minValue: 0, maxValue: 1023,
            defaultValue: 0,
            label: "TVA Env Time3",
            notes: "TVA Env Time3 (0 - 1023)"
        ),
        "tva_env_time4": ZCParam(
            name: "tva_env_time4",
            offset: 0x0010,
            size: 4,
            minValue: 0, maxValue: 1023,
            defaultValue: 0,
            label: "TVA Env Time4",
            notes: "TVA Env Time4 (0 - 1023)"
        ),
        "tva_env_level1": ZCParam(
            name: "tva_env_level1",
            offset: 0x0014,
            size: 4,
            minValue: 0, maxValue: 1023,
            defaultValue: 0,
            label: "TVA Env Level1",
            notes: "TVA Env Level1 (0 - 1023)"
        ),
        "tva_env_level2": ZCParam(
            name: "tva_env_level2",
            offset: 0x0018,
            size: 4,
            minValue: 0, maxValue: 1023,
            defaultValue: 0,
            label: "TVA Env Level2",
            notes: "TVA Env Level2 (0 - 1023)"
        ),
        "tva_env_level3": ZCParam(
            name: "tva_env_level3",
            offset: 0x001C,
            size: 4,
            minValue: 0, maxValue: 1023,
            defaultValue: 0,
            label: "TVA Env Level3",
            notes: "TVA Env Level3 (0 - 1023)"
        ),
    ]

    /// TONE_COMMON: 24 parameters, total size 0x36 bytes
    public static let TONE_COMMON: [String: ZCParam] = [
        "name": ZCParam(
            name: "name",
            offset: 0x0000,
            size: 16,
            minValue: 32, maxValue: 127,
            defaultValue: 0,
            label: "Name",
            notes: "16-byte ASCII string"
        ),
        "category": ZCParam(
            name: "category",
            offset: 0x0010,
            size: 1,
            minValue: 0, maxValue: 50,
            defaultValue: 0,
            label: "Category",
            notes: "(*)Category (0 - 50)"
        ),
        "level": ZCParam(
            name: "level",
            offset: 0x0015,
            size: 1,
            minValue: 0, maxValue: 127,
            defaultValue: 0,
            label: "Level",
            notes: "Level (0 - 127)"
        ),
        "pan": ZCParam(
            name: "pan",
            offset: 0x0016,
            size: 1,
            minValue: 0, maxValue: 127,
            defaultValue: 0,
            label: "Pan",
            notes: "Pan (0 - 127)"
        ),
        "priority": ZCParam(
            name: "priority",
            offset: 0x0017,
            size: 1,
            minValue: 0, maxValue: 1,
            defaultValue: 0,
            label: "Priority",
            notes: "Priority (0 - 1)"
        ),
        "tone_coarse_tune": ZCParam(
            name: "tone_coarse_tune",
            offset: 0x0018,
            size: 1,
            minValue: 16, maxValue: 112,
            defaultValue: 0,
            label: "Tone Coarse Tune",
            notes: "Tone Coarse Tune (16 - 112)"
        ),
        "tone_fine_tune": ZCParam(
            name: "tone_fine_tune",
            offset: 0x0019,
            size: 1,
            minValue: 14, maxValue: 114,
            defaultValue: 0,
            label: "Tone Fine Tune",
            notes: "Tone Fine Tune (14 - 114)"
        ),
        "octave_shift": ZCParam(
            name: "octave_shift",
            offset: 0x001A,
            size: 1,
            minValue: 61, maxValue: 67,
            defaultValue: 0,
            label: "Octave Shift",
            notes: "Octave Shift (61 - 67)"
        ),
        "stretch_tune_depth": ZCParam(
            name: "stretch_tune_depth",
            offset: 0x001B,
            size: 1,
            minValue: 0, maxValue: 3,
            defaultValue: 0,
            label: "Stretch Tune Depth",
            notes: "Stretch Tune Depth (0 - 3)"
        ),
        "analog_feel": ZCParam(
            name: "analog_feel",
            offset: 0x001C,
            size: 1,
            minValue: 0, maxValue: 127,
            defaultValue: 0,
            label: "Analog Feel",
            notes: "Analog Feel (0 - 127)"
        ),
        "mono_poly": ZCParam(
            name: "mono_poly",
            offset: 0x001D,
            size: 1,
            minValue: 0, maxValue: 1,
            defaultValue: 0,
            label: "Mono/Poly",
            notes: "Mono/Poly (0 - 1)"
        ),
        "legato_switch": ZCParam(
            name: "legato_switch",
            offset: 0x001E,
            size: 1,
            minValue: 0, maxValue: 1,
            defaultValue: 0,
            label: "Legato Switch",
            notes: "Legato Switch (0 - 1)"
        ),
        "legato_retrigger_interval": ZCParam(
            name: "legato_retrigger_interval",
            offset: 0x001F,
            size: 1,
            minValue: 0, maxValue: 13,
            defaultValue: 0,
            label: "Legato Retrigger Interval",
            notes: "Legato Retrigger Interval (0 - 13)"
        ),
        "portamento_switch": ZCParam(
            name: "portamento_switch",
            offset: 0x0020,
            size: 1,
            minValue: 0, maxValue: 1,
            defaultValue: 0,
            label: "Portamento Switch",
            notes: "Portamento Switch (0 - 1)"
        ),
        "portamento_mode": ZCParam(
            name: "portamento_mode",
            offset: 0x0021,
            size: 1,
            minValue: 0, maxValue: 1,
            defaultValue: 0,
            label: "Portamento Mode",
            notes: "Portamento Mode (0 - 1)"
        ),
        "portamento_type": ZCParam(
            name: "portamento_type",
            offset: 0x0022,
            size: 1,
            minValue: 0, maxValue: 1,
            defaultValue: 0,
            label: "Portamento Type",
            notes: "Portamento Type (0 - 1)"
        ),
        "portamento_start": ZCParam(
            name: "portamento_start",
            offset: 0x0023,
            size: 1,
            minValue: 0, maxValue: 1,
            defaultValue: 0,
            label: "Portamento Start",
            notes: "Portamento Start (0 - 1)"
        ),
        "portamento_time": ZCParam(
            name: "portamento_time",
            offset: 0x0024,
            size: 4,
            minValue: 0, maxValue: 1023,
            defaultValue: 0,
            label: "Portamento Time",
            notes: "Portamento Time (0 - 1023)"
        ),
        "bend_range_up": ZCParam(
            name: "bend_range_up",
            offset: 0x0028,
            size: 1,
            minValue: 0, maxValue: 48,
            defaultValue: 0,
            label: "Bend Range Up",
            notes: "Bend Range Up (0 - 48)"
        ),
        "bend_range_down": ZCParam(
            name: "bend_range_down",
            offset: 0x0029,
            size: 1,
            minValue: 0, maxValue: 48,
            defaultValue: 0,
            label: "Bend Range Down",
            notes: "Bend Range Down (0 - 48)"
        ),
        "bend_mode": ZCParam(
            name: "bend_mode",
            offset: 0x002A,
            size: 1,
            minValue: 0, maxValue: 1,
            defaultValue: 0,
            label: "Bend Mode",
            notes: "Bend Mode (0 - 1)"
        ),
        "soft_level_sens": ZCParam(
            name: "soft_level_sens",
            offset: 0x002B,
            size: 1,
            minValue: 0, maxValue: 100,
            defaultValue: 0,
            label: "Soft Level Sens",
            notes: "Soft Level Sens (0 - 100)"
        ),
        "portamento_curve_type": ZCParam(
            name: "portamento_curve_type",
            offset: 0x0034,
            size: 1,
            minValue: 1, maxValue: 3,
            defaultValue: 0,
            label: "Portamento Curve Type",
            notes: "Portamento Curve Type (1 - 3)"
        ),
        "partial_lfo_phase_lock": ZCParam(
            name: "partial_lfo_phase_lock",
            offset: 0x0035,
            size: 1,
            minValue: 0, maxValue: 1,
            defaultValue: 0,
            label: "Partial LFO Phase Lock",
            notes: "Partial LFO Phase Lock (0 - 1)"
        ),
    ]

    /// TONE_PMT: 38 parameters, total size 0x26 bytes
    public static let TONE_PMT: [String: ZCParam] = [
        "velocity_control": ZCParam(
            name: "velocity_control",
            offset: 0x0000,
            size: 1,
            minValue: 0, maxValue: 3,
            defaultValue: 0,
            label: "Velocity Control",
            notes: "Velocity Control (0 - 3)"
        ),
        "pmt_level_curve": ZCParam(
            name: "pmt_level_curve",
            offset: 0x0001,
            size: 1,
            minValue: 0, maxValue: 1,
            defaultValue: 0,
            label: "PMT Level Curve",
            notes: "PMT Level Curve (0 - 1)"
        ),
        "partial_switch_1": ZCParam(
            name: "partial_switch_1",
            offset: 0x0002,
            size: 1,
            minValue: 0, maxValue: 1,
            defaultValue: 0,
            label: "Partial Switch 1",
            notes: "Partial Switch (0 - 1)"
        ),
        "keyrange_lower_1": ZCParam(
            name: "keyrange_lower_1",
            offset: 0x0003,
            size: 1,
            minValue: 0, maxValue: 127,
            defaultValue: 0,
            label: "KeyRange Lower 1",
            notes: "KeyRange Lower (0 - 127)"
        ),
        "keyrange_upper_1": ZCParam(
            name: "keyrange_upper_1",
            offset: 0x0004,
            size: 1,
            minValue: 0, maxValue: 127,
            defaultValue: 0,
            label: "KeyRange Upper 1",
            notes: "KeyRange Upper (0 - 127)"
        ),
        "keyfade_lower_1": ZCParam(
            name: "keyfade_lower_1",
            offset: 0x0005,
            size: 1,
            minValue: 0, maxValue: 127,
            defaultValue: 0,
            label: "KeyFade Lower 1",
            notes: "KeyFade Lower (0 - 127)"
        ),
        "keyfade_upper_1": ZCParam(
            name: "keyfade_upper_1",
            offset: 0x0006,
            size: 1,
            minValue: 0, maxValue: 127,
            defaultValue: 0,
            label: "KeyFade Upper 1",
            notes: "KeyFade Upper (0 - 127)"
        ),
        "velorange_lower_1": ZCParam(
            name: "velorange_lower_1",
            offset: 0x0007,
            size: 1,
            minValue: 1, maxValue: 127,
            defaultValue: 0,
            label: "VeloRange Lower 1",
            notes: "VeloRange Lower (1 - 127)"
        ),
        "velorange_upper_1": ZCParam(
            name: "velorange_upper_1",
            offset: 0x0008,
            size: 1,
            minValue: 1, maxValue: 127,
            defaultValue: 0,
            label: "VeloRange Upper 1",
            notes: "VeloRange Upper (1 - 127)"
        ),
        "velofade_lower_1": ZCParam(
            name: "velofade_lower_1",
            offset: 0x0009,
            size: 1,
            minValue: 0, maxValue: 127,
            defaultValue: 0,
            label: "VeloFade Lower 1",
            notes: "VeloFade Lower (0 - 127)"
        ),
        "velofade_upper_1": ZCParam(
            name: "velofade_upper_1",
            offset: 0x000A,
            size: 1,
            minValue: 0, maxValue: 127,
            defaultValue: 0,
            label: "VeloFade Upper 1",
            notes: "VeloFade Upper (0 - 127)"
        ),
        "partial_switch_2": ZCParam(
            name: "partial_switch_2",
            offset: 0x000B,
            size: 1,
            minValue: 0, maxValue: 1,
            defaultValue: 0,
            label: "Partial Switch 2",
            notes: "Partial Switch (0 - 1)"
        ),
        "keyrange_lower_2": ZCParam(
            name: "keyrange_lower_2",
            offset: 0x000C,
            size: 1,
            minValue: 0, maxValue: 127,
            defaultValue: 0,
            label: "KeyRange Lower 2",
            notes: "KeyRange Lower (0 - 127)"
        ),
        "keyrange_upper_2": ZCParam(
            name: "keyrange_upper_2",
            offset: 0x000D,
            size: 1,
            minValue: 0, maxValue: 127,
            defaultValue: 0,
            label: "KeyRange Upper 2",
            notes: "KeyRange Upper (0 - 127)"
        ),
        "keyfade_lower_2": ZCParam(
            name: "keyfade_lower_2",
            offset: 0x000E,
            size: 1,
            minValue: 0, maxValue: 127,
            defaultValue: 0,
            label: "KeyFade Lower 2",
            notes: "KeyFade Lower (0 - 127)"
        ),
        "keyfade_upper_2": ZCParam(
            name: "keyfade_upper_2",
            offset: 0x000F,
            size: 1,
            minValue: 0, maxValue: 127,
            defaultValue: 0,
            label: "KeyFade Upper 2",
            notes: "KeyFade Upper (0 - 127)"
        ),
        "velorange_lower_2": ZCParam(
            name: "velorange_lower_2",
            offset: 0x0010,
            size: 1,
            minValue: 1, maxValue: 127,
            defaultValue: 0,
            label: "VeloRange Lower 2",
            notes: "VeloRange Lower (1 - 127)"
        ),
        "velorange_upper_2": ZCParam(
            name: "velorange_upper_2",
            offset: 0x0011,
            size: 1,
            minValue: 1, maxValue: 127,
            defaultValue: 0,
            label: "VeloRange Upper 2",
            notes: "VeloRange Upper (1 - 127)"
        ),
        "velofade_lower_2": ZCParam(
            name: "velofade_lower_2",
            offset: 0x0012,
            size: 1,
            minValue: 0, maxValue: 127,
            defaultValue: 0,
            label: "VeloFade Lower 2",
            notes: "VeloFade Lower (0 - 127)"
        ),
        "velofade_upper_2": ZCParam(
            name: "velofade_upper_2",
            offset: 0x0013,
            size: 1,
            minValue: 0, maxValue: 127,
            defaultValue: 0,
            label: "VeloFade Upper 2",
            notes: "VeloFade Upper (0 - 127)"
        ),
        "partial_switch_3": ZCParam(
            name: "partial_switch_3",
            offset: 0x0014,
            size: 1,
            minValue: 0, maxValue: 1,
            defaultValue: 0,
            label: "Partial Switch 3",
            notes: "Partial Switch (0 - 1)"
        ),
        "keyrange_lower_3": ZCParam(
            name: "keyrange_lower_3",
            offset: 0x0015,
            size: 1,
            minValue: 0, maxValue: 127,
            defaultValue: 0,
            label: "KeyRange Lower 3",
            notes: "KeyRange Lower (0 - 127)"
        ),
        "keyrange_upper_3": ZCParam(
            name: "keyrange_upper_3",
            offset: 0x0016,
            size: 1,
            minValue: 0, maxValue: 127,
            defaultValue: 0,
            label: "KeyRange Upper 3",
            notes: "KeyRange Upper (0 - 127)"
        ),
        "keyfade_lower_3": ZCParam(
            name: "keyfade_lower_3",
            offset: 0x0017,
            size: 1,
            minValue: 0, maxValue: 127,
            defaultValue: 0,
            label: "KeyFade Lower 3",
            notes: "KeyFade Lower (0 - 127)"
        ),
        "keyfade_upper_3": ZCParam(
            name: "keyfade_upper_3",
            offset: 0x0018,
            size: 1,
            minValue: 0, maxValue: 127,
            defaultValue: 0,
            label: "KeyFade Upper 3",
            notes: "KeyFade Upper (0 - 127)"
        ),
        "velorange_lower_3": ZCParam(
            name: "velorange_lower_3",
            offset: 0x0019,
            size: 1,
            minValue: 1, maxValue: 127,
            defaultValue: 0,
            label: "VeloRange Lower 3",
            notes: "VeloRange Lower (1 - 127)"
        ),
        "velorange_upper_3": ZCParam(
            name: "velorange_upper_3",
            offset: 0x001A,
            size: 1,
            minValue: 1, maxValue: 127,
            defaultValue: 0,
            label: "VeloRange Upper 3",
            notes: "VeloRange Upper (1 - 127)"
        ),
        "velofade_lower_3": ZCParam(
            name: "velofade_lower_3",
            offset: 0x001B,
            size: 1,
            minValue: 0, maxValue: 127,
            defaultValue: 0,
            label: "VeloFade Lower 3",
            notes: "VeloFade Lower (0 - 127)"
        ),
        "velofade_upper_3": ZCParam(
            name: "velofade_upper_3",
            offset: 0x001C,
            size: 1,
            minValue: 0, maxValue: 127,
            defaultValue: 0,
            label: "VeloFade Upper 3",
            notes: "VeloFade Upper (0 - 127)"
        ),
        "partial_switch_4": ZCParam(
            name: "partial_switch_4",
            offset: 0x001D,
            size: 1,
            minValue: 0, maxValue: 1,
            defaultValue: 0,
            label: "Partial Switch 4",
            notes: "Partial Switch (0 - 1)"
        ),
        "keyrange_lower_4": ZCParam(
            name: "keyrange_lower_4",
            offset: 0x001E,
            size: 1,
            minValue: 0, maxValue: 127,
            defaultValue: 0,
            label: "KeyRange Lower 4",
            notes: "KeyRange Lower (0 - 127)"
        ),
        "keyrange_upper_4": ZCParam(
            name: "keyrange_upper_4",
            offset: 0x001F,
            size: 1,
            minValue: 0, maxValue: 127,
            defaultValue: 0,
            label: "KeyRange Upper 4",
            notes: "KeyRange Upper (0 - 127)"
        ),
        "keyfade_lower_4": ZCParam(
            name: "keyfade_lower_4",
            offset: 0x0020,
            size: 1,
            minValue: 0, maxValue: 127,
            defaultValue: 0,
            label: "KeyFade Lower 4",
            notes: "KeyFade Lower (0 - 127)"
        ),
        "keyfade_upper_4": ZCParam(
            name: "keyfade_upper_4",
            offset: 0x0021,
            size: 1,
            minValue: 0, maxValue: 127,
            defaultValue: 0,
            label: "KeyFade Upper 4",
            notes: "KeyFade Upper (0 - 127)"
        ),
        "velorange_lower_4": ZCParam(
            name: "velorange_lower_4",
            offset: 0x0022,
            size: 1,
            minValue: 1, maxValue: 127,
            defaultValue: 0,
            label: "VeloRange Lower 4",
            notes: "VeloRange Lower (1 - 127)"
        ),
        "velorange_upper_4": ZCParam(
            name: "velorange_upper_4",
            offset: 0x0023,
            size: 1,
            minValue: 1, maxValue: 127,
            defaultValue: 0,
            label: "VeloRange Upper 4",
            notes: "VeloRange Upper (1 - 127)"
        ),
        "velofade_lower_4": ZCParam(
            name: "velofade_lower_4",
            offset: 0x0024,
            size: 1,
            minValue: 0, maxValue: 127,
            defaultValue: 0,
            label: "VeloFade Lower 4",
            notes: "VeloFade Lower (0 - 127)"
        ),
        "velofade_upper_4": ZCParam(
            name: "velofade_upper_4",
            offset: 0x0025,
            size: 1,
            minValue: 0, maxValue: 127,
            defaultValue: 0,
            label: "VeloFade Upper 4",
            notes: "VeloFade Upper (0 - 127)"
        ),
    ]

    /// TONE_PARTIAL: 89 parameters, total size 0x100 bytes
    public static let TONE_PARTIAL: [String: ZCParam] = [
        "level": ZCParam(
            name: "level",
            offset: 0x0000,
            size: 1,
            minValue: 0, maxValue: 127,
            defaultValue: 0,
            label: "Level",
            notes: "Level (0 - 127)"
        ),
        "coarse_tune": ZCParam(
            name: "coarse_tune",
            offset: 0x0001,
            size: 1,
            minValue: 16, maxValue: 112,
            defaultValue: 0,
            label: "Coarse Tune",
            notes: "Coarse Tune (16 - 112)"
        ),
        "fine_tune": ZCParam(
            name: "fine_tune",
            offset: 0x0002,
            size: 1,
            minValue: 14, maxValue: 114,
            defaultValue: 0,
            label: "Fine Tune",
            notes: "Fine Tune (14 - 114)"
        ),
        "random_pitch_depth": ZCParam(
            name: "random_pitch_depth",
            offset: 0x0003,
            size: 4,
            minValue: 0, maxValue: 1200,
            defaultValue: 0,
            label: "Random Pitch Depth",
            notes: "Random Pitch Depth (0 - 1200)"
        ),
        "pan": ZCParam(
            name: "pan",
            offset: 0x0007,
            size: 1,
            minValue: 0, maxValue: 127,
            defaultValue: 0,
            label: "Pan",
            notes: "Pan (0 - 127)"
        ),
        "pan_keyfollow": ZCParam(
            name: "pan_keyfollow",
            offset: 0x0008,
            size: 2,
            minValue: 28, maxValue: 228,
            defaultValue: 0,
            label: "Pan Keyfollow",
            notes: "Pan Keyfollow (28 - 228)"
        ),
        "random_pan_depth": ZCParam(
            name: "random_pan_depth",
            offset: 0x000A,
            size: 1,
            minValue: 0, maxValue: 63,
            defaultValue: 0,
            label: "Random Pan Depth",
            notes: "Random Pan Depth (0 - 63)"
        ),
        "alternate_pan_depth": ZCParam(
            name: "alternate_pan_depth",
            offset: 0x000B,
            size: 1,
            minValue: 0, maxValue: 127,
            defaultValue: 0,
            label: "Alternate Pan Depth",
            notes: "Alternate Pan Depth (0 - 127)"
        ),
        "envelope_mode": ZCParam(
            name: "envelope_mode",
            offset: 0x000C,
            size: 1,
            minValue: 0, maxValue: 1,
            defaultValue: 0,
            label: "Envelope Mode",
            notes: "Envelope Mode (0 - 1)"
        ),
        "delay_mode": ZCParam(
            name: "delay_mode",
            offset: 0x000D,
            size: 1,
            minValue: 0, maxValue: 3,
            defaultValue: 0,
            label: "Delay Mode",
            notes: "Delay Mode (0 - 3)"
        ),
        "delaytime_sync": ZCParam(
            name: "delaytime_sync",
            offset: 0x000E,
            size: 1,
            minValue: 0, maxValue: 1,
            defaultValue: 0,
            label: "DelayTime Sync",
            notes: "DelayTime Sync (0 - 1)"
        ),
        "delaytime_note": ZCParam(
            name: "delaytime_note",
            offset: 0x000F,
            size: 1,
            minValue: 0, maxValue: 21,
            defaultValue: 0,
            label: "DelayTime (note)",
            notes: "DelayTime (note) (0 - 21)"
        ),
        "delaytime": ZCParam(
            name: "delaytime",
            offset: 0x0010,
            size: 4,
            minValue: 0, maxValue: 1023,
            defaultValue: 0,
            label: "DelayTime",
            notes: "DelayTime (0 - 1023)"
        ),
        "chorus_send_level": ZCParam(
            name: "chorus_send_level",
            offset: 0x0014,
            size: 1,
            minValue: 0, maxValue: 127,
            defaultValue: 0,
            label: "Chorus Send Level",
            notes: "Chorus Send Level (0 - 127)"
        ),
        "reverb_send_level": ZCParam(
            name: "reverb_send_level",
            offset: 0x0015,
            size: 1,
            minValue: 0, maxValue: 127,
            defaultValue: 0,
            label: "Reverb Send Level",
            notes: "Reverb Send Level (0 - 127)"
        ),
        "receive_bender": ZCParam(
            name: "receive_bender",
            offset: 0x0016,
            size: 1,
            minValue: 0, maxValue: 1,
            defaultValue: 0,
            label: "Receive Bender",
            notes: "Receive Bender (0 - 1)"
        ),
        "receive_expression": ZCParam(
            name: "receive_expression",
            offset: 0x0017,
            size: 1,
            minValue: 0, maxValue: 1,
            defaultValue: 0,
            label: "Receive Expression",
            notes: "Receive Expression (0 - 1)"
        ),
        "receive_hold_1": ZCParam(
            name: "receive_hold_1",
            offset: 0x0018,
            size: 1,
            minValue: 0, maxValue: 1,
            defaultValue: 0,
            label: "Receive Hold-1",
            notes: "Receive Hold-1 (0 - 1)"
        ),
        "redamper_switch": ZCParam(
            name: "redamper_switch",
            offset: 0x0019,
            size: 1,
            minValue: 0, maxValue: 1,
            defaultValue: 0,
            label: "Redamper Switch",
            notes: "Redamper Switch (0 - 1)"
        ),
        "output_assign": ZCParam(
            name: "output_assign",
            offset: 0x001A,
            size: 1,
            minValue: 0, maxValue: 1,
            defaultValue: 0,
            label: "Output Assign",
            notes: "Output Assign (0 - 1)"
        ),
        "wave_group_type": ZCParam(
            name: "wave_group_type",
            offset: 0x001B,
            size: 1,
            minValue: 0, maxValue: 3,
            defaultValue: 0,
            label: "Wave Group Type",
            notes: "Wave Group Type (0 - 3)"
        ),
        "wave_group_id": ZCParam(
            name: "wave_group_id",
            offset: 0x001C,
            size: 4,
            minValue: 0, maxValue: 16383,
            defaultValue: 0,
            label: "Wave Group ID",
            notes: "Wave Group ID (0 - 16383)"
        ),
        "wave_number_l": ZCParam(
            name: "wave_number_l",
            offset: 0x0020,
            size: 4,
            minValue: 0, maxValue: 16383,
            defaultValue: 0,
            label: "Wave Number L",
            notes: "Wave Number L (0 - 16383)"
        ),
        "wave_number_r": ZCParam(
            name: "wave_number_r",
            offset: 0x0024,
            size: 4,
            minValue: 0, maxValue: 16383,
            defaultValue: 0,
            label: "Wave Number R",
            notes: "Wave Number R (0 - 16383)"
        ),
        "gain": ZCParam(
            name: "gain",
            offset: 0x0028,
            size: 1,
            minValue: 0, maxValue: 5,
            defaultValue: 0,
            label: "Gain",
            notes: "Gain (0 - 5)"
        ),
        "fxm_switch": ZCParam(
            name: "fxm_switch",
            offset: 0x0029,
            size: 1,
            minValue: 0, maxValue: 1,
            defaultValue: 0,
            label: "FXM Switch",
            notes: "FXM Switch (0 - 1)"
        ),
        "fxm_color": ZCParam(
            name: "fxm_color",
            offset: 0x002A,
            size: 1,
            minValue: 1, maxValue: 4,
            defaultValue: 0,
            label: "FXM Color",
            notes: "FXM Color (1 - 4)"
        ),
        "fxm_depth": ZCParam(
            name: "fxm_depth",
            offset: 0x002B,
            size: 1,
            minValue: 0, maxValue: 16,
            defaultValue: 0,
            label: "FXM Depth",
            notes: "FXM Depth (0 - 16)"
        ),
        "pitch_keyfollow": ZCParam(
            name: "pitch_keyfollow",
            offset: 0x002C,
            size: 4,
            minValue: 824, maxValue: 1224,
            defaultValue: 0,
            label: "Pitch Keyfollow",
            notes: "Pitch Keyfollow (824 - 1224)"
        ),
        "soft_eq_sens": ZCParam(
            name: "soft_eq_sens",
            offset: 0x0030,
            size: 1,
            minValue: 0, maxValue: 100,
            defaultValue: 0,
            label: "Soft EQ Sens",
            notes: "Soft EQ Sens (0 - 100)"
        ),
        "tvf_filter_type": ZCParam(
            name: "tvf_filter_type",
            offset: 0x0031,
            size: 1,
            minValue: 0, maxValue: 6,
            defaultValue: 0,
            label: "TVF Filter Type",
            notes: "TVF Filter Type (0 - 6)"
        ),
        "tvf_cutoff_frequency": ZCParam(
            name: "tvf_cutoff_frequency",
            offset: 0x0032,
            size: 4,
            minValue: 0, maxValue: 1023,
            defaultValue: 0,
            label: "TVF Cutoff Frequency",
            notes: "TVF Cutoff Frequency (0 - 1023)"
        ),
        "tvf_cutoff_keyfollow": ZCParam(
            name: "tvf_cutoff_keyfollow",
            offset: 0x0036,
            size: 4,
            minValue: 824, maxValue: 1224,
            defaultValue: 0,
            label: "TVF Cutoff Keyfollow",
            notes: "TVF Cutoff Keyfollow (824 - 1224)"
        ),
        "tvf_cutoff_velocity_curve": ZCParam(
            name: "tvf_cutoff_velocity_curve",
            offset: 0x003A,
            size: 1,
            minValue: 0, maxValue: 7,
            defaultValue: 0,
            label: "TVF Cutoff Velocity Curve",
            notes: "TVF Cutoff Velocity Curve (0 - 7)"
        ),
        "tvf_cutoff_velocity_sens": ZCParam(
            name: "tvf_cutoff_velocity_sens",
            offset: 0x003B,
            size: 2,
            minValue: 28, maxValue: 228,
            defaultValue: 0,
            label: "TVF Cutoff Velocity Sens",
            notes: "TVF Cutoff Velocity Sens (28 - 228)"
        ),
        "tvf_resonance": ZCParam(
            name: "tvf_resonance",
            offset: 0x003D,
            size: 4,
            minValue: 0, maxValue: 1023,
            defaultValue: 0,
            label: "TVF Resonance",
            notes: "TVF Resonance (0 - 1023)"
        ),
        "tvf_resonance_velocity_sens": ZCParam(
            name: "tvf_resonance_velocity_sens",
            offset: 0x0041,
            size: 2,
            minValue: 28, maxValue: 228,
            defaultValue: 0,
            label: "TVF Resonance Velocity Sens",
            notes: "TVF Resonance Velocity Sens (28 - 228)"
        ),
        "bias_level": ZCParam(
            name: "bias_level",
            offset: 0x0043,
            size: 2,
            minValue: 28, maxValue: 228,
            defaultValue: 0,
            label: "Bias Level",
            notes: "Bias Level (28 - 228)"
        ),
        "bias_position": ZCParam(
            name: "bias_position",
            offset: 0x0045,
            size: 1,
            minValue: 0, maxValue: 127,
            defaultValue: 0,
            label: "Bias Position",
            notes: "Bias Position (0 - 127)"
        ),
        "bias_direction": ZCParam(
            name: "bias_direction",
            offset: 0x0046,
            size: 1,
            minValue: 0, maxValue: 3,
            defaultValue: 0,
            label: "Bias Direction",
            notes: "Bias Direction (0 - 3)"
        ),
        "tva_level_velocity_curve": ZCParam(
            name: "tva_level_velocity_curve",
            offset: 0x0047,
            size: 1,
            minValue: 0, maxValue: 7,
            defaultValue: 0,
            label: "TVA Level Velocity Curve",
            notes: "TVA Level Velocity Curve (0 - 7)"
        ),
        "tva_level_velocity_sens": ZCParam(
            name: "tva_level_velocity_sens",
            offset: 0x0048,
            size: 2,
            minValue: 28, maxValue: 228,
            defaultValue: 0,
            label: "TVA Level Velocity Sens",
            notes: "TVA Level Velocity Sens (28 - 228)"
        ),
        "pitch_env_time_keyfollow": ZCParam(
            name: "pitch_env_time_keyfollow",
            offset: 0x004A,
            size: 2,
            minValue: 28, maxValue: 228,
            defaultValue: 0,
            label: "Pitch Env Time Keyfollow",
            notes: "Pitch Env Time Keyfollow (28 - 228)"
        ),
        "tvf_env_time_keyfollow": ZCParam(
            name: "tvf_env_time_keyfollow",
            offset: 0x004C,
            size: 2,
            minValue: 28, maxValue: 228,
            defaultValue: 0,
            label: "TVF Env Time Keyfollow",
            notes: "TVF Env Time Keyfollow (28 - 228)"
        ),
        "tva_env_time_keyfollow": ZCParam(
            name: "tva_env_time_keyfollow",
            offset: 0x004E,
            size: 2,
            minValue: 28, maxValue: 228,
            defaultValue: 0,
            label: "TVA Env Time Keyfollow",
            notes: "TVA Env Time Keyfollow (28 - 228)"
        ),
        "vibrato_pitch_sens": ZCParam(
            name: "vibrato_pitch_sens",
            offset: 0x0050,
            size: 2,
            minValue: 28, maxValue: 228,
            defaultValue: 0,
            label: "Vibrato Pitch Sens",
            notes: "Vibrato Pitch Sens (28 - 228)"
        ),
        "vibrato_cutoff_sens": ZCParam(
            name: "vibrato_cutoff_sens",
            offset: 0x0052,
            size: 2,
            minValue: 28, maxValue: 228,
            defaultValue: 0,
            label: "Vibrato Cutoff Sens",
            notes: "Vibrato Cutoff Sens (28 - 228)"
        ),
        "vibrato_level_sens": ZCParam(
            name: "vibrato_level_sens",
            offset: 0x0054,
            size: 2,
            minValue: 28, maxValue: 228,
            defaultValue: 0,
            label: "Vibrato Level Sens",
            notes: "Vibrato Level Sens (28 - 228)"
        ),
        "source_1": ZCParam(
            name: "source_1",
            offset: 0x0056,
            size: 1,
            minValue: 0, maxValue: 108,
            defaultValue: 0,
            label: "Source 1",
            notes: "Source (0 - 108)"
        ),
        "destination_1_1": ZCParam(
            name: "destination_1_1",
            offset: 0x0057,
            size: 1,
            minValue: 0, maxValue: 48,
            defaultValue: 0,
            label: "Destination 1 1",
            notes: "Destination 1 (0 - 48)"
        ),
        "sens_1_1": ZCParam(
            name: "sens_1_1",
            offset: 0x0058,
            size: 1,
            minValue: 1, maxValue: 127,
            defaultValue: 0,
            label: "Sens 1 1",
            notes: "Sens 1 (1 - 127)"
        ),
        "destination_2_1": ZCParam(
            name: "destination_2_1",
            offset: 0x0059,
            size: 1,
            minValue: 0, maxValue: 48,
            defaultValue: 0,
            label: "Destination 2 1",
            notes: "Destination 2 (0 - 48)"
        ),
        "sens_2_1": ZCParam(
            name: "sens_2_1",
            offset: 0x005A,
            size: 1,
            minValue: 1, maxValue: 127,
            defaultValue: 0,
            label: "Sens 2 1",
            notes: "Sens 2 (1 - 127)"
        ),
        "destination_3_1": ZCParam(
            name: "destination_3_1",
            offset: 0x005B,
            size: 1,
            minValue: 0, maxValue: 48,
            defaultValue: 0,
            label: "Destination 3 1",
            notes: "Destination 3 (0 - 48)"
        ),
        "sens_3_1": ZCParam(
            name: "sens_3_1",
            offset: 0x005C,
            size: 1,
            minValue: 1, maxValue: 127,
            defaultValue: 0,
            label: "Sens 3 1",
            notes: "Sens 3 (1 - 127)"
        ),
        "destination_4_1": ZCParam(
            name: "destination_4_1",
            offset: 0x005D,
            size: 1,
            minValue: 0, maxValue: 48,
            defaultValue: 0,
            label: "Destination 4 1",
            notes: "Destination 4 (0 - 48)"
        ),
        "sens_4_1": ZCParam(
            name: "sens_4_1",
            offset: 0x005E,
            size: 1,
            minValue: 1, maxValue: 127,
            defaultValue: 0,
            label: "Sens 4 1",
            notes: "Sens 4 (1 - 127)"
        ),
        "source_2": ZCParam(
            name: "source_2",
            offset: 0x005F,
            size: 1,
            minValue: 0, maxValue: 108,
            defaultValue: 0,
            label: "Source 2",
            notes: "Source (0 - 108)"
        ),
        "destination_1_2": ZCParam(
            name: "destination_1_2",
            offset: 0x0060,
            size: 1,
            minValue: 0, maxValue: 48,
            defaultValue: 0,
            label: "Destination 1 2",
            notes: "Destination 1 (0 - 48)"
        ),
        "sens_1_2": ZCParam(
            name: "sens_1_2",
            offset: 0x0061,
            size: 1,
            minValue: 1, maxValue: 127,
            defaultValue: 0,
            label: "Sens 1 2",
            notes: "Sens 1 (1 - 127)"
        ),
        "destination_2_2": ZCParam(
            name: "destination_2_2",
            offset: 0x0062,
            size: 1,
            minValue: 0, maxValue: 48,
            defaultValue: 0,
            label: "Destination 2 2",
            notes: "Destination 2 (0 - 48)"
        ),
        "sens_2_2": ZCParam(
            name: "sens_2_2",
            offset: 0x0063,
            size: 1,
            minValue: 1, maxValue: 127,
            defaultValue: 0,
            label: "Sens 2 2",
            notes: "Sens 2 (1 - 127)"
        ),
        "destination_3_2": ZCParam(
            name: "destination_3_2",
            offset: 0x0064,
            size: 1,
            minValue: 0, maxValue: 48,
            defaultValue: 0,
            label: "Destination 3 2",
            notes: "Destination 3 (0 - 48)"
        ),
        "sens_3_2": ZCParam(
            name: "sens_3_2",
            offset: 0x0065,
            size: 1,
            minValue: 1, maxValue: 127,
            defaultValue: 0,
            label: "Sens 3 2",
            notes: "Sens 3 (1 - 127)"
        ),
        "destination_4_2": ZCParam(
            name: "destination_4_2",
            offset: 0x0066,
            size: 1,
            minValue: 0, maxValue: 48,
            defaultValue: 0,
            label: "Destination 4 2",
            notes: "Destination 4 (0 - 48)"
        ),
        "sens_4_2": ZCParam(
            name: "sens_4_2",
            offset: 0x0067,
            size: 1,
            minValue: 1, maxValue: 127,
            defaultValue: 0,
            label: "Sens 4 2",
            notes: "Sens 4 (1 - 127)"
        ),
        "source_3": ZCParam(
            name: "source_3",
            offset: 0x0068,
            size: 1,
            minValue: 0, maxValue: 108,
            defaultValue: 0,
            label: "Source 3",
            notes: "Source (0 - 108)"
        ),
        "destination_1_3": ZCParam(
            name: "destination_1_3",
            offset: 0x0069,
            size: 1,
            minValue: 0, maxValue: 48,
            defaultValue: 0,
            label: "Destination 1 3",
            notes: "Destination 1 (0 - 48)"
        ),
        "sens_1_3": ZCParam(
            name: "sens_1_3",
            offset: 0x006A,
            size: 1,
            minValue: 1, maxValue: 127,
            defaultValue: 0,
            label: "Sens 1 3",
            notes: "Sens 1 (1 - 127)"
        ),
        "destination_2_3": ZCParam(
            name: "destination_2_3",
            offset: 0x006B,
            size: 1,
            minValue: 0, maxValue: 48,
            defaultValue: 0,
            label: "Destination 2 3",
            notes: "Destination 2 (0 - 48)"
        ),
        "sens_2_3": ZCParam(
            name: "sens_2_3",
            offset: 0x006C,
            size: 1,
            minValue: 1, maxValue: 127,
            defaultValue: 0,
            label: "Sens 2 3",
            notes: "Sens 2 (1 - 127)"
        ),
        "destination_3_3": ZCParam(
            name: "destination_3_3",
            offset: 0x006D,
            size: 1,
            minValue: 0, maxValue: 48,
            defaultValue: 0,
            label: "Destination 3 3",
            notes: "Destination 3 (0 - 48)"
        ),
        "sens_3_3": ZCParam(
            name: "sens_3_3",
            offset: 0x006E,
            size: 1,
            minValue: 1, maxValue: 127,
            defaultValue: 0,
            label: "Sens 3 3",
            notes: "Sens 3 (1 - 127)"
        ),
        "destination_4_3": ZCParam(
            name: "destination_4_3",
            offset: 0x006F,
            size: 1,
            minValue: 0, maxValue: 48,
            defaultValue: 0,
            label: "Destination 4 3",
            notes: "Destination 4 (0 - 48)"
        ),
        "sens_4_3": ZCParam(
            name: "sens_4_3",
            offset: 0x0070,
            size: 1,
            minValue: 1, maxValue: 127,
            defaultValue: 0,
            label: "Sens 4 3",
            notes: "Sens 4 (1 - 127)"
        ),
        "source_4": ZCParam(
            name: "source_4",
            offset: 0x0071,
            size: 1,
            minValue: 0, maxValue: 108,
            defaultValue: 0,
            label: "Source 4",
            notes: "Source (0 - 108)"
        ),
        "destination_1_4": ZCParam(
            name: "destination_1_4",
            offset: 0x0072,
            size: 1,
            minValue: 0, maxValue: 48,
            defaultValue: 0,
            label: "Destination 1 4",
            notes: "Destination 1 (0 - 48)"
        ),
        "sens_1_4": ZCParam(
            name: "sens_1_4",
            offset: 0x0073,
            size: 1,
            minValue: 1, maxValue: 127,
            defaultValue: 0,
            label: "Sens 1 4",
            notes: "Sens 1 (1 - 127)"
        ),
        "destination_2_4": ZCParam(
            name: "destination_2_4",
            offset: 0x0074,
            size: 1,
            minValue: 0, maxValue: 48,
            defaultValue: 0,
            label: "Destination 2 4",
            notes: "Destination 2 (0 - 48)"
        ),
        "sens_2_4": ZCParam(
            name: "sens_2_4",
            offset: 0x0075,
            size: 1,
            minValue: 1, maxValue: 127,
            defaultValue: 0,
            label: "Sens 2 4",
            notes: "Sens 2 (1 - 127)"
        ),
        "destination_3_4": ZCParam(
            name: "destination_3_4",
            offset: 0x0076,
            size: 1,
            minValue: 0, maxValue: 48,
            defaultValue: 0,
            label: "Destination 3 4",
            notes: "Destination 3 (0 - 48)"
        ),
        "sens_3_4": ZCParam(
            name: "sens_3_4",
            offset: 0x0077,
            size: 1,
            minValue: 1, maxValue: 127,
            defaultValue: 0,
            label: "Sens 3 4",
            notes: "Sens 3 (1 - 127)"
        ),
        "destination_4_4": ZCParam(
            name: "destination_4_4",
            offset: 0x0078,
            size: 1,
            minValue: 0, maxValue: 48,
            defaultValue: 0,
            label: "Destination 4 4",
            notes: "Destination 4 (0 - 48)"
        ),
        "sens_4_4": ZCParam(
            name: "sens_4_4",
            offset: 0x0079,
            size: 1,
            minValue: 1, maxValue: 127,
            defaultValue: 0,
            label: "Sens 4 4",
            notes: "Sens 4 (1 - 127)"
        ),
        "damper_free_note": ZCParam(
            name: "damper_free_note",
            offset: 0x007A,
            size: 1,
            minValue: 0, maxValue: 127,
            defaultValue: 0,
            label: "Damper Free Note",
            notes: "Damper Free Note (0 - 127)"
        ),
        "stereo_width": ZCParam(
            name: "stereo_width",
            offset: 0x007B,
            size: 1,
            minValue: 0, maxValue: 100,
            defaultValue: 0,
            label: "Stereo Width",
            notes: "Stereo Width (0 - 100)"
        ),
        "stereo_detune": ZCParam(
            name: "stereo_detune",
            offset: 0x007C,
            size: 1,
            minValue: 14, maxValue: 114,
            defaultValue: 0,
            label: "Stereo Detune",
            notes: "Stereo Detune (14 - 114)"
        ),
        "damper_free_decay_offset": ZCParam(
            name: "damper_free_decay_offset",
            offset: 0x007D,
            size: 2,
            minValue: 28, maxValue: 228,
            defaultValue: 0,
            label: "Damper Free Decay Offset",
            notes: "Damper Free Decay Offset (28 - 228)"
        ),
        "wave_tempo_sync": ZCParam(
            name: "wave_tempo_sync",
            offset: 0x007F,
            size: 1,
            minValue: 0, maxValue: 1,
            defaultValue: 0,
            label: "Wave Tempo Sync",
            notes: "Wave Tempo Sync (0 - 1)"
        ),
    ]

    /// TONE_SYNTH_COMMON: 8 parameters, total size 0x9 bytes
    public static let TONE_SYNTH_COMMON: [String: ZCParam] = [
        "unison_switch": ZCParam(
            name: "unison_switch",
            offset: 0x0000,
            size: 1,
            minValue: 0, maxValue: 1,
            defaultValue: 0,
            label: "Unison Switch",
            notes: "Unison Switch (0 - 1)"
        ),
        "unison_size": ZCParam(
            name: "unison_size",
            offset: 0x0001,
            size: 1,
            minValue: 2, maxValue: 8,
            defaultValue: 0,
            label: "Unison Size",
            notes: "Unison Size (2 - 8)"
        ),
        "unison_detune": ZCParam(
            name: "unison_detune",
            offset: 0x0002,
            size: 1,
            minValue: 0, maxValue: 100,
            defaultValue: 0,
            label: "Unison Detune",
            notes: "Unison Detune (0 - 100)"
        ),
        "bend_range_fine_up": ZCParam(
            name: "bend_range_fine_up",
            offset: 0x0003,
            size: 1,
            minValue: 0, maxValue: 100,
            defaultValue: 0,
            label: "Bend Range Fine Up",
            notes: "Bend Range Fine Up (0 - 100)"
        ),
        "bend_range_fine_down": ZCParam(
            name: "bend_range_fine_down",
            offset: 0x0004,
            size: 1,
            minValue: 0, maxValue: 100,
            defaultValue: 0,
            label: "Bend Range Fine Down",
            notes: "Bend Range Fine Down (0 - 100)"
        ),
        "pitch_drift": ZCParam(
            name: "pitch_drift",
            offset: 0x0005,
            size: 2,
            minValue: 0, maxValue: 255,
            defaultValue: 0,
            label: "Pitch Drift",
            notes: "Pitch Drift (0 - 255)"
        ),
        "pitch_drift_cycle_number": ZCParam(
            name: "pitch_drift_cycle_number",
            offset: 0x0007,
            size: 1,
            minValue: 0, maxValue: 8,
            defaultValue: 0,
            label: "Pitch Drift Cycle Number",
            notes: "Pitch Drift Cycle Number (0 - 8)"
        ),
        "condition": ZCParam(
            name: "condition",
            offset: 0x0008,
            size: 1,
            minValue: 0, maxValue: 100,
            defaultValue: 0,
            label: "Condition",
            notes: "Condition (0 - 100)"
        ),
    ]

    /// TONE_SYNTH_PMT: 17 parameters, total size 0x17 bytes
    public static let TONE_SYNTH_PMT: [String: ZCParam] = [
        "structure1_2": ZCParam(
            name: "structure1_2",
            offset: 0x0000,
            size: 1,
            minValue: 0, maxValue: 4,
            defaultValue: 0,
            label: "Structure1-2",
            notes: "Structure1-2 (0 - 4)"
        ),
        "structure3_4": ZCParam(
            name: "structure3_4",
            offset: 0x0001,
            size: 1,
            minValue: 0, maxValue: 4,
            defaultValue: 0,
            label: "Structure3-4",
            notes: "Structure3-4 (0 - 4)"
        ),
        "ring1_2_level": ZCParam(
            name: "ring1_2_level",
            offset: 0x0002,
            size: 1,
            minValue: 0, maxValue: 127,
            defaultValue: 0,
            label: "RING1-2 Level",
            notes: "RING1-2 Level (0 - 127)"
        ),
        "ring3_4_level": ZCParam(
            name: "ring3_4_level",
            offset: 0x0003,
            size: 1,
            minValue: 0, maxValue: 127,
            defaultValue: 0,
            label: "RING3-4 Level",
            notes: "RING3-4 Level (0 - 127)"
        ),
        "ring_osc1_level": ZCParam(
            name: "ring_osc1_level",
            offset: 0x0004,
            size: 1,
            minValue: 0, maxValue: 127,
            defaultValue: 0,
            label: "RING OSC1 Level",
            notes: "RING OSC1 Level (0 - 127)"
        ),
        "ring_osc2_level": ZCParam(
            name: "ring_osc2_level",
            offset: 0x0005,
            size: 1,
            minValue: 0, maxValue: 127,
            defaultValue: 0,
            label: "RING OSC2 Level",
            notes: "RING OSC2 Level (0 - 127)"
        ),
        "ring_osc3_level": ZCParam(
            name: "ring_osc3_level",
            offset: 0x0006,
            size: 1,
            minValue: 0, maxValue: 127,
            defaultValue: 0,
            label: "RING OSC3 Level",
            notes: "RING OSC3 Level (0 - 127)"
        ),
        "ring_osc4_level": ZCParam(
            name: "ring_osc4_level",
            offset: 0x0007,
            size: 1,
            minValue: 0, maxValue: 127,
            defaultValue: 0,
            label: "RING OSC4 Level",
            notes: "RING OSC4 Level (0 - 127)"
        ),
        "crossmod1_2_depth": ZCParam(
            name: "crossmod1_2_depth",
            offset: 0x0008,
            size: 4,
            minValue: 0, maxValue: 10800,
            defaultValue: 0,
            label: "CrossMod1-2 Depth",
            notes: "CrossMod1-2 Depth (0 - 10800)"
        ),
        "crossmod3_4_depth": ZCParam(
            name: "crossmod3_4_depth",
            offset: 0x000C,
            size: 4,
            minValue: 0, maxValue: 10800,
            defaultValue: 0,
            label: "CrossMod3-4 Depth",
            notes: "CrossMod3-4 Depth (0 - 10800)"
        ),
        "crossmod_osc1_level": ZCParam(
            name: "crossmod_osc1_level",
            offset: 0x0010,
            size: 1,
            minValue: 0, maxValue: 127,
            defaultValue: 0,
            label: "CrossMod OSC1 Level",
            notes: "CrossMod OSC1 Level (0 - 127)"
        ),
        "crossmod_osc2_level": ZCParam(
            name: "crossmod_osc2_level",
            offset: 0x0011,
            size: 1,
            minValue: 0, maxValue: 127,
            defaultValue: 0,
            label: "CrossMod OSC2 Level",
            notes: "CrossMod OSC2 Level (0 - 127)"
        ),
        "crossmod_osc3_level": ZCParam(
            name: "crossmod_osc3_level",
            offset: 0x0012,
            size: 1,
            minValue: 0, maxValue: 127,
            defaultValue: 0,
            label: "CrossMod OSC3 Level",
            notes: "CrossMod OSC3 Level (0 - 127)"
        ),
        "crossmod_osc4_level": ZCParam(
            name: "crossmod_osc4_level",
            offset: 0x0013,
            size: 1,
            minValue: 0, maxValue: 127,
            defaultValue: 0,
            label: "CrossMod OSC4 Level",
            notes: "CrossMod OSC4 Level (0 - 127)"
        ),
        "partial_phase_lock": ZCParam(
            name: "partial_phase_lock",
            offset: 0x0014,
            size: 1,
            minValue: 0, maxValue: 1,
            defaultValue: 0,
            label: "Partial Phase Lock",
            notes: "Partial Phase Lock (0 - 1)"
        ),
        "crossmod2_1_2_depth": ZCParam(
            name: "crossmod2_1_2_depth",
            offset: 0x0015,
            size: 1,
            minValue: 0, maxValue: 127,
            defaultValue: 0,
            label: "CrossMod2 1-2 Depth",
            notes: "CrossMod2 1-2 Depth (0 - 127)"
        ),
        "crossmod2_3_4_depth": ZCParam(
            name: "crossmod2_3_4_depth",
            offset: 0x0016,
            size: 1,
            minValue: 0, maxValue: 127,
            defaultValue: 0,
            label: "CrossMod2 3-4 Depth",
            notes: "CrossMod2 3-4 Depth (0 - 127)"
        ),
    ]

    /// TONE_SYNTH_PARTIAL: 22 parameters, total size 0x1E bytes
    public static let TONE_SYNTH_PARTIAL: [String: ZCParam] = [
        "osc_type": ZCParam(
            name: "osc_type",
            offset: 0x0000,
            size: 1,
            minValue: 0, maxValue: 4,
            defaultValue: 0,
            label: "OSC Type",
            notes: "OSC Type (0 - 4)"
        ),
        "va_waveform": ZCParam(
            name: "va_waveform",
            offset: 0x0001,
            size: 1,
            minValue: 0, maxValue: 8,
            defaultValue: 0,
            label: "VA Waveform",
            notes: "VA Waveform (0 - 8)"
        ),
        "pcm_sync_wave_number": ZCParam(
            name: "pcm_sync_wave_number",
            offset: 0x0002,
            size: 4,
            minValue: 0, maxValue: 16383,
            defaultValue: 0,
            label: "PCM-Sync Wave Number",
            notes: "PCM-Sync Wave Number (0 - 16383)"
        ),
        "pulse_width": ZCParam(
            name: "pulse_width",
            offset: 0x0006,
            size: 1,
            minValue: 0, maxValue: 127,
            defaultValue: 0,
            label: "Pulse Width",
            notes: "Pulse Width (0 - 127)"
        ),
        "pwm_depth": ZCParam(
            name: "pwm_depth",
            offset: 0x0007,
            size: 1,
            minValue: 1, maxValue: 127,
            defaultValue: 0,
            label: "PWM Depth",
            notes: "PWM Depth (1 - 127)"
        ),
        "supersaw_detune": ZCParam(
            name: "supersaw_detune",
            offset: 0x0008,
            size: 1,
            minValue: 0, maxValue: 127,
            defaultValue: 0,
            label: "SuperSAW Detune",
            notes: "SuperSAW Detune (0 - 127)"
        ),
        "click_type": ZCParam(
            name: "click_type",
            offset: 0x0009,
            size: 1,
            minValue: 0, maxValue: 3,
            defaultValue: 0,
            label: "Click Type",
            notes: "Click Type (0 - 3)"
        ),
        "hpf_cutoff": ZCParam(
            name: "hpf_cutoff",
            offset: 0x000A,
            size: 4,
            minValue: 0, maxValue: 1023,
            defaultValue: 0,
            label: "HPF Cutoff",
            notes: "HPF Cutoff (0 - 1023)"
        ),
        "filter_type": ZCParam(
            name: "filter_type",
            offset: 0x000E,
            size: 1,
            minValue: 0, maxValue: 1,
            defaultValue: 0,
            label: "Filter Type",
            notes: "Filter Type (0 - 1)"
        ),
        "filter_slope": ZCParam(
            name: "filter_slope",
            offset: 0x000F,
            size: 1,
            minValue: 0, maxValue: 2,
            defaultValue: 0,
            label: "Filter Slope",
            notes: "Filter Slope (0 - 2)"
        ),
        "adsrenv_switch": ZCParam(
            name: "adsrenv_switch",
            offset: 0x0010,
            size: 1,
            minValue: 0, maxValue: 1,
            defaultValue: 0,
            label: "ADSREnv Switch",
            notes: "ADSREnv Switch (0 - 1)"
        ),
        "fat": ZCParam(
            name: "fat",
            offset: 0x0011,
            size: 1,
            minValue: 0, maxValue: 127,
            defaultValue: 0,
            label: "Fat",
            notes: "Fat (0 - 127)"
        ),
        "vcf_type": ZCParam(
            name: "vcf_type",
            offset: 0x0012,
            size: 1,
            minValue: 0, maxValue: 3,
            defaultValue: 0,
            label: "VCF Type",
            notes: "VCF Type (0 - 3)"
        ),
        "osc_attenuator": ZCParam(
            name: "osc_attenuator",
            offset: 0x0013,
            size: 2,
            minValue: 0, maxValue: 255,
            defaultValue: 0,
            label: "OSC Attenuator",
            notes: "OSC Attenuator (0 - 255)"
        ),
        "cutoff_keyfollow_base_point": ZCParam(
            name: "cutoff_keyfollow_base_point",
            offset: 0x0015,
            size: 1,
            minValue: 0, maxValue: 127,
            defaultValue: 0,
            label: "Cutoff Keyfollow Base Point",
            notes: "Cutoff Keyfollow Base Point (0 - 127)"
        ),
        "va_waveform_invert_switch": ZCParam(
            name: "va_waveform_invert_switch",
            offset: 0x0016,
            size: 1,
            minValue: 0, maxValue: 1,
            defaultValue: 0,
            label: "VA Waveform Invert Switch",
            notes: "VA Waveform Invert Switch (0 - 1)"
        ),
        "tvf_env_fine_depth": ZCParam(
            name: "tvf_env_fine_depth",
            offset: 0x0017,
            size: 1,
            minValue: 1, maxValue: 127,
            defaultValue: 0,
            label: "TVF Env Fine Depth",
            notes: "TVF Env Fine Depth (1 - 127)"
        ),
        "penv_lfo_trigger_switch": ZCParam(
            name: "penv_lfo_trigger_switch",
            offset: 0x0018,
            size: 1,
            minValue: 0, maxValue: 1,
            defaultValue: 0,
            label: "PENV LFO Trigger Switch",
            notes: "PENV LFO Trigger Switch (0 - 1)"
        ),
        "fenv_lfo_trigger_switch": ZCParam(
            name: "fenv_lfo_trigger_switch",
            offset: 0x0019,
            size: 1,
            minValue: 0, maxValue: 1,
            defaultValue: 0,
            label: "FENV LFO Trigger Switch",
            notes: "FENV LFO Trigger Switch (0 - 1)"
        ),
        "aenv_lfo_trigger_switch": ZCParam(
            name: "aenv_lfo_trigger_switch",
            offset: 0x001A,
            size: 1,
            minValue: 0, maxValue: 1,
            defaultValue: 0,
            label: "AENV LFO Trigger Switch",
            notes: "AENV LFO Trigger Switch (0 - 1)"
        ),
        "pitch_down_depth": ZCParam(
            name: "pitch_down_depth",
            offset: 0x001B,
            size: 1,
            minValue: 0, maxValue: 120,
            defaultValue: 0,
            label: "Pitch Down Depth",
            notes: "Pitch Down Depth (0 - 120)"
        ),
        "va_init_phase": ZCParam(
            name: "va_init_phase",
            offset: 0x001D,
            size: 1,
            minValue: 0, maxValue: 72,
            defaultValue: 0,
            label: "VA Init Phase",
            notes: "VA Init Phase (0 - 72)"
        ),
    ]

    /// TONE_MFX: 48 parameters, total size 0x110 bytes
    public static let TONE_MFX: [String: ZCParam] = [
        "type": ZCParam(
            name: "type",
            offset: 0x0000,
            size: 1,
            minValue: 0, maxValue: 93,
            defaultValue: 0,
            label: "Type",
            notes: "Type (* appendix 1) (0 - 93)"
        ),
        "mfx_switch": ZCParam(
            name: "mfx_switch",
            offset: 0x0001,
            size: 1,
            minValue: 0, maxValue: 1,
            defaultValue: 0,
            label: "MFX Switch",
            notes: "MFX Switch (0 - 1)"
        ),
        "mfx_chorus_send_level": ZCParam(
            name: "mfx_chorus_send_level",
            offset: 0x0002,
            size: 1,
            minValue: 0, maxValue: 127,
            defaultValue: 0,
            label: "MFX Chorus Send Level",
            notes: "MFX Chorus Send Level (0 - 127)"
        ),
        "mfx_reverb_send_level": ZCParam(
            name: "mfx_reverb_send_level",
            offset: 0x0003,
            size: 1,
            minValue: 0, maxValue: 127,
            defaultValue: 0,
            label: "MFX Reverb Send Level",
            notes: "MFX Reverb Send Level (0 - 127)"
        ),
        "mfx_ctrlsrc_1": ZCParam(
            name: "mfx_ctrlsrc_1",
            offset: 0x0004,
            size: 1,
            minValue: 0, maxValue: 100,
            defaultValue: 0,
            label: "MFX CtrlSrc 1",
            notes: "MFX CtrlSrc 1 (0 - 100)"
        ),
        "mfx_ctrlsens_1": ZCParam(
            name: "mfx_ctrlsens_1",
            offset: 0x0005,
            size: 1,
            minValue: 1, maxValue: 127,
            defaultValue: 0,
            label: "MFX CtrlSens 1",
            notes: "MFX CtrlSens 1 (1 - 127)"
        ),
        "mfx_ctrlsrc_2": ZCParam(
            name: "mfx_ctrlsrc_2",
            offset: 0x0006,
            size: 1,
            minValue: 0, maxValue: 100,
            defaultValue: 0,
            label: "MFX CtrlSrc 2",
            notes: "MFX CtrlSrc 2 (0 - 100)"
        ),
        "mfx_ctrlsens_2": ZCParam(
            name: "mfx_ctrlsens_2",
            offset: 0x0007,
            size: 1,
            minValue: 1, maxValue: 127,
            defaultValue: 0,
            label: "MFX CtrlSens 2",
            notes: "MFX CtrlSens 2 (1 - 127)"
        ),
        "mfx_ctrlsrc_3": ZCParam(
            name: "mfx_ctrlsrc_3",
            offset: 0x0008,
            size: 1,
            minValue: 0, maxValue: 100,
            defaultValue: 0,
            label: "MFX CtrlSrc 3",
            notes: "MFX CtrlSrc 3 (0 - 100)"
        ),
        "mfx_ctrlsens_3": ZCParam(
            name: "mfx_ctrlsens_3",
            offset: 0x0009,
            size: 1,
            minValue: 1, maxValue: 127,
            defaultValue: 0,
            label: "MFX CtrlSens 3",
            notes: "MFX CtrlSens 3 (1 - 127)"
        ),
        "mfx_ctrlsrc_4": ZCParam(
            name: "mfx_ctrlsrc_4",
            offset: 0x000A,
            size: 1,
            minValue: 0, maxValue: 100,
            defaultValue: 0,
            label: "MFX CtrlSrc 4",
            notes: "MFX CtrlSrc 4 (0 - 100)"
        ),
        "mfx_ctrlsens_4": ZCParam(
            name: "mfx_ctrlsens_4",
            offset: 0x000B,
            size: 1,
            minValue: 1, maxValue: 127,
            defaultValue: 0,
            label: "MFX CtrlSens 4",
            notes: "MFX CtrlSens 4 (1 - 127)"
        ),
        "mfx_ctrlasgn_1": ZCParam(
            name: "mfx_ctrlasgn_1",
            offset: 0x000C,
            size: 1,
            minValue: 0, maxValue: 11,
            defaultValue: 0,
            label: "MFX CtrlAsgn 1",
            notes: "MFX CtrlAsgn (0 - 11)"
        ),
        "mfx_ctrlasgn_2": ZCParam(
            name: "mfx_ctrlasgn_2",
            offset: 0x000D,
            size: 1,
            minValue: 0, maxValue: 11,
            defaultValue: 0,
            label: "MFX CtrlAsgn 2",
            notes: "MFX CtrlAsgn (0 - 11)"
        ),
        "mfx_ctrlasgn_3": ZCParam(
            name: "mfx_ctrlasgn_3",
            offset: 0x000E,
            size: 1,
            minValue: 0, maxValue: 11,
            defaultValue: 0,
            label: "MFX CtrlAsgn 3",
            notes: "MFX CtrlAsgn (0 - 11)"
        ),
        "mfx_ctrlasgn_4": ZCParam(
            name: "mfx_ctrlasgn_4",
            offset: 0x000F,
            size: 1,
            minValue: 0, maxValue: 11,
            defaultValue: 0,
            label: "MFX CtrlAsgn 4",
            notes: "MFX CtrlAsgn (0 - 11)"
        ),
        "mfx_parameter_1": ZCParam(
            name: "mfx_parameter_1",
            offset: 0x0010,
            size: 4,
            minValue: 12768, maxValue: 52768,
            defaultValue: 0,
            label: "MFX Parameter 1",
            notes: "MFX Parameter 1 (12768 - 52768)"
        ),
        "mfx_parameter_2": ZCParam(
            name: "mfx_parameter_2",
            offset: 0x0014,
            size: 4,
            minValue: 12768, maxValue: 52768,
            defaultValue: 0,
            label: "MFX Parameter 2",
            notes: "MFX Parameter 2 (12768 - 52768)"
        ),
        "mfx_parameter_3": ZCParam(
            name: "mfx_parameter_3",
            offset: 0x0018,
            size: 4,
            minValue: 12768, maxValue: 52768,
            defaultValue: 0,
            label: "MFX Parameter 3",
            notes: "MFX Parameter 3 (12768 - 52768)"
        ),
        "mfx_parameter_4": ZCParam(
            name: "mfx_parameter_4",
            offset: 0x001C,
            size: 4,
            minValue: 12768, maxValue: 52768,
            defaultValue: 0,
            label: "MFX Parameter 4",
            notes: "MFX Parameter 4 (12768 - 52768)"
        ),
        "mfx_parameter_5": ZCParam(
            name: "mfx_parameter_5",
            offset: 0x0020,
            size: 4,
            minValue: 12768, maxValue: 52768,
            defaultValue: 0,
            label: "MFX Parameter 5",
            notes: "MFX Parameter 5 (12768 - 52768)"
        ),
        "mfx_parameter_6": ZCParam(
            name: "mfx_parameter_6",
            offset: 0x0024,
            size: 4,
            minValue: 12768, maxValue: 52768,
            defaultValue: 0,
            label: "MFX Parameter 6",
            notes: "MFX Parameter 6 (12768 - 52768)"
        ),
        "mfx_parameter_7": ZCParam(
            name: "mfx_parameter_7",
            offset: 0x0028,
            size: 4,
            minValue: 12768, maxValue: 52768,
            defaultValue: 0,
            label: "MFX Parameter 7",
            notes: "MFX Parameter 7 (12768 - 52768)"
        ),
        "mfx_parameter_8": ZCParam(
            name: "mfx_parameter_8",
            offset: 0x002C,
            size: 4,
            minValue: 12768, maxValue: 52768,
            defaultValue: 0,
            label: "MFX Parameter 8",
            notes: "MFX Parameter 8 (12768 - 52768)"
        ),
        "mfx_parameter_9": ZCParam(
            name: "mfx_parameter_9",
            offset: 0x0030,
            size: 4,
            minValue: 12768, maxValue: 52768,
            defaultValue: 0,
            label: "MFX Parameter 9",
            notes: "MFX Parameter 9 (12768 - 52768)"
        ),
        "mfx_parameter_10": ZCParam(
            name: "mfx_parameter_10",
            offset: 0x0034,
            size: 4,
            minValue: 12768, maxValue: 52768,
            defaultValue: 0,
            label: "MFX Parameter 10",
            notes: "MFX Parameter 10 (12768 - 52768)"
        ),
        "mfx_parameter_11": ZCParam(
            name: "mfx_parameter_11",
            offset: 0x0038,
            size: 4,
            minValue: 12768, maxValue: 52768,
            defaultValue: 0,
            label: "MFX Parameter 11",
            notes: "MFX Parameter 11 (12768 - 52768)"
        ),
        "mfx_parameter_12": ZCParam(
            name: "mfx_parameter_12",
            offset: 0x003C,
            size: 4,
            minValue: 12768, maxValue: 52768,
            defaultValue: 0,
            label: "MFX Parameter 12",
            notes: "MFX Parameter 12 (12768 - 52768)"
        ),
        "mfx_parameter_13": ZCParam(
            name: "mfx_parameter_13",
            offset: 0x0040,
            size: 4,
            minValue: 12768, maxValue: 52768,
            defaultValue: 0,
            label: "MFX Parameter 13",
            notes: "MFX Parameter 13 (12768 - 52768)"
        ),
        "mfx_parameter_14": ZCParam(
            name: "mfx_parameter_14",
            offset: 0x0044,
            size: 4,
            minValue: 12768, maxValue: 52768,
            defaultValue: 0,
            label: "MFX Parameter 14",
            notes: "MFX Parameter 14 (12768 - 52768)"
        ),
        "mfx_parameter_15": ZCParam(
            name: "mfx_parameter_15",
            offset: 0x0048,
            size: 4,
            minValue: 12768, maxValue: 52768,
            defaultValue: 0,
            label: "MFX Parameter 15",
            notes: "MFX Parameter 15 (12768 - 52768)"
        ),
        "mfx_parameter_16": ZCParam(
            name: "mfx_parameter_16",
            offset: 0x004C,
            size: 4,
            minValue: 12768, maxValue: 52768,
            defaultValue: 0,
            label: "MFX Parameter 16",
            notes: "MFX Parameter 16 (12768 - 52768)"
        ),
        "mfx_parameter_17": ZCParam(
            name: "mfx_parameter_17",
            offset: 0x0050,
            size: 4,
            minValue: 12768, maxValue: 52768,
            defaultValue: 0,
            label: "MFX Parameter 17",
            notes: "MFX Parameter 17 (12768 - 52768)"
        ),
        "mfx_parameter_18": ZCParam(
            name: "mfx_parameter_18",
            offset: 0x0054,
            size: 4,
            minValue: 12768, maxValue: 52768,
            defaultValue: 0,
            label: "MFX Parameter 18",
            notes: "MFX Parameter 18 (12768 - 52768)"
        ),
        "mfx_parameter_19": ZCParam(
            name: "mfx_parameter_19",
            offset: 0x0058,
            size: 4,
            minValue: 12768, maxValue: 52768,
            defaultValue: 0,
            label: "MFX Parameter 19",
            notes: "MFX Parameter 19 (12768 - 52768)"
        ),
        "mfx_parameter_20": ZCParam(
            name: "mfx_parameter_20",
            offset: 0x005C,
            size: 4,
            minValue: 12768, maxValue: 52768,
            defaultValue: 0,
            label: "MFX Parameter 20",
            notes: "MFX Parameter 20 (12768 - 52768)"
        ),
        "mfx_parameter_21": ZCParam(
            name: "mfx_parameter_21",
            offset: 0x0060,
            size: 4,
            minValue: 12768, maxValue: 52768,
            defaultValue: 0,
            label: "MFX Parameter 21",
            notes: "MFX Parameter 21 (12768 - 52768)"
        ),
        "mfx_parameter_22": ZCParam(
            name: "mfx_parameter_22",
            offset: 0x0064,
            size: 4,
            minValue: 12768, maxValue: 52768,
            defaultValue: 0,
            label: "MFX Parameter 22",
            notes: "MFX Parameter 22 (12768 - 52768)"
        ),
        "mfx_parameter_23": ZCParam(
            name: "mfx_parameter_23",
            offset: 0x0068,
            size: 4,
            minValue: 12768, maxValue: 52768,
            defaultValue: 0,
            label: "MFX Parameter 23",
            notes: "MFX Parameter 23 (12768 - 52768)"
        ),
        "mfx_parameter_24": ZCParam(
            name: "mfx_parameter_24",
            offset: 0x006C,
            size: 4,
            minValue: 12768, maxValue: 52768,
            defaultValue: 0,
            label: "MFX Parameter 24",
            notes: "MFX Parameter 24 (12768 - 52768)"
        ),
        "mfx_parameter_25": ZCParam(
            name: "mfx_parameter_25",
            offset: 0x0070,
            size: 4,
            minValue: 12768, maxValue: 52768,
            defaultValue: 0,
            label: "MFX Parameter 25",
            notes: "MFX Parameter 25 (12768 - 52768)"
        ),
        "mfx_parameter_26": ZCParam(
            name: "mfx_parameter_26",
            offset: 0x0074,
            size: 4,
            minValue: 12768, maxValue: 52768,
            defaultValue: 0,
            label: "MFX Parameter 26",
            notes: "MFX Parameter 26 (12768 - 52768)"
        ),
        "mfx_parameter_27": ZCParam(
            name: "mfx_parameter_27",
            offset: 0x0078,
            size: 4,
            minValue: 12768, maxValue: 52768,
            defaultValue: 0,
            label: "MFX Parameter 27",
            notes: "MFX Parameter 27 (12768 - 52768)"
        ),
        "mfx_parameter_28": ZCParam(
            name: "mfx_parameter_28",
            offset: 0x007C,
            size: 4,
            minValue: 12768, maxValue: 52768,
            defaultValue: 0,
            label: "MFX Parameter 28",
            notes: "MFX Parameter 28 (12768 - 52768)"
        ),
        "mfx_parameter_29": ZCParam(
            name: "mfx_parameter_29",
            offset: 0x0100,
            size: 4,
            minValue: 12768, maxValue: 52768,
            defaultValue: 0,
            label: "MFX Parameter 29",
            notes: "MFX Parameter 29 (12768 - 52768)"
        ),
        "mfx_parameter_30": ZCParam(
            name: "mfx_parameter_30",
            offset: 0x0104,
            size: 4,
            minValue: 12768, maxValue: 52768,
            defaultValue: 0,
            label: "MFX Parameter 30",
            notes: "MFX Parameter 30 (12768 - 52768)"
        ),
        "mfx_parameter_31": ZCParam(
            name: "mfx_parameter_31",
            offset: 0x0108,
            size: 4,
            minValue: 12768, maxValue: 52768,
            defaultValue: 0,
            label: "MFX Parameter 31",
            notes: "MFX Parameter 31 (12768 - 52768)"
        ),
        "mfx_parameter_32": ZCParam(
            name: "mfx_parameter_32",
            offset: 0x010C,
            size: 4,
            minValue: 12768, maxValue: 52768,
            defaultValue: 0,
            label: "MFX Parameter 32",
            notes: "MFX Parameter 32 (12768 - 52768)"
        ),
    ]

    public static let sectionTotalSize: [String: UInt32] = [
        "PARTIAL_EQ": 0x1B,
        "PARTIAL_LFO": 0x11E,
        "PARTIAL_PITCH_ENV": 0x2D,
        "PARTIAL_FILTER_ENV": 0x2C,
        "PARTIAL_AMP_ENV": 0x20,
        "TONE_COMMON": 0x36,
        "TONE_PMT": 0x26,
        "TONE_PARTIAL": 0x100,
        "TONE_SYNTH_COMMON": 0x9,
        "TONE_SYNTH_PMT": 0x17,
        "TONE_SYNTH_PARTIAL": 0x1E,
        "TONE_MFX": 0x110,
    ]
}
