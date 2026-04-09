# MCProgrammer (Swift)

Talk to your **Roland MC-101**, **MC-707** or **MV-1** from Swift. Read
and write any sequencer step, change ZEN-Core synth parameters, switch
scenes and clips — all over MIDI System Exclusive, **no front-panel menu
diving required**.

Built from the test scripts and the *MC Programming* / *ZC Programming*
video transcripts in the repo above this directory. Native CoreMIDI
transport included; works on macOS and iOS. Linux builds without
CoreMIDI (the transport is wrapped in `#if canImport(CoreMIDI)`).

---

## Quick start (5 lines)

You don't need a real MC connected to try this — it runs against an
in-memory fake device:

```swift
import MCProgrammer

let client = MCClient(transport: LoopbackTransport(), modelID: .mc707)
try client.setPattern(trackNumber: 1, clipNumber: 1, notes: [
    "C1":  [1, 5, 9, 13],   // kick on every downbeat
    "F#1": [3, 7, 11, 15],  // closed hat on the offbeats
])
```

Replace `LoopbackTransport()` with one line and you're driving real
hardware:

```swift
let transport = try CoreMIDITransport(deviceName: "MC-707")
let client = MCClient(transport: transport, modelID: .mc707)
```

That's it. The rest of this README is "what else can I do" recipes plus
a reference for the lower-level surface.

---

## Add to a project

```swift
// Package.swift
dependencies: [
    .package(path: "../mc-programmer/swift_lib"),
],
targets: [
    .target(name: "MyApp", dependencies: ["MCProgrammer"]),
]
```

Build, test, and run the bundled examples directly:

```bash
cd swift_lib
swift build
swift test
swift run MCProgrammerExamples           # runs every example
swift run MCProgrammerExamples four      # just FourOnTheFloor
```

---

## I'm a musician — what can I do?

