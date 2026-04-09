# mc_programmer (Python)

Talk to your **Roland MC-101**, **MC-707** or **MV-1** from Python. Read
and write any sequencer step, change ZEN-Core synth parameters, switch
scenes and clips, all over MIDI System Exclusive — **no front-panel menu
diving required**.

Built from the test scripts and the *MC Programming* / *ZC Programming*
video transcripts in the repo above this directory. Works on macOS,
Linux, and Windows.

---

## Quick start (5 lines)

You don't need a real MC connected to try this — it runs against an
in-memory fake device:

```python
from mc_programmer import LoopbackTransport, MCClient, ModelID

client = MCClient(LoopbackTransport(), model_id=ModelID.MC_707)
client.set_pattern(track=1, clip=1, notes={
    "C1":  [1, 5, 9, 13],   # kick on every downbeat
    "F#1": [3, 7, 11, 15],  # closed hat on the offbeats
})
```

Replace `LoopbackTransport()` with one line and you're driving real
hardware:

```python
from mc_programmer.coremidi import RtMidiTransport
client = MCClient(RtMidiTransport.connect("MC-707"), model_id=ModelID.MC_707)
```

That's it. The rest of this README is "what else can I do" recipes plus a
reference for the lower-level surface.

---

## Install

```bash
pip install python-rtmidi   # only needed for real hardware
pip install -e python_lib   # the library itself (zero hard deps)
```

`python-rtmidi` is what talks to **CoreMIDI on macOS**, ALSA on Linux,
and WinMM on Windows. Skip it if you only want to use `LoopbackTransport`
to dry-run code without the device.

---

## I'm a musician — what can I do?

