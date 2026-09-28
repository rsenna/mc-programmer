# Roadmap

What the libraries currently cover, what's still missing, and how to add
the missing pieces. The Python (`python_lib/mc_programmer/`), Swift
(`swift_lib/Sources/MCProgrammer/`), and Rust (`rust_lib/src/`) packages share
this roadmap — every feature ships in all three, with parallel implementations
and tests.

---

## Done

### Protocol layer

- [x] Roland Sys-Ex framing — RQ1 (data request) and DT1 (data set)
      message construction with correct 32-bit address encoding,
      payload checksum (complement-to-128), and parsing of replies
- [x] Universal device-inquiry for model-ID detection
- [x] Model-ID constants for MC-707 (0x5D), MC-101 (0x5E), and MV-1 (0x5F)

### Address map

- [x] Sequencer base addresses for all 8 tracks × 16 clips, with the
      0x80808080 high-MSB-skip rule from the reference scripts
- [x] Per-track settings block addresses (color, sound source, type)
- [x] Per-clip parameter block (step scale + step count)
- [x] Per-clip note step records (256 bytes each, 128 steps per clip)
- [x] Per-clip motion lane records (same shape as notes)
- [x] Per-clip pitch-bend records
- [x] ZEN-Core tone descriptor base addresses (8 tracks × 17 tones each)
- [x] System / setup block (`0x10000000..0x10004000`) — opaque-byte
      capture covering Master EQ, Master Comp, mixer, system common
- [x] Tiny system header (`0x00000000..0x00000030`)

### Sequencer

- [x] `NoteStep` / `NoteComponent` / `SubStep` data model
- [x] Encode and decode for the full 256-byte note step record:
      16 lanes × {note, velocity, signed start cents, end cents (with
      tie support at end=100), substep mode}
- [x] `MotionStep` data type for the eight motion lanes per step
      (C1-C4 + four extras)
- [x] Constants for the empty-note sentinel, default values, and substep modes

### ZEN-Core tone parameter framework (~389 named parameters)

The MC family uses both halves of the ZCore tone descriptor — the
conventional `[Tone Common]` / `[Tone PMT]` / `[Tone Partial]` block AND
the `[Tone Synth Common]` / `[Tone Synth PMT]` / `[Tone Synth Partial]`
sub-block. Both are now in the framework:

| Substructure | Offset | Params | Verified by |
|---|---|---|---|
| Tone Common | `0x0000` | 24 | ZC video + zen_core_tools.py + MC-707 Ref Manual SOUND/TUNING |
| Tone MFX | `0x0100` | 48 | MC-707 Ref Manual MFX/IFX (page 80) + 1.80 update |
| Tone PMT | `0x1000` | 38 | MC-707 Ref Manual PTL CTRL / PTL RANGE |
| Tone Partial 1..4 | `0x2000` step `0x100` | 89 | MC-707 Ref Manual MIX/FILTER/AMP/MATRIX |
| Partial Pitch Env 1..4 | `0x2400` step `0x100` | 14 | MC-707 Ref Manual PENV |
| Partial Filter Env 1..4 | `0x2800` step `0x100` | 14 | MC-707 Ref Manual FENV |
| Partial Amp Env 1..4 | `0x2C00` step `0x100` | 9 | MC-707 Ref Manual AMP |
| Partial LFO 1..4 | `0x3000` step `0x200` | 98 | MC-707 Ref Manual LFO |
| Partial EQ 1..4 | `0x3800` step `0x100` | 8 | MC-707 Ref Manual EQ |
| Tone Synth Common | `0x3C00` | 8 | MC-707 Ref Manual UNISON |
| Tone Synth PMT | `0x3D00` | 17 | MC-707 Ref Manual STRUCTURE / OSC MOD COM |
| Tone Synth Partial 1..4 | `0x3E00` step `0x100` | 22 | MC-707 Ref Manual OSC |

- [x] `ZCParam` descriptor type (1-byte / 2-byte / 4-byte nibble-packed
      / variable-length ASCII encodings)
- [x] Auto-generated parameter table (`zencore_data.py` /
      `ZenCoreData.swift`) — built from the Jupiter-X PDF by
      `tools/gen_zencore_params.py`, regenerable any time
- [x] Generic `read_zc_param` / `write_zc_param` (Python) and
      `readZCParam` / `writeZCParam` (Swift)
- [x] "Did you mean…" suggestions on parameter typos via `difflib` /
      Levenshtein
- [x] Discovery — `MCClient.list_parameters(group=...)` and
      `describe_parameter(name)`

### Tone preset and full-project save / restore

- [x] `TonePreset` (one tone, ~15 KB across 22 substructures) — JSON
      file format with hex byte strings, model-tagged, refuses
      cross-model loads unless `force=True`
