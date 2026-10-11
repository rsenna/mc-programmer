# Use cases

Concrete things you'd actually do with `mc_programmer` / `MCProgrammer`. Every
example below has been verified to compile and (where it doesn't need
real hardware) run cleanly against `LoopbackTransport`.

If you only want to know what's possible at a glance, the headings:

1. [Live performance — recall scenes and clips from a script](#1-live-performance--recall-scenes-and-clips-from-a-script)
2. [Sound-design backup and version control](#2-sound-design-backup-and-version-control)
3. [Cross-device sound transfer](#3-cross-device-sound-transfer)
4. [Algorithmic pattern generation](#4-algorithmic-pattern-generation)
5. [Custom hardware controllers](#5-custom-hardware-controllers)
6. [DAW round-tripping — MIDI files in, MC clips out](#6-daw-round-tripping--midi-files-in-mc-clips-out)
7. [Automated project archival](#7-automated-project-archival)
8. [Visual editors and piano-roll viewers](#8-visual-editors-and-piano-roll-viewers)
9. [Bulk parameter editing](#9-bulk-parameter-editing)
10. [Test automation for music apps](#10-test-automation-for-music-apps)
11. [MIDI-controller integration](#11-midi-controller-integration)
12. [Reverse-engineering / address-space exploration](#12-reverse-engineering--address-space-exploration)

---

## 1. Live performance — recall scenes and clips from a script

The most direct win for an MC owner. Pre-load 16 clips into one track,
each with a different sound, then jump between them from your code via
Program Change. Same trick works for whole-Scene recall.

```python
from mc_programmer import MCClient, ModelID
from mc_programmer.coremidi import RtMidiTransport

client = MCClient(RtMidiTransport.connect("MC-707"), model_id=ModelID.MC_707)

# Walk through the eight clips of track 1, holding each for 4 bars at 120 BPM.
import time
for clip_number in range(1, 9):
    client.select_clip(track=1, clip_number=clip_number)
    print(f"Now playing clip {clip_number}")
    time.sleep(8)   # 4 bars at 120 BPM
```

```swift
import MCProgrammer
let transport = try CoreMIDITransport(deviceName: "MC-707")
let client = MCClient(transport: transport, modelID: .mc707)

for clipNumber in 1...8 {
    try client.selectClip(trackNumber: 1, clipNumber: clipNumber)
    print("Now playing clip \(clipNumber)")
    Thread.sleep(forTimeInterval: 8)
}
```

This is how you'd build a "scene chain" outside the MC's built-in scene
chain function — useful when you want non-linear or conditional jumps
("if the audience is into it, hold on this scene; otherwise advance").

---

## 2. Sound-design backup and version control

Spent an hour sculpting a bass patch and want to make sure you don't
lose it? Dump it to a JSON file and check it into git. The format is
human-readable hex, so `git diff` between two versions of the same
patch shows you exactly which bytes changed.

```python
preset = client.dump_tone(track=1, clip=None)
preset.save("patches/bass-monster.tone")
# git add patches/bass-monster.tone
# git commit -m "Bass patch v3 — added unison detune, more cutoff env"
```

```swift
let preset = try client.dumpTone(trackNumber: 1, clipNumber: nil)
try preset.save(toPath: "patches/bass-monster.tone")
```

To restore later (possibly months later, possibly to a different track):

```python
from mc_programmer import TonePreset
preset = TonePreset.load("patches/bass-monster.tone")
client.load_tone(track=3, clip=2, preset=preset)
```

The library refuses cross-model loads unless you pass `force=True`, so
you can't accidentally flash a 707 patch onto a 101.

---

## 3. Cross-device sound transfer

Got a sound on track 1 of one MC-707 and want it on track 5 of another
one (same model)? No file needed:

```python
src_client = MCClient(RtMidiTransport.connect("MC-707 1"), model_id=ModelID.MC_707)
dst_client = MCClient(RtMidiTransport.connect("MC-707 2"), model_id=ModelID.MC_707)

preset = src_client.dump_tone(track=1, clip=None)
dst_client.load_tone(track=5, clip=None, preset=preset)
```

Or for moving sounds *within* the same device — say, you want to
audition the sound from track 1 on every other track:

```python
sound = client.dump_tone(track=1, clip=None)
for target_track in range(2, 9):
    client.load_tone(track=target_track, clip=None, preset=sound)
```

---

## 4. Algorithmic pattern generation

Generate beats from data — a CSV of drum hits, a probability table, a
markov chain, output from a music-theory library, anything Python can
produce. The high-level `set_pattern` helper takes a dict of pitches to
step positions.

```python
import random
from mc_programmer import MCClient, ModelID, LoopbackTransport

client = MCClient(LoopbackTransport(), model_id=ModelID.MC_707)

# Probability-based fills: kick on every downbeat, snare on 5 and 13,
# hat on every offbeat with a 30% chance of an extra ghost note.
random.seed(42)
hat_steps = [s for s in range(1, 17) if s % 2 == 0]
hat_steps += [s for s in range(1, 17) if s % 2 == 1 and random.random() < 0.3]

client.set_pattern(track=1, clip=1, notes={
    "C1":  [1, 5, 9, 13],
    "D1":  [5, 13],
    "F#1": sorted(set(hat_steps)),
})
```

Or polyphonic chord progressions on a tone track:

```python
chords = {
    "Cmaj7":  ["C3", "E3", "G3", "B3"],
    "Am7":    ["A2", "C3", "E3", "G3"],
    "Fmaj7":  ["F2", "A2", "C3", "E3"],
    "G7":     ["G2", "B2", "D3", "F3"],
}

# One chord per clip, each held for the full clip length.
for clip_idx, (name, voices) in enumerate(chords.items(), start=1):
    client.clear_clip(track=2, clip=clip_idx)
    for pitch in voices:
        client.place_note(
            track=2, clip=clip_idx, step=1,
            pitch=pitch, velocity=80, length_steps=16,
        )
```

---

## 5. Custom hardware controllers

Build a Raspberry Pi (or any Python-running microcontroller with USB
MIDI) that maps physical controls to MC parameters. Plug a USB knob
into the Pi, plug the Pi into the MC, and the knob now drives Tone
Coarse Tune in semitones — no menu diving.

```python
from mc_programmer import MCClient, ModelID
from mc_programmer.coremidi import RtMidiTransport

client = MCClient(RtMidiTransport.connect("MC-707"), model_id=ModelID.MC_707)

def on_knob_event(value: int):
    # value is 0..127 from the controller; map to -12..+12 semitones.
    semitones = round((value - 64) / 64 * 12)
    client.write_coarse_tune_semitones(track=1, clip=None, semitones=semitones)

# Hook on_knob_event up to your USB controller's input loop.
```

Or use the discoverability helpers to build a "any parameter" knob:

```python
from mc_programmer import ParamGroup
all_params = client.list_parameters(group=ParamGroup.PARTIAL)
print(f"Knob can drive any of {len(all_params)} partial parameters")
# 89 parameters available — let the user pick one with another input
```

---

## 6. DAW round-tripping — MIDI files in, MC clips out

The MC is a fantastic groovebox but a clumsy step-sequencer compared
to a DAW. Sketch your beat in your favourite DAW, export as MIDI, and
push it to a clip:

```python
import mido       # any standard MIDI file library works
from mc_programmer import MCClient, ModelID, LoopbackTransport

client = MCClient(LoopbackTransport(), model_id=ModelID.MC_707)

mid = mido.MidiFile("my-beat.mid")
ticks_per_step = mid.ticks_per_beat // 4   # assume 16 steps/bar

# Walk the first track, accumulate (note, step, velocity) triples.
notes = {}
abs_tick = 0
for msg in mid.tracks[0]:
    abs_tick += msg.time
    if msg.type == "note_on" and msg.velocity > 0:
        step = abs_tick // ticks_per_step + 1   # 1-based for set_pattern
        notes.setdefault(msg.note, []).append(step)

client.set_pattern(track=1, clip=1, notes=notes)
```

The reverse direction (MC clip → MIDI file) is also straightforward:
walk `client.read_note_step(...)` for every step in the clip, decode
its components, and write them out via `mido`.

---

## 7. Automated project archival

Run a cron job that snapshots your live device every hour. The
`Project` class handles the chunked I/O for everything — note steps,
motion lanes, pitch bends, all 8 (or 4) tracks' worth of tones, the
system block, the lot.

```python
#!/usr/bin/env python3
"""Hourly project backup. Schedule via cron or launchd."""
import datetime
from pathlib import Path
from mc_programmer import MCClient, ModelID, Project
from mc_programmer.coremidi import RtMidiTransport

OUT_DIR = Path.home() / "mc-backups"
OUT_DIR.mkdir(exist_ok=True)

client = MCClient(RtMidiTransport.connect("MC-707"), model_id=ModelID.MC_707)
stamp = datetime.datetime.now().strftime("%Y-%m-%d-%H%M")
project = Project.dump_from(client, name=f"hourly-{stamp}")
project.save(str(OUT_DIR / f"mc707-{stamp}.mctproject"))
```

```cron
0 * * * * /usr/bin/python3 /home/me/scripts/backup-mc.py
```

---

## 8. Visual editors and piano-roll viewers

Build a macOS or iOS app that shows a clip as a piano-roll. The Swift
library was designed exactly for this — `CoreMIDITransport` handles
the device, `NoteStep` exposes the lane structure, and `Project` lets
you load a snapshot and inspect any clip without touching the
hardware.

```swift
import SwiftUI
import MCProgrammer

struct PianoRollView: View {
    let project: Project   // loaded from a .mctproject file

    var body: some View {
        let clip = project.tracks[0].clips[0]
        // Decode every step's note components into rectangles, draw on a Canvas.
        // Each component has note, velocity, startCents, endCents, ties — exactly
        // what you need for a faithful piano-roll rendering.
        return Text("Steps: \(clip.stepCount), tempo scale: \(clip.stepScale)")
    }
}
```

Or in Python with `tkinter` / `pyglet` / `matplotlib`, or in Rust with a
native GUI toolkit, for cross-platform prototyping. The reference repository's
video transcripts include diagrams of how the lane structure should be visualised.

---

## 9. Bulk parameter editing

Apply the same change to every clip on every track. Drop the cutoff on
every clip's tone by 5 semitones. Reset every drum kit's reverb send.
Set every track's coarse tune to A=440 just-intonation. The framework
exposes all 389 parameters by name; you can iterate any axis you like.

```python
# Lower TVF Cutoff Frequency on every clip's partial 1.
from mc_programmer import ParamGroup
for track in range(1, client.num_tracks + 1):
    for clip in range(1, 17):
        current = client.read_zc_param(
            track - 1, clip - 1, "tvf_cutoff_frequency",
            group=ParamGroup.PARTIAL, partial_idx=1,
        )
        new_value = max(0, current - 100)
        client.write_zc_param(
            track - 1, clip - 1, "tvf_cutoff_frequency", new_value,
            group=ParamGroup.PARTIAL, partial_idx=1,
        )
```

---

## 10. Test automation for music apps

If you're building any software that touches an MC, you probably can't
test it cleanly because real MIDI hardware is awkward to drive in CI.
`LoopbackTransport` is exactly the answer — an in-memory fake MC that
implements every Sys-Ex address you can read or write, returns
realistic empty-step values for unwritten regions, and parses Program
Change so your scene/clip selection logic is testable too.

```python
import unittest
from mc_programmer import LoopbackTransport, MCClient, ModelID

class MyMCAppTests(unittest.TestCase):
    def test_pattern_writer_writes_correct_kicks(self):
        # Set up a fake device.
        client = MCClient(LoopbackTransport(), model_id=ModelID.MC_707)

        # Drive your application's "write a four-on-the-floor" routine
        # against the fake device — exactly the same code path as
        # production, but no hardware required.
        my_app.write_four_on_the_floor(client, track=1, clip=1)

        # Verify by reading back through the same client.
        for step_idx in (0, 4, 8, 12):
            step = client.read_note_step(0, 0, step_idx)
            self.assertEqual(step.components[0].note, 36)
```

The same trick works for Swift via `XCTest` and Rust via `cargo test`, both with `LoopbackTransport`.

---

## 11. MIDI-controller integration

Translate hardware controller events (any MIDI source) into ZEN-Core
parameter writes. Your USB controller doesn't need to "know" about
ZEN-Core — your script does the translation.

```python
import mido
from mc_programmer import MCClient, ModelID, ParamGroup
from mc_programmer.coremidi import RtMidiTransport

client = MCClient(RtMidiTransport.connect("MC-707"), model_id=ModelID.MC_707)
controller = mido.open_input("My USB Controller")

# Map controller knobs (CC 1-4) to ZEN-Core filter envelope on partial 1.
KNOB_MAP = {
    1: "tvf_env_attack",
    2: "tvf_env_decay",
    3: "tvf_env_sustain",
    4: "tvf_env_release",
}

for msg in controller:
    if msg.type != "control_change":
        continue
    name = KNOB_MAP.get(msg.control)
    if name is None:
        continue
    # Map 0..127 to 0..1023 for the env time.
    value = msg.value * 8
    client.write_zc_param(
        0, 0, name, value,
        group=ParamGroup.PARTIAL_FILTER_ENV, partial_idx=1,
    )
```

The same pattern works for any hardware: an iPad TouchOSC layout, a
Novation Launchpad, an Arduino with rotary encoders, even another DAW
sending CC out.

---

## 12. Reverse-engineering / address-space exploration

The MC's address space has many regions that aren't yet documented in
this library — drum kits, scene-level Part parameters, the looper.
You can explore them by walking the address space and looking for
non-zero responses, the same way `hack_tools.py` from the original
test scripts did. The low-level `MCClient.read(addr, count)` is
deliberately exposed for this:

```python
# Sweep a 64 KB region looking for parameter blocks that respond.
import mc_programmer

client = MCClient(RtMidiTransport.connect("MC-707"), model_id=ModelID.MC_707)
start = 0x40000000
end   = 0x40010000
step  = 0x40            # 64 bytes per probe

for addr in range(start, end, step):
    if addr & 0x80808080:
        continue   # skip addresses with high-MSB bytes (invalid in 7-bit MIDI)
    data = client.read(addr, 16)
    if data and any(b != 0 for b in data):
        print(f"0x{addr:08X}: " + " ".join(f"{b:02X}" for b in data))
```

Anything you find can be added to the framework — see [ROADMAP.md](../ROADMAP.md)
for the contributor guide. Verified addresses go into
`python_lib/mc_programmer/addresses.py`; new parameter substructures go into
the auto-generator at `tools/gen_zencore_params.py`.

---

## What this library is *not* designed for

To set realistic expectations:

* **Real-time performance per-note triggering.** This is a Sys-Ex
  library, not a low-latency MIDI host. Each Sys-Ex round-trip is
  measured in milliseconds, not microseconds. If you want to play
  notes from your code in time with a metronome, send standard
  Note-On / Note-Off messages over MIDI directly — the MC is a
  GM/MIDI synth and accepts those without any of this machinery.
* **Audio recording or audio routing.** This library only touches the
  MC's parameter memory. The Looper track has audio buffers that
  aren't exposed via Sys-Ex.
* **Replacing the MC's UI entirely.** The front panel is still the
  fastest way to do most things. Where this library shines is the
  things the front panel is *bad* at — bulk edits, repeatable
  programmatic configurations, version control, and integrations with
  software you write yourself.
