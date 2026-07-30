#!/usr/bin/env python3
"""Tests for separated application and WASM command execution."""

from __future__ import annotations

import importlib.util
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SCRIPT = ROOT / "scripts" / "run_changed_frontend.py"
sys.dont_write_bytecode = True
spec = importlib.util.spec_from_file_location("run_changed_frontend", SCRIPT)
assert spec and spec.loader
runner = importlib.util.module_from_spec(spec)
sys.modules["run_changed_frontend"] = runner
spec.loader.exec_module(runner)


class RunChangedFrontendTests(unittest.TestCase):
    def test_splits_application_and_wasm_commands(self) -> None:
        scope = {
            "frontend_commands": [
                "bun run ui:typecheck",
                "bun run web:test:unit",
                "bun run --cwd packages/text-core-wasm build",
                "bun run --cwd packages/text-core-wasm test",
            ]
        }
        self.assertEqual(
            runner.selected_commands(scope, "application"),
            ["bun run ui:typecheck", "bun run web:test:unit"],
        )
        self.assertEqual(
            runner.selected_commands(scope, "wasm"),
            [
                "bun run --cwd packages/text-core-wasm build",
                "bun run --cwd packages/text-core-wasm test",
            ],
        )

    def test_selected_commands_are_static_argument_vectors(self) -> None:
        scope = {"frontend_commands": ["bun run --cwd packages/text-stats-wasm test"]}
        self.assertEqual(
            runner.selected_commands(scope, "wasm"),
            ["bun run --cwd packages/text-stats-wasm test"],
        )


if __name__ == "__main__":
    unittest.main()
