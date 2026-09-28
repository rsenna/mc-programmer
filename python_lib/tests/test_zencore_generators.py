"""Unit tests for the Rust table generator and canonical generator handoff.

Run with: python -m unittest python_lib.tests.test_zencore_generators
No reference PDFs, output files, or Rust subprocesses are needed.
"""
import contextlib
import io
import os
from pathlib import Path
import subprocess
import sys
from types import SimpleNamespace
import unittest
from unittest.mock import mock_open, patch

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT))
sys.path.insert(0, str(ROOT / "python_lib"))

from mc_programmer import zencore_data  # noqa: E402
from tools import gen_rust_zencore as rust  # noqa: E402
from tools import gen_zencore_params as canonical  # noqa: E402


class RustGeneratorTests(unittest.TestCase):
    def test_rust_string_escapes_quotes_and_backslashes(self):
        for source, expected in [
            ("", '""'),
            ('Tone "A"', '"Tone \\"A\\""'),
            (r"C:\tones", '"C:\\\\tones"'),
            ('\\"', '"\\\\\\""'),
        ]:
            with self.subTest(source=source):
                self.assertEqual(rust.rust_str(source), expected)

    def test_emit_param_preserves_numeric_metadata_and_escapes_text(self):
        param = SimpleNamespace(
            name="coarse_tune", offset=0x18, size=2,
            min_value=-24, max_value=24, label='Tune "A"', notes=r"path\value",
        )
        self.assertEqual(rust.emit_param(param), '\n'.join([
            '    ZCParam::new(',
            '        "coarse_tune",',
            '        0x0018,',
            '        2,',
            '        -24,',
            '        24,',
            '        "Tune \\"A\\"",',
            '        "path\\\\value",',
            '    ),',
        ]))

    def test_main_emits_all_canonical_sections_and_formats_after_write(self):
        opened = mock_open()
        out_path = str(ROOT / "rust_lib" / "src" / "zencore_data.rs")

        def format_output(*args, **kwargs):
            opened.return_value.write.assert_called_once()
            opened.return_value.__exit__.assert_called_once()

        with patch("builtins.open", opened), patch.object(
            rust.subprocess, "run", side_effect=format_output
        ) as run, patch.object(sys, "path", sys.path[:]), contextlib.redirect_stdout(io.StringIO()):
            self.assertEqual(rust.main(), 0)

        opened.assert_called_once_with(out_path, "w")
        run.assert_called_once_with(["rustfmt", out_path], cwd=str(ROOT), check=True)
        output = opened.return_value.write.call_args.args[0]
        self.assertTrue(output.startswith(rust.HEADER))
        self.assertEqual(set(rust.SECTIONS), set(zencore_data.SECTION_TOTAL_SIZE))
        positions = []
        total_params = 0
        for section in rust.SECTIONS:
            with self.subTest(section=section):
                declaration = f"pub static {section}: &[ZCParam] = &["
                self.assertEqual(output.count(declaration), 1)
                positions.append(output.index(declaration))
                table = getattr(zencore_data, section)
                total_params += len(table)
                self.assertIn(
                    f'("{section}", 0x{zencore_data.SECTION_TOTAL_SIZE[section]:X}),', output
                )
                section_body = output.split(declaration, 1)[1].split("];", 1)[0]
                self.assertEqual(section_body.count("ZCParam::new("), len(table))
                for param in table.values():
                    self.assertIn(f'        "{param.name}",', section_body)
        self.assertEqual(positions, sorted(positions))
        self.assertEqual(output.count("ZCParam::new("), total_params)

    def test_rustfmt_failure_is_propagated(self):
        failure = subprocess.CalledProcessError(1, ["rustfmt"])
        with patch("builtins.open", mock_open()), patch.object(
            rust.subprocess, "run", side_effect=failure
        ), patch.object(sys, "path", sys.path[:]):
            with self.assertRaises(subprocess.CalledProcessError) as raised:
                rust.main()
        self.assertIs(raised.exception, failure)


class CanonicalRustHandoffTests(unittest.TestCase):
    def run_canonical(self, inherited_path, failure=None):
        """Isolate orchestration from PDF parsing and filesystem writes."""
        opened = mock_open(read_data="* [Tone Common]\n")
        environment = {"GENERATOR_TEST_MARKER": "preserved"}
        if inherited_path is not None:
            environment["PYTHONPATH"] = inherited_path

        def invoke_rust(*args, **kwargs):
            # Both outputs must be written and closed before Rust imports Python.
            writes = [call.args[0] for call in opened.return_value.write.call_args_list]
            self.assertEqual(writes, ["python table", "swift table"])
            self.assertEqual(opened.return_value.__exit__.call_count, 3)
            if failure is not None:
                raise failure

        with contextlib.ExitStack() as stack:
            stack.enter_context(patch.dict(os.environ, environment, clear=True))
            stack.enter_context(patch.object(canonical.os.path, "exists", return_value=True))
            stack.enter_context(patch("builtins.open", opened))
            stack.enter_context(patch.object(canonical, "parse_section", return_value=("Tone Common", 1, [], 1, None)))
            stack.enter_context(patch.object(canonical, "emit_python", return_value="python table"))
            stack.enter_context(patch.object(canonical, "emit_swift", return_value="swift table"))
            run = stack.enter_context(patch.object(canonical.subprocess, "run", side_effect=invoke_rust))
            stack.enter_context(contextlib.redirect_stdout(io.StringIO()))
            if failure is None:
                self.assertEqual(canonical.main(), 0)
            else:
                with self.assertRaises(subprocess.CalledProcessError) as raised:
                    canonical.main()
                self.assertIs(raised.exception, failure)
            self.assertEqual(dict(os.environ), environment)
        return run

    def test_handoff_uses_current_interpreter_and_prepends_python_source_path(self):
        for inherited in [None, "", os.pathsep.join(["existing-one", "existing-two"])]:
            with self.subTest(inherited=inherited):
                run = self.run_canonical(inherited)
                expected_path = str(ROOT / "python_lib")
                if inherited:
                    expected_path += os.pathsep + inherited
                run.assert_called_once_with(
                    [sys.executable, str(ROOT / "tools" / "gen_rust_zencore.py")],
                    cwd=str(ROOT),
                    env={"GENERATOR_TEST_MARKER": "preserved", "PYTHONPATH": expected_path},
                    check=True,
                )

    def test_rust_generator_failure_is_not_reported_as_success(self):
        failure = subprocess.CalledProcessError(2, ["rust-generator"])
        self.run_canonical(None, failure).assert_called_once()

    def test_missing_source_does_not_launch_generator_or_write_outputs(self):
        with patch.object(canonical.os.path, "exists", return_value=False), patch(
            "builtins.open", mock_open()
        ) as opened, patch.object(canonical.subprocess, "run") as run, contextlib.redirect_stderr(io.StringIO()):
            self.assertEqual(canonical.main(), 1)
        opened.assert_not_called()
        run.assert_not_called()


if __name__ == "__main__":
    unittest.main()
