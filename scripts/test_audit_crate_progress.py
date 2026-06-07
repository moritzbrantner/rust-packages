#!/usr/bin/env python3
"""Focused tests for scripts/audit_crate_progress.py."""

from __future__ import annotations

import importlib.util
import os
import sys
import tempfile
import unittest
from datetime import date, timedelta
from pathlib import Path
from unittest import mock

ROOT = Path(__file__).resolve().parents[1]
SCRIPT = ROOT / "scripts" / "audit_crate_progress.py"
sys.dont_write_bytecode = True
spec = importlib.util.spec_from_file_location("audit_crate_progress", SCRIPT)
assert spec and spec.loader
audit = importlib.util.module_from_spec(spec)
sys.modules["audit_crate_progress"] = audit
spec.loader.exec_module(audit)


class CrateProgressAuditTests(unittest.TestCase):
    def test_cargo_env_does_not_force_target_dir(self) -> None:
        with mock.patch.dict(os.environ, {}, clear=True):
            env = audit.cargo_env()
        self.assertNotIn("CARGO_TARGET_DIR", env)

    def test_cargo_env_preserves_explicit_target_dir(self) -> None:
        with mock.patch.dict(os.environ, {"CARGO_TARGET_DIR": "/tmp/custom-target"}, clear=True):
            env = audit.cargo_env()
        self.assertEqual(env["CARGO_TARGET_DIR"], "/tmp/custom-target")

    def test_generated_ledger_is_stable(self) -> None:
        first = audit.render_ledger(audit.audit_records(ROOT, "moritzbrantner-text-core"))
        second = audit.render_ledger(audit.audit_records(ROOT, "moritzbrantner-text-core"))
        self.assertEqual(first, second)

    def test_complete_companion_crate_reaches_transport_complete(self) -> None:
        record = audit.audit_records(ROOT, "moritzbrantner-text-core")[0]
        self.assertGreaterEqual(audit.LEVEL_RANK[record.level], audit.LEVEL_RANK["L3 Transport Complete"])

    def test_app_defaulting_to_describe_cannot_reach_usable(self) -> None:
        level = audit.maturity_level(
            surface_present=True,
            discoverable=True,
            metadata_complete=True,
            no_scaffold=True,
            parity={"cli": True, "server": True, "rust_wasm": True, "bun_wasm": True, "app": True},
            app_status="app defaults to describe",
            readme=True,
            tests=True,
        )
        self.assertEqual(level, "L3 Transport Complete")

    def test_score_regression_fails_without_allowlist(self) -> None:
        current = {
            "moritzbrantner-demo": self.record("moritzbrantner-demo", "L3 Transport Complete", 80),
        }
        base = {
            "moritzbrantner-demo": self.record("moritzbrantner-demo", "L3 Transport Complete", 90),
        }
        failures = audit.regression_failures(current, base, {"moritzbrantner-demo"}, [])
        self.assertTrue(any("score regressed" in failure for failure in failures))

    def test_expired_allowlist_entry_is_detected(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            path = Path(temp) / "allow"
            yesterday = date.today() - timedelta(days=1)
            path.write_text(
                f"moritzbrantner-demo\tscore\t{yesterday.isoformat()}\ttemporary test\n",
                encoding="utf-8",
            )
            entries = audit.read_regression_allowlist(path)
        self.assertEqual(len(audit.expired_allowlist_entries(entries)), 1)

    def test_shared_package_surface_change_selects_all_crates(self) -> None:
        packages = [
            audit.LibraryPackage("moritzbrantner-a", Path("crates/data/a/Cargo.toml")),
            audit.LibraryPackage("moritzbrantner-b", Path("crates/data/b/Cargo.toml")),
        ]
        touched = audit.touched_package_names(
            ["packages/video-analysis-ui/src/package-surface/OperationWorkbench.tsx"],
            packages,
        )
        self.assertEqual(touched, {"moritzbrantner-a", "moritzbrantner-b"})

    def test_generated_snapshot_allowlist_includes_progress_ledger(self) -> None:
        allow = (ROOT / "scripts/generated_snapshots.allow").read_text(encoding="utf-8")
        self.assertIn("docs/CRATE_PROGRESS_LEDGER.md", allow)

    @staticmethod
    def record(library: str, level: str, score: int):
        return audit.ProgressRecord(
            library=library,
            domain="test",
            path="crates/test/demo",
            level=level,
            score=score,
            workflow_operations=["demo.run"],
            debug_operations=["describe"],
            parity={"cli": True, "server": True, "rust_wasm": True, "bun_wasm": True, "app": True},
            readme_quickstart=True,
            primary_workflow_test=True,
            app_default_status="workflow default",
        )


if __name__ == "__main__":
    unittest.main()
