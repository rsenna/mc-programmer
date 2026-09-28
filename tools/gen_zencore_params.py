#!/usr/bin/env python3
"""Generate canonical Python, Swift, and Rust ZEN-Core parameter tables.

Reads the raw text extracted from the Jupiter-X/Xm MIDI Implementation PDF
(``pdftotext -raw -f 76 -l 82 JUPITER-X_Xm_MIDI_imple_eng06_W.pdf``) and
emits the Python and Swift tables, then refreshes the Rust table from Python.

  * ``python_lib/mc_programmer/zencore_data.py``  — native Python dicts of ZCParam
  * ``swift_lib/Sources/MCProgrammer/ZenCoreData.swift`` — same data for Swift
  * ``rust_lib/src/zencore_data.rs`` — same table data for Rust

Rust derives ``rust_lib/src/zencore_data.rs`` from the committed Python table
via ``tools/gen_rust_zencore.py``. This script is the only place that knows the
PDF row format. To refresh the tables for a new Roland revision, regenerate
Python and Swift here; the canonical generator then emits Rust from the fresh Python table.

The MC family uses both halves of the ZCore tone descriptor: the conventional
Tone Common / PMT / Partial / Pitch-Filter-Amp Envs / LFO / EQ path and the
Tone Synth Common / PMT / Partial substructures. All are included in the
section map below.

Per the ZC programming-video transcript, this parameter map applies as-is
to the MC-101 / MC-707 / MV-1 — only the *base addresses* of the tone
descriptors differ, and those live in ``mc_programmer.addresses`` /
``MCProgrammer.AddressMap``.
"""
from __future__ import annotations

import os
import re
import subprocess
import sys
from dataclasses import dataclass
from typing import Dict, List, Optional, Tuple


# ---------------------------------------------------------------------------
# Parser
# ---------------------------------------------------------------------------

# Match a row of the form:
#   |       00 18 | 0aaa aaaa | Tone Coarse Tune                  (16 - 112) |
ROW_RE = re.compile(
    r"^\|#?\s+([0-9A-F]{2})\s+([0-9A-F]{2})\s*\|\s*([01a-d]{4} [01a-d]{4})\s*\|\s*"
    r"(.*?)\s*\|"   # trailing `|` then whatever — page-number footers may follow
)
# Match a "size row" of the form:
#   | 00 00 00 36 |Total Size                                                 |
SIZE_RE = re.compile(r"^\|\s*([0-9A-F]{2})\s+([0-9A-F]{2})\s+([0-9A-F]{2})\s+([0-9A-F]{2})\s*\|Total Size")
SECTION_RE = re.compile(r"^\* \[(.+)\]\s*$")


@dataclass
class RawRow:
    offset: int          # 16-bit offset within substructure
    bit_pattern: str     # e.g. "0aaa aaaa" or "0000 dddd"
    description: str     # the right-hand text — may include name + range
    is_field_start: bool # True for `|#` rows that begin a multi-byte field


@dataclass
class Param:
    name: str            # snake_case machine name
    label: str           # original label
    offset: int          # in containing substructure
    size: int            # bytes on the wire
    min_value: int
    max_value: int
    notes: str = ""      # range description from doc, used for human-readable display

    @property
    def is_signed(self) -> bool:
        # Per Roland convention, "(28 - 228)" with sub-range "-100 - +100"
        # encodes a signed 8-bit centered at 128. We don't actually do
        # signed transformation here — we just record the raw wire range.
        return False


