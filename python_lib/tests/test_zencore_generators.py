"""Unit tests for the Rust table generator and canonical generator handoff.

Run with: python -m unittest python_lib.tests.test_zencore_generators
No reference PDFs or rustfmt installation are needed.
"""
import contextlib
import io
from pathlib import Path
import subprocess
import sys
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT))
sys.path.insert(0, str(ROOT / "python_lib"))

from mc_programmer import zencore_data  # noqa: E402
from tools import gen_rust_zencore as rust  # noqa: E402
from tools import gen_zencore_params as canonical  # noqa: E402


class RustGeneratorTests(unittest.TestCase):
    def test_rust_string_escapes_quotes_and_backslashes(self):
        slash = chr(92)
        cases = [
            ("", '""'),
            ('Tone "A"', '"Tone \\"A\\""'),
            ("C:" + slash + "tones", '"C:' + slash * 2 + 'tones"'),
            (slash + '"', '"' + slash * 2 + slash + '""'),
        ]
        for source, expected in cases:
            with self.subTest(source=source):
                self.assertEqual(rust.rust_str(source), expected)

    def test_emit_param_preserves_numeric_metadata_and_escapes_text(self):
        slash = chr(92)
        param = SimpleNamespace(
            name="coarse_tune", offset=0x18, size=2,
            min_value=-24, max_value=24, label='Tune "A"', notes="path" + slash + "value",
        )
        self.assertEqual(rust.emit_param(param), '\n'.join([
            '    ZCParam::new(',
            '        "coarse_tune",',
            '        0x0018,',
            '        2,',
            '        -24,',
            '        24,',
            '        "Tune \\"A\\"",',
            '        "path' + slash * 2 + 'value",',
            '    ),',
        ]))

    def test_rustfmt_failure_preserves_existing_output(self):
        with tempfile.TemporaryDirectory() as temp:
            output = Path(temp) / "zencore_data.rs"
            output.write_text("existing table")
            with patch.object(
                rust.subprocess,
                "run",
                side_effect=subprocess.CalledProcessError(1, ["rustfmt"]),
            ):
                with self.assertRaises(subprocess.CalledProcessError):
                    rust.main(["--output", str(output)])
            self.assertEqual(output.read_text(), "existing table")
            self.assertEqual(
                {path.name for path in Path(temp).iterdir()},
                {"zencore_data.rs"},
            )
    def test_generator_emits_each_current_table_section(self):
        with tempfile.TemporaryDirectory() as temp:
            output = Path(temp) / "zencore_data.rs"
            with patch.object(rust.subprocess, "run"):
                rust.main(["--output", str(output)])
            generated = output.read_text()

        sections = set(zencore_data.SECTION_TOTAL_SIZE)
        declarations = {
            line.split()[2].removesuffix(":")
            for line in generated.splitlines()
            if line.startswith("pub static ") and "&[ZCParam]" in line
        }
        self.assertEqual(declarations, sections)
        for section in sections:
            self.assertIn(f'("{section}", 0x{zencore_data.SECTION_TOTAL_SIZE[section]:X}),', generated)


class CanonicalAtomicGenerationTests(unittest.TestCase):
    def run_canonical_with_failure(self, failure):
        with tempfile.TemporaryDirectory() as temp:
            repo = Path(temp)
            (repo / "tools").mkdir()
            (repo / "reference").mkdir()
            (repo / "reference" / "tools_jp_zcore.txt").write_text("* [Tone Common]\n")
            paths = [
                repo / "python_lib" / "mc_programmer" / "zencore_data.py",
                repo / "swift_lib" / "Sources" / "MCProgrammer" / "ZenCoreData.swift",
                repo / "rust_lib" / "src" / "zencore_data.rs",
            ]
            for path in paths:
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text("preexisting committed data")

            def fail_after_staging(command, **kwargs):
                self.assertEqual(command[0], sys.executable)
                python_data = Path(command[command.index("--python-data") + 1])
                rust_output = Path(command[command.index("--output") + 1])
                swift_data = python_data.parent / "ZenCoreData.swift"
                self.assertEqual(python_data.read_text(), "new Python table")
                self.assertEqual(swift_data.read_text(), "new Swift table")
                self.assertEqual(rust_output.name, "zencore_data.rs")
                self.assertTrue(python_data.parent.name.startswith(".zencore-"))
                raise failure
            with contextlib.ExitStack() as stack:
                stack.enter_context(patch.object(canonical, "__file__", str(repo / "tools" / "gen_zencore_params.py")))
                stack.enter_context(patch.object(canonical, "parse_section", return_value=("Tone Common", 1, [], 1, None)))
                stack.enter_context(patch.object(canonical, "emit_python", return_value="new Python table"))
                stack.enter_context(patch.object(canonical, "emit_swift", return_value="new Swift table"))
                stack.enter_context(patch.object(canonical.subprocess, "run", side_effect=fail_after_staging))
                stack.enter_context(contextlib.redirect_stdout(io.StringIO()))
                with self.assertRaises(type(failure)):
                    canonical.main()

            self.assertEqual([path.read_text() for path in paths], ["preexisting committed data"] * 3)
            leftovers = [p.name for p in repo.rglob("*") if p.name.startswith(".zencore-")]
            self.assertEqual(leftovers, [])

    def test_rust_generation_failure_preserves_all_existing_tables(self):
        self.run_canonical_with_failure(subprocess.CalledProcessError(2, ["rust generator"]))

    def test_rust_generator_launch_failure_preserves_all_existing_tables(self):
        self.run_canonical_with_failure(OSError("rust generator unavailable"))


if __name__ == "__main__":
    unittest.main()
