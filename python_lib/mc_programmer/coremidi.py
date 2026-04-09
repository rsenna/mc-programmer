"""CoreMIDI / rtmidi / mido transports for the mc_programmer library.

Python doesn't ship a native CoreMIDI binding, so on macOS the standard
route is the cross-platform `python-rtmidi` package, which is implemented
on top of CoreMIDI on Apple platforms (and ALSA on Linux, WinMM on
Windows). The two transports in this module use that route:

* :class:`RtMidiTransport` — minimal wrapper around `python-rtmidi`. No
  middleware. The fastest path; recommended.
* :class:`MidoTransport`  — wrapper around `mido`, for users who already
  have a mido-based setup. `mido` itself can use rtmidi or other
  backends; either is fine.

Both implementations satisfy :class:`mc_programmer.Transport` and can be passed
straight to :class:`mc_programmer.MCClient`. They reassemble Sys-Ex frames at
the 0xF0 / 0xF7 boundaries, so a single ``transfer`` call always returns
exactly one complete reply (or an empty list on timeout).

These are *imported lazily*: the surrounding mc_programmer package has no hard
dependency on rtmidi or mido. You only need whichever package you decide
to use::

    pip install python-rtmidi   # for RtMidiTransport
    pip install mido            # for MidoTransport (also pulls in rtmidi)

Example::

    from mc_programmer import MCClient, ModelID
    from mc_programmer.coremidi import RtMidiTransport

    transport = RtMidiTransport.connect("MC-707")
    client = MCClient(transport, model_id=ModelID.MC_707)
    print("coarse tune:", client.read_coarse_tune(trk_idx=1))

The classmethod :meth:`RtMidiTransport.list_ports` is handy when you
don't already know your device's port name::

    >>> RtMidiTransport.list_ports()
    {'inputs': ['MC-707', 'IAC Driver Bus 1'],
     'outputs': ['MC-707', 'IAC Driver Bus 1']}
"""
from __future__ import annotations

import time
from typing import Dict, List, Optional, Sequence


# ---------------------------------------------------------------------------
# RtMidiTransport
# ---------------------------------------------------------------------------

class RtMidiTransport:
    """Synchronous Sys-Ex transport built on `python-rtmidi`.

    On macOS this routes through CoreMIDI; on Linux through ALSA; on
    Windows through WinMM. The transport reassembles Sys-Ex messages at
    the 0xF0/0xF7 boundaries, so each ``transfer`` call returns at most
    one complete reply.
    """

    def __init__(self, midi_in, midi_out) -> None:
        # Construct via the helpers below — direct construction takes the
        # already-opened `rtmidi.MidiIn` / `rtmidi.MidiOut` instances.
        self._in = midi_in
        self._out = midi_out
        self._frames: List[List[int]] = []
        self._pending: List[int] = []
        # Tell rtmidi to deliver us Sys-Ex (it ignores it by default).
        self._in.ignore_types(sysex=False, timing=True, active_sense=True)
        self._in.set_callback(self._on_message)

    # ---- construction helpers ---------------------------------------------

    @classmethod
    def list_ports(cls) -> Dict[str, List[str]]:
        """Return ``{'inputs': [...], 'outputs': [...]}``.

        Useful for letting a user pick a device by name.
        """
        rtmidi = _import_rtmidi()
        midi_in = rtmidi.MidiIn()
        midi_out = rtmidi.MidiOut()
        try:
            return {
                "inputs": list(midi_in.get_ports()),
                "outputs": list(midi_out.get_ports()),
            }
        finally:
            midi_in.delete()
            midi_out.delete()

    @classmethod
    def connect(
        cls,
        device_name: str,
        *,
        in_name: Optional[str] = None,
        out_name: Optional[str] = None,
        client_name: str = "mc_programmer",
    ) -> "RtMidiTransport":
        """Open the input and output ports for a Roland MC device.

        ``device_name`` is matched against the available rtmidi port names
        with a substring match (Roland tends to vary the suffix between
        macOS and Linux). Pass ``in_name`` / ``out_name`` explicitly if
        the device exposes its source and destination under different
        names.
        """
        rtmidi = _import_rtmidi()
        in_target = in_name or device_name
        out_target = out_name or device_name

        midi_in = rtmidi.MidiIn(name=client_name)
        midi_out = rtmidi.MidiOut(name=client_name)

        in_idx = _find_port(midi_in.get_ports(), in_target)
        out_idx = _find_port(midi_out.get_ports(), out_target)
        if in_idx is None:
            midi_in.delete()
            midi_out.delete()
            raise RuntimeError(
                f"no MIDI input matching {in_target!r}; "
                f"available: {midi_in.get_ports()}"
            )
        if out_idx is None:
            midi_in.delete()
            midi_out.delete()
            raise RuntimeError(
                f"no MIDI output matching {out_target!r}; "
                f"available: {midi_out.get_ports()}"
            )

        midi_in.open_port(in_idx)
        midi_out.open_port(out_idx)
        return cls(midi_in, midi_out)

    def close(self) -> None:
        try:
            self._in.cancel_callback()
        except Exception:
            pass
        try:
            self._in.close_port()
        finally:
            self._out.close_port()
        self._in.delete()
        self._out.delete()

    # ---- Transport protocol -----------------------------------------------

    def send_raw(self, message: Sequence[int]) -> None:
        """Send a non-Sys-Ex MIDI message (CC, Program Change, …)."""
        self._out.send_message(list(message))

    def transfer(self, message: Sequence[int], timeout: float = 0.1) -> List[int]:
        # Drop anything stale before sending. The framing callback may have
        # accumulated bytes from earlier transfers; flush them now.
        self._frames.clear()
        self._pending.clear()

        self._out.send_message(list(message))

        deadline = time.monotonic() + timeout
        while time.monotonic() < deadline:
            if self._frames:
                return self._frames.pop(0)
            time.sleep(0.001)
        return []

    # ---- rtmidi callback --------------------------------------------------

    def _on_message(self, event, _data=None) -> None:
        # rtmidi delivers each message as ``(message, delta_time)``.
        msg, _ = event
        for b in msg:
            if not self._pending and b != 0xF0:
                continue   # ignore active-sensing / clock / stray bytes
            self._pending.append(b)
            if b == 0xF7:
                self._frames.append(self._pending)
                self._pending = []