def parse_section(lines: List[str], start: int) -> Tuple[str, int, List[Param], int, int]:
    """Parse one ``* [SectionName]`` block. Returns (name, total_size_bytes,
    params, end_line_idx, last_offset)."""
    section_name = SECTION_RE.match(lines[start]).group(1)
    rows: List[RawRow] = []

    i = start + 1
    total_size: Optional[int] = None
    while i < len(lines):
        line = lines[i]
        if SECTION_RE.match(line):
            break
        size_m = SIZE_RE.match(line)
        if size_m:
            b3, b2, b1, b0 = (int(x, 16) for x in size_m.groups())
            total_size = (b3 << 24) | (b2 << 16) | (b1 << 8) | b0
            i += 1
            break

        row_m = ROW_RE.match(line)
        if row_m:
            hi = int(row_m.group(1), 16)
            lo = int(row_m.group(2), 16)
            offset = (hi << 8) | lo
            bit_pattern = row_m.group(3).strip()
            desc = row_m.group(4).strip()
            is_start = line.lstrip().startswith("|#")
            rows.append(RawRow(offset, bit_pattern, desc, is_start))
        i += 1

    params = build_params(rows)
    return section_name, total_size or 0, params, i, rows[-1].offset if rows else 0


def build_params(rows: List[RawRow]) -> List[Param]:
    """Coalesce adjacent rows into 1/2/4-byte parameters with names + ranges.

    Roland's notation:
      * A row whose label has text and bit pattern ``0aaa aaaa`` is a
        single 1-byte param.
      * A row marked ``|#`` (``is_field_start``) begins a multi-byte field.
        The named row at the end of the run carries the human label and
        the range. The number of bytes is the count of consecutive rows
        starting with the ``|#`` marker (1, 2, or 4 — i.e. how many
        nibbles make up the value).
    """
    out: List[Param] = []
    i = 0
    while i < len(rows):
        row = rows[i]
        # Skip rows with empty descriptions that aren't part of a multi-byte
        # field (shouldn't happen, but defensive).
        if not row.description and not row.is_field_start:
            i += 1
            continue

        if row.is_field_start:
            # Multi-byte field. Collect rows until we find one with a label.
            start_offset = row.offset
            j = i + 1
            while j < len(rows) and rows[j].offset == rows[j - 1].offset + 1 and not rows[j].description:
                j += 1
            # rows[j] is either past-end or has the label
            if j >= len(rows):
                i += 1
                continue
            named = rows[j]
            size = (named.offset - start_offset) + 1
            label, low, high = parse_label_and_range(named.description)
            if label:  # skip pure "Reserved" entries
                out.append(make_param(label, start_offset, size, low, high, named.description))
            i = j + 1
            continue

        # Single-byte parameter.
        label, low, high = parse_label_and_range(row.description)
        if label:
            out.append(make_param(label, row.offset, 1, low, high, row.description))
        i += 1

    return out


LABEL_RANGE_RE = re.compile(r"^(.+?)\s*\(\s*(-?\d+)\s*-\s*(-?\d+)\s*\)\s*$")


_FOOTNOTE_RE = re.compile(r"\s*\(\*[^)]*\)")


def parse_label_and_range(text: str) -> Tuple[str, int, int]:
    """Pull ``"Tone Coarse Tune (16 - 112)"`` apart into ``("Tone Coarse Tune", 16, 112)``.

    Returns ``("", 0, 0)`` for reserved / unparseable rows. Footnote
    markers like ``(* appendix 1)`` are stripped from the label so they
    don't end up in the snake_case machine name.
    """
    text = text.strip()
    if not text or text.lower().startswith("reserved"):
        return ("", 0, 0)
    m = LABEL_RANGE_RE.match(text)
    if not m:
        return ("", 0, 0)
    label = _FOOTNOTE_RE.sub("", m.group(1)).strip()
    low = int(m.group(2))
    high = int(m.group(3))
    return (label, low, high)


def make_param(label: str, offset: int, size: int, low: int, high: int, raw_desc: str) -> Param:
    name = label_to_snake_case(label)
    return Param(
        name=name,
        label=label,
        offset=offset,
        size=size,
        min_value=low,
        max_value=high,
        notes=raw_desc,
    )


def label_to_snake_case(label: str) -> str:
    s = label.lower()
    s = re.sub(r"[^a-z0-9]+", "_", s)
    s = s.strip("_")
    return s


# ---------------------------------------------------------------------------
# Disambiguation: Tone PMT and modulation matrix repeat per partial / slot.
# We append a numeric suffix when the same machine name appears more than
# once in a section.
# ---------------------------------------------------------------------------

