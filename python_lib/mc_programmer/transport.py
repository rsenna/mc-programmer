"""Pluggable MIDI transport for the mc_programmer library.

The library deliberately does *not* depend on ``mido`` or any specific MIDI
backend — it speaks raw Sys-Ex byte lists. Provide an implementation of
:class:`Transport` that knows how to send a Sys-Ex message and collect any
reply, and the rest of the library will work.

A trivial in-memory :class:`NullTransport` is included for unit tests.
"""
from __future__ import annotations

from typing import List, Protocol, Sequence


class Transport(Protocol):
    """Two-way Sys-Ex transport for the MC family.

    Implementations need two methods:

    * :meth:`transfer` — send a complete Sys-Ex message (including
      ``0xF0`` and ``0xF7``) and return the reply, if any. ``timeout`` is
      in seconds. An empty list means *no reply*.
    * :meth:`send_raw` — send a non-Sys-Ex MIDI message (Control Change,
      Program Change, etc.) and return immediately. The MC family uses
      this for Scene and Clip selection via Program Change — see
      :meth:`MCClient.select_scene` and :meth:`MCClient.select_clip`.
    """

    def transfer(self, message: Sequence[int], timeout: float = 0.05) -> List[int]:
        ...

    def send_raw(self, message: Sequence[int]) -> None:
        ...


class NullTransport:
    """Records messages without sending anything. Useful for tests.

    Set ``responses`` to a list-of-lists to script replies for ``transfer``.
    """

    def __init__(self, responses: List[List[int]] | None = None) -> None:
        self.sent: List[List[int]] = []
        self.responses: List[List[int]] = list(responses or [])

    def transfer(self, message: Sequence[int], timeout: float = 0.05) -> List[int]:
        self.sent.append(list(message))
        if self.responses:
            return self.responses.pop(0)
        return []

    def send_raw(self, message: Sequence[int]) -> None:
        self.sent.append(list(message))