# ---------------------------------------------------------------------------
# MidoTransport
# ---------------------------------------------------------------------------

class MidoTransport:
    """Synchronous Sys-Ex transport built on `mido`.

    Use this if you already have a mido-based setup. mido normalises
    Sys-Ex into a single ``Message(type='sysex', data=...)`` frame, so we
    don't have to do any byte-level reassembly here.
    """

    def __init__(self, in_port, out_port) -> None:
        self._in = in_port
        self._out = out_port

    @classmethod
    def list_ports(cls) -> Dict[str, List[str]]:
        mido = _import_mido()
        return {
            "inputs": list(mido.get_input_names()),
            "outputs": list(mido.get_output_names()),
        }

    @classmethod
    def connect(
        cls,
        device_name: str,
        *,
        in_name: Optional[str] = None,
        out_name: Optional[str] = None,
    ) -> "MidoTransport":
        mido = _import_mido()
        in_target = in_name or device_name
        out_target = out_name or device_name

        in_match = _find_port(mido.get_input_names(), in_target)
        out_match = _find_port(mido.get_output_names(), out_target)
        if in_match is None:
            raise RuntimeError(
                f"no MIDI input matching {in_target!r}; "
                f"available: {mido.get_input_names()}"
            )
        if out_match is None:
            raise RuntimeError(
                f"no MIDI output matching {out_target!r}; "
                f"available: {mido.get_output_names()}"
            )

        in_port = mido.open_input(mido.get_input_names()[in_match])
        out_port = mido.open_output(mido.get_output_names()[out_match])
        return cls(in_port, out_port)

    def close(self) -> None:
        self._in.close()
        self._out.close()

    def send_raw(self, message: Sequence[int]) -> None:
        """Send a non-Sys-Ex MIDI message (CC, Program Change, …)."""
        mido = _import_mido()
        msg = list(message)
        if not msg:
            return
        status = msg[0] & 0xF0
        channel = msg[0] & 0x0F
        if status == 0xC0 and len(msg) >= 2:           # Program Change
            self._out.send(mido.Message("program_change", channel=channel, program=msg[1]))
        elif status == 0xB0 and len(msg) >= 3:         # Control Change
            self._out.send(mido.Message("control_change", channel=channel,
                                         control=msg[1], value=msg[2]))
        else:
            # Last-resort: pack as bytes through a generic message.
            self._out.send(mido.parse(msg))

    def transfer(self, message: Sequence[int], timeout: float = 0.1) -> List[int]:
        mido = _import_mido()
        # Drain stale input.
        while self._in.poll() is not None:
            pass

        # mido's sysex Message wants the *payload* (between 0xF0 and 0xF7),
        # not the framed message. Strip the framing.
        if message and message[0] == 0xF0 and message[-1] == 0xF7:
            payload = list(message)[1:-1]
        else:
            payload = list(message)
        self._out.send(mido.Message("sysex", data=payload))

        deadline = time.monotonic() + timeout
        while time.monotonic() < deadline:
            msg = self._in.poll()
            if msg is None:
                time.sleep(0.001)
                continue
            if msg.type == "sysex":
                return [0xF0] + list(msg.data) + [0xF7]
        return []


# ---------------------------------------------------------------------------
# Lazy imports + helpers
# ---------------------------------------------------------------------------

def _import_rtmidi():
    try:
        import rtmidi  # type: ignore
    except ImportError as exc:
        raise ImportError(
            "RtMidiTransport requires the python-rtmidi package — "
            "install with `pip install python-rtmidi`"
        ) from exc
    return rtmidi


def _import_mido():
    try:
        import mido  # type: ignore
    except ImportError as exc:
        raise ImportError(
            "MidoTransport requires the mido package — "
            "install with `pip install mido python-rtmidi`"
        ) from exc
    return mido


def _find_port(names: Sequence[str], target: str) -> Optional[int]:
    """Return the first index in ``names`` whose entry contains ``target``.

    Match is case-insensitive substring. ``mido``/``rtmidi`` decorate port
    names with platform-specific prefixes/suffixes, so an exact match is
    too strict — substring is the right tradeoff.
    """
    target_lower = target.lower()
    for i, name in enumerate(names):
        if target_lower in name.lower():
            return i
    return None
