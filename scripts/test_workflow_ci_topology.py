#!/usr/bin/env python3
"""Regression tests for PR cancellation and sibling-job isolation."""

from __future__ import annotations

import unittest
import os
import subprocess
import tempfile
import textwrap
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


class WorkflowCiTopologyTests(unittest.TestCase):
    def test_required_full_ci_tier_uses_current_labels_and_fails_closed(self) -> None:
        workflow = (ROOT / ".github/workflows/workspace-ci.yml").read_text(encoding="utf-8")
        step = workflow.split("      - name: Classify changed surfaces\n", 1)[1].split("      - name:", 1)[0]
        script = textwrap.dedent(step.split("        run: |\n", 1)[1])
        with tempfile.TemporaryDirectory() as directory:
            fixture = Path(directory)
            gh = fixture / "gh"
            gh.write_text('#!/bin/sh\n[ "$CURRENT_LABEL" != error ] || exit 9\nprintf "%s\\n" "$CURRENT_LABEL"\n')
            gh.chmod(0o755)
            python = fixture / "python3"
            python.write_text('#!/bin/sh\nprintf "%s\\n" "$@" > "$FIXTURE_ARGV"\n')
            python.chmod(0o755)
            for current, stale, success in [("true", "false", True), ("false", "true", True), ("error", "false", False), ("unknown", "false", False)]:
                with self.subTest(current=current, stale=stale):
                    arguments = fixture / "arguments"
                    arguments.unlink(missing_ok=True)
                    env = dict(os.environ, PATH=f"{fixture}:{os.environ['PATH']}", CURRENT_LABEL=current, FULL_CI=stale, GITHUB_EVENT_NAME="pull_request", GITHUB_REPOSITORY="owner/repo", PR_NUMBER="1", GITHUB_OUTPUT=str(fixture / "output"), FIXTURE_ARGV=str(arguments))
                    result = subprocess.run(["bash", "-c", script], cwd=ROOT, env=env, capture_output=True, text=True)
                    self.assertEqual(result.returncode == 0, success, result.stderr)
                    if success:
                        self.assertEqual("--full-ci" in arguments.read_text().splitlines(), current == "true")
                    else:
                        self.assertFalse(arguments.exists(), "scope planning must not proceed after a label-read failure")

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
        self.assertEqual(workflow.count("install_playwright: false"), 1)
        self.assertIn("storybook:test:ci", workflow)
        self.assertIn("bun run --cwd packages/video-analysis-ui playwright install --with-deps chromium", workflow)

    def test_ui_validation_shares_one_browser_and_wasm_setup(self) -> None:
        workflow = (ROOT / ".github/workflows/workspace-ci.yml").read_text(
            encoding="utf-8"
        )
        storybook_job = workflow.split("  storybook-checks:", 1)[1].split(
            "  browser-e2e-checks:", 1
        )[0]
        self.assertIn("e2e-validation.yml", storybook_job)
        self.assertEqual(storybook_job.count("install_playwright: false"), 1)
        self.assertEqual(storybook_job.count("cargo install wasm-pack"), 1)
        self.assertIn("bun run --cwd packages/video-analysis-ui playwright install --with-deps chromium", storybook_job)
        self.assertIn("ui:test:e2e", storybook_job)
        self.assertIn("web:test:e2e", storybook_job)
        self.assertIn("storybook:test:ci", storybook_job)

    def test_browser_validation_coalesces_changed_wasm_builds(self) -> None:
        workflow = (ROOT / ".github/workflows/workspace-ci.yml").read_text(
            encoding="utf-8"
        )
        browser_job = workflow.split("  browser-e2e-checks:", 1)[1].split(
            "  full-workspace-checks:", 1
        )[0]
        self.assertEqual(browser_job.count("cargo install wasm-pack"), 1)
        self.assertIn(
            "scripts/run_changed_frontend.py --base "
            '"origin/${GITHUB_BASE_REF:-main}" --kind wasm --allow-empty',
            browser_job,
        )

    def test_architecture_job_runs_live_validators(self) -> None:
        workflow = (ROOT / ".github/workflows/workspace-ci.yml").read_text(
            encoding="utf-8"
        )
        architecture_job = workflow.split("  architecture-checks:", 1)[1].split(
            "  rust-checks:", 1
        )[0]
        for command in (
            'scripts/check_generated_snapshots.sh --base '
            '"origin/${GITHUB_BASE_REF:-main}"',
            "scripts/generate_repository_split_inventory.py --check",
            "scripts/check_repository_boundaries.py --check",
            "scripts/check_release_plan.py --check "
            "docs/repository-split/release-plan.example.json",
        ):
            with self.subTest(command=command):
                self.assertIn(command, architecture_job)


if __name__ == "__main__":
    unittest.main()