- [x] `MCClient.dump_tone(...)` / `load_tone(...)` with chunked I/O
      (≤128-byte RQ1/DT1 chunks to respect the MC's per-request limit)
- [x] `Project` (whole device) — TrackDump × 8 (or × 4 on MC-101),
      ClipDump × 16 each, system header + system block as opaque blobs
- [x] `Project.dump_from(client)` / `load_into(client)` — walks every
      track and clip, dumps notes + motion + pitch-bend (only the
      active step count), tones, and per-track settings
- [x] Round-trip tests against `LoopbackTransport` for both layers
- [x] Strict model-ID safety: `Project.load_into` refuses to flash an
      MC-707 dump onto an MC-101 unless `force=True`

### Transports

- [x] `Transport` protocol — request/reply Sys-Ex (`transfer`) and
      fire-and-forget MIDI (`send_raw` / `sendRaw`)
- [x] `NullTransport` — recording-only stub for unit tests
- [x] `LoopbackTransport` — in-memory fake MC with sparse address space,
      handles Universal Identity Reply, parses Program Change to track
      active scene/clip per channel
- [x] `RtMidiTransport` (Python) — wraps `python-rtmidi` (CoreMIDI on
      macOS, ALSA on Linux, WinMM on Windows), reframes Sys-Ex at
      0xF0/0xF7 boundaries
- [x] `MidoTransport` (Python) — for users on `mido`
- [x] `CoreMIDITransport` (Swift) — native CoreMIDI; thread-safe; uses
      `MIDIInputPortCreateWithBlock` for receive and `MIDISend` with
      packed `MIDIPacketList`s for transmit

### High-level "musician helpers" on `MCClient`

- [x] `place_note(track, clip, step, pitch, velocity, length_steps)` —
      1-based front-panel numbering, accepts note names ("C1") or MIDI
      numbers (36), automatically packs concurrent notes into the
      lowest free lane, automatically ties notes that span step boundaries
- [x] `clear_step(...)`, `clear_clip(...)`, `set_pattern(...)` —
      pattern programming in one call
- [x] `read_coarse_tune_semitones(...)` / `write_coarse_tune_semitones(...)`
      — exposes the wire-level 16..112 range as a friendly -48..+48
- [x] Note name parser (`mc_programmer.notes.note(...)` / `Note.parse(...)`)
      with Roland octave convention (C3 = MIDI 60, kick = C1 = 36)
- [x] Drum-key constants — KICK, SNARE, HAT_CLOSED, HAT_OPEN, etc.

### Scene / Clip selection

- [x] `select_scene(scene_number, control_channel)` — Program Change
      on the configured control channel (1..128 → PC 0..127)
- [x] `select_clip(track, clip_number, track_channel)` — Program
      Change on the track's MIDI channel (1..16 → PC 0..15), defaults
      to "track N → channel N" with explicit override

### Compatibility

- [x] **MC-101 verified end-to-end** — same parameter table, same MFX
      list, same address map (verified against MC-101 reference manual
      and update notes through 1.80)
- [x] `MCClient.num_tracks` returns 4 for MC-101, 8 for MC-707/MV-1
- [x] Track validation in high-level helpers respects this
- [x] `Project.dump_from()` walks `client.num_tracks` so MC-101
      project files contain 4 tracks, not 8

### Tooling and packaging

- [x] `pyproject.toml` for the Python package (`pip install -e python_lib`)
- [x] Optional extras: `pip install mc_programmer[rtmidi]` and `mc_programmer[mido]`
- [x] Swift Package manifest for SwiftPM
- [x] `tools/gen_zencore_params.py` — parser/generator for the Python and Swift tables
- [x] `tools/gen_rust_zencore.py` — generates `zencore_data.rs` from the committed Python table
- [x] Rust Cargo crate with optional `midir` hardware transport

### Tests

- [x] Python: 70 tests via `unittest`, all passing, runs in ~13 seconds
- [x] Swift: 42 tests via `XCTest`, all passing, runs in ~40 seconds
      (project round-trip dominates the time — exercises ~15 KB per tone
      through chunked I/O)
- [x] Rust: 84 tests via `cargo test --all-features`, all passing

---

## Pending

### High value, address-known

These are mechanical additions — the address layouts are already in the
Jupiter-X PDF but I haven't transcribed the per-effect parameter labels
yet. Either is "drop in a table from the existing extracted text".

- [ ] **Per-MFX-type parameter labels.** The 32 generic MFX numeric slots
      get type-specific meanings depending on the selected effect (e.g.
      for Type=01 Equalizer, slot 1 = Low Freq, slot 2 = Low Gain, etc.).
      MC-707 Reference Manual pages 84–119 plus the 1.80 update document
      every label for all 94 effect types. Adding them is ~900 entries
      of mechanical transcription — would benefit from a second
      pdftotext-driven parser.
- [ ] **Master EQ / Master Comp / mixer named parameters.** The system
      block at `0x10000000..0x10004000` is already dumped wholesale, but
      the inner fields aren't decoded into named parameters. The
      Jupiter-X PDF's `[System Common]`, `[Master EQ]`, `[Master Comp]`,
      `[Reverb]`, `[Chorus]`, `[Drive]` tables document the byte
      offsets. The MC inner layouts are very likely identical (same
      ZEN-Core engine).

### Medium value, address-unverified

These need either an MC-specific reference (which we don't have) or
hardware exploration via `hack_tools.py` to find the correct SysEx
address before the parameter table can be added safely.

- [ ] **Drum Kit Tone parameters.** MC-707 / MC-101 reference manuals
      document the parameters (KIT MFX CTRL, DRUM COMP1-6, PAD CTRL,
      PAD EQ, INST COMMON, INST WAVE) but at an unknown SysEx address.
      Jupiter-X has no equivalent block. Needs hardware probing.
- [ ] **Scene "Part" parameters.** Per-track-per-scene overlay
      parameters (Level, Pan, Coarse/Fine Tune, Cutoff Offset,
      Resonance Offset, Attack/Decay/Release Offset, Vib Rate/Depth/Delay).
      Documented in MC reference manual page 64. Probably at the
      `[Scene Part]` address from the Jupiter-X map (`0x01000000` base),
      but unverified for MC.
- [ ] **Scene block as a whole.** Tempo, scene name, scene-level FX
      assignments. Same situation as Scene Part parameters.

### Low value or device-specific

- [ ] **Looper track parameters.** MC-101 / MC-707 reference manuals
      document playback / time-stretch / sample-edit metadata for
      Looper tracks. Probably not at any standard ZEN-Core address.
- [ ] **Sample Edit metadata** (Start, Loop, End, Mode, Level, Tune,
      Gain, Original Key) — same situation.
- [ ] **Arpeggio user pattern data** — Jupiter-X documents it; MC layout
      unverified.

### Quality-of-life improvements

- [ ] CI: GitHub Actions workflow that runs `python -m unittest`,
      `swift test`, and `cargo test --all-features` on every push
- [ ] Publish `mc_programmer` to PyPI under a friendly version
- [ ] Add the Swift package to the Swift Package Index
- [ ] Hex-dump pretty-printer for debugging (decode a raw 256-byte step
      record to a human-readable format)
- [ ] CLI tool — `mc-programmer dump my-project.mctproject` (Python entry point)
- [ ] Type-stub-only file (`py.typed` marker is in place; could add
      explicit `.pyi` files for tighter type checking)

### Known small bugs / cleanups

- [ ] The Swift Project test takes ~40 seconds due to single-threaded
      chunked I/O against `LoopbackTransport`. The Python equivalent is
      ~13 seconds. Both are exercising real code paths so the speed
      difference isn't a correctness issue, but the Swift test could be
      sped up by parallelising the loopback dispatch.
- [ ] `LoopbackTransport.snapshot()` defaults its unread bytes to `0`
      (for test inspection) while `transfer()` defaults them to `0xFF`
      (matching the real MC's empty-step encoding). The asymmetry is
      documented in the doc string but is mildly surprising.

---

## How to contribute

1. **Adding a parameter**: edit
   `tools/gen_zencore_params.py`'s `SECTION_NAME_MAP` if you've found a
   new ZCore substructure in the extracted Jupiter-X text, or
   re-extract from a newer Jupiter-X PDF revision. Run
   `python tools/gen_zencore_params.py` to regenerate
   `python_lib/mc_programmer/zencore_data.py` and
   `swift_lib/Sources/MCProgrammer/ZenCoreData.swift`, then run
   `PYTHONPATH=python_lib python3 tools/gen_rust_zencore.py` for
   `rust_lib/src/zencore_data.rs`.
2. **Adding a verified MC address**: add the matching constants to the Python,
   Swift, and Rust address-map modules, then build a parameter table for it the
   same way (or, for opaque blobs, add it to `Project` directly).
3. **Probing for unverified addresses**: the original
   `hack_tools.py` from the test scripts (now removed from this repo
   but visible in git history) is the reference for memory-dump
   exploration on real MC hardware. The same approach can be ported
   into the new library by extending `MCClient` with a `walk_address_space`
   helper.
4. Run all three test suites and make sure everything still passes:
   ```bash
   PYTHONPATH=python_lib python -m unittest python_lib.tests.test_mc_programmer
   (cd swift_lib && swift test)
   (cd rust_lib && cargo test --all-features)
   ```
