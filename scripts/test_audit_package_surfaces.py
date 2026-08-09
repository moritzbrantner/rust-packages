#!/usr/bin/env python3
"""Focused tests for scripts/audit_package_surfaces.py."""

from __future__ import annotations

import importlib.util
import sys
import unittest
from pathlib import Path
from unittest import mock

ROOT = Path(__file__).resolve().parents[1]
SCRIPT = ROOT / "scripts" / "audit_package_surfaces.py"
sys.dont_write_bytecode = True
spec = importlib.util.spec_from_file_location("audit_package_surfaces", SCRIPT)
assert spec and spec.loader
audit = importlib.util.module_from_spec(spec)
sys.modules["audit_package_surfaces"] = audit
spec.loader.exec_module(audit)


class PackageSurfaceAuditTests(unittest.TestCase):
    def test_only_enumerated_contract_foundations_are_exempt_from_companion_surfaces(self) -> None:
        metadata = {
            "packages": [
                {
                    "name": "moenarch-media-core",
                    "manifest_path": str(
                        ROOT / "crates/media/media-core/Cargo.toml"
                    ),
                    "targets": [{"kind": ["lib"]}],
                },
                {
                    "name": "moenarch-audio-contracts",
                    "manifest_path": str(
                        ROOT / "crates/audio/audio-contracts/Cargo.toml"
                    ),
                    "targets": [{"kind": ["lib"]}],
                },
                {
                    "name": "moenarch-video-analysis-core",
                    "manifest_path": str(
                        ROOT / "crates/video/video-analysis-core/Cargo.toml"
                    ),
                    "targets": [{"kind": ["lib"]}],
                },
            ]
        }

        with mock.patch.object(audit, "run_json", return_value=metadata):
            self.assertEqual(
                [package.name for package in audit.library_packages(None)],
                ["moenarch-video-analysis-core"],
            )

    def test_transcription_required_resource_example_is_not_executed_offline(
        self,
    ) -> None:
        operation = {
            "id": "audio.transcription.transcribe",
            "exampleRequest": {"source": {"path": "speech.wav"}},
            "inputSchema": {
                "xExecutionPlan": {
                    "requirements": [
                        {"name": "model-bundle", "required": True},
                    ]
                }
            },
        }
        failures: list[str] = []

        with mock.patch.object(audit.subprocess, "run") as run:
            audit.run_operation_example(
                "moenarch-audio-analysis-transcription", operation, failures
            )

        run.assert_not_called()
        self.assertEqual(failures, [])

    def test_other_required_resource_examples_still_execute(self) -> None:
        operation = {
            "id": "image.captioning.caption",
            "exampleRequest": {},
            "inputSchema": {
                "xExecutionPlan": {
                    "requirements": [
                        {"name": "model-bundle", "required": True},
                    ]
                }
            },
        }
        failures: list[str] = []
        completed = mock.Mock(
            returncode=0,
            stdout='{"operation":"image.captioning.caption","value":{"operation":"image.captioning.caption","title":"Caption","message":"Done","summary":{},"result":{}}}',
            stderr="",
        )

        with mock.patch.object(
            audit.subprocess, "run", return_value=completed
        ) as run:
            audit.run_operation_example(
                "moenarch-image-analysis-captioning", operation, failures
            )

        run.assert_called_once()
        self.assertEqual(failures, [])

    def test_optional_runtime_requirements_do_not_bypass_example_execution(self) -> None:
        operation = {
            "id": "audio.transcription.transcribe",
            "exampleRequest": {},
            "inputSchema": {
                "xExecutionPlan": {
                    "requirements": [
                        {"name": "optional-cache", "required": False},
                    ]
                }
            },
        }
        failures: list[str] = []
        completed = mock.Mock(
            returncode=0,
            stdout='{"operation":"audio.transcription.transcribe","value":{"operation":"audio.transcription.transcribe","title":"Demo","message":"Done","summary":{},"result":{}}}',
            stderr="",
        )

        with mock.patch.object(
            audit.subprocess, "run", return_value=completed
        ) as run:
            audit.run_operation_example(
                "moenarch-audio-analysis-transcription", operation, failures
            )

        run.assert_called_once()
        self.assertEqual(failures, [])

    def test_malformed_execution_plan_does_not_bypass_example_execution(self) -> None:
        operation = {
            "id": "audio.transcription.transcribe",
            "exampleRequest": {},
            "inputSchema": {"xExecutionPlan": {"requirements": "model-bundle"}},
        }
        failures: list[str] = []
        completed = mock.Mock(
            returncode=0,
            stdout='{"operation":"audio.transcription.transcribe","value":{"operation":"audio.transcription.transcribe","title":"Demo","message":"Done","summary":{},"result":{}}}',
            stderr="",
        )

        with mock.patch.object(
            audit.subprocess, "run", return_value=completed
        ) as run:
            audit.run_operation_example(
                "moenarch-audio-analysis-transcription", operation, failures
            )

        run.assert_called_once()
        self.assertEqual(failures, [])

    def test_tracer_gate_fails_loose_primary_schema(self) -> None:
        failures: list[str] = []
        audit.validate_tracer_primary_workflow(
            "moenarch-text-index",
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
                        "xLowerContractProof": {"crates": ["moenarch-text-core"]},
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
            "moenarch-text-index",
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
                        "xLowerContractProof": {"crates": ["moenarch-text-core"]},
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