def coalesce_ascii_runs(params: List[Param]) -> List[Param]:
    """Merge consecutive 1-byte rows with the same label into one ASCII field.

    Roland's tables list each character of a Tone Name as its own row (16
    rows for a 16-byte name). We collapse those into one ``size=16`` entry
    that the runtime treats as ASCII.
    """
    out: List[Param] = []
    i = 0
    while i < len(params):
        p = params[i]
        if p.size == 1 and p.min_value == 32 and p.max_value == 127:
            # ASCII character row — gather the contiguous run with the same label.
            j = i + 1
            while (
                j < len(params)
                and params[j].size == 1
                and params[j].label == p.label
                and params[j].offset == params[j - 1].offset + 1
                and params[j].min_value == 32
                and params[j].max_value == 127
            ):
                j += 1
            run_size = j - i
            out.append(
                Param(
                    name=label_to_snake_case(p.label),
                    label=p.label,
                    offset=p.offset,
                    size=run_size,
                    min_value=32,
                    max_value=127,
                    notes=f"{run_size}-byte ASCII string",
                )
            )
            i = j
            continue
        out.append(p)
        i += 1
    return out


def disambiguate(params: List[Param]) -> List[Param]:
    seen: Dict[str, int] = {}
    out: List[Param] = []
    counts: Dict[str, int] = {}
    for p in params:
        counts[p.name] = counts.get(p.name, 0) + 1

    indices: Dict[str, int] = {}
    for p in params:
        if counts[p.name] > 1:
            indices[p.name] = indices.get(p.name, 0) + 1
            new_name = f"{p.name}_{indices[p.name]}"
            out.append(Param(new_name, f"{p.label} {indices[p.name]}",
                             p.offset, p.size, p.min_value, p.max_value, p.notes))
        else:
            out.append(p)
    return out


# ---------------------------------------------------------------------------
# Code generation
# ---------------------------------------------------------------------------

PY_HEADER = '''"""Auto-generated conventional ZEN-Core Tone parameter tables.

DO NOT EDIT BY HAND. Regenerate with::

    pdftotext -raw -f 76 -l 82 reference/JUPITER-X_Xm_MIDI_imple_eng06_W.pdf reference/tools_jp_zcore.txt
    pdftotext -raw -f 85 -l 88 reference/JUPITER-X_Xm_MIDI_imple_eng06_W.pdf - \\
        | sed -n '/^* \\[MFX\\]/,/^* \\[Reverb\\]/p' >> reference/tools_jp_zcore.txt
    python tools/gen_zencore_params.py

Source: Roland Jupiter-X/Xm MIDI Implementation v1.06 (Apr 2022),
``[ZCore]`` address map. Includes the conventional Tone path and the
Tone Synth Common / PMT / Partial substructures used by the MC family.
"""
from __future__ import annotations

from .zencore import ZCParam

'''

SWIFT_HEADER = '''// Auto-generated conventional ZEN-Core Tone parameter tables.
//
// DO NOT EDIT BY HAND. Regenerate with:
//
//     pdftotext -raw -f 76 -l 82 reference/JUPITER-X_Xm_MIDI_imple_eng06_W.pdf reference/tools_jp_zcore.txt
//     pdftotext -raw -f 85 -l 88 reference/JUPITER-X_Xm_MIDI_imple_eng06_W.pdf - \\
//         | sed -n '/^* \\[MFX\\]/,/^* \\[Reverb\\]/p' >> reference/tools_jp_zcore.txt
//     python tools/gen_zencore_params.py
//
// Source: Roland Jupiter-X/Xm MIDI Implementation v1.06 (Apr 2022),
// [ZCore] address map. Includes the conventional Tone path and the
// Tone Synth Common / PMT / Partial substructures used by the MC family.

import Foundation

'''


