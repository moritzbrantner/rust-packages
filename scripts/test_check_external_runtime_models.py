#!/usr/bin/env python3
"""Focused tests for the external runtime/model check orchestrator."""

from __future__ import annotations

import importlib.util
from pathlib import Path
import subprocess
import sys
import unittest


SCRIPT = Path(__file__).with_name("check_external_runtime_models.py")
SPEC = importlib.util.spec_from_file_location("check_external_runtime_models", SCRIPT)
assert SPEC and SPEC.loader
orchestrator = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = orchestrator
SPEC.loader.exec_module(orchestrator)


def completed(stdout: str = "", stderr: str = "", returncode: int = 0) -> subprocess.CompletedProcess[str]:
    return subprocess.CompletedProcess(
        args="test-command",
        returncode=returncode,
        stdout=stdout,
        stderr=stderr,
    )


def check(expected_tests: tuple[str, ...] = ()) -> orchestrator.Check:
    return orchestrator.Check(
        "unit-test-check",
        ("moritzbrantner-model-runtime",),
        "unit-test runtime",
        "setup",
        "run",
        expected_tests=expected_tests,
    )


class ExternalRuntimeModelCheckTests(unittest.TestCase):
    def classify(
        self,
        output: str,
        *,
        returncode: int = 0,
        expected_tests: tuple[str, ...] = (),
        strict: bool = True,
    ) -> tuple[str, list[str], list[str], list[str], list[str]]:
        return orchestrator.classify_completed_check(
            check(expected_tests),
            completed(stdout=output, returncode=returncode),
            strict,
        )

    def test_expected_test_passes_despite_empty_cargo_harness_sections(self) -> None:
        output = """
running 1 test
test tokenizer_presets_have_honest_load_reports ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
"""
        status, blockers, evidence, missing, skips = self.classify(
            output,
            expected_tests=("tokenizer_presets_have_honest_load_reports",),
        )

        self.assertEqual(status, "pass")
        self.assertEqual(blockers, [])
        self.assertEqual(evidence, ["tokenizer_presets_have_honest_load_reports"])
        self.assertEqual(missing, [])
        self.assertEqual(skips, [])

    def test_explicit_skipping_line_is_strict_failure(self) -> None:
        output = """
running 1 test
test real_external_smoke ... ok
skipping configured external smoke because required tool is unavailable
"""
        status, blockers, evidence, missing, skips = self.classify(
            output,
            expected_tests=("real_external_smoke",),
        )

        self.assertEqual(status, "fail")
        self.assertEqual(blockers, ["strict mode detected skipped smoke output"])
        self.assertEqual(evidence, ["real_external_smoke"])
        self.assertEqual(missing, [])
        self.assertEqual(
            skips,
            ["skipping configured external smoke because required tool is unavailable"],
        )

    def test_missing_expected_test_is_failure(self) -> None:
        output = """
running 1 test
test another_smoke ... ok
"""
        status, blockers, evidence, missing, skips = self.classify(
            output,
            expected_tests=("expected_smoke",),
        )

        self.assertEqual(status, "fail")
        self.assertEqual(
            blockers,
            ["command succeeded but expected test evidence was missing"],
        )
        self.assertEqual(evidence, [])
        self.assertEqual(missing, ["expected_smoke"])
        self.assertEqual(skips, [])

    def test_setup_failures_classify_as_blocked_setup(self) -> None:
        cases = [
            "ModuleNotFoundError: No module named 'huggingface_hub'",
            "HF_TOKEN is required for gated model access",
            "CUDA error: CUBLAS_STATUS_NOT_INITIALIZED",
            "ORT_DYLIB_PATH is required",
        ]
        for output in cases:
            with self.subTest(output=output):
                status, blockers, evidence, missing, skips = self.classify(
                    output,
                    returncode=1,
                    expected_tests=("expected_smoke",),
                )
                self.assertEqual(status, "blocked-setup")
                self.assertEqual(blockers, ["command exited with 1"])
                self.assertEqual(evidence, [])
                self.assertEqual(missing, ["expected_smoke"])
                self.assertEqual(skips, [])


if __name__ == "__main__":
    unittest.main(verbosity=2)
