#!/usr/bin/env python3
"""Classify changed files for the fast local check gate."""

from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Iterable

ROOT = Path(__file__).resolve().parents[1]

RUST_WORKSPACE_FILES = {
    "Cargo.toml",
    "Cargo.lock",
    ".cargo/config.toml",
    "src/lib.rs",
}
FRONTEND_WORKSPACE_FILES = {
    "package.json",
    "bun.lock",
}
FULL_WORKSPACE_FILES = {
    "Cargo.toml",
    "Cargo.lock",
    ".cargo/config.toml",
    "package.json",
    "bun.lock",
    "docs/repository-split/package-ownership.json",
    "docs/repository-split/dependency-boundary-baseline.json",
}
FULL_WORKSPACE_PREFIXES = (
    "scripts/fixtures/release_plans/",
)
RELEASE_PLAN_MARKERS = (
    "release-plan",
    "release_plan",
)
ARCHITECTURE_PREFIXES = (
    ".github/workflows/",
    "scripts/",
    "docs/repository-split/",
)
SNAPSHOT_FILES = {
    "docs/CRATE_PROGRESS_LEDGER.md",
    "docs/CRATE_INVENTORY.md",
    "docs/DEPENDENCY_GRAPH.md",
    "docs/PACKAGE_SURFACE_MATRIX.md",
    "docs/CURATED_LANDSCAPE_MATRIX.md",
    "docs/COMFYUI_TYPE_MATRIX.md",
}
PROGRESS_ALL_FILES = {
    "docs/CRATE_PROGRESS_POLICY.md",
    "scripts/audit_crate_progress.py",
    "scripts/audit_package_surfaces.py",
    "scripts/crate_progress_regressions.allow",
}
PROGRESS_ALL_PREFIXES = (
    "packages/video-analysis-ui/src/package-surface/",
)
DOC_PREFIXES = (
    "docs/",
)
DOC_EXACT_FILES = {
    "README.md",
    "CONTRIBUTING.md",
    "CONTEXT.md",
    "AGENTS.md",
    "SECURITY.md",
    "LICENSE-MIT",
    "LICENSE-APACHE",
}
RUST_SOURCE_SUFFIXES = {".rs"}
FRONTEND_SOURCE_SUFFIXES = {".ts", ".tsx", ".js", ".jsx", ".json", ".html", ".css"}
WRAPPER_SUFFIXES = ("-cli", "-server", "-wasm")
PACKAGE_PREFIXES = ("moritzbrantner-", "moenarch-")


@dataclass(frozen=True)
class CargoPackage:
    name: str
    manifest_dir: str
    manifest_path: str
    dependencies: tuple[str, ...] = ()

    @property
    def base(self) -> str:
        return package_base_name(self.name)

    @property
    def domain_dir(self) -> str | None:
        parts = self.manifest_dir.split("/")
        if len(parts) >= 2 and parts[0] == "crates":
            return "/".join(parts[:2])
        return None


