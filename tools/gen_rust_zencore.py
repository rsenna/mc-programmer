#!/usr/bin/env python3
"""Generate the Rust ZEN-Core Tone parameter table.

Unlike ``tools/gen_zencore_params.py`` (which parses Roland's Jupiter-X PDF to
build *all* three tables), this script sources the Python table and emits only
the Rust table. Run it whenever the committed table changes:

    PYTHONPATH=python_lib python3 tools/gen_rust_zencore.py

`rust_lib/src/zencore_data.rs` is written only after the staged output passes
`rustfmt`; a missing or failing formatter leaves the existing table untouched.
"""
from __future__ import annotations

import argparse
import importlib.util
import os
import stat
import subprocess
import sys
import tempfile


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


def load_data(path: str, python_lib: str):
    """Load the requested generated data module, including a staged version."""
    if python_lib not in sys.path:
        sys.path.insert(0, python_lib)
    import mc_programmer  # noqa: F401

    spec = importlib.util.spec_from_file_location("mc_programmer.zencore_data", path)
    if spec is None or spec.loader is None:
        raise ImportError(f"cannot load ZEN-Core data from {path}")
    data = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = data
    spec.loader.exec_module(data)
    return data


def main(argv=None) -> int:
    """Write and rustfmt the Rust table from Python data; return zero."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--python-data", help="Python ZEN-Core data module to read")
    parser.add_argument("--output", help="Rust table path to write")
    args = parser.parse_args(argv)

    here = os.path.dirname(os.path.abspath(__file__))
    repo = os.path.dirname(here)
    python_lib = os.path.join(repo, "python_lib")
    data_path = args.python_data or os.path.join(python_lib, "mc_programmer", "zencore_data.py")
    out_path = os.path.abspath(args.output or os.path.join(repo, "rust_lib", "src", "zencore_data.rs"))
    data = load_data(data_path, python_lib)
    sections = tuple(data.SECTION_TOTAL_SIZE)

    lines = [HEADER]
    for section in sections:
        table = getattr(data, section)
        total = data.SECTION_TOTAL_SIZE[section]
        lines.append(f"/// `{section}`: {len(table)} parameters, total size 0x{total:X} bytes\n")
        lines.append(f"pub static {section}: &[ZCParam] = &[\n")
        for p in table.values():
            lines.append(emit_param(p) + "\n")
        lines.append("];\n\n")

    lines.append("/// Section name → total size in bytes (from the auto-generated table).\n")
    lines.append("pub static SECTION_TOTAL_SIZE: &[(&str, u32)] = &[\n")
    for section in sections:
        lines.append(f'    ("{section}", 0x{data.SECTION_TOTAL_SIZE[section]:X}),\n')
    lines.append("];\n")

    # rustfmt edits files in place, so keep it away from the committed target
    # until formatting succeeds. A same-directory replace is atomic.
    mode = stat.S_IMODE(os.stat(out_path).st_mode) if os.path.exists(out_path) else 0o644
    with tempfile.NamedTemporaryFile(
        mode="w", prefix=".zencore-", suffix=".rs",
        dir=os.path.dirname(out_path), delete=False
    ) as staged_file:
        staged_path = staged_file.name
        staged_file.write("".join(lines))
    try:
        os.chmod(staged_path, mode)
        subprocess.run(["rustfmt", staged_path], cwd=repo, check=True)
        os.replace(staged_path, out_path)
    finally:
        if os.path.exists(staged_path):
            os.unlink(staged_path)

    print(f"wrote {out_path}")
    for section in sections:
        table = getattr(data, section)
        print(f"  {section:24s}  size 0x{data.SECTION_TOTAL_SIZE[section]:04X}  {len(table):4d} params")
    return 0


if __name__ == "__main__":
    sys.exit(main())