def emit_python(sections: Dict[str, List[Param]], totals: Dict[str, int]) -> str:
    lines = [PY_HEADER]
    lines.append("# (section_python_var, total_size_bytes)\n")
    for var_name, params in sections.items():
        total = totals[var_name]
        lines.append(f"# {var_name}: {len(params)} parameters, total size 0x{total:X} bytes\n")
        lines.append(f"{var_name}: dict[str, ZCParam] = {{\n")
        for p in params:
            lines.append(
                f'    "{p.name}": ZCParam(\n'
                f'        name="{p.name}",\n'
                f'        offset=0x{p.offset:04X},\n'
                f'        size={p.size},\n'
                f'        min_value={p.min_value}, max_value={p.max_value},\n'
                f'        label={py_repr(p.label)},\n'
                f'        notes={py_repr(p.notes)},\n'
                f'    ),\n'
            )
        lines.append("}\n\n")
    # Section -> total size lookup
    lines.append("SECTION_TOTAL_SIZE: dict[str, int] = {\n")
    for var_name in sections:
        lines.append(f'    "{var_name}": 0x{totals[var_name]:X},\n')
    lines.append("}\n")
    return "".join(lines)


def emit_swift(sections: Dict[str, List[Param]], totals: Dict[str, int]) -> str:
    lines = [SWIFT_HEADER]
    lines.append("public enum ZenCoreData {\n\n")
    for var_name, params in sections.items():
        total = totals[var_name]
        lines.append(f"    /// {var_name}: {len(params)} parameters, total size 0x{total:X} bytes\n")
        lines.append(f"    public static let {var_name}: [String: ZCParam] = [\n")
        for p in params:
            lines.append(
                f'        "{p.name}": ZCParam(\n'
                f'            name: "{p.name}",\n'
                f'            offset: 0x{p.offset:04X},\n'
                f'            size: {p.size},\n'
                f'            minValue: {p.min_value}, maxValue: {p.max_value},\n'
                f'            defaultValue: 0,\n'
                f'            label: {swift_str(p.label)},\n'
                f'            notes: {swift_str(p.notes)}\n'
                f'        ),\n'
            )
        lines.append("    ]\n\n")

    lines.append("    public static let sectionTotalSize: [String: UInt32] = [\n")
    for var_name in sections:
        lines.append(f'        "{var_name}": 0x{totals[var_name]:X},\n')
    lines.append("    ]\n")
    lines.append("}\n")
    return "".join(lines)


def py_repr(s: str) -> str:
    return repr(s)


def swift_str(s: str) -> str:
    escaped = s.replace("\\", "\\\\").replace('"', '\\"')
    return f'"{escaped}"'


# ---------------------------------------------------------------------------
# Driver
# ---------------------------------------------------------------------------

# Map raw section name → Python variable name in the generated table.
#
# Both halves of a ZCore tone descriptor are populated, because the MC-707
# Reference Manual page 61–65 documents Z-Core Tone Parameters that span
# both the conventional Tone Common/PMT/Partial sub-block AND the Tone
# Synth Common/PMT/Partial sub-block. Concretely:
#
#   * SOUND / TUNING / PTL CTRL / PTL RANGE → Tone Common + Tone PMT + Tone Partial
#   * STRUCTURE / OSC MOD COM                → Tone Synth PMT
#   * UNISON                                  → Tone Synth Common
#   * OSC (OSC Type, VA Waveform, etc.)       → Tone Synth Partial
#   * Pitch/Filter/Amp Env, LFO, EQ           → Partial Pitch/Filter/Amp Env, Partial LFO, Partial EQ
#   * MFX (per-tone)                          → MFX (between Tone Common and Tone PMT)
#
SECTION_NAME_MAP: Dict[str, str] = {
    "Tone Common":         "TONE_COMMON",
    "Tone PMT":            "TONE_PMT",
    "Tone Partial":        "TONE_PARTIAL",
    "Partial Pitch Env":   "PARTIAL_PITCH_ENV",
    "Partial Filter Env":  "PARTIAL_FILTER_ENV",
    "Partial Amp Env":     "PARTIAL_AMP_ENV",
    "Partial LFO":         "PARTIAL_LFO",
    "Partial EQ":          "PARTIAL_EQ",
    "Tone Synth Common":   "TONE_SYNTH_COMMON",
    "Tone Synth PMT":      "TONE_SYNTH_PMT",
    "Tone Synth Partial":  "TONE_SYNTH_PARTIAL",
    "MFX":                 "TONE_MFX",
}


