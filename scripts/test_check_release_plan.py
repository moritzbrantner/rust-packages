#!/usr/bin/env python3
"""Focused behavior tests for exact release-plan and Cargo-manifest validation."""

from __future__ import annotations

import copy
import json
import shutil
import tempfile
import unittest
from pathlib import Path

from check_release_plan import extract_release_authorization, load_document, validate_plan

ROOT = Path(__file__).resolve().parents[1]
FIXTURES = ROOT / "scripts/fixtures/release_plans"
WORKSPACE = FIXTURES / "workspace"


class ReleasePlanCheckTests(unittest.TestCase):
    def setUp(self) -> None:
        self.plan = load_document(FIXTURES / "valid.json")
        self.ownership = load_document(FIXTURES / "ownership.json")
        self.invalid = load_document(FIXTURES / "invalid-cases.json")
        self.authorization = {
            "authorization": "publish",
            "repository": self.plan["repository"],
            "release_issue": self.plan["release_issue"],
            "source_sha": self.plan["source_sha"],
            "default_branch_base_sha": self.plan["default_branch_base_sha"],
            "required_checks": self.plan["required_checks"],
            "packages": [
                {
                    "name": package["name"],
                    "version": package["new_version"],
                }
                for package in self.plan["packages"]
                if package["publish"]
            ],
            "_issue_state": "OPEN",
            "_issue_url": self.plan["release_issue"],
        }

    def errors(
        self,
        plan: dict,
        *,
        root: Path = WORKSPACE,
        ownership: dict | None = None,
        actual_head_sha: str | None = "2222222222222222222222222222222222222222",
        actual_base_sha: str | None = "1111111111111111111111111111111111111111",
        release_authorization: dict | None = None,
    ) -> str:
        return "\n".join(
            validate_plan(
                plan,
                ownership or self.ownership,
                root,
                actual_head_sha,
                actual_base_sha,
                (
                    self.authorization
                    if release_authorization is None
                    else release_authorization
                ),
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

    def test_required_checks_must_be_nonempty_commands(self) -> None:
        plan = copy.deepcopy(self.plan)
        plan["required_checks"] = ["  "]
        self.assertIn(
            "required_checks entries must be nonempty strings",
            self.errors(plan),
        )

    def test_trivial_required_check_cannot_replace_reviewed_executable_check(self) -> None:
        plan = copy.deepcopy(self.plan)
        plan["required_checks"] = ["true"]
        self.assertIn(
            "required_checks does not match live release-issue authorization",
            self.errors(plan),
        )

    def test_publishable_tag_must_match_exact_package_and_version(self) -> None:
        plan = copy.deepcopy(self.plan)
        old_tag = plan["packages"][0]["expected_tag"]
        wrong_tag = "release-v0.2.0"
        plan["packages"][0]["expected_tag"] = wrong_tag
        plan["expected_tags"] = [
            wrong_tag if tag == old_tag else tag for tag in plan["expected_tags"]
        ]
        self.assertIn(
            "foundation-a: expected_tag must be foundation-a-v0.2.0",
            self.errors(plan),
        )

    def test_nonpublish_entry_cannot_authorize_version_or_tag(self) -> None:
        plan = copy.deepcopy(self.plan)
        plan["packages"][0]["publish"] = False
        errors = self.errors(plan)
        self.assertIn(
            "foundation-a: nonpublish entry must keep new_version equal to old_version",
            errors,
        )
        self.assertIn(
            "foundation-a: nonpublish entry must not declare expected_tag",
            errors,
        )

    def test_expected_tags_must_not_contain_duplicates(self) -> None:
        plan = copy.deepcopy(self.plan)
        plan["expected_tags"].append(plan["expected_tags"][0])
        self.assertIn("expected_tags contains duplicates", self.errors(plan))

    def test_missing_release_issue_is_rejected(self) -> None:
        plan = copy.deepcopy(self.plan)
        del plan["release_issue"]
        self.assertIn("missing exact release issue", self.errors(plan))

    def test_release_issue_must_be_canonical_for_release_repository(self) -> None:
        for issue in (
            "https://github.com/moritzbrantner/rust-packages/issues/111",
            "https://github.com/moritzbrantner/moenarch-foundation/issues/not-a-number",
            "https://github.com/moritzbrantner/moenarch-foundation/issues/111?draft=1",
        ):
            with self.subTest(issue=issue):
                plan = copy.deepcopy(self.plan)
                plan["release_issue"] = issue
                self.assertIn(
                    "release issue must be a canonical numeric issue URL for "
                    "moritzbrantner/moenarch-foundation",
                    self.errors(plan),
                )

    def test_canonical_but_unauthorized_release_issue_is_rejected(self) -> None:
        plan = copy.deepcopy(self.plan)
        plan["release_issue"] = (
            "https://github.com/moritzbrantner/moenarch-foundation/issues/999"
        )
        self.assertIn(
            "release_issue does not match live release-issue authorization",
            self.errors(plan),
        )

    def test_publish_flag_must_be_boolean(self) -> None:
        plan = copy.deepcopy(self.plan)
        plan["packages"][0]["publish"] = "yes"
        self.assertIn("foundation-a: publish must be a boolean", self.errors(plan))

    def test_missing_new_required_fields_are_rejected(self) -> None:
        for field in ("required_features", "compatibility_or_deprecation_packages"):
            with self.subTest(field=field):
                plan = copy.deepcopy(self.plan)
                del plan[field]
                self.assertIn(f"missing required field {field}", self.errors(plan))

    def test_source_and_base_sha_bindings_are_enforced(self) -> None:
        errors = self.errors(
            self.plan,
            actual_head_sha="3333333333333333333333333333333333333333",
            actual_base_sha="4444444444444444444444444444444444444444",
        )
        self.assertIn("source_sha", errors)
        self.assertIn("default_branch_base_sha", errors)

    def test_publishable_plan_requires_external_sha_bindings(self) -> None:
        errors = "\n".join(
            validate_plan(
                self.plan,
                self.ownership,
                repository_root=WORKSPACE,
            )
        )
        self.assertIn("actual Git head SHA is required for a publishable plan", errors)
        self.assertIn(
            "actual Git base SHA is required for a publishable plan",
            errors,
        )

    def test_publishable_plan_requires_external_release_authorization(self) -> None:
        errors = "\n".join(
            validate_plan(
                self.plan,
                self.ownership,
                repository_root=WORKSPACE,
                actual_head_sha="2222222222222222222222222222222222222222",
                actual_base_sha="1111111111111111111111111111111111111111",
            )
        )
        self.assertIn("live release-issue authorization is required", errors)

    def test_live_authorization_binds_checks_and_versions(self) -> None:
        authorization = copy.deepcopy(self.authorization)
        authorization["required_checks"] = ["true"]
        authorization["packages"][0]["version"] = "9.9.9"
        errors = self.errors(
            self.plan,
            release_authorization=authorization,
        )
        self.assertIn(
            "required_checks does not match live release-issue authorization",
            errors,
        )
        self.assertIn(
            "publishable packages and versions do not match live "
            "release-issue authorization",
            errors,
        )

    def test_authorization_is_parsed_only_from_explicit_json_contract(self) -> None:
        body = (
            "Human prose is not authorization.\n\n```json\n"
            + json.dumps({"release_authorization": self.authorization})
            + "\n```\n"
        )
        parsed = extract_release_authorization(body)
        self.assertEqual(parsed["authorization"], "publish")
        with self.assertRaises(ValueError):
            extract_release_authorization("Please publish everything.")
        with self.assertRaises(ValueError):
            extract_release_authorization(
                "```json\n"
                + json.dumps(
                    {
                        "authorization": "publish",
                        "repository": self.plan["repository"],
                    }
                )
                + "\n```"
            )

    def test_multiple_live_authorization_blocks_are_rejected(self) -> None:
        block = (
            "```json\n"
            + json.dumps({"release_authorization": self.authorization})
            + "\n```"
        )
        with self.assertRaisesRegex(ValueError, "multiple"):
            extract_release_authorization(block + "\n" + block)

    def test_malformed_authorization_package_entries_are_rejected(self) -> None:
        for malformed in (
            [*self.authorization["packages"], "ignored"],
            [
                *self.authorization["packages"],
                {"name": "extra", "version": "1.0.0", "ignored": True},
            ],
        ):
            with self.subTest(malformed=malformed):
                authorization = copy.deepcopy(self.authorization)
                authorization["packages"] = malformed
                self.assertIn(
                    "authorization packages must contain only exact name/version objects",
                    self.errors(
                        self.plan,
                        release_authorization=authorization,
                    ),
                )

    def test_manifest_must_match_reviewed_path_and_package_name(self) -> None:
        plan = copy.deepcopy(self.plan)
        plan["packages"][0]["manifest_path"] = "foundation-b/Cargo.toml"
        errors = self.errors(plan)
        self.assertIn("manifest_path does not match reviewed ownership", errors)
        self.assertIn("manifest package name", errors)


if __name__ == "__main__":
    unittest.main(verbosity=2)
