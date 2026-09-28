#!/usr/bin/env python3
"""Generate the Rust ZEN-Core Tone parameter table.

Unlike ``tools/gen_zencore_params.py`` (which parses Roland's Jupiter-X PDF to
build *all* three tables), this script sources the **already-committed** Python
table (``python_lib/mc_programmer/zencore_data.py``) and emits only the Rust
table. Run it whenever the committed table changes:

    PYTHONPATH=python_lib python3 tools/gen_rust_zencore.py

The output is committed at ``rust_lib/src/zencore_data.rs``. ``rustfmt`` from
the Rust toolchain formats it after generation.
"""
from __future__ import annotations

import os
import subprocess
import sys

# Canonical section order — matches the order in the committed Python file.
SECTIONS = [
    "PARTIAL_EQ",
    "PARTIAL_LFO",
    "PARTIAL_PITCH_ENV",
    "PARTIAL_FILTER_ENV",
    "PARTIAL_AMP_ENV",
    "TONE_COMMON",
    "TONE_PMT",
    "TONE_PARTIAL",
    "TONE_SYNTH_COMMON",
    "TONE_SYNTH_PMT",
    "TONE_SYNTH_PARTIAL",
    "TONE_MFX",
]

HEADER = """// Auto-generated conventional ZEN-Core Tone parameter tables.
//
// DO NOT EDIT BY HAND. Regenerate with:
//
//     PYTHONPATH=python_lib python3 tools/gen_rust_zencore.py
//
// Source: Roland Jupiter-X/Xm MIDI Implementation v1.06 (Apr 2022),
// `[ZCore]` address map. The Rust table mirrors every section committed in
// `python_lib/mc_programmer/zencore_data.py`, including the conventional Tone
// path and the Tone Synth Common / PMT / Partial substructures.

use crate::zencore::ZCParam;

"""


def rust_str(s: str) -> str:
    """Quote a table string for Rust, escaping backslashes and double quotes."""
    return '"' + s.replace("\\", "\\\\").replace('"', '\\"') + '"'


def emit_param(p) -> str:
    """Render one Python parameter descriptor as a Rust ZCParam constructor."""
    return "\n".join(
        (
            "    ZCParam::new(",
            f"        {rust_str(p.name)},",
            f"        0x{p.offset:04X},",
            f"        {int(p.size)},",
            f"        {p.min_value},",
            f"        {p.max_value},",
            f"        {rust_str(p.label)},",
            f"        {rust_str(p.notes)},",
            "    ),",
        )
    )


def main() -> int:
    """Write and rustfmt the Rust table from committed Python data; return zero."""
    here = os.path.dirname(os.path.abspath(__file__))
    repo = os.path.dirname(here)
    python_lib = os.path.join(repo, "python_lib")
    sys.path.insert(0, python_lib)

    from mc_programmer import zencore_data as data  # noqa: E402

    out_path = os.path.join(repo, "rust_lib", "src", "zencore_data.rs")

    lines = [HEADER]
    for section in SECTIONS:
        table = getattr(data, section)
        total = data.SECTION_TOTAL_SIZE[section]
        lines.append(f"/// `{section}`: {len(table)} parameters, total size 0x{total:X} bytes\n")
        lines.append(f"pub static {section}: &[ZCParam] = &[\n")
        for p in table.values():
            lines.append(emit_param(p) + "\n")
        lines.append("];\n\n")

    lines.append("/// Section name → total size in bytes (from the auto-generated table).\n")
    lines.append("pub static SECTION_TOTAL_SIZE: &[(&str, u32)] = &[\n")
    for section in SECTIONS:
        lines.append(f'    ("{section}", 0x{data.SECTION_TOTAL_SIZE[section]:X}),\n')
    lines.append("];\n")

    with open(out_path, "w") as f:
        f.write("".join(lines))
    subprocess.run(["rustfmt", out_path], cwd=repo, check=True)

    print(f"wrote {out_path}")
    for section in SECTIONS:
        table = getattr(data, section)
        print(f"  {section:24s}  size 0x{data.SECTION_TOTAL_SIZE[section]:04X}  {len(table):4d} params")
    return 0


if __name__ == "__main__":
    sys.exit(main())
