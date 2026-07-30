#!/usr/bin/env python3
"""Focused behavior tests for capability repository boundary enforcement."""

from __future__ import annotations

import copy
import subprocess
import sys
import unittest
from pathlib import Path

from check_repository_boundaries import validate
from repository_split import (
    OWNERSHIP_PATH,
    TARGET_REPOSITORIES,
    cargo_metadata,
    find_cycle,
    load_json,
)

ROOT = Path(__file__).resolve().parents[1]
SCRIPT = ROOT / "scripts/check_repository_boundaries.py"
NEGATIVE_FIXTURE = (
    ROOT / "scripts/fixtures/repository_boundaries/negative-new-edge"
)


def target_graph() -> dict[str, list[str]]:
    return {
        "audio-analysis": ["moenarch-foundation", "nlp-stack"],
        "moenarch-foundation": [],
        "nlp-stack": ["moenarch-foundation"],
        "rust-packages": [
            "audio-analysis",
            "moenarch-foundation",
            "nlp-stack",
            "spatial-analysis",
            "visual-analysis",
        ],
        "spatial-analysis": ["moenarch-foundation", "visual-analysis"],
        "visual-analysis": ["moenarch-foundation", "nlp-stack"],
    }


def package(name: str, dependencies: list[dict] | None = None) -> dict:
    return {"name": name, "dependencies": dependencies or []}


def dependency(
    name: str, kind: str = "normal", optional: bool = False
) -> dict:
    return {
        "name": name,
        "kind": None if kind == "normal" else kind,
        "optional": optional,
    }


def record(
    name: str,
    repository: str,
    kind: str = "library",
    wrapped: str | None = None,
) -> dict:
    return {
        "current_package_name": name,
        "ecosystem": "cargo",
        "package_kind": kind,
        "target_repository": repository,
        "wrapped_library": wrapped,
    }


def baseline_entry(
    source: str,
    target: str,
    kind: str = "normal",
    *,
    optional: bool = False,
    reason: str = "reviewed exact migration edge",
) -> dict:
    return {
        "source_package": source,
        "dependency_package": target,
        "dependency_kind": kind,
        "optional": optional,
        "reason": reason,
        "migration_issue": "https://github.com/moritzbrantner/rust-packages/issues/109",
        "target_phase": "foundation",
    }