@dataclass
class Scope:
    base_ref: str
    changed_files: list[str]
    rust_scope: str = "none"
    rust_packages: list[str] | None = None
    rust_tests: list[str] | None = None
    frontend_scope: str = "none"
    frontend_commands: list[str] | None = None
    progress_scope: str = "none"
    progress_packages: list[str] | None = None
    docs_only: bool = False
    workspace_reason: str | None = None
    rust_reasons: list[str] | None = None
    frontend_reasons: list[str] | None = None
    progress_reasons: list[str] | None = None
    snapshot_paths: list[str] | None = None
    ci_plan: dict[str, bool] | None = None

    def to_json(self) -> dict:
        return {
            "base_ref": self.base_ref,
            "changed_files": self.changed_files,
            "rust_scope": self.rust_scope,
            "rust_packages": self.rust_packages or [],
            "rust_tests": self.rust_tests or [],
            "frontend_scope": self.frontend_scope,
            "frontend_commands": self.frontend_commands or [],
            "progress_scope": self.progress_scope,
            "progress_packages": self.progress_packages or [],
            "docs_only": self.docs_only,
            "workspace_reason": self.workspace_reason,
            "rust_reasons": self.rust_reasons or [],
            "frontend_reasons": self.frontend_reasons or [],
            "progress_reasons": self.progress_reasons or [],
            "snapshot_paths": self.snapshot_paths or [],
            "ci_plan": self.ci_plan or {},
        }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--base", default=os.environ.get("BASE_REF", "origin/main"))
    parser.add_argument(
        "--paths-file",
        help="read changed paths from a file instead of git; use - for stdin",
    )
    parser.add_argument("--full-ci", action="store_true")
    parser.add_argument("--github-output")
    args = parser.parse_args()

    root = ROOT
    changed_files = read_paths_file(args.paths_file) if args.paths_file else git_changed_paths(root, args.base)
    packages = cargo_packages(root)
    package_json_paths = sorted(path.relative_to(root).as_posix() for path in (root / "packages").glob("*/package.json"))
    package_json_paths.extend(
        path.relative_to(root).as_posix() for path in (root / "prototypes" / "web").glob("*/package.json")
    )
    scope = classify_changed_files(
        changed_files=changed_files,
        packages=packages,
        package_json_paths=package_json_paths,
        base_ref=args.base,
        root=root,
        full_ci=args.full_ci,
    )
    payload = scope.to_json()
    if args.github_output:
        write_github_outputs(Path(args.github_output), payload)
    print(json.dumps(payload, indent=2, sort_keys=True))
    return 0


