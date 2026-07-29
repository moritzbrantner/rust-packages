#!/usr/bin/env python3
"""Focused behavior tests for exact release-plan and Cargo-manifest validation."""

from __future__ import annotations

import copy
import json
import shutil
import tempfile
import unittest
from pathlib import Path

from check_release_plan import load_document, validate_plan

ROOT = Path(__file__).resolve().parents[1]
FIXTURES = ROOT / "scripts/fixtures/release_plans"
WORKSPACE = FIXTURES / "workspace"


class ReleasePlanCheckTests(unittest.TestCase):
    def setUp(self) -> None:
        self.plan = load_document(FIXTURES / "valid.json")
        self.ownership = load_document(FIXTURES / "ownership.json")
        self.invalid = load_document(FIXTURES / "invalid-cases.json")

    def errors(
        self,
        plan: dict,
        *,
        root: Path = WORKSPACE,
        ownership: dict | None = None,
        expected_sha: str | None = None,
        expected_base_sha: str | None = None,
    ) -> str:
        return "\n".join(
            validate_plan(
                plan,
                ownership or self.ownership,
                expected_sha,
                expected_base_sha,
                root,
            )
        )

    def with_manifest(self, relative: str, content: str, assertion) -> None:
        with tempfile.TemporaryDirectory() as raw_directory:
            root = Path(raw_directory)
            shutil.copytree(WORKSPACE, root, dirs_exist_ok=True)
            (root / relative).write_text(content, encoding="utf-8")
            assertion(root)

    def test_valid_json_topological_plan_is_accepted(self) -> None:
        self.assertEqual(self.errors(self.plan), "")

    def test_valid_toml_plan_is_accepted(self) -> None:
        plan = load_document(FIXTURES / "valid.toml")
        self.assertEqual(self.errors(plan), "")

    def test_cycle_is_rejected(self) -> None:
        plan = copy.deepcopy(self.plan)
        for package in plan["packages"]:
            package["release_dependencies"] = self.invalid["cyclic"][package["name"]]
        self.assertIn("release dependency cycle", self.errors(plan))

    def test_duplicate_package_is_rejected(self) -> None:
        plan = copy.deepcopy(self.plan)
        plan["packages"].append(copy.deepcopy(plan["packages"][0]))
        self.assertIn("duplicate package names", self.errors(plan))

    def test_wrong_order_is_rejected(self) -> None:
        plan = copy.deepcopy(self.plan)
        plan["dependency_order"] = self.invalid["wrong-order"]
        self.assertIn("wrong dependency order", self.errors(plan))

    def test_wrong_owner_is_rejected(self) -> None:
        plan = copy.deepcopy(self.plan)
        plan["packages"][0]["owner"] = self.invalid["wrong-owner"]
        self.assertIn("wrong owner", self.errors(plan))

    def test_old_version_must_match_reviewed_source(self) -> None:
        plan = copy.deepcopy(self.plan)
        plan["packages"][0]["old_version"] = "0.0.9"
        self.assertIn("does not match reviewed source version", self.errors(plan))

    def test_malformed_unchanged_and_regressed_versions_are_rejected(self) -> None:
        for value in ("not-semver", "0.1.0", "0.0.9"):
            with self.subTest(value=value):
                plan = copy.deepcopy(self.plan)
                plan["packages"][0]["new_version"] = value
                errors = self.errors(plan)
                self.assertTrue(
                    "malformed new_version" in errors
                    or "strictly greater" in errors
                )

    def test_path_only_dependency_is_rejected_from_real_manifest(self) -> None:
        content = """
[package]
name = "foundation-b"
version = "0.2.0"
edition = "2024"
[dependencies]
foundation-a = { path = "../foundation-a" }
"""
        self.with_manifest(
            "foundation-b/Cargo.toml",
            content,
            lambda root: self.assertIn("path-only dependency", self.errors(self.plan, root=root)),
        )

    def test_outside_repository_path_is_rejected_from_real_manifest(self) -> None:
        content = """
[package]
name = "foundation-b"
version = "0.2.0"
edition = "2024"
[dependencies]
outside = { path = "../../outside", version = "=0.2.0" }
"""
        self.with_manifest(
            "foundation-b/Cargo.toml",
            content,
            lambda root: self.assertIn(
                "outside-repository path dependency", self.errors(self.plan, root=root)
            ),
        )

    def test_cross_owner_path_is_rejected_from_real_manifest(self) -> None:
        ownership = copy.deepcopy(self.ownership)
        ownership["packages"][0]["target_repository"] = "audio-analysis"
        self.assertIn(
            "cross-owner path dependency",
            self.errors(self.plan, ownership=ownership),
        )

    def test_moving_branch_and_short_git_revision_are_rejected(self) -> None:
        for git_spec in (
            'git = "https://example.invalid/repo", branch = "main"',
            'git = "https://example.invalid/repo", rev = "abc123"',
            'git = "https://example.invalid/repo", tag = "v1.0.0"',
        ):
            with self.subTest(git_spec=git_spec):
                content = f"""
[package]
name = "foundation-b"
version = "0.2.0"
edition = "2024"
[dependencies]
remote = {{ {git_spec} }}
"""
                plan = copy.deepcopy(self.plan)
                plan["packages"][1]["release_dependencies"] = []
                self.with_manifest(
                    "foundation-b/Cargo.toml",
                    content,
                    lambda root: self.assertIn(
                        "non-immutable Git dependency",
                        self.errors(plan, root=root),
                    ),
                )

    def test_missing_foundation_consumer_gates_are_rejected(self) -> None:
        plan = copy.deepcopy(self.plan)
        plan["required_consumer_checks"] = []
        self.assertIn("foundation publication requires consumer gates", self.errors(plan))

    def test_missing_release_issue_is_rejected(self) -> None:
        plan = copy.deepcopy(self.plan)
        del plan["release_issue"]
        self.assertIn("missing exact release issue", self.errors(plan))

    def test_missing_new_required_fields_are_rejected(self) -> None:
        for field in ("required_features", "compatibility_or_deprecation_packages"):
            with self.subTest(field=field):
                plan = copy.deepcopy(self.plan)
                del plan[field]
                self.assertIn(f"missing required field {field}", self.errors(plan))

    def test_source_and_base_sha_bindings_are_enforced(self) -> None:
        errors = self.errors(
            self.plan,
            expected_sha="3333333333333333333333333333333333333333",
            expected_base_sha="4444444444444444444444444444444444444444",
        )
        self.assertIn("source_sha", errors)
        self.assertIn("default_branch_base_sha", errors)

    def test_manifest_must_match_reviewed_path_and_package_name(self) -> None:
        plan = copy.deepcopy(self.plan)
        plan["packages"][0]["manifest_path"] = "foundation-b/Cargo.toml"
        errors = self.errors(plan)
        self.assertIn("manifest_path does not match reviewed ownership", errors)
        self.assertIn("manifest package name", errors)


if __name__ == "__main__":
    unittest.main(verbosity=2)
