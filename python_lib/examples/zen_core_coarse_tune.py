"""Mirror of the original ``zen_core_tools.py`` demo: nudge coarse tune
five times for the track-2 ZEN-Core tone, reading back the value each step.

Runs end-to-end against ``LoopbackTransport`` (an in-memory fake MC).
To drive real hardware, swap in ``RtMidiTransport.connect("MC-707")``
from ``mc_programmer.coremidi``.

    PYTHONPATH=python_lib python python_lib/examples/zen_core_coarse_tune.py
"""
from mc_programmer import LoopbackTransport, MCClient, ModelID


def main() -> None:
    client = MCClient(LoopbackTransport(), model_id=ModelID.MC_707)

    # Initialise the coarse-tune parameter to center (64).
    client.write_coarse_tune(trk_idx=1, clip_idx=None, value=64)
    print(f"starting coarse tune = {client.read_coarse_tune(trk_idx=1, clip_idx=None)}")

    for i in range(5):
        v = client.nudge_coarse_tune(trk_idx=1, clip_idx=None, step=+1)
        print(f"step {i + 1}: coarse tune now = {v}")


if __name__ == "__main__":
    main()
