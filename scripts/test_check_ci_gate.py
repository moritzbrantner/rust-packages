#!/usr/bin/env python3
"""Tests for the final conditional CI gate."""

from __future__ import annotations

import importlib.util
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SCRIPT = ROOT / "scripts" / "check_ci_gate.py"
sys.dont_write_bytecode = True
spec = importlib.util.spec_from_file_location("check_ci_gate", SCRIPT)
assert spec and spec.loader
gate = importlib.util.module_from_spec(spec)
sys.modules["check_ci_gate"] = gate
spec.loader.exec_module(gate)


class CheckCiGateTests(unittest.TestCase):
    def test_selected_boolean_must_be_canonical(self) -> None:
        with self.assertRaisesRegex(ValueError, "expected true or false"):
            gate.parse_assignments(["rust_checks=not-a-boolean"], boolean=True)

    def test_selected_success_and_unselected_skip_pass(self) -> None:
        result = gate.evaluate_gate(
            planner_result="success",
            selected={"rust_checks": True, "frontend_checks": False},
            results={"rust_checks": "success", "frontend_checks": "skipped"},
        )
        self.assertTrue(result["passed"])

    def test_selected_skipped_fails(self) -> None:
        result = gate.evaluate_gate(
            planner_result="success",
            selected={"rust_checks": True},
            results={"rust_checks": "skipped"},
        )
        self.assertFalse(result["passed"])
        self.assertIn("rust_checks:selected-but-skipped", result["failures"])

    def test_selected_failure_fails(self) -> None:
        result = gate.evaluate_gate(
            planner_result="success",
            selected={"browser_e2e_checks": True},
            results={"browser_e2e_checks": "failure"},
        )
        self.assertFalse(result["passed"])

    def test_planner_failure_fails_closed(self) -> None:
        result = gate.evaluate_gate(
            planner_result="failure",
            selected={},
            results={},
        )
        self.assertEqual(result["failures"], ["planner:failure"])

    def test_missing_planner_selection_is_not_an_intentional_skip(self) -> None:
        with self.assertRaisesRegex(ValueError, "expected true or false"):
            gate.parse_assignments(["rust_checks="], boolean=True)

    def test_cancelled_or_missing_required_results_fail(self) -> None:
        for results in ({"rust_checks": "cancelled"}, {}):
            with self.subTest(results=results):
                result = gate.evaluate_gate(
                    planner_result="success",
                    selected={"rust_checks": True},
                    results=results,
                )
                self.assertFalse(result["passed"])

    def test_documentation_scope_can_intentionally_skip_all_heavy_jobs(self) -> None:
        result = gate.evaluate_gate(
            planner_result="success",
            selected={"rust_checks": False, "frontend_checks": False},
            results={"rust_checks": "skipped", "frontend_checks": "skipped"},
        )
        self.assertTrue(result["passed"])


if __name__ == "__main__":
    unittest.main()
