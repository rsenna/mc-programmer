# Promotional copy

A grab bag of project descriptions, taglines, and topic tags at
different lengths and tones. Pick whichever fits the surface you're
filling out — GitHub "About" field, README opener, social post,
package index listing, conference talk abstract.

Every variant below describes the same project: `mc-programmer`, the parallel
Python + Swift + Rust libraries for the Roland MC-101 / MC-707 / MV-1
grooveboxes.

---

## GitHub "About" sidebar (recommended)

The "About" field on a repo's main page caps at ~350 characters and is
what shows up in GitHub search results, topic listings, and the social
preview card.

> **Python, Swift, and Rust libraries for programming the Roland MC-101,
> MC-707 and MV-1 grooveboxes over MIDI System Exclusive — sequencer,
> ZEN-Core synth parameters, scenes, clips, and full project save/restore.**

*279 chars. Says exactly what it is, what platforms it covers, what
hardware it targets, and what it can do. Won't get cut off anywhere.*

### Slightly punchier alternative

> **Stop menu-diving on your Roland MC-707 / MC-101 / MV-1. Read and
> write any sequencer step, edit ~389 ZEN-Core synth parameters, switch
> scenes and clips, and back up whole projects from Python, Swift, or Rust
> over MIDI Sys-Ex.**

*Calls out the pain point musicians actually feel ("menu-diving") and
gives a concrete number for the parameter coverage. ~280 chars.*

---

## One-line tagline / subtitle

For use as a project subtitle, the line just under the project name on
the README, or anywhere a single concise sentence is wanted.

> **Code-driven sound design and pattern programming for the Roland MC
> family.**

*72 chars. The shortest variant that still tells the reader what the
project does and which hardware it targets.*

---

## README opening paragraph (long form)

For the first paragraph of the project README, where you have room to
set expectations and mention the offline development story.

> **`mc-programmer`** is a set of three parallel libraries — Python,
> Swift, and Rust — that talk to the Roland MC-101, MC-707, and MV-1
> grooveboxes over MIDI System Exclusive. You can read and write any
> sequencer step, change any of ~389 ZEN-Core synth parameters, switch scenes
> and clips, dump and restore individual sounds or entire projects, and drive
> the device from any code that can send MIDI. All three libraries share the
> same data model, parameter table, and `LoopbackTransport` in-memory fake
> device so you can develop and test without hardware.

*Three sentences, comprehensive, sets expectations, mentions the
offline development story.*

---

## Social-media post

For Mastodon, Bluesky, Hacker News title, link-roundup blog, etc.

> **`mc-programmer`**: the sound librarian / pattern editor / MIDI
> Sys-Ex toolkit Roland never shipped for the MC-101 and MC-707.
> Python, Swift, and Rust; MIT-licensed; no menu diving.

*~170 chars. Fits in a Mastodon post or HN title with room for a link.
The "Roland never shipped" framing positions it clearly against the
PG-200 / PG-800 hardware programmer precedent.*

### Hacker News title alternative

> **Show HN: `mc-programmer` — Python/Swift/Rust libraries for the Roland
> MC-707 and MC-101 grooveboxes**

*"Show HN" is the standard prefix for posting your own project; this
one fits HN's title length and tells the audience exactly what to
expect.*

---

## Package-index listings

### PyPI long description

(Paste this into the `description` field of `pyproject.toml`, or use
it as the body of a longer `[project.long_description]` section.)

> **Read and write any sequencer step, edit any of ~389 ZEN-Core synth
> parameters, switch scenes and clips, and dump or restore whole
> projects on the Roland MC-101, MC-707, and MV-1 grooveboxes — over
> MIDI System Exclusive, from Python. Built on a pluggable transport
> protocol with a CoreMIDI / `python-rtmidi` backend, plus a
> `LoopbackTransport` in-memory fake device for offline development
> and CI. Companion Swift and Rust libraries available.**

### Swift Package Index description

> **Native Swift library for programming the Roland MC-101, MC-707 and
> MV-1 grooveboxes over MIDI System Exclusive. CoreMIDI transport,
> ~389 ZEN-Core synth parameters, sequencer step encode/decode,
> tone-preset and full-project save/restore. Companion Python and Rust
> libraries available.**

