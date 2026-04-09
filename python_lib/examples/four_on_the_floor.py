"""Write a 4-on-the-floor kick to clip 1 of track 1, then read it back.

Uses ``LoopbackTransport`` — an in-memory fake MC device — so this runs
end-to-end without any MIDI hardware. The ``MCClient`` calls below are
exactly what you would write against a real device; just swap in
``RtMidiTransport.connect("MC-707")`` from ``mc_programmer.coremidi`` when
you're ready to drive hardware.

Run from the repo root:

    PYTHONPATH=python_lib python python_lib/examples/four_on_the_floor.py
"""
from mc_programmer import (
    LoopbackTransport,
    MCClient,
    ModelID,
    NoteComponent,
    NoteStep,
)


def main() -> None:
    client = MCClient(LoopbackTransport(), model_id=ModelID.MC_707)

    # 1) Wipe the first 16 steps of clip 1 / track 1.
    for s in range(16):
        client.write_note_step(0, 0, s, NoteStep())

    # 2) Place a kick (C1 = MIDI note 36) on every downbeat.
    for s in (0, 4, 8, 12):
        step = NoteStep()
        step.components[0] = NoteComponent(note=36, velocity=120)
        client.write_note_step(0, 0, s, step)

    # 3) Read the steps back to verify.
    hits = []
    for s in range(16):
        step = client.read_note_step(0, 0, s)
        if step.active_components():
            hits.append(s)
    print(f"Steps with notes: {hits}")
    step4 = client.read_note_step(0, 0, 4)
    print(f"Step 4 lane 0 note: {step4.components[0].note}")


if __name__ == "__main__":
    main()