Every recipe below uses 1-based numbering (`trackNumber: 1` is the first
track, just like the MC's screen) and accepts note names like `"C1"` *or*
MIDI numbers via the `Note.Pitch` literal type. The MC family follows the
**Roland octave convention** where **C3 = MIDI 60** and the kick drum
lives at **C1 = 36**.

### Make a beat

```swift
let client = MCClient(transport: LoopbackTransport(), modelID: .mc707)

try client.setPattern(trackNumber: 1, clipNumber: 1, notes: [
    "C1":  [1, 5, 9, 13],     // kick
    "D1":  [5, 13],           // snare on the backbeats
    "F#1": [3, 7, 11, 15],    // offbeat closed hat
    "A#1": [9],               // one open hat
])
```

### Place individual notes

```swift
try client.placeNote(trackNumber: 1, clipNumber: 1, stepNumber: 1,
                     pitch: "C1", velocity: 120)
try client.placeNote(trackNumber: 2, clipNumber: 1, stepNumber: 5,
                     pitch: 60)            // MIDI number works too
try client.placeNote(trackNumber: 2, clipNumber: 1, stepNumber: 5,
                     pitch: "E3")          // adds to lane 2 of step 5
try client.placeNote(trackNumber: 2, clipNumber: 1, stepNumber: 5,
                     pitch: "G3")          // …lane 3, …
```

`placeNote` automatically packs concurrent notes into the lowest free
lane (the same way the MC's firmware does), so the order in which you
write them doesn't matter.

### Long notes that tie across steps

```swift
// A note that runs for 2.5 steps starting on step 9.
try client.placeNote(trackNumber: 3, clipNumber: 1, stepNumber: 9,
                     pitch: "C3", lengthSteps: 2.5)
```

The MC's "tie" mechanism (end-time = 100) is handled automatically.

### Erase

```swift
try client.clearStep(trackNumber: 1, clipNumber: 1, stepNumber: 4)
try client.clearClip(trackNumber: 1, clipNumber: 1)
```

### Switch scenes and clips from your code

The MC family uses standard MIDI **Program Change** for scene and clip
recall (per the MC-707 / MC-101 MIDI Implementation Chart):

```swift
try client.selectScene(sceneNumber: 5)              // PC on the control channel (default 1)
try client.selectClip(trackNumber: 2, clipNumber: 8) // PC on the track's MIDI channel
```

If your MC's system menu has the control channel set to something other
than 1, pass it explicitly:

```swift
try client.selectScene(sceneNumber: 5, controlChannel: 16)
```

### Tune a track up by 5 semitones

```swift
try client.writeCoarseTuneSemitones(trackNumber: 2, clipNumber: nil, semitones: +5)
print(try client.readCoarseTuneSemitones(trackNumber: 2, clipNumber: nil))   // → 5
```

`clipNumber: nil` targets the *track-wide* tone. Pass `clipNumber: 1...16`
to target a specific clip's tone.

### Browse what synth parameters you can change

There are ~294 ZEN-Core parameters spread across 8 substructures:

```swift
let common = client.listParameters(group: .common)
print(common.prefix(5))
// → ["analog_feel", "bend_mode", "bend_range_down", "bend_range_up", "category"]
```

---

## Glossary (in plain English)

| Term | What it actually means |
|---|---|
| **Sys-Ex** | A MIDI message specifically for one make/model. The MC family hides 99% of its internals behind these. The library handles the framing for you. |
| **DT1** | "Set this parameter to this value." Roland's name for a Sys-Ex *write*. |
| **RQ1** | "Tell me the value of this parameter." Roland's name for a Sys-Ex *read*. The device replies with a DT1. |
| **ZEN-Core** | The synth engine inside your MC. The same engine ships in the Jupiter-X / Fantom / etc., so the parameter map is shared. |
| **Tone** | One ZEN-Core "patch" — the full synth setup. Each clip has its own tone slot. |
| **Partial** | One of the four "voices" inside a tone. Layer them for richer sounds. |
| **Step** | One slot in the sequencer grid (a clip has up to 128 steps). Each step holds up to 16 concurrent notes. |
| **Lane / component** | One of the 16 note slots inside a single step. The library calls these "components" to match the test scripts. |
| **Tie** | A note that crosses a step boundary. The library does this for you when you pass `lengthSteps > 1`. |
| **Coarse Tune** | Transposition in semitones. The wire range is 16..112 with 64 = no transposition; the library exposes `…Semitones` helpers that use -48...+48 instead. |

---

## I'm a developer — give me the reference

### Architecture

```
┌────────────────────┐
│      MCClient      │  high-level read/write + musician helpers
└─────────┬──────────┘
          │ uses
┌─────────┴──────────┐  ┌──────────────┐  ┌──────────────┐
│  RolandProtocol    │  │  AddressMap  │  │ NoteStepCodec│
│  RQ1/DT1, checksum │  │  + constants │  │ encode/decode│
└─────────┬──────────┘  └──────────────┘  └──────────────┘
          │
┌─────────┴──────────┐
│     Transport      │  ← LoopbackTransport, CoreMIDITransport
└────────────────────┘
```

| File | Public symbols |
|---|---|
| `Protocol.swift` | `SysEx`, `ModelID`, `MCProgrammerError`, `RolandProtocol` |
| `Addresses.swift` | `SequencerMap`, `ZenCoreLayout`, `NoteStepLayout`, `MotionLayout`, `StepScale`, `AddressMap` |
| `Sequencer.swift` | `NoteComponent`, `NoteStep`, `SubStep`, `NoteStepCodec`, `MotionStep` |
| `ZenCore.swift` | `ZCParam` framework + 8 substructure offset/lookup helpers |
| `ZenCoreData.swift` | Auto-generated parameter table (294 entries) |
| `Notes.swift` | `Note.parse(...)` + `DrumKey` constants + `Note.Pitch` literal type |
| `Transport.swift` | `Transport` protocol + `NullTransport` |
| `LoopbackTransport.swift` | In-memory fake MC for offline use |
| `CoreMIDITransport.swift` | Native CoreMIDI on Apple platforms |
| `MCClient.swift` | High-level client + musician helpers |

### Indexing convention

* **Low-level methods** take `track:` / `clip:` / `step:` parameters
  that are **0-based** and map directly onto the underlying address
  arithmetic. Example: `try client.readNoteStep(track: 0, clip: 0, step: 0)`.
* **High-level helpers** take `trackNumber:` / `clipNumber:` / `stepNumber:`
  parameters that are **1-based** and match the labels on the MC's screen.
  Example: `try client.placeNote(trackNumber: 1, clipNumber: 1, stepNumber: 1, pitch: "C1")`.

The `Number` suffix is the discriminator.

### Notes about notes

The Roland MC family follows the **Roland octave convention**, where
**middle C is C3 = MIDI 60**. `Note.parse(_:)` accepts:

* Note names with explicit octave — `"C1"` → 36, `"F#3"` → 66
* Note names without an octave (assumes octave 3) — `"C"` → 60
* Lower or mixed case — `"eb2"` → 51
* Negative octaves — `"C-1"` → 12
* Both `#` and `b` accidentals — `"Bb1"` → 46

The musician helpers take a `Note.Pitch` parameter that conforms to
both `ExpressibleByStringLiteral` and `ExpressibleByIntegerLiteral`,
so you can pass either form interchangeably:

```swift
try client.placeNote(trackNumber: 1, clipNumber: 1, stepNumber: 1, pitch: "C1")
try client.placeNote(trackNumber: 1, clipNumber: 1, stepNumber: 2, pitch: 36)
```

`DrumKey` exposes the common drum-kit constants: `KICK`, `SNARE`, `CLAP`,
`HAT_CLOSED`, `HAT_OPEN`, `RIDE`, `CRASH`, …

### Transport protocol

```swift
public protocol Transport {
    func transfer(_ message: [UInt8], timeout: TimeInterval) throws -> [UInt8]
    func sendRaw(_ message: [UInt8]) throws    // default no-op
}
```

`transfer` is request-with-reply (Sys-Ex). `sendRaw` is fire-and-forget
(any other MIDI message — used by `selectScene` / `selectClip`).

Implementations shipped:

| Transport | Use case |
|---|---|
| `LoopbackTransport` | Offline testing / examples — in-memory fake MC |
| `NullTransport` | Recording-only stub for unit tests |
| `CoreMIDITransport` | Native CoreMIDI on macOS / iOS |

### CoreMIDI transport

```swift
// Single-device convenience init.
let transport = try CoreMIDITransport(deviceName: "MC-707")

// Or distinct source / destination endpoints.
let transport = try CoreMIDITransport(
    sourceName: "MC-707 OUT",
    destinationName: "MC-707 IN"
)

// Discover available endpoints (e.g. for a picker UI).
CoreMIDITransport.availableSources()
CoreMIDITransport.availableDestinations()
```

The transport is thread-safe and synchronous: `transfer(_:timeout:)`
blocks the caller while it waits for one Sys-Ex reply. Receive framing
reassembles bytes between 0xF0 and 0xF7 boundaries — the device may
split a long reply across multiple `MIDIPacket`s.

### Low-level Sys-Ex / address-map API

```swift
// Build / parse Roland Sys-Ex
RolandProtocol.buildRQ1(modelID: 0x5D, address: 0x20030000, count: 256)
RolandProtocol.buildDT1(modelID: 0x5D, address: 0x30000018, data: [70])
try RolandProtocol.parseDT1(replyBytes)

// Address map
let map = AddressMap()
try map.noteStepAddress(track: 0, clip: 0, step: 4)        // 0-based
try map.zenCoreToneAddress(track: 0, clip: nil)            // clip nil = track tone

// MCClient low-level
try client.read(address: addr, count: 256)
try client.write(address: addr, data: bytes)
try client.readNoteStep(track: 0, clip: 0, step: 4)
try client.writeNoteStep(track: 0, clip: 0, step: 4, step)
try client.readZCParam(track: 0, clip: 0, name: "tone_coarse_tune")
try client.writeZCParam(track: 0, clip: 0, name: "tvf_cutoff_frequency",
                         value: .int(512), group: .partial, partialIdx: 1)
```

### What about the SuperNATURAL Synth (SN-S)?

The MC family uses the **conventional ZEN-Core Tone path**, *not* the
SuperNATURAL Synth sub-section. The library deliberately only exposes
the conventional `[Tone Common]` / `[Tone PMT]` / `[Tone Partial]` /
`[Partial Pitch Env]` / `[Partial Filter Env]` / `[Partial Amp Env]` /
`[Partial LFO]` / `[Partial EQ]` substructures from Roland's Jupiter-X
MIDI Implementation, which the ZC programming-video transcript states
applies as-is to MC-101 / MC-707 / MV-1.

### Regenerate the parameter tables

```bash
pdftotext -raw -f 76 -l 82 JUPITER-X_Xm_MIDI_imple_eng06_W.pdf tools_jp_zcore.txt
python tools/gen_zencore_params.py
```

This rewrites both `python_lib/mc_programmer/zencore_data.py` and
`swift_lib/Sources/MCProgrammer/ZenCoreData.swift`.

---

## Tests

```bash
swift test
```

32 tests. They run offline against `LoopbackTransport`, no MIDI device
required.
