#!/usr/bin/env python3
"""Tests for scripts/check_changed_scope.py."""

from __future__ import annotations

import importlib.util
import sys
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
    def test_rust_library_change_maps_to_one_package(self) -> None:
        scope = self.classify(["crates/text/text-core/src/lib.rs"])
        self.assertEqual(scope["rust_scope"], "changed")
        self.assertEqual(scope["rust_packages"], ["moenarch-text-core"])
        self.assertEqual(scope["progress_scope"], "changed")
        self.assertEqual(scope["progress_packages"], ["moenarch-text-core"])

    def test_cli_adapter_change_maps_to_adapter_package(self) -> None:
        scope = self.classify(["crates/text/text-core-cli/src/main.rs"])
        self.assertEqual(scope["rust_scope"], "changed")
        self.assertEqual(scope["rust_packages"], ["moenarch-text-core-cli"])
        self.assertNotEqual(scope["rust_scope"], "workspace")
        self.assertEqual(scope["progress_packages"], ["moenarch-text-core"])

    def test_server_adapter_change_maps_to_adapter_package(self) -> None:
        scope = self.classify(["crates/text/text-core-server/src/lib.rs"])
        self.assertEqual(scope["rust_packages"], ["moenarch-text-core-server"])
        self.assertNotEqual(scope["rust_scope"], "workspace")

    def test_wasm_binding_change_maps_to_rust_and_bun_package(self) -> None:
        scope = self.classify(["crates/bindings/text-core-wasm/src/lib.rs"])
        self.assertEqual(scope["rust_packages"], ["moenarch-text-core-wasm"])
        self.assertEqual(scope["frontend_scope"], "none")
        self.assertEqual(scope["progress_packages"], ["moenarch-text-core"])

    def test_wasm_package_change_runs_matching_bun_test(self) -> None:
        scope = self.classify(["packages/text-core-wasm/tests/package.test.ts"])
        self.assertEqual(scope["frontend_scope"], "changed")
        self.assertEqual(scope["frontend_commands"], ["bun run text-wasm:test:all"])
        self.assertEqual(scope["progress_packages"], ["moenarch-text-core"])

    def test_text_app_change_runs_text_app_typecheck(self) -> None:
        scope = self.classify(["packages/text-core-app/src/App.tsx"])
        self.assertEqual(scope["frontend_scope"], "changed")
        self.assertEqual(scope["frontend_commands"], ["bun run text-app:typecheck"])
        self.assertEqual(scope["progress_packages"], ["moenarch-text-core"])

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

    def classify(self, paths: list[str]) -> dict:
        return scope_mod.classify_changed_files(
            changed_files=paths,
            packages=[
                scope_mod.CargoPackage("moenarch-video-analysis", ".", "Cargo.toml"),
                scope_mod.CargoPackage(
                    "moenarch-text-core",
                    "crates/text/text-core",
                    "crates/text/text-core/Cargo.toml",
                ),
                scope_mod.CargoPackage(
                    "moenarch-text-core-cli",
                    "crates/text/text-core-cli",
                    "crates/text/text-core-cli/Cargo.toml",
                ),
                scope_mod.CargoPackage(
                    "moenarch-text-core-server",
                    "crates/text/text-core-server",
                    "crates/text/text-core-server/Cargo.toml",
                ),
                scope_mod.CargoPackage(
                    "moenarch-text-core-wasm",
                    "crates/bindings/text-core-wasm",
                    "crates/bindings/text-core-wasm/Cargo.toml",
                ),
            ],
            package_json_paths=[
                "packages/text-core-wasm/package.json",
                "packages/text-core-app/package.json",
                "packages/video-analysis-ui/package.json",
                "prototypes/web/video-analysis-web/package.json",
            ],
        ).to_json()


if __name__ == "__main__":
    unittest.main()
