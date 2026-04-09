"""Roland System Exclusive protocol primitives for MC-101, MC-707, MV-1.

The MC family uses the same RQ1 (data request, 0x11) and DT1 (data set, 0x12)
message format that is documented for many other Roland instruments. The only
device-specific bit is the model ID byte (index 6 of the message), which the
ZC programming video calls out as 0x5D for MC-707 and 0x5E for MC-101 — these
match what the test scripts auto-detect via universal device-inquiry.

Address and length fields are 32 bits laid out big-endian across four 7-bit
MIDI bytes. The checksum is the standard Roland complement-to-128 over the
address + payload bytes.
"""
from __future__ import annotations

from dataclasses import dataclass
from enum import IntEnum
from typing import List, Sequence, Tuple


SYSEX_START = 0xF0
SYSEX_END = 0xF7
ROLAND_ID = 0x41
DEVICE_ID = 0x10  # default Roland device ID

CMD_RQ1 = 0x11  # data request 1
CMD_DT1 = 0x12  # data set 1


class ModelID(IntEnum):
    """Known model identification bytes for the MC / MV family.

    Detected automatically by the ``SysExComm.detect_id`` routine in the
    reference Python scripts via a universal device-inquiry exchange. The
    values below are the ones the ZC and MC video transcripts confirm.
    """

    MC_707 = 0x5D
    MC_101 = 0x5E
    MV_1 = 0x5F  # observed working similarly per the MC video


def roland_checksum(payload: Sequence[int]) -> int:
    """Compute the Roland Sys-Ex checksum.

    ``payload`` is the sequence of bytes that the checksum is computed over —
    typically the four address bytes followed by the data bytes (i.e. every
    byte from index 8 of the full message up to but not including the
    checksum byte itself).
    """
    s = sum(b & 0x7F for b in payload) % 128
    return (128 - s) & 0x7F


def _address_bytes(address: int) -> List[int]:
    return [
        (address >> 24) & 0x7F,
        (address >> 16) & 0x7F,
        (address >> 8) & 0x7F,
        address & 0x7F,
    ]


def _length_bytes(count: int) -> List[int]:
    """Encode a request length as four 7-bit bytes (big-endian).

    Note: the reference Python scripts only use the low byte of the length
    field, but here we encode all four bytes correctly so callers can request
    larger blocks if the device permits it.
    """
    return [
        (count >> 21) & 0x7F,
        (count >> 14) & 0x7F,
        (count >> 7) & 0x7F,
        count & 0x7F,
    ]


def build_rq1(model_id: int, address: int, count: int) -> List[int]:
    """Build a complete RQ1 (data request 1) Sys-Ex message."""
    header = [
        SYSEX_START,
        ROLAND_ID,
        DEVICE_ID,
        0x00,
        0x00,
        0x00,
        model_id & 0x7F,
        CMD_RQ1,
    ]
    body = _address_bytes(address) + _length_bytes(count)
    checksum = roland_checksum(body)
    return header + body + [checksum, SYSEX_END]


def build_dt1(model_id: int, address: int, data: Sequence[int]) -> List[int]:
    """Build a complete DT1 (data set 1) Sys-Ex message."""
    header = [
        SYSEX_START,
        ROLAND_ID,
        DEVICE_ID,
        0x00,
        0x00,
        0x00,
        model_id & 0x7F,
        CMD_DT1,
    ]
    body = _address_bytes(address) + [b & 0x7F for b in data]
    checksum = roland_checksum(body)
    return header + body + [checksum, SYSEX_END]


@dataclass(frozen=True)
class DT1Message:
    address: int
    data: List[int]


def parse_dt1_payload(message: Sequence[int]) -> DT1Message:
    """Extract address and payload bytes from a complete DT1 Sys-Ex message.

    Raises ``ValueError`` if the framing or command byte does not match.
    """
    if len(message) < 14:
        raise ValueError(f"DT1 message too short: {len(message)} bytes")
    if message[0] != SYSEX_START or message[-1] != SYSEX_END:
        raise ValueError("Missing Sys-Ex framing bytes")
    if message[1] != ROLAND_ID:
        raise ValueError(f"Not a Roland message (id 0x{message[1]:02X})")
    if message[7] != CMD_DT1:
        raise ValueError(f"Not a DT1 message (cmd 0x{message[7]:02X})")

    address = (
        (message[8] << 24)
        | (message[9] << 16)
        | (message[10] << 8)
        | message[11]
    )
    data = list(message[12:-2])
    return DT1Message(address=address, data=data)


def universal_device_inquiry() -> List[int]:
    """The universal device inquiry message used to detect the model ID."""
    return [0xF0, 0x7E, 0x7F, 0x06, 0x01, 0xF7]