Every recipe below uses 1-based numbering (`track=1` is the first track,
just like the MC's screen) and accepts note names like `"C1"` *or* MIDI
numbers. The MC family follows the **Roland octave convention** where
**C3 = MIDI 60** and the kick drum lives at **C1 = 36**.

### Make a beat

```python
from mc_programmer import MCClient, ModelID, LoopbackTransport
client = MCClient(LoopbackTransport(), model_id=ModelID.MC_707)

# A 16-step house pattern on track 1, clip 1.
client.set_pattern(track=1, clip=1, notes={
    "C1":  [1, 5, 9, 13],     # kick
    "D1":  [5, 13],           # snare on the backbeats
    "F#1": [3, 7, 11, 15],    # offbeat closed hat
    "A#1": [9],               # one open hat
})
```

### Place individual notes

```python
client.place_note(track=1, clip=1, step=1, pitch="C1", velocity=120)
client.place_note(track=2, clip=1, step=5, pitch=60)         # MIDI number works too
client.place_note(track=2, clip=1, step=5, pitch="E3")       # adds to lane 2 of step 5
client.place_note(track=2, clip=1, step=5, pitch="G3")       # …lane 3, …
```

`place_note` automatically packs concurrent notes into the lowest free
lane (the same way the MC's firmware does), so the order in which you
write them doesn't matter.

### Long notes that tie across steps

```python
# A note that runs for 2.5 steps starting on step 9.
client.place_note(track=3, clip=1, step=9, pitch="C3", length_steps=2.5)
```

The MC's "tie" mechanism (end-time = 100) is handled automatically.

### Erase

```python
client.clear_step(track=1, clip=1, step=4)   # one step
client.clear_clip(track=1, clip=1)           # entire clip
```

### Switch scenes and clips from your code

The MC family uses standard MIDI **Program Change** for scene and clip
recall (per the MC-707 / MC-101 MIDI Implementation Chart):

```python
client.select_scene(scene_number=5)                  # PC on the control channel (default 1)
client.select_clip(track=2, clip_number=8)           # PC on the track's MIDI channel
```

If your MC's system menu has the control channel set to something other
than 1, pass it explicitly:

```python
client.select_scene(scene_number=5, control_channel=16)
```

### Tune a track up by 5 semitones

```python
client.write_coarse_tune_semitones(track=2, clip=None, semitones=+5)
print(client.read_coarse_tune_semitones(track=2, clip=None))   # → 5
```

`clip=None` targets the *track-wide* tone (loaded when the track's sound
source is set to TRACK). Pass `clip=1..16` to target a specific clip's
tone.

### Browse what synth parameters you can change

There are ~294 ZEN-Core parameters spread across 8 substructures. Browse
them by name:

```python
from mc_programmer import ParamGroup
client.list_parameters(group=ParamGroup.COMMON)[:5]
# → ['analog_feel', 'bend_mode', 'bend_range_down', 'bend_range_up', 'category']

client.describe_parameter("tvf_cutoff_frequency")
# → 'TVF Cutoff Frequency (partial, offset 0x0035, size 4, range 0..1023)'
```

If you misspell a name, the error has a hint:

```python
client.read_zc_param(0, 0, "tone_coarse_tunes")
# KeyError: "unknown common parameter: 'tone_coarse_tunes'.
#            Did you mean 'tone_coarse_tune'?"
```

### Read what's in a clip

```python
scale, step_count = client.read_clip_step_params(0, 0)   # low-level — 0-based
print(f"{step_count} steps at {scale.label}")
for s in range(step_count):
    step = client.read_note_step(0, 0, s)
    for c in step.active_components():
        print(f"  step {s+1}: note={c.note} vel={c.velocity}")
```

---

## Glossary (in plain English)

| Term | What it actually means |
|---|---|
| **Sys-Ex** | A MIDI message specifically for one make/model. The MC family hides 99% of its internals behind these. The library handles the framing for you. |
| **DT1** | "Set this parameter to this value." Roland's name for a Sys-Ex *write*. |
| **RQ1** | "Tell me the value of this parameter." Roland's name for a Sys-Ex *read*. The device replies with a DT1. |
| **ZEN-Core** | The synth engine inside your MC. The same engine ships in the Jupiter-X / Fantom / etc., so the parameter map is shared. |
| **Tone** | One ZEN-Core "patch" — the full synth setup (oscillators, filter, envelopes, LFO, EQ). Each clip has its own tone slot. |
| **Partial** | One of the four "voices" inside a tone. Layer them for richer sounds. |
| **Step** | One slot in the sequencer grid (a clip has up to 128 steps). Each step can hold up to 16 concurrent notes. |
| **Lane / component** | One of the 16 note slots inside a single step. The library calls these "components" to match the test scripts; you can think of them as "voices stacked on this step". |
| **Tie** | A note that crosses a step boundary. Internally encoded as end-time = 100 in the previous step. The library does this for you when you pass `length_steps > 1`. |
| **Coarse Tune** | Transposition in semitones. The wire range is 16..112 with 64 = no transposition; the library exposes a `_semitones` form that uses -48..+48 instead. |

---

## I'm a developer — give me the reference

### Architecture

```
┌────────────────────┐
│      MCClient      │  high-level read/write + musician helpers
└─────────┬──────────┘
          │ uses
┌─────────┴──────────┐  ┌──────────────┐  ┌──────────────┐
│      protocol      │  │  addresses   │  │  sequencer   │
│  RQ1/DT1, checksum │  │  AddressMap  │  │ encode/decode│
└─────────┬──────────┘  └──────────────┘  └──────────────┘
          │
┌─────────┴──────────┐
│     Transport      │  ← LoopbackTransport, RtMidiTransport, MidoTransport
└────────────────────┘
```

| Module | Responsibility |
|---|---|
| `mc_programmer.protocol` | Roland RQ1/DT1 framing, checksum, model-ID detection |
| `mc_programmer.addresses` | MC address-map constants + `AddressMap` arithmetic |
| `mc_programmer.sequencer` | `NoteStep` / `NoteComponent` / `SubStep`, encode/decode |
| `mc_programmer.zencore` | `ZCParam` framework + 8 substructure offset/lookup helpers |
| `mc_programmer.zencore_data` | Auto-generated parameter table (294 entries) |
| `mc_programmer.notes` | Note name parser + drum-key constants |
| `mc_programmer.transport` | `Transport` protocol + `NullTransport` |
| `mc_programmer.loopback` | `LoopbackTransport` — in-memory fake MC for offline use |
| `mc_programmer.coremidi` | `RtMidiTransport` + `MidoTransport` (lazy imports) |
| `mc_programmer.client` | `MCClient` — ties it all together |

### Indexing convention

* **Low-level methods** take `trk_idx` / `clip_idx` / `step_idx` parameters
  that are **0-based** and map directly onto the underlying address
  arithmetic. Examples: `read_note_step(0, 0, 0)`, `write_zc_param(0, 0, "name", "BASS")`.
* **High-level helpers** take `track` / `clip` / `step` parameters that
  are **1-based** and match the labels on the MC's screen. Examples:
  `place_note(track=1, clip=1, step=1, pitch="C1")`,
  `select_scene(scene_number=5)`.

The parameter name is the discriminator: `_idx` → 0-based, no `_idx`
→ 1-based.

### Notes about notes

The Roland MC family follows the **Roland octave convention**, where
**middle C is C3 = MIDI 60**. The `mc_programmer.notes.note(...)` helper accepts:

* Plain MIDI numbers — `note(60)` → 60
* Note names with explicit octave — `note("C1")` → 36, `note("F#3")` → 66
* Note names without an octave (assumes octave 3) — `note("C")` → 60
* Lower or mixed case — `note("eb2")` → 51
* Negative octaves — `note("C-1")` → 12
* Both `#` and `b` accidentals — `note("Bb1")` → 46

Common drum constants are exported at the package root: `KICK`, `SNARE`,
`CLAP`, `HAT_CLOSED`, `HAT_OPEN`, `RIDE`, `CRASH`, …

### Transport protocol

```python
class Transport(Protocol):
    def transfer(self, message: Sequence[int],
                 timeout: float = 0.05) -> List[int]: ...
    def send_raw(self, message: Sequence[int]) -> None: ...
```

`transfer` is request-with-reply (Sys-Ex). `send_raw` is fire-and-forget
(any other MIDI message — used by `select_scene` / `select_clip`).

Implementations shipped:

| Transport | Where it lives | Use case |
|---|---|---|
| `LoopbackTransport` | `mc_programmer.loopback` | Offline testing / examples — in-memory fake MC |
| `NullTransport` | `mc_programmer.transport` | Recording-only stub for unit tests |
| `RtMidiTransport` | `mc_programmer.coremidi` | **CoreMIDI on macOS**, ALSA on Linux, WinMM on Windows |
| `MidoTransport` | `mc_programmer.coremidi` | For users already on `mido` |

Implement `Transport` yourself if you have a custom MIDI backend.

### Low-level Sys-Ex / address-map API

```python
# RQ1 / DT1 protocol
build_rq1(model_id, address, count) -> list[int]
build_dt1(model_id, address, data)  -> list[int]
parse_dt1_payload(message) -> DT1Message
roland_checksum(payload) -> int

# Address map
amap = AddressMap.build()
amap.note_step_address(trk_idx, clip_idx, step_idx)        # 0-based
amap.zencore_tone_address(trk_idx, clip_idx)               # clip=None for track tone

# MCClient low-level
client.read(addr, count)
client.write(addr, data)
client.read_note_step(trk_idx, clip_idx, step_idx) -> NoteStep
client.write_note_step(trk_idx, clip_idx, step_idx, NoteStep)
client.read_zc_param(trk_idx, clip_idx, name, group=..., partial_idx=1)
client.write_zc_param(trk_idx, clip_idx, name, value, group=..., partial_idx=1)
```

### What about the SuperNATURAL Synth (SN-S)?

The MC family uses the **conventional ZEN-Core Tone path**, *not* the
SuperNATURAL Synth sub-section. The library deliberately only exposes
the conventional `[Tone Common]` / `[Tone PMT]` / `[Tone Partial]` /
`[Partial Pitch Env]` / `[Partial Filter Env]` / `[Partial Amp Env]` /
`[Partial LFO]` / `[Partial EQ]` substructures from Roland's Jupiter-X
MIDI Implementation, which the ZC programming-video transcript states
applies as-is to MC-101 / MC-707 / MV-1.

### Regenerate the ZEN-Core parameter tables

```bash
pdftotext -raw -f 76 -l 82 JUPITER-X_Xm_MIDI_imple_eng06_W.pdf tools_jp_zcore.txt
python tools/gen_zencore_params.py
```

This rewrites `mc_programmer/zencore_data.py` and the matching Swift file. The
existing tables are checked in, so you only need to do this when Roland
publishes a new revision of the doc.

---

## Tests

```bash
PYTHONPATH=python_lib python -m unittest python_lib.tests.test_mc_programmer
```

51 tests. They run offline against `LoopbackTransport`, no MIDI device
required.