def classify_changed_files(
    *,
    changed_files: Iterable[str],
    packages: list[CargoPackage],
    package_json_paths: Iterable[str],
    base_ref: str = "origin/main",
    root: Path = ROOT,
    full_ci: bool = False,
) -> Scope:
    paths = sorted(set(normalize_path(path) for path in changed_files if normalize_path(path)))
    package_json_set = set(package_json_paths)
    scope = Scope(base_ref=base_ref, changed_files=paths)
    rust_packages: set[str] = set()
    rust_tests: set[str] = set()
    frontend_commands: set[str] = set()
    progress_packages: set[str] = set()
    rust_reasons: list[str] = []
    frontend_reasons: list[str] = []
    progress_reasons: list[str] = []
    snapshot_paths: set[str] = set()
    docs_only = bool(paths)

    for path in paths:
        if not is_docs_path(path):
            docs_only = False

        if path in RUST_WORKSPACE_FILES:
            scope.rust_scope = "workspace"
            reason = f"{path} affects the Rust workspace"
            scope.workspace_reason = scope.workspace_reason or reason
            rust_reasons.append(reason)
            snapshot_paths.add("docs/DEPENDENCY_GRAPH.md")
        elif path.startswith("tests/") and path.endswith(".rs"):
            rust_tests.add(path)
            rust_reasons.append(f"{path} is a root integration test")
        elif path.startswith("crates/") or path.startswith("prototypes/rust/") or path.startswith("src/"):
            package = package_for_path(path, packages)
            if package:
                rust_packages.add(package.name)
                rust_reasons.append(f"{path} maps to {package.name}")
                if package.manifest_dir.startswith("crates/bindings/"):
                    package_dir = f"packages/{Path(package.manifest_dir).name}"
                    if f"{package_dir}/package.json" in package_json_set:
                        frontend_commands.update(
                            frontend_package_commands(package_dir, root)
                        )
                        frontend_reasons.append(
                            f"{path} maps to the {package_dir} WASM package"
                        )
                surface = surface_package_for_path(path, packages)
                if surface:
                    progress_packages.add(surface.name)
                    progress_reasons.append(f"{path} can affect {surface.name} package-surface maturity")
                    snapshot_paths.update({"docs/CRATE_PROGRESS_LEDGER.md", "docs/PACKAGE_SURFACE_MATRIX.md"})
            elif Path(path).suffix in RUST_SOURCE_SUFFIXES or path.endswith("Cargo.toml"):
                scope.rust_scope = "workspace"
                reason = f"{path} is a deleted or unknown Rust package path"
                scope.workspace_reason = scope.workspace_reason or reason
                rust_reasons.append(reason)

        if path in FRONTEND_WORKSPACE_FILES:
            scope.frontend_scope = "all"
            frontend_reasons.append(f"{path} affects the frontend workspace")
        elif path.startswith("packages/video-analysis-ui/"):
            frontend_commands.update({"bun run ui:typecheck", "bun run ui:test:unit"})
            frontend_reasons.append(f"{path} affects packages/video-analysis-ui")
        elif path.startswith("prototypes/web/video-analysis-web/"):
            frontend_commands.update({"bun run web:typecheck", "bun run web:test:unit", "bun run web:test:api"})
            frontend_reasons.append(f"{path} affects prototypes/web/video-analysis-web")
        elif path.startswith("packages/"):
            package_dir = frontend_package_dir(path, package_json_set)
            if package_dir and package_dir.endswith(("-app", "-wasm")):
                frontend_commands.update(frontend_package_commands(package_dir, root))
                frontend_reasons.append(f"{path} affects {package_dir}")

        if path in PROGRESS_ALL_FILES or any(path.startswith(prefix) for prefix in PROGRESS_ALL_PREFIXES):
            scope.progress_scope = "all"
            progress_reasons.append(f"{path} affects all package-surface maturity scoring")
            snapshot_paths.update({"docs/CRATE_PROGRESS_LEDGER.md", "docs/PACKAGE_SURFACE_MATRIX.md"})
        elif path in SNAPSHOT_FILES:
            snapshot_paths.add(path)
            progress_reasons.append(f"{path} is checked by generated snapshot validation")
        elif path.startswith("packages/"):
            surface = surface_package_for_frontend_path(path, packages)
            if surface:
                progress_packages.add(surface.name)
                progress_reasons.append(f"{path} can affect {surface.name} package-surface maturity")
                snapshot_paths.update({"docs/CRATE_PROGRESS_LEDGER.md", "docs/PACKAGE_SURFACE_MATRIX.md"})

        if path == "scripts/generate_dependency_chart.py":
            snapshot_paths.add("docs/DEPENDENCY_GRAPH.md")
        elif path == "scripts/audit_package_surfaces.py":
            snapshot_paths.add("docs/PACKAGE_SURFACE_MATRIX.md")

    if scope.rust_scope != "workspace":
        if rust_packages or rust_tests:
            scope.rust_scope = "changed"
        else:
            scope.rust_scope = "none"

    if scope.frontend_scope != "all":
        scope.frontend_scope = "changed" if frontend_commands else "none"

    if scope.progress_scope != "all":
        scope.progress_scope = "changed" if progress_packages else "none"

    scope.rust_packages = sorted(rust_packages)
    scope.rust_tests = sorted(rust_tests)
    scope.frontend_commands = order_frontend_commands(frontend_commands)
    scope.progress_packages = sorted(progress_packages)
    scope.docs_only = docs_only and scope.rust_scope == "none" and scope.frontend_scope == "none"
    scope.rust_reasons = dedupe(rust_reasons)
    scope.frontend_reasons = dedupe(frontend_reasons)
    scope.progress_reasons = dedupe(progress_reasons)
    scope.snapshot_paths = sorted(snapshot_paths)
    if scope.rust_scope == "changed":
        scope.rust_packages = reverse_dependency_closure(rust_packages, packages)
    scope.ci_plan = build_ci_plan(paths, scope, full_ci=full_ci)
    return scope


def cargo_packages(root: Path) -> list[CargoPackage]:
    metadata = run_json(root, ["cargo", "metadata", "--no-deps", "--format-version", "1"])
    packages: list[CargoPackage] = []
    for package in metadata["packages"]:
        manifest_path = Path(package["manifest_path"]).resolve()
        manifest_dir = manifest_path.parent
        packages.append(
            CargoPackage(
                name=package["name"],
                manifest_dir=manifest_dir.relative_to(root).as_posix(),
                manifest_path=manifest_path.relative_to(root).as_posix(),
                dependencies=tuple(
                    sorted(
                        {
                            str(dependency.get("name") or "")
                            for dependency in package.get("dependencies") or []
                            if dependency.get("name")
                        }
                    )
                ),
            )
        )
    packages.sort(key=lambda package: len(package.manifest_dir), reverse=True)
    return packages


