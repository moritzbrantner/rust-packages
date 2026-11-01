#!/usr/bin/env python3
"""Tests that reviewed ownership remains authority over generated projections."""

from __future__ import annotations

import copy
import subprocess
import unittest
from unittest.mock import patch

from generate_repository_split_inventory import (
    bun_manifest_facts,
    destination_markdown,
    generate,
    validate_authority,
)
from repository_split import BASELINE_PATH, ROOT, cargo_metadata, load_json


class RepositorySplitInventoryTests(unittest.TestCase):
    def test_live_reviewed_authority_matches_manifests(self) -> None:
        authority, _, _, errors = generate()
        self.assertEqual(errors, [])
        self.assertEqual(len(authority["packages"]), 102)
        self.assertEqual(authority["post_baseline_packages"], [])
        self.assertEqual(authority["resolved_boundary_violations"], [])
        self.assertEqual(authority["schema_version"], 2)

    def test_boundary_resolutions_are_exact_unique_and_absent_from_baseline(
        self,
    ) -> None:
        authority, _, _, errors = generate()
        self.assertEqual(errors, [])
        metadata = cargo_metadata()
        violation = {
            "source_package": "moenarch-comfyui-data",
            "dependency_package": "moenarch-video-analysis-test-support",
            "dependency_kind": "dev",
            "optional": False,
            "migration_issue": "https://github.com/moritzbrantner/rust-packages/issues/107",
            "target_phase": "phase-a",
        }
        resolution = violation | {
            "resolved_by_issue": "https://github.com/moritzbrantner/rust-packages/issues/109"
        }

        unknown = copy.deepcopy(authority)
        unknown["resolved_boundary_violations"].append(
            resolution | {"source_package": "unknown-package"}
        )
        self.assertTrue(
            any(
                "does not match an immutable Phase A annotation" in error
                for error in self.validate(unknown, metadata)
            )
        )

        duplicate = copy.deepcopy(authority)
        duplicate["resolved_boundary_violations"].extend(
            [copy.deepcopy(resolution), copy.deepcopy(resolution)]
        )
        self.assertTrue(
            any(
                "duplicate boundary resolutions" in error
                for error in self.validate(duplicate, metadata)
            )
        )

        still_baselined = copy.deepcopy(authority)
        still_baselined["resolved_boundary_violations"].append(resolution)
        baseline = load_json(BASELINE_PATH)
        baseline["violations"] = [violation | {"reason": "fixture"}]
        self.assertTrue(
            any(
                "must be absent from the current baseline" in error
                for error in self.validate(still_baselined, metadata, baseline)
            )
        )

    def test_projection_uses_package_specific_reviewed_decision(self) -> None:
        authority, _, _, errors = generate()
        self.assertEqual(errors, [])
        changed = copy.deepcopy(authority)
        package = next(
            record
            for record in changed["packages"]
            if record["id"] == "cargo:moenarch-video-analysis"
        )
        package["target_repository"] = "nlp-stack"
        markdown = destination_markdown(changed)
        row = next(
            line
            for line in markdown.splitlines()
            if "| `moenarch-video-analysis` |" in line
        )
        self.assertIn("`moritzbrantner/nlp-stack`", row)

    def test_post_baseline_package_requires_exact_provenance(self) -> None:
        authority, metadata = self.authority_with_media_package()
        errors = self.validate(authority, metadata)
        self.assertEqual(errors, [])

        invalid_cases = {
            "missing": None,
            "mutable": {
                "introduced_after_commit": "HEAD",
                "issue": "https://github.com/moritzbrantner/rust-packages/issues/108",
            },
            "unavailable": {
                "introduced_after_commit": "f" * 40,
                "issue": "https://github.com/moritzbrantner/rust-packages/issues/108",
            },
            "bad-issue": {
                "introduced_after_commit": self.head(),
                "issue": "#108",
            },
            "extra-key": {
                "introduced_after_commit": self.head(),
                "issue": "https://github.com/moritzbrantner/rust-packages/issues/108",
                "note": "unreviewed",
            },
        }
        for name, provenance in invalid_cases.items():
            with self.subTest(name=name):
                changed = copy.deepcopy(authority)
                if provenance is None:
                    changed["post_baseline_packages"][0].pop("provenance")
                else:
                    changed["post_baseline_packages"][0]["provenance"] = provenance
                self.assertNotEqual(self.validate(changed, metadata), [])

    def test_unrecorded_and_duplicate_post_baseline_packages_fail(self) -> None:
        authority, metadata = self.authority_with_media_package()
        unrecorded = copy.deepcopy(authority)
        unrecorded["post_baseline_packages"] = []
        self.assertTrue(
            any(
                "unclassified cargo packages" in error.lower()
                and "moenarch-media-core" in error.lower()
                for error in self.validate(unrecorded, metadata)
            )
        )

        duplicate = copy.deepcopy(authority)
        existing = copy.deepcopy(duplicate["packages"][0])
        existing["provenance"] = {
            "introduced_after_commit": self.head(),
            "issue": "https://github.com/moritzbrantner/rust-packages/issues/108",
        }
        duplicate["post_baseline_packages"].append(existing)
        self.assertTrue(
            any(
                "duplicate ownership ids" in error
                or "classified more than once" in error
                for error in self.validate(duplicate, metadata)
            )
        )

    def test_phase_a_package_array_is_immutable(self) -> None:
        authority, metadata = self.authority_with_media_package()
        moved = copy.deepcopy(authority)
        moved["packages"].append(moved["post_baseline_packages"].pop())
        self.assertTrue(
            any(
                "immutable phase a packages" in error.lower()
                for error in self.validate(moved, metadata)
            )
        )

    def test_creating_issue_must_be_canonical(self) -> None:
        authority, metadata = self.authority_with_media_package()
        for suffix in ("0", "0108"):
            with self.subTest(suffix=suffix):
                changed = copy.deepcopy(authority)
                changed["post_baseline_packages"][0]["provenance"]["issue"] = (
                    "https://github.com/moritzbrantner/rust-packages/issues/"
                    + suffix
                )
                self.assertTrue(
                    any(
                        "invalid creating issue" in error
                        for error in self.validate(changed, metadata)
                    )
                )

    def test_future_divergent_provenance_commit_is_rejected(self) -> None:
        authority, metadata = self.authority_with_media_package()
        with (
            patch("repository_split.git_commit_exists", return_value=True),
            patch(
                "repository_split.git_commit_is_ancestor",
                side_effect=lambda *args: len(args) == 3,
            ),
        ):
            self.assertTrue(
                any(
                    "must be an ancestor of head" in error.lower()
                    for error in self.validate(authority, metadata)
                )
            )

    def authority_with_media_package(self) -> tuple[dict, dict]:
        authority, _, _, errors = generate()
        self.assertEqual(errors, [])
        changed = copy.deepcopy(authority)
        changed["post_baseline_packages"] = [
            {
                "id": "cargo:moenarch-media-core",
                "ecosystem": "cargo",
                "current_package_name": "moenarch-media-core",
                "intended_next_release_owner": "moritzbrantner/moenarch-foundation",
                "manifest_path": "crates/media/media-core/Cargo.toml",
                "package_kind": "library",
                "source_version": "0.1.0",
                "target_repository": "moenarch-foundation",
                "temporary_boundary_violations": [],
                "provenance": {
                    "introduced_after_commit": self.head(),
                    "issue": "https://github.com/moritzbrantner/rust-packages/issues/108",
                },
            },
        ]
        metadata = cargo_metadata()
        metadata["packages"].append(
            {
                "name": "moenarch-media-core",
                "manifest_path": str(ROOT / "crates/media/media-core/Cargo.toml"),
                "version": "0.1.0",
                "dependencies": [],
            }
        )
        return changed, metadata

    def validate(
        self, authority: dict, metadata: dict, baseline: dict | None = None
    ) -> list[str]:
        return validate_authority(
            authority,
            metadata,
            bun_manifest_facts(),
            baseline if baseline is not None else load_json(BASELINE_PATH),
        )

    def head(self) -> str:
        return subprocess.check_output(
            ["git", "rev-parse", "HEAD"],
            cwd=ROOT,
            text=True,
        ).strip()


if __name__ == "__main__":
    unittest.main(verbosity=2)
