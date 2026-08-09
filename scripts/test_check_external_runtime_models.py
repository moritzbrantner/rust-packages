#!/usr/bin/env python3
"""Focused tests for the external runtime/model check orchestrator."""

from __future__ import annotations

import importlib.util
import os
from pathlib import Path
import subprocess
import sys
import tempfile
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
    def test_standard_e2e_uses_generated_audio_whisper_cli_and_native_stays_opt_in(self) -> None:
        e2e = SCRIPT.with_name("check-e2e.sh").read_text(encoding="utf-8")
        self.assertIn(
            "-p moenarch-video-analysis-use-cases --test external_tools "
            "real_whisper_cli_transcribes_generated_speech_with_timing",
            e2e,
        )
        self.assertNotIn("RUN_NATIVE_WHISPER_TESTS", e2e)
        self.assertNotIn("whisper_native_external", e2e)

        native = next(
            row
            for row in orchestrator.CHECKS
            if row.check_id == "audio-transcription-whisper-cpp"
        )
        self.assertIn("RUN_NATIVE_WHISPER_TESTS=1", native.run_command or "")
        self.assertIn("--test whisper_native_external", native.run_command or "")
        self.assertEqual(
            native.expected_tests,
            ("native_whisper_cpp_smoke_when_requested",),
        )

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
            "colmap: error while loading shared libraries: libcudart.so.12",
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

    def test_missing_colmap_text_dir_is_blocked_setup(self) -> None:
        row = orchestrator.Check(
            "colmap-text",
            ("moritzbrantner-video-analysis-radiance-io",),
            "COLMAP sparse text",
            "setup",
            "run",
            required_colmap_text_dirs=("COLMAP_SPARSE_TEXT_DIR",),
        )
        blockers = orchestrator.precondition_blockers(row, os.environ.copy() | {"COLMAP_SPARSE_TEXT_DIR": ""})

        self.assertEqual(blockers, ["missing required COLMAP sparse text dir env COLMAP_SPARSE_TEXT_DIR"])

    def test_empty_colmap_text_dir_is_blocked_setup(self) -> None:
        row = orchestrator.Check(
            "colmap-text",
            ("moritzbrantner-video-analysis-radiance-io",),
            "COLMAP sparse text",
            "setup",
            "run",
            required_colmap_text_dirs=("COLMAP_SPARSE_TEXT_DIR",),
        )
        with tempfile.TemporaryDirectory() as temp:
            blockers = orchestrator.precondition_blockers(
                row,
                os.environ.copy() | {"COLMAP_SPARSE_TEXT_DIR": str(Path(temp).resolve())},
            )

        self.assertTrue(any("cameras.txt" in blocker for blocker in blockers))
        self.assertTrue(any("images.txt" in blocker for blocker in blockers))
        self.assertTrue(any("points3D.txt" in blocker for blocker in blockers))

    def test_complete_colmap_text_dir_clears_preconditions(self) -> None:
        row = orchestrator.Check(
            "colmap-text",
            ("moritzbrantner-video-analysis-radiance-io",),
            "COLMAP sparse text",
            "setup",
            "run",
            required_colmap_text_dirs=("COLMAP_SPARSE_TEXT_DIR",),
        )
        with tempfile.TemporaryDirectory() as temp:
            path = Path(temp).resolve()
            for name in ("cameras.txt", "images.txt", "points3D.txt"):
                path.joinpath(name).write_text("# fixture\n", encoding="utf-8")

            blockers = orchestrator.precondition_blockers(
                row,
                os.environ.copy() | {"COLMAP_SPARSE_TEXT_DIR": str(path)},
            )

        self.assertEqual(blockers, [])

    def test_gpu_only_crate_without_included_rows_is_excluded(self) -> None:
        summary = orchestrator.aggregate_crates([])
        by_package = {item["package"]: item for item in summary}

        radiance_fields = by_package["moritzbrantner-video-analysis-radiance-fields"]
        self.assertEqual(radiance_fields["status"], "excluded")
        self.assertIn("--gpu", radiance_fields["blockers"][0])

        runtime_onnx = by_package["moritzbrantner-runtime-onnx"]
        self.assertEqual(runtime_onnx["status"], "missing-coverage")


if __name__ == "__main__":
    unittest.main(verbosity=2)