### crates.io description

> **Rust library for programming the Roland MC-101, MC-707, and MV-1 over
> MIDI System Exclusive. Includes the full sequencer and ~389 ZEN-Core
> parameters, tone/project save and restore, an optional cross-platform
> `midir` transport, and an in-memory fake device for offline development.**

---

## GitHub topic tags

Recommended tags for the repo's topics list (the small chips that show
under the description). These are all recognized GitHub topics that
put you in the discovery feed for the relevant category:

```
roland
mc-707
mc-101
mv-1
groovebox
midi
sysex
zen-core
swift
python
rust
coremidi
synthesizer-editor
sound-librarian
```

Pick the first 12 (GitHub's display limit) — these are ordered by
likely-search-value to a musician browsing the categories.

---

## Conference talk / blog post abstract

For if you ever write this up at length — a conference proposal, a
"how I built this" blog post, an interview question.

> The Roland MC-707 and MC-101 grooveboxes ship with a powerful
> ZEN-Core synth engine, a 16-lane polyphonic sequencer, and ~400
> tweakable parameters per tone — all hidden behind a small screen and
> a few knobs. Roland never shipped a software editor or librarian for
> these devices. `mc-programmer` is a clean-room reimplementation of
> the underlying MIDI System Exclusive protocol as three parallel
> libraries (Python + Swift + Rust), built from publicly available reference
> documentation and verified against real hardware. It exposes the
> sequencer, the full ZEN-Core parameter table, scene and clip
> selection, and full-project save/restore as ordinary read/write
> calls — so you can program patterns from a script, version-control
> your patches in git, build custom hardware controllers, drive the
> MC from a DAW, or just stop menu-diving.

---

## Use-case headlines

For when you need a five-word answer to "what's it for?". Mix and
match in marketing copy.

* "Save a sound. Load it tomorrow."
* "Patches in git. Diffs that work."
* "Your MC, scriptable from Python."
* "Build a hardware controller in an afternoon."
* "16-step house pattern in 5 lines of code."
* "Snapshot your live set every hour. Automatically."
* "The PG-200 Roland never built — for the MC family."

---

## Audience-specific framings

### For musicians

> **You bought an MC-707 because the sounds are great. You stopped
> using half its features because the menu diving is terrible. This
> library lets you do the deep parameter editing, the patch backup,
> and the bulk pattern programming from your computer — keeping the
> MC's amazing sound engine and dropping the awkward UI.**

### For developers

> **A clean, layered MIDI Sys-Ex client library for the Roland MC
> family, with the parameter table auto-generated from the official
> Jupiter-X MIDI Implementation, 202 tests across three languages, an
> in-memory `LoopbackTransport` so you can develop without hardware,
> and a strict no-Roland-copyrighted-content policy. The Python, Swift,
> and Rust implementations share the same address map and data model — pick
> whichever ecosystem fits your project.**

### For Roland nerds

> **Remember the PG-200 and PG-800 hardware programmers Roland used
> to ship for the JX-3P and MKS-50? `mc-programmer` is the same idea
> for the MC-101 and MC-707, but in software, in three languages,
> covering both halves of the ZCore tone descriptor (conventional
> Tone Common/PMT/Partial *and* Synth Common/PMT/Partial), with full
> project save/restore as a bonus. The library is sourced from the
> Jupiter-X MIDI Implementation document because Roland's published
> ZEN-Core engine spec applies to the MC family unchanged — verified
> against the MC-707 and MC-101 reference manuals through firmware
> 1.80.**

---

## Honest-disclosure footer

Anywhere you publicly describe the project, this footer is worth
including so users have realistic expectations and so you can't be
accused of overselling:

> *`mc-programmer` is an unofficial library and is not affiliated
> with, sponsored by, or endorsed by Roland Corporation. It is built
> from publicly available reference documentation and the original
> reverse-engineering work documented in the MC Programming Video and
> ZC Programming Video transcripts that ship with the repository.
> Tested against the MC-707 firmware up to version 1.80; should work
> equally well on the MC-101 and (untested but supported in code) the
> MV-1. Use at your own risk — the library can write any byte of the
> device's parameter memory, so always back up your projects before
> running unfamiliar scripts.*