def git_changed_paths(root: Path, base: str) -> list[str]:
    merge_base = git_output(root, ["git", "merge-base", base, "HEAD"])
    commands = [
        ["git", "diff", "--name-only", f"{merge_base}...HEAD"],
        ["git", "diff", "--name-only"],
        ["git", "diff", "--name-only", "--cached"],
    ]
    paths: set[str] = set()
    for command in commands:
        output = git_output(root, command)
        paths.update(line.strip() for line in output.splitlines() if line.strip())
    return sorted(paths)


def reverse_dependency_closure(
    changed_packages: set[str],
    packages: list[CargoPackage],
) -> list[str]:
    selected = set(changed_packages)
    while True:
        dependents = {
            package.name
            for package in packages
            if package.name not in selected
            and any(dependency in selected for dependency in package.dependencies)
        }
        if not dependents:
            return sorted(selected)
        selected.update(dependents)


def build_ci_plan(paths: list[str], scope: Scope, *, full_ci: bool) -> dict[str, bool]:
    release_change = any(
        marker in path.lower() for path in paths for marker in RELEASE_PLAN_MARKERS
    )
    ownership_change = any(
        path in FULL_WORKSPACE_FILES
        or any(path.startswith(prefix) for prefix in FULL_WORKSPACE_PREFIXES)
        for path in paths
    )
    full_workspace = full_ci or release_change or ownership_change
    ui_change = any(path.startswith("packages/video-analysis-ui/") for path in paths)
    web_change = any(path.startswith("prototypes/web/video-analysis-web/") for path in paths)
    wasm_change = any(
        (
            path.startswith("packages/")
            and "-wasm/" in path
        )
        or path.startswith("crates/bindings/")
        for path in paths
    )
    application_frontend_change = any(
        (path.startswith("packages/") and "-app/" in path)
        or path.startswith("packages/video-analysis-ui/")
        or path.startswith("prototypes/web/")
        for path in paths
    )
    architecture = any(
        path in FULL_WORKSPACE_FILES
        or any(path.startswith(prefix) for prefix in ARCHITECTURE_PREFIXES)
        for path in paths
    )
    return {
        "architecture_checks": architecture,
        "rust_checks": scope.rust_scope != "none" and not full_workspace,
        "frontend_checks": application_frontend_change and not full_workspace,
        # UI validation coalesces its WASM, browser E2E, and Storybook work so the
        # pinned browser and wasm-pack setup is paid once. Full validation does
        # the same inside the full-workspace job.
        "wasm_checks": wasm_change and not (full_workspace or ui_change),
        "storybook_checks": ui_change and not full_workspace,
        "browser_e2e_checks": web_change and not (full_workspace or ui_change),
        "full_workspace_checks": full_workspace,
    }


def write_github_outputs(path: Path, payload: dict) -> None:
    plan = payload.get("ci_plan") or {}
    lines = [
        f"{name}={'true' if enabled else 'false'}"
        for name, enabled in sorted(plan.items())
    ]
    lines.extend(
        [
            "rust_scope=" + str(payload.get("rust_scope") or "none"),
            "rust_packages_json=" + json.dumps(payload.get("rust_packages") or [], separators=(",", ":")),
            "frontend_commands_json="
            + json.dumps(payload.get("frontend_commands") or [], separators=(",", ":")),
        ]
    )
    with path.open("a", encoding="utf-8") as handle:
        handle.write("\n".join(lines) + "\n")


def package_for_path(path: str, packages: list[CargoPackage]) -> CargoPackage | None:
    for package in packages:
        if package.manifest_dir == ".":
            continue
        if path == package.manifest_path or path.startswith(package.manifest_dir + "/"):
            return package
    if path.startswith("src/"):
        return next((package for package in packages if package.manifest_dir == "."), None)
    return None


