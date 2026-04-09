"""Note-name and drum-key helpers — the musician-friendly entry points.

The Roland MC family follows the **Roland / Yamaha** octave convention:
middle C is **C3 = MIDI 60**, so a kick on C1 is MIDI 36. (The "scientific
pitch notation" used by some software puts middle C at C4 = 60, which
maps the same kick to C2. We follow Roland here so the numbers match
what shows on the MC-707 / MC-101 screen.)

Quick reference for the most common drum keys::

    note("C1") == 36   # KICK
    note("D1") == 38   # SNARE
    note("F#1") == 42  # HAT_CLOSED
    note("A#1") == 46  # HAT_OPEN

You can also pass plain MIDI numbers — :func:`note` is a no-op on ints,
so it's safe to wrap any value the user gives you::

    note(60)     == 60
    note("C3")   == 60
    note("Bb-1") == 10
"""
from __future__ import annotations

from typing import Union


# --- Drum-kit shortcuts (General MIDI / Roland defaults) -------------------
KICK         = 36   # C1
SIDE_STICK   = 37   # C#1
SNARE        = 38   # D1
CLAP         = 39   # D#1
SNARE_RIM    = 40   # E1
TOM_LOW      = 41   # F1
HAT_CLOSED   = 42   # F#1
TOM_LOW_MID  = 43   # G1
HAT_PEDAL    = 44   # G#1
TOM_MID      = 45   # A1
HAT_OPEN     = 46   # A#1
TOM_HIGH_MID = 47   # B1
TOM_HIGH     = 48   # C2
CRASH        = 49   # C#2
RIDE         = 51   # D#2


_NOTE_OFFSETS = {
    "C":  0, "C#": 1, "DB": 1, "D":  2, "D#": 3, "EB": 3, "E":  4,
    "F":  5, "F#": 6, "GB": 6, "G":  7, "G#": 8, "AB": 8,
    "A":  9, "A#": 10, "BB": 10, "B": 11,
}


def note(name: Union[str, int]) -> int:
    """Convert a note name like ``"C1"`` to a MIDI note number (0..127).

    Accepts:

    * Plain MIDI numbers (returned unchanged) — ``note(60)`` → ``60``
    * Note names with explicit octave — ``note("C1")`` → ``36``,
      ``note("F#3")`` → ``66``
    * Note names without an octave (assumes octave 3, i.e. middle-octave)
      — ``note("C")`` → ``60``
    * Lower or mixed case — ``note("eb2")`` → ``51``
    * Negative octaves — ``note("C-1")`` → ``0``
    * Both ``#`` and ``b`` accidentals — ``note("Bb1")`` → ``46``
    """
    if isinstance(name, int):
        if not 0 <= name <= 127:
            raise ValueError(f"MIDI note out of range: {name} (must be 0..127)")
        return name

    s = str(name).strip()
    if not s:
        raise ValueError("note name is empty")

    # Pull off the letter + optional accidental.
    upper = s.upper()
    if len(upper) >= 2 and upper[1] in "#B":
        letter = upper[:2]
        rest = upper[2:]
    else:
        letter = upper[:1]
        rest = upper[1:]

    if letter not in _NOTE_OFFSETS:
        raise ValueError(
            f"unrecognised note name {name!r} — expected something like "
            "'C1', 'F#3', 'Bb-1', or a MIDI number 0..127"
        )

    if rest == "":
        octave = 3   # Roland convention: C3 is middle C
    else:
        try:
            octave = int(rest)
        except ValueError as e:
            raise ValueError(
                f"unrecognised note name {name!r} — couldn't parse "
                f"octave {rest!r}"
            ) from e

    midi = (octave + 2) * 12 + _NOTE_OFFSETS[letter]
    if not 0 <= midi <= 127:
        raise ValueError(
            f"note {name!r} resolves to MIDI {midi} which is out of range 0..127"
        )
    return midi


def note_name(midi: int) -> str:
    """Inverse of :func:`note` — turn a MIDI number back into a name like ``"C3"``.

    Returns the sharp form for accidentals (``"C#1"``, never ``"Db1"``).
    """
    if not 0 <= midi <= 127:
        raise ValueError(f"MIDI note out of range: {midi}")
    sharps = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"]
    octave = midi // 12 - 2          # Roland convention (C3 = 60)
    return f"{sharps[midi % 12]}{octave}"
