#!/usr/bin/env python3
"""Focused tests for scripts/audit_package_surfaces.py."""

from __future__ import annotations

import importlib.util
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SCRIPT = ROOT / "scripts" / "audit_package_surfaces.py"
sys.dont_write_bytecode = True
spec = importlib.util.spec_from_file_location("audit_package_surfaces", SCRIPT)
assert spec and spec.loader
audit = importlib.util.module_from_spec(spec)
sys.modules["audit_package_surfaces"] = audit
spec.loader.exec_module(audit)


class PackageSurfaceAuditTests(unittest.TestCase):
    def test_tracer_gate_fails_loose_primary_schema(self) -> None:
        failures: list[str] = []
        audit.validate_tracer_primary_workflow(
            "moritzbrantner-text-index",
            [
                {
                    "id": "index.search",
                    "inputSchema": {
                        "type": "object",
                        "additionalProperties": True,
                        "xOperationCategory": "workflow",
                        "xErrorShape": {"code": "string"},
                        "xReleaseStability": "stable",
                        "xContractPolicy": "additiveOnly",
                        "xLowerContractProof": {"crates": ["moritzbrantner-text-core"]},
                    },
                    "exampleRequest": {"query": {"text": "rust"}},
                }
            ],
            failures,
        )

        self.assertTrue(
            any("additionalProperties: false" in failure for failure in failures)
        )

    def test_tracer_gate_accepts_strict_primary_schema(self) -> None:
        failures: list[str] = []
        audit.validate_tracer_primary_workflow(
            "moritzbrantner-text-index",
            [
                {
                    "id": "index.search",
                    "inputSchema": {
                        "type": "object",
                        "additionalProperties": False,
                        "xOperationCategory": "workflow",
                        "xErrorShape": {"code": "string"},
                        "xReleaseStability": "stable",
                        "xContractPolicy": "additiveOnly",
                        "xLowerContractProof": {"crates": ["moritzbrantner-text-core"]},
                    },
                    "exampleRequest": {"query": {"text": "rust"}},
                }
            ],
            failures,
        )

        self.assertEqual(failures, [])

    def test_classify_operation_prefers_typed_curation_role(self) -> None:
        operation = {
            "id": "demo.inspect",
            "name": "Inspect workflow",
            "curation": {"role": "support", "primary": False, "sortOrder": 500},
            "inputSchema": {"xOperationCategory": "debug"},
            "outputSchema": {"xOperationCategory": "workflow"},
        }

        self.assertEqual(audit.classify_operation(operation), "support")

    def test_classify_operation_falls_back_to_legacy_schema_category(self) -> None:
        operation = {
            "id": "demo.inspect",
            "name": "Inspect workflow",
            "inputSchema": {"xOperationCategory": "support"},
            "outputSchema": {"xOperationCategory": "workflow"},
        }

        self.assertEqual(audit.classify_operation(operation), "support")


if __name__ == "__main__":
    unittest.main()
