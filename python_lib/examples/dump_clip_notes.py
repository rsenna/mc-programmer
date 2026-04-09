"""Print the active notes of every step in clip 1 of track 1.

This example talks to a real MC over MIDI via ``RtMidiTransport``, which
uses CoreMIDI on macOS, ALSA on Linux, and WinMM on Windows under the
hood. To run it, install one extra package::

    pip install python-rtmidi

Then plug in your MC, find the device's MIDI port name, and run::

    PYTHONPATH=python_lib python python_lib/examples/dump_clip_notes.py

If you're not sure of the port name, this is the easiest way to discover
it::

    >>> from mc_programmer.coremidi import RtMidiTransport
    >>> RtMidiTransport.list_ports()
    {'inputs': ['MC-707', 'IAC Driver Bus 1'],
     'outputs': ['MC-707', 'IAC Driver Bus 1']}

You can swap ``RtMidiTransport`` for ``LoopbackTransport`` to dry-run the
example offline.
"""
from mc_programmer import MCClient, ModelID
from mc_programmer.coremidi import RtMidiTransport


def dump_clip(client: MCClient, trk_idx: int, clip_idx: int) -> None:
    scale, step_count = client.read_clip_step_params(trk_idx, clip_idx)
    print(f"Track {trk_idx + 1} clip {clip_idx + 1}: "
          f"{step_count} steps @ {scale.label}")
    for s in range(step_count):
        step = client.read_note_step(trk_idx, clip_idx, s)
        for c in step.active_components():
            tag = " (tie)" if c.ties_to_next else ""
            print(f"  step {s:3d}: note={c.note:3d} vel={c.velocity:3d} "
                  f"start={c.start_cents:+d}c end={c.end_cents}c{tag}")


if __name__ == "__main__":
    transport = RtMidiTransport.connect("MC-707")
    try:
        client = MCClient(transport, model_id=ModelID.MC_707)
        dump_clip(client, trk_idx=0, clip_idx=0)
    finally:
        transport.close()
