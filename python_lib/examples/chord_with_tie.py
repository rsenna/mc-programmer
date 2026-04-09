"""Build a polyphonic chord step and a tied note crossing two steps,
encode them, and pretty-print the resulting on-device step records.

Runs offline — no MIDI hardware required.

    PYTHONPATH=python_lib python python_lib/examples/chord_with_tie.py
"""
from mc_programmer import (
    NoteComponent,
    NoteStep,
    SubStep,
    encode_note_step,
)


def hexdump(label: str, raw: bytes) -> None:
    print(f"{label} ({len(raw)} bytes):")
    for i in range(0, len(raw), 16):
        row = raw[i:i + 16]
        hex_part = " ".join(f"{b:02X}" for b in row)
        print(f"  {i:03X}  {hex_part}")


def main() -> None:
    # --- 1) A C-major chord on lanes 0..3 of one step --------------------
    chord = NoteStep()
    for lane, midi in enumerate((60, 64, 67, 72)):  # C E G C
        chord.components[lane] = NoteComponent(
            note=midi, velocity=100, start_cents=0, end_cents=80,
        )
    chord.compact()  # match how the firmware packs lanes
    chord.substep = SubStep.NONE

    print("== Polyphonic chord step ==")
    for c in chord.active_components():
        print(f"  lane note={c.note} vel={c.velocity} end={c.end_cents}c")
    hexdump("encoded chord", encode_note_step(chord))

    # --- 2) A tied note that spans two steps -----------------------------
    # Per the MC video: a note 1.92 steps long that starts 10c before step 11.
    step_a = NoteStep()
    step_a.components[0] = NoteComponent(
        note=48, velocity=100, start_cents=-10, end_cents=100,
    )

    step_b = NoteStep()
    step_b.components[0] = NoteComponent(
        note=48, velocity=100, start_cents=0, end_cents=82,
    )

    print("\n== Tied note across step A → step B ==")
    print("  step A: note=48 start=-10c end=100c (tie)")
    print("  step B: note=48 start=  0c end= 82c")
    hexdump("encoded step A", encode_note_step(step_a))
    hexdump("encoded step B", encode_note_step(step_b))


if __name__ == "__main__":
    main()
