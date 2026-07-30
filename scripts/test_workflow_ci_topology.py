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
        self.assertEqual(workflow.count("install_playwright: true"), 1)
        self.assertIn("storybook:test:ci", workflow)

    def test_ui_validation_shares_one_browser_and_wasm_setup(self) -> None:
        workflow = (ROOT / ".github/workflows/workspace-ci.yml").read_text(
            encoding="utf-8"
        )
        storybook_job = workflow.split("  storybook-checks:", 1)[1].split(
            "  browser-e2e-checks:", 1
        )[0]
        self.assertIn("e2e-validation.yml", storybook_job)
        self.assertEqual(storybook_job.count("install_playwright: true"), 1)
        self.assertEqual(storybook_job.count("cargo install wasm-pack"), 1)
        self.assertIn("ui:test:e2e", storybook_job)
        self.assertIn("web:test:e2e", storybook_job)
        self.assertIn("storybook:test:ci", storybook_job)


if __name__ == "__main__":
    unittest.main()
