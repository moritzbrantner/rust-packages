#!/usr/bin/env python3
"""Tests for lightweight CI sanity checks."""

from __future__ import annotations

import importlib.util
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SCRIPT = ROOT / "scripts" / "check_ci_sanity.py"
sys.dont_write_bytecode = True
spec = importlib.util.spec_from_file_location("check_ci_sanity", SCRIPT)
assert spec and spec.loader
sanity = importlib.util.module_from_spec(spec)
sys.modules["check_ci_sanity"] = sanity
spec.loader.exec_module(sanity)


class CiSanityTests(unittest.TestCase):
    def test_valid_manifests_pass(self) -> None:
        self.assertIsNone(sanity.manifest_error("package.json", b'{"name":"ok"}'))
        self.assertIsNone(sanity.manifest_error("Cargo.toml", b'[package]\nname="ok"\n'))

    def test_invalid_manifests_fail_without_echoing_content(self) -> None:
        error = sanity.manifest_error("package.json", b'{"token":"sensitive",}')
        self.assertEqual(error, "package.json: invalid manifest (JSONDecodeError)")
        self.assertNotIn("sensitive", error)

    def test_secret_material_reports_location_not_value(self) -> None:
        value = "ghp_" + "a" * 24
        findings = sanity.secret_findings("scripts/config.py", [f'TOKEN="{value}"'])
        self.assertEqual(
            findings,
            ["scripts/config.py: added secret-like material (added line 1)"],
        )
        self.assertNotIn(value, findings[0])

    def test_secret_bearing_filename_is_rejected(self) -> None:
        self.assertEqual(
            sanity.secret_findings(".env", []),
            [".env: secret-bearing filename is not allowed"],
        )


if __name__ == "__main__":
    unittest.main()
