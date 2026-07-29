#!/usr/bin/env python3
"""Regression tests for PR cancellation and sibling-job isolation."""

from __future__ import annotations

import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


class WorkflowCiTopologyTests(unittest.TestCase):
    def test_pr_runs_cancel_obsolete_runs_but_not_sibling_jobs(self) -> None:
        workflow = (ROOT / ".github/workflows/workspace-ci.yml").read_text(
            encoding="utf-8"
        )
        self.assertIn("cancel-in-progress: true", workflow)
        for suffix in (
            "architecture",
            "rust",
            "frontend",
            "wasm",
            "storybook",
            "browser-e2e",
            "full-workspace",
        ):
            self.assertEqual(workflow.count(f"-{suffix}\n"), 1)

    def test_scheduled_full_workflow_has_no_automatic_cancellation(self) -> None:
        workflow = (ROOT / ".github/workflows/full-workspace-ci.yml").read_text(
            encoding="utf-8"
        )
        self.assertNotIn("cancel-in-progress:", workflow)
        self.assertNotIn("\nconcurrency:", workflow)


if __name__ == "__main__":
    unittest.main()