def surface_package_for_path(path: str, packages: list[CargoPackage]) -> CargoPackage | None:
    package = package_for_path(path, packages)
    if not package:
        return None
    if package.name.endswith(WRAPPER_SUFFIXES):
        base = package.name.removesuffix("-cli").removesuffix("-server").removesuffix("-wasm")
        return next((candidate for candidate in packages if candidate.name == base), None)
    return package


def surface_package_for_frontend_path(path: str, packages: list[CargoPackage]) -> CargoPackage | None:
    parts = path.split("/")
    if len(parts) < 2 or parts[0] != "packages":
        return None
    package_dir = parts[1]
    for suffix in ("-wasm", "-app"):
        if package_dir.endswith(suffix):
            base = package_dir.removesuffix(suffix)
            return next((package for package in packages if package.base == base), None)
    return None


def frontend_package_dir(path: str, package_json_paths: set[str]) -> str | None:
    parts = path.split("/")
    if len(parts) < 2:
        return None
    package_dir = "/".join(parts[:2])
    return package_dir if f"{package_dir}/package.json" in package_json_paths else None


def frontend_package_commands(package_dir: str, root: Path) -> set[str]:
    package_json = root / package_dir / "package.json"
    scripts: set[str] = set()
    if package_json.is_file():
        payload = json.loads(package_json.read_text(encoding="utf-8"))
        scripts = set((payload.get("scripts") or {}).keys())

    commands: set[str] = set()
    if package_dir.endswith("-app"):
        # App packages consistently expose typecheck. Keep the fallback for
        # deleted paths and fixture roots where package.json is unavailable.
        if not scripts or "typecheck" in scripts:
            commands.add(f"bun run --cwd {package_dir} typecheck")
    elif package_dir.endswith("-wasm"):
        # A changed WASM surface must compile when it exposes a build script;
        # tests alone do not prove that the distributable binding still builds.
        if "build" in scripts:
            commands.add(f"bun run --cwd {package_dir} build")
        if not scripts or "test" in scripts:
            commands.add(f"bun run --cwd {package_dir} test")
    return commands


def root_package_name(packages: list[CargoPackage]) -> str:
    root_package = next((package for package in packages if package.manifest_dir == "."), None)
    return root_package.name if root_package else "moenarch-video-analysis"


def package_base_name(package_name: str) -> str:
    for prefix in PACKAGE_PREFIXES:
        if package_name.startswith(prefix):
            return package_name.removeprefix(prefix)
    return package_name


def is_docs_path(path: str) -> bool:
    return path in DOC_EXACT_FILES or path.startswith(DOC_PREFIXES)


def normalize_path(path: str) -> str:
    return path.strip().removeprefix("./")


def dedupe(values: Iterable[str]) -> list[str]:
    seen: set[str] = set()
    result: list[str] = []
    for value in values:
        if value not in seen:
            seen.add(value)
            result.append(value)
    return result


def order_frontend_commands(commands: Iterable[str]) -> list[str]:
    preferred = [
        "bun run ui:typecheck",
        "bun run ui:test:unit",
        "bun run web:typecheck",
        "bun run web:test:unit",
        "bun run web:test:api",
    ]
    command_set = set(commands)
    ordered = [command for command in preferred if command in command_set]
    ordered.extend(sorted(command for command in command_set if command not in preferred))
    return ordered


def read_paths_file(path: str | None) -> list[str]:
    if not path:
        return []
    if path == "-":
        return [line.strip() for line in sys.stdin if line.strip()]
    return [line.strip() for line in Path(path).read_text(encoding="utf-8").splitlines() if line.strip()]


def run_json(root: Path, command: list[str]) -> dict:
    return json.loads(subprocess.check_output(command, cwd=root, text=True))


def git_output(root: Path, command: list[str]) -> str:
    return subprocess.check_output(command, cwd=root, text=True).strip()


if __name__ == "__main__":
    raise SystemExit(main())
