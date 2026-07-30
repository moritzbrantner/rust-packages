#!/usr/bin/env python3
"""Tests that reviewed ownership remains authority over generated projections."""

from __future__ import annotations

import copy
import subprocess
import unittest

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
        self.assertEqual(len(authority["packages"]), 520)
        self.assertEqual(authority["post_baseline_packages"], [])
        self.assertEqual(authority["schema_version"], 2)

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
                "unclassified cargo packages: moenarch-media-core" in error.lower()
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
            }
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

    def validate(self, authority: dict, metadata: dict) -> list[str]:
        return validate_authority(
            authority,
            metadata,
            bun_manifest_facts(),
            load_json(BASELINE_PATH),
        )

    def head(self) -> str:
        return subprocess.check_output(
            ["git", "rev-parse", "HEAD"],
            cwd=ROOT,
            text=True,
        ).strip()


if __name__ == "__main__":
    unittest.main(verbosity=2)
