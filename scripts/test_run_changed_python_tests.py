#!/usr/bin/env python3
"""Tests for changed Python test discovery."""

from __future__ import annotations

import importlib.util
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SCRIPT = ROOT / "scripts" / "run_changed_python_tests.py"
sys.dont_write_bytecode = True
spec = importlib.util.spec_from_file_location("run_changed_python_tests", SCRIPT)
assert spec and spec.loader
runner = importlib.util.module_from_spec(spec)
sys.modules["run_changed_python_tests"] = runner
spec.loader.exec_module(runner)


class ChangedPythonTests(unittest.TestCase):
    def test_selects_changed_tests_and_companion_tests(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            scripts = root / "scripts"
            scripts.mkdir()
            (scripts / "test_alpha.py").write_text("", encoding="utf-8")
            (scripts / "test_beta.py").write_text("", encoding="utf-8")
            self.assertEqual(
                runner.selected_tests(
                    [
                        "scripts/alpha.py",
                        "scripts/test_beta.py",
                        "scripts/no_test.py",
                        "docs/guide.py",
                    ],
                    root,
                ),
                ["scripts/test_alpha.py", "scripts/test_beta.py"],
            )

    def test_deduplicates_companion_and_direct_test_changes(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            scripts = root / "scripts"
            scripts.mkdir()
            (scripts / "test_alpha.py").write_text("", encoding="utf-8")
            self.assertEqual(
                runner.selected_tests(
                    ["scripts/alpha.py", "./scripts/test_alpha.py"],
                    root,
                ),
                ["scripts/test_alpha.py"],
            )


if __name__ == "__main__":
    unittest.main()