class RepositoryBoundaryCheckTests(unittest.TestCase):
    def validate(
        self, packages: list[dict], records: list[dict], violations: list[dict]
    ) -> tuple[str, list[dict], list[list[str]]]:
        errors, actual, cycles = validate(
            {"packages": packages},
            {
                "packages": records,
                "target_repository_dependencies": target_graph(),
            },
            {"violations": violations},
            enforce_authority=False,
        )
        return "\n".join(errors), actual, cycles

    def test_live_workspace_matches_exact_baseline_and_target_law(self) -> None:
        completed = subprocess.run(
            [sys.executable, str(SCRIPT), "--check"],
            cwd=ROOT,
            capture_output=True,
            text=True,
        )
        self.assertEqual(completed.returncode, 0, completed.stderr)
        self.assertIn("50 reviewed violations", completed.stdout)
        self.assertIn("46 normal", completed.stdout)
        self.assertIn("4 dev", completed.stdout)
        self.assertNotIn("BASELINED CYCLE", completed.stdout)

    def test_live_authority_provenance_is_validated_by_boundary_consumer(self) -> None:
        ownership = load_json(OWNERSHIP_PATH)
        ownership["post_baseline_packages"] = [
            copy.deepcopy(ownership["packages"][0])
        ]
        errors, _, _ = validate(
            cargo_metadata(),
            ownership,
            {"violations": []},
        )
        self.assertTrue(
            any("missing provenance" in error for error in errors),
            errors,
        )

    def test_complete_and_unique_ownership_is_required(self) -> None:
        packages = [package("foundation"), package("audio")]
        missing, _, _ = self.validate(
            packages,
            [record("foundation", "moenarch-foundation")],
            [],
        )
        self.assertIn("unclassified Cargo packages: audio", missing)
        duplicate, _, _ = self.validate(
            packages,
            [
                record("foundation", "moenarch-foundation"),
                record("foundation", "moenarch-foundation"),
                record("audio", "audio-analysis"),
            ],
            [],
        )
        self.assertIn("classified more than once", duplicate)

    def test_dependency_kinds_remain_distinct_from_optionality(self) -> None:
        kinds = ("normal", "build", "dev")
        packages = []
        records = []
        baseline = []
        for kind in kinds:
            source = f"foundation-{kind}"
            target = f"audio-{kind}"
            packages.extend(
                [
                    package(
                        source,
                        [
                            dependency(target, kind)
                        ],
                    ),
                    package(target),
                ]
            )
            records.extend(
                [
                    record(source, "moenarch-foundation"),
                    record(target, "audio-analysis"),
                ]
            )
            baseline.append(baseline_entry(source, target, kind))
        errors, violations, _ = self.validate(packages, records, baseline)
        self.assertEqual(errors, "")
        self.assertEqual(
            {violation["dependency_kind"] for violation in violations}, set(kinds)
        )

    def test_optional_build_dependency_cannot_masquerade_as_optional_kind(self) -> None:
        errors, violations, _ = self.validate(
            [
                package(
                    "foundation",
                    [
                        dependency("audio", kind="build", optional=True),
                    ],
                ),
                package("audio"),
            ],
            [
                record("foundation", "moenarch-foundation"),
                record("audio", "audio-analysis"),
            ],
            [
                baseline_entry("foundation", "audio", kind="optional"),
            ],
        )
        self.assertIn(
            "new forbidden edge: foundation -> audio "
            "(build optional; moenarch-foundation -> audio-analysis)",
            errors,
        )
        self.assertIn(
            "stale baseline violations must be removed after the edge is fixed: "
            "foundation->audio(optional required)",
            errors,
        )
        self.assertEqual(len(violations), 1)
        self.assertEqual(
            violations[0]["dependency_kind"],
            "build",
        )
        self.assertTrue(violations[0]["optional"])

    def test_required_and_optional_declarations_remain_distinct_edges(self) -> None:
        errors, violations, _ = self.validate(
            [
                package(
                    "foundation",
                    [
                        dependency("audio", optional=False),
                        dependency("audio", optional=True),
                    ],
                ),
                package("audio"),
            ],
            [
                record("foundation", "moenarch-foundation"),
                record("audio", "audio-analysis"),
            ],
            [
                baseline_entry("foundation", "audio", optional=False),
                baseline_entry("foundation", "audio", optional=True),
            ],
        )
        self.assertEqual(errors, "")
        self.assertEqual(len(violations), 2)
        self.assertEqual(
            {violation["optional"] for violation in violations},
            {False, True},
        )

    def test_adapter_must_name_a_wrapped_library_with_same_owner(self) -> None:
        errors, _, _ = self.validate(
            [package("foundation-cli"), package("audio-lib")],
            [
                record(
                    "foundation-cli",
                    "moenarch-foundation",
                    "CLI",
                    "audio-lib",
                ),
                record("audio-lib", "audio-analysis"),
            ],
            [],
        )
        self.assertIn("differs from wrapped library", errors)
        missing, _, _ = self.validate(
            [package("foundation-cli")],
            [record("foundation-cli", "moenarch-foundation", "CLI")],
            [],
        )
        self.assertIn("adapter is missing wrapped_library", missing)

    def test_missing_stale_duplicate_and_wildcard_baselines_fail(self) -> None:
        source = package("foundation", [dependency("audio")])
        target = package("audio")
        records = [
            record("foundation", "moenarch-foundation"),
            record("audio", "audio-analysis"),
        ]
        missing, _, _ = self.validate(
            [source, target],
            records,
            [
                {
                    "source_package": "foundation",
                    "dependency_package": "audio",
                    "dependency_kind": "normal",
                }
            ],
        )
        self.assertIn("missing reason", missing)
        stale, _, _ = self.validate(
            [package("foundation"), target],
            records,
            [baseline_entry("foundation", "audio")],
        )
        self.assertIn("stale baseline violations", stale)
        duplicate, _, _ = self.validate(
            [source, target],
            records,
            [
                baseline_entry("foundation", "audio"),
                baseline_entry("foundation", "audio"),
            ],
        )
        self.assertIn("duplicate baseline entries", duplicate)
        wildcard, _, _ = self.validate(
            [source, target],
            records,
            [baseline_entry("foundation", "audio", reason="all audio *")],
        )
        self.assertIn("wildcard in reason", wildcard)

    def test_reintroduced_forbidden_edge_without_baseline_fails(self) -> None:
        errors, _, _ = self.validate(
            [package("foundation", [dependency("audio")]), package("audio")],
            [
                record("foundation", "moenarch-foundation"),
                record("audio", "audio-analysis"),
            ],
            [],
        )
        self.assertIn("new forbidden edge", errors)

    def test_checked_in_negative_edge_fixture_fails_through_cli(self) -> None:
        completed = subprocess.run(
            [
                sys.executable,
                str(SCRIPT),
                "--check",
                "--metadata",
                str(NEGATIVE_FIXTURE / "metadata.json"),
                "--ownership",
                str(NEGATIVE_FIXTURE / "ownership.json"),
                "--baseline",
                str(NEGATIVE_FIXTURE / "baseline.json"),
            ],
            cwd=ROOT,
            capture_output=True,
            text=True,
        )
        self.assertNotEqual(completed.returncode, 0)
        self.assertIn(
            "new forbidden edge: moenarch-foundation-core -> "
            "moenarch-audio-core "
            "(normal required; moenarch-foundation -> audio-analysis)",
            completed.stderr,
        )

    def test_unknown_repository_fails(self) -> None:
        errors, _, _ = self.validate(
            [package("unknown")],
            [record("unknown", "made-up-repository")],
            [],
        )
        self.assertIn("unknown target repository", errors)

    def test_baselined_current_reverse_edge_does_not_change_target_graph(self) -> None:
        packages = [
            package("foundation", [dependency("visual")]),
            package("visual", [dependency("foundation")]),
        ]
        records = [
            record("foundation", "moenarch-foundation"),
            record("visual", "visual-analysis"),
        ]
        errors, _, cycles = self.validate(
            packages,
            records,
            [baseline_entry("foundation", "visual")],
        )
        self.assertEqual(errors, "")
        self.assertFalse(cycles)

    def test_current_reverse_edge_without_baseline_fails(self) -> None:
        errors, _, cycles = self.validate(
            [
                package("foundation", [dependency("visual")]),
                package("visual", [dependency("foundation")]),
            ],
            [
                record("foundation", "moenarch-foundation"),
                record("visual", "visual-analysis"),
            ],
            [],
        )
        self.assertFalse(cycles)
        self.assertIn("new forbidden edge", errors)

    def test_allowed_architecture_law_is_acyclic(self) -> None:
        graph = target_graph()
        cycle = find_cycle(
            TARGET_REPOSITORIES,
            (
                (source, dependency)
                for source, dependencies in graph.items()
                for dependency in dependencies
            ),
        )
        self.assertIsNone(cycle)

    def test_reviewed_target_graph_cycle_is_rejected(self) -> None:
        errors, _, cycles = validate(
            {"packages": []},
            {
                "packages": [],
                "target_repository_dependencies": {
                    "audio-analysis": ["moenarch-foundation"],
                    "moenarch-foundation": ["audio-analysis"],
                    "nlp-stack": ["moenarch-foundation"],
                    "rust-packages": [],
                    "spatial-analysis": ["visual-analysis"],
                    "visual-analysis": ["moenarch-foundation", "nlp-stack"],
                },
            },
            {"violations": []},
        )
        self.assertTrue(cycles)
        self.assertIn("reviewed target repository graph is cyclic", "\n".join(errors))

    def test_acyclic_target_graph_cannot_broaden_directional_law(self) -> None:
        graph = target_graph()
        graph["audio-analysis"] = []
        graph["moenarch-foundation"] = ["audio-analysis"]
        errors, _, cycles = validate(
            {"packages": []},
            {
                "packages": [],
                "target_repository_dependencies": graph,
            },
            {"violations": []},
        )
        self.assertFalse(cycles)
        self.assertIn(
            "reviewed target repository graph does not match required "
            "directional law",
            "\n".join(errors),
        )

    def test_reviewed_target_graph_is_required(self) -> None:
        errors, _, _ = validate(
            {"packages": []},
            {"packages": []},
            {"violations": []},
        )
        self.assertIn(
            "missing target_repository_dependencies",
            "\n".join(errors),
        )


if __name__ == "__main__":
    unittest.main(verbosity=2)
