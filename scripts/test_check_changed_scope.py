#!/usr/bin/env python3
"""Tests for scripts/check_changed_scope.py."""

from __future__ import annotations

import importlib.util
import sys
import subprocess
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SCRIPT = ROOT / "scripts" / "check_changed_scope.py"
sys.dont_write_bytecode = True
spec = importlib.util.spec_from_file_location("check_changed_scope", SCRIPT)
assert spec and spec.loader
scope_mod = importlib.util.module_from_spec(spec)
sys.modules["check_changed_scope"] = scope_mod
spec.loader.exec_module(scope_mod)


class CheckChangedScopeTests(unittest.TestCase):
    def test_committed_source_to_docs_rename_keeps_both_paths(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)

            def git(*args: str) -> str:
                return subprocess.run(
                    ["git", *args], cwd=root, check=True,
                    text=True, capture_output=True,
                ).stdout.strip()

            git("init", "--quiet")
            git("config", "user.name", "CI Test")
            git("config", "user.email", "ci-test@example.invalid")
            (root / "src").mkdir()
            (root / "src/lib.rs").write_text("pub fn original() {}\n")
            git("add", ".")
            git("commit", "--quiet", "-m", "baseline")
            baseline = git("rev-parse", "HEAD")
            (root / "docs").mkdir()
            git("mv", "src/lib.rs", "docs/lib.rs")
            git("commit", "--quiet", "-m", "rename")
            self.assertEqual(
                scope_mod.git_changed_paths(root, baseline),
                ["docs/lib.rs", "src/lib.rs"],
            )

    def test_rust_library_change_maps_to_one_package(self) -> None:
        scope = self.classify(["crates/comfyui/comfyui-models/src/lib.rs"])
        self.assertEqual(scope["rust_scope"], "changed")
        self.assertEqual(
            scope["rust_packages"],
            [
                "moenarch-comfyui-models",
                "moenarch-comfyui-models-cli",
                "moenarch-comfyui-models-server",
                "moenarch-comfyui-models-wasm",
            ],
        )
        self.assertEqual(scope["progress_scope"], "changed")
        self.assertEqual(scope["progress_packages"], ["moenarch-comfyui-models"])

    def test_cli_adapter_change_maps_to_adapter_package(self) -> None:
        scope = self.classify(["crates/comfyui/comfyui-models-cli/src/main.rs"])
        self.assertEqual(scope["rust_scope"], "changed")
        self.assertEqual(scope["rust_packages"], ["moenarch-comfyui-models-cli"])
        self.assertNotEqual(scope["rust_scope"], "workspace")
        self.assertEqual(scope["progress_packages"], ["moenarch-comfyui-models"])

    def test_server_adapter_change_maps_to_adapter_package(self) -> None:
        scope = self.classify(["crates/comfyui/comfyui-models-server/src/lib.rs"])
        self.assertEqual(scope["rust_packages"], ["moenarch-comfyui-models-server"])
        self.assertNotEqual(scope["rust_scope"], "workspace")

    def test_wasm_binding_change_maps_to_rust_and_bun_package(self) -> None:
        scope = self.classify(["crates/bindings/comfyui-models-wasm/src/lib.rs"])
        self.assertEqual(scope["rust_packages"], ["moenarch-comfyui-models-wasm"])
        self.assertEqual(scope["frontend_scope"], "changed")
        self.assertEqual(
            scope["frontend_commands"],
            [
                "bun run --cwd packages/comfyui-models-wasm build",
                "bun run --cwd packages/comfyui-models-wasm test",
            ],
        )
        self.assertEqual(scope["progress_packages"], ["moenarch-comfyui-models"])

    def test_wasm_package_change_runs_matching_bun_test(self) -> None:
        scope = self.classify(["packages/comfyui-models-wasm/tests/package.test.ts"])
        self.assertEqual(scope["frontend_scope"], "changed")
        self.assertEqual(
            scope["frontend_commands"],
            [
                "bun run --cwd packages/comfyui-models-wasm build",
                "bun run --cwd packages/comfyui-models-wasm test",
            ],
        )
        self.assertEqual(scope["progress_packages"], ["moenarch-comfyui-models"])

    def test_app_change_runs_app_typecheck(self) -> None:
        scope = self.classify(["packages/comfyui-models-app/src/App.tsx"])
        self.assertEqual(scope["frontend_scope"], "changed")
        self.assertEqual(
            scope["frontend_commands"],
            ["bun run --cwd packages/comfyui-models-app typecheck"],
        )
        self.assertEqual(scope["progress_packages"], ["moenarch-comfyui-models"])

    def test_unlisted_app_change_has_an_executable_typecheck(self) -> None:
        scope = self.classify(
            ["packages/audio-analysis-core-app/src/App.tsx"],
            package_json_paths=[
                *self.package_json_paths(),
                "packages/audio-analysis-core-app/package.json",
            ],
        )
        self.assertEqual(
            scope["frontend_commands"],
            ["bun run --cwd packages/audio-analysis-core-app typecheck"],
        )
        self.assertTrue(scope["ci_plan"]["frontend_checks"])

    def test_root_cargo_toml_selects_workspace(self) -> None:
        scope = self.classify(["Cargo.toml"])
        self.assertEqual(scope["rust_scope"], "workspace")
        self.assertIn("Cargo.toml", scope["workspace_reason"])

    def test_root_cargo_lock_does_not_force_progress_all(self) -> None:
        scope = self.classify(["Cargo.lock"])
        self.assertEqual(scope["rust_scope"], "workspace")
        self.assertEqual(scope["progress_scope"], "none")

    def test_docs_only_skips_rust_and_frontend(self) -> None:
        scope = self.classify(["docs/development.md"])
        self.assertTrue(scope["docs_only"])
        self.assertEqual(scope["rust_scope"], "none")
        self.assertEqual(scope["frontend_scope"], "none")

    def test_ui_change_runs_ui_checks(self) -> None:
        scope = self.classify(["packages/video-analysis-ui/src/core/index.tsx"])
        self.assertEqual(scope["frontend_scope"], "changed")
        self.assertEqual(scope["frontend_commands"], ["bun run ui:typecheck", "bun run ui:test:unit"])

    def test_web_change_runs_web_checks(self) -> None:
        scope = self.classify(["prototypes/web/video-analysis-web/src/main.tsx"])
        self.assertEqual(scope["frontend_scope"], "changed")
        self.assertEqual(
            scope["frontend_commands"],
            ["bun run web:typecheck", "bun run web:test:unit", "bun run web:test:api"],
        )

    def test_package_json_selects_frontend_all(self) -> None:
        scope = self.classify(["package.json"])
        self.assertEqual(scope["frontend_scope"], "all")

    def test_package_surface_ui_change_forces_all_progress(self) -> None:
        scope = self.classify(["packages/video-analysis-ui/src/package-surface/OperationWorkbench.tsx"])
        self.assertEqual(scope["frontend_scope"], "changed")
        self.assertEqual(scope["progress_scope"], "all")

    def test_deleted_unknown_crate_path_explains_workspace_scope(self) -> None:
        scope = self.classify(["crates/data/finance-data/src/lib.rs"])
        self.assertEqual(scope["rust_scope"], "workspace")
        self.assertIn("deleted or unknown Rust package path", scope["workspace_reason"])

    def test_documentation_only_ci_plan_avoids_heavy_jobs(self) -> None:
        plan = self.classify(["docs/development.md"])["ci_plan"]
        self.assertFalse(plan["rust_checks"])
        self.assertFalse(plan["storybook_checks"])
        self.assertFalse(plan["browser_e2e_checks"])

    def test_video_ui_ci_plan_selects_frontend_storybook_and_e2e(self) -> None:
        plan = self.classify(["packages/video-analysis-ui/src/core/index.tsx"])["ci_plan"]
        self.assertTrue(plan["frontend_checks"])
        self.assertTrue(plan["storybook_checks"])
        # The Storybook job also runs UI and web E2E with one Playwright setup.
        self.assertFalse(plan["browser_e2e_checks"])

    def test_wasm_ci_plan_is_separate_from_application_frontend(self) -> None:
        plan = self.classify(["packages/comfyui-models-wasm/tests/package.test.ts"])["ci_plan"]
        self.assertTrue(plan["wasm_checks"])
        self.assertFalse(plan["frontend_checks"])

    def test_wasm_without_package_build_script_builds_binding_crate(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            binding = root / "crates/bindings/vision-core-wasm"
            binding.mkdir(parents=True)
            (binding / "Cargo.toml").write_text(
                '[package]\nname = "moenarch-vision-core-wasm"\n', encoding="utf-8"
            )
            scope = scope_mod.classify_changed_files(
                changed_files=["packages/vision-core-wasm/index.js"],
                packages=self.packages(),
                package_json_paths=[
                    *self.package_json_paths(),
                    "packages/vision-core-wasm/package.json",
                ],
                root=root,
            ).to_json()
        self.assertIn(
            "cargo build --package moenarch-vision-core-wasm --target wasm32-unknown-unknown",
            scope["frontend_commands"],
        )

    def test_root_manifest_and_release_changes_select_full_workspace(self) -> None:
        root_plan = self.classify(["Cargo.toml"])["ci_plan"]
        release_plan = self.classify(
            ["docs/repository-split/release-plan.example.json"]
        )["ci_plan"]
        for plan in (root_plan, release_plan):
            self.assertTrue(plan["full_workspace_checks"])
            self.assertFalse(plan["storybook_checks"])
            self.assertFalse(plan["browser_e2e_checks"])

    def test_ui_and_wasm_change_coalesce_heavy_tool_setup(self) -> None:
        plan = self.classify(
            [
                "packages/video-analysis-ui/src/core/index.tsx",
                "packages/comfyui-models-wasm/src/index.ts",
            ]
        )["ci_plan"]
        self.assertTrue(plan["storybook_checks"])
        self.assertFalse(plan["wasm_checks"])
        self.assertFalse(plan["browser_e2e_checks"])

    def test_web_and_wasm_change_coalesce_heavy_tool_setup(self) -> None:
        plan = self.classify(
            [
                "prototypes/web/video-analysis-web/src/main.tsx",
                "packages/comfyui-models-wasm/src/index.ts",
            ]
        )["ci_plan"]
        self.assertTrue(plan["browser_e2e_checks"])
        self.assertFalse(plan["wasm_checks"])
        self.assertFalse(plan["storybook_checks"])

    def test_every_selected_changed_frontend_surface_has_commands(self) -> None:
        fixtures = (
            ["crates/bindings/comfyui-models-wasm/src/lib.rs"],
            ["packages/comfyui-models-wasm/src/index.ts"],
            ["packages/comfyui-models-app/src/App.tsx"],
            ["packages/video-analysis-ui/src/core/index.tsx"],
            ["prototypes/web/video-analysis-web/src/main.tsx"],
        )
        for paths in fixtures:
            with self.subTest(paths=paths):
                scope = self.classify(paths)
                plan = scope["ci_plan"]
                if plan["frontend_checks"] or plan["wasm_checks"]:
                    self.assertTrue(scope["frontend_commands"])

    def test_explicit_full_ci_selects_full_workspace(self) -> None:
        scope = scope_mod.classify_changed_files(
            changed_files=["docs/development.md"],
            packages=self.packages(),
            package_json_paths=self.package_json_paths(),
            full_ci=True,
        ).to_json()
        self.assertTrue(scope["ci_plan"]["full_workspace_checks"])

    def test_repository_split_generator_selects_both_generated_matrices(self) -> None:
        scope = self.classify(["scripts/generate_repository_split_inventory.py"])
        self.assertEqual(
            scope["snapshot_paths"],
            [
                "docs/CONSUMER_RELEASE_MATRIX.md",
                "docs/REPOSITORY_DESTINATION_MATRIX.md",
            ],
        )

    def test_repository_split_matrix_change_selects_its_snapshot(self) -> None:
        scope = self.classify(["docs/REPOSITORY_DESTINATION_MATRIX.md"])
        self.assertEqual(
            scope["snapshot_paths"],
            ["docs/REPOSITORY_DESTINATION_MATRIX.md"],
        )

    def classify(
        self,
        paths: list[str],
        *,
        package_json_paths: list[str] | None = None,
    ) -> dict:
        return scope_mod.classify_changed_files(
            changed_files=paths,
            packages=self.packages(),
            package_json_paths=package_json_paths or self.package_json_paths(),
        ).to_json()

    def packages(self) -> list:
        return [
                scope_mod.CargoPackage("moenarch-video-analysis", ".", "Cargo.toml"),
                scope_mod.CargoPackage(
                    "moenarch-comfyui-models",
                    "crates/comfyui/comfyui-models",
                    "crates/comfyui/comfyui-models/Cargo.toml",
                ),
                scope_mod.CargoPackage(
                    "moenarch-comfyui-models-cli",
                    "crates/comfyui/comfyui-models-cli",
                    "crates/comfyui/comfyui-models-cli/Cargo.toml",
                    ("moenarch-comfyui-models",),
                ),
                scope_mod.CargoPackage(
                    "moenarch-comfyui-models-server",
                    "crates/comfyui/comfyui-models-server",
                    "crates/comfyui/comfyui-models-server/Cargo.toml",
                    ("moenarch-comfyui-models",),
                ),
                scope_mod.CargoPackage(
                    "moenarch-comfyui-models-wasm",
                    "crates/bindings/comfyui-models-wasm",
                    "crates/bindings/comfyui-models-wasm/Cargo.toml",
                    ("moenarch-comfyui-models",),
                ),
            ]

    def package_json_paths(self) -> list[str]:
        return [
                "packages/comfyui-models-wasm/package.json",
                "packages/comfyui-models-app/package.json",
                "packages/video-analysis-ui/package.json",
                "prototypes/web/video-analysis-web/package.json",
            ]


if __name__ == "__main__":
    unittest.main()