def main() -> int:
    here = os.path.dirname(os.path.abspath(__file__))
    repo = os.path.dirname(here)
    raw_path = os.path.join(repo, "reference", "tools_jp_zcore.txt")
    if not os.path.exists(raw_path):
        print(
            "missing reference/tools_jp_zcore.txt — this generator's input is\n"
            "extracted from Roland's Jupiter-X/Xm MIDI Implementation PDF, which\n"
            "is copyrighted and not committed to this repo. To regenerate:\n"
            "\n"
            "  1. Download the Jupiter-X/Xm MIDI Implementation PDF from Roland:\n"
            "     https://www.roland.com/global/support/by_product/jupiter-x/owners_manuals/\n"
            "     (look for the 'MIDI Implementation' link and accept Roland's licence)\n"
            "  2. Save it locally as:\n"
            "     reference/JUPITER-X_Xm_MIDI_imple_eng06_W.pdf\n"
            "  3. Extract the parameter sections (needs poppler's pdftotext —\n"
            "     `brew install poppler` on macOS, `apt install poppler-utils` on Debian):\n"
            "\n"
            "     pdftotext -raw -f 76 -l 82 \\\n"
            "         reference/JUPITER-X_Xm_MIDI_imple_eng06_W.pdf \\\n"
            "         reference/tools_jp_zcore.txt\n"
            "     pdftotext -raw -f 85 -l 88 \\\n"
            "         reference/JUPITER-X_Xm_MIDI_imple_eng06_W.pdf - \\\n"
            "       | sed -n '/^* \\[MFX\\]/,/^* \\[Reverb\\]/p' \\\n"
            "       >> reference/tools_jp_zcore.txt\n"
            "  4. Re-run this script.\n"
            "\n"
            "You only need to do this if you're updating the parameter table.\n"
            "The auto-generated tables are committed to the repo, so the\n"
            "library itself works without ever running this script.\n",
            file=sys.stderr,
        )
        return 1

    with open(raw_path) as f:
        lines = f.readlines()

    sections: Dict[str, List[Param]] = {}
    totals: Dict[str, int] = {}

    i = 0
    while i < len(lines):
        if SECTION_RE.match(lines[i]):
            section_name, total_size, params, end_idx, _ = parse_section(lines, i)
            if section_name in SECTION_NAME_MAP:
                var_name = SECTION_NAME_MAP[section_name]
                params = coalesce_ascii_runs(params)
                params = disambiguate(params)
                sections[var_name] = params
                totals[var_name] = total_size
            i = end_idx
        else:
            i += 1

    if not sections:
        print("no sections matched — check the PDF or the section name map", file=sys.stderr)
        return 1

    py_path = os.path.join(repo, "python_lib", "mc_programmer", "zencore_data.py")
    sw_path = os.path.join(repo, "swift_lib", "Sources", "MCProgrammer", "ZenCoreData.swift")

    with open(py_path, "w") as f:
        f.write(emit_python(sections, totals))
    with open(sw_path, "w") as f:
        f.write(emit_swift(sections, totals))
    python_lib = os.path.join(repo, "python_lib")
    rust_generator = os.path.join(here, "gen_rust_zencore.py")
    generator_env = os.environ.copy()
    generator_env["PYTHONPATH"] = os.pathsep.join(
        path for path in (python_lib, generator_env.get("PYTHONPATH", "")) if path
    )
    subprocess.run(
        [sys.executable, rust_generator], cwd=repo, env=generator_env, check=True
    )

    print(f"wrote {py_path}")
    print(f"wrote {sw_path}")
    print("\nsection summary:")
    for name, params in sections.items():
        print(f"  {name:24s}  size 0x{totals[name]:04X}  {len(params):4d} params")

    return 0


if __name__ == "__main__":
    sys.exit(main())
