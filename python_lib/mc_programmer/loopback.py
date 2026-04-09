"""LoopbackTransport — an in-memory fake MC device.

Implements :class:`mc_programmer.Transport` by maintaining a sparse in-memory
address space: every DT1 (data set) message it receives gets written
into the store, and every RQ1 (data request) it receives is answered
with a DT1 reply built from the store.

This lets examples and tests run end-to-end without any MIDI hardware
using exactly the same :class:`MCClient` calls they would use against a
real device. No reply staging, no message inspection — just write code
that looks like the production path.

Universal device-inquiry messages get a synthetic Identity Reply
containing the configured model ID, so :meth:`MCClient.detect_model_id`
also works against a LoopbackTransport.
"""
from __future__ import annotations

import threading
from typing import Dict, List, Sequence

from .protocol import build_dt1, ModelID


class LoopbackTransport:
    """In-memory fake MC device.

    Construct it like a real transport and pass it straight to
    :class:`MCClient`::

        from mc_programmer import MCClient, ModelID
        from mc_programmer.loopback import LoopbackTransport

        client = MCClient(LoopbackTransport(), model_id=ModelID.MC_707)
        client.write_coarse_tune(trk_idx=1, clip_idx=None, value=70)
        assert client.read_coarse_tune(trk_idx=1, clip_idx=None) == 70
    """

    #: Track of all non-Sys-Ex messages sent through ``send_raw``.
    #: Each entry is the raw byte list. Test code can also inspect
    #: :attr:`active_scene` and :attr:`active_clip_per_channel`, which
    #: are populated whenever a Program Change passes through.
    sent_raw: List[List[int]]

    #: Most-recently-selected scene number, 1-based, or ``None``.
    #: Updated whenever a Program Change is sent on any channel — the
    #: caller is responsible for using the same channel as their MC's
    #: configured "control channel" if they want this to be meaningful.
    active_scene: Optional[int]

    #: Per-MIDI-channel record of the most-recently-selected clip on
    #: that channel, 1-based.
    active_clip_per_channel: Dict[int, int]

    def __init__(
        self,
        model_id: ModelID = ModelID.MC_707,
        unread_byte: int = 0xFF,
    ) -> None:
        """Create an in-memory fake MC.

        ``unread_byte`` is what reads from never-written addresses return.
        Defaults to ``0xFF``, which is what the real MC reports for empty
        sequencer steps — note slots are encoded as ``0x0F 0x0F`` nibble
        pairs and decoded as the ``EMPTY_NOTE`` sentinel ``0xFF``. Test
        code that wants raw zeroed memory can pass ``unread_byte=0``.
        """
        self._memory: Dict[int, int] = {}
        self._model_id = int(model_id)
        self._unread_byte = unread_byte & 0xFF
        self._lock = threading.Lock()
        self.sent_raw = []
        self.active_scene = None
        self.active_clip_per_channel = {}

    # ---- direct manipulation (useful in tests) ----------------------------

    def preload(self, address: int, bytes_: Sequence[int]) -> None:
        """Pre-load a region of the address space."""
        with self._lock:
            for i, b in enumerate(bytes_):
                self._memory[address + i] = b & 0xFF

    def snapshot(self, address: int, count: int) -> List[int]:
        """Read back what's currently stored at an address."""
        with self._lock:
            return [self._memory.get(address + i, self._unread_byte) for i in range(count)]

    # ---- Transport protocol ----------------------------------------------

    def transfer(self, message: Sequence[int], timeout: float = 0.05) -> List[int]:
        msg = list(message)

        # Universal Identity Request → synthetic Identity Reply.
        if (len(msg) == 6 and msg[0] == 0xF0 and msg[1] == 0x7E
                and msg[3] == 0x06 and msg[4] == 0x01 and msg[5] == 0xF7):
            return self._identity_reply()

        if (len(msg) < 14 or msg[0] != 0xF0 or msg[-1] != 0xF7 or msg[1] != 0x41):
            return []

        cmd = msg[7]
        address = (msg[8] << 24) | (msg[9] << 16) | (msg[10] << 8) | msg[11]

        if cmd == 0x12:   # DT1 — store the payload (everything between addr and the checksum)
            payload = msg[12:-2]
            with self._lock:
                for i, b in enumerate(payload):
                    self._memory[address + i] = b & 0xFF
            return []

        if cmd == 0x11:   # RQ1 — synthesise a DT1 reply
            # Length is four nibble-packed bytes at indices 12..15.
            count = (msg[12] << 21) | (msg[13] << 14) | (msg[14] << 7) | msg[15]
            with self._lock:
                data = [self._memory.get(address + i, self._unread_byte) for i in range(count)]
            return build_dt1(self._model_id, address, data)

        return []

    def send_raw(self, message: Sequence[int]) -> None:
        """Record a non-Sys-Ex message and parse Program Change for inspection.

        Note: this is a *fake*. The MC's behavior is "Program Change on
        the control channel selects a Scene; Program Change on a track
        channel selects that track's Clip". Since LoopbackTransport has
        no concept of which channel is the "control channel" — that's
        device-side configuration — we just record both interpretations:
        ``active_scene`` is updated on every PC, and the per-channel
        ``active_clip_per_channel[channel]`` is also updated.
        """
        msg = list(message)
        with self._lock:
            self.sent_raw.append(msg)
        if not msg:
            return
        status = msg[0] & 0xF0
        channel = (msg[0] & 0x0F) + 1
        if status == 0xC0 and len(msg) >= 2:
            program = msg[1] & 0x7F
            with self._lock:
                self.active_scene = program + 1
                self.active_clip_per_channel[channel] = program + 1

    # ---- helpers ----------------------------------------------------------

    def _identity_reply(self) -> List[int]:
        # MCClient.detect_model_id reads index 6 of the reply.
        return [
            0xF0, 0x7E, 0x7F, 0x06, 0x02, 0x41,
            self._model_id & 0x7F,
            0x00, 0x00, 0x00, 0x00, 0x00,
            0xF7,
        ]
