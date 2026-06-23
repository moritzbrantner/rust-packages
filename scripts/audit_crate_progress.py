#!/usr/bin/env python3
"""Audit crate progress, generate the progress ledger, and detect drift."""

from __future__ import annotations

import argparse
import json
import os
import re
import subprocess
import sys
import tempfile
from dataclasses import dataclass, field
from datetime import date
from pathlib import Path

DEFAULT_ROOT = Path(__file__).resolve().parents[1]
LEDGER_PATH = Path("docs/CRATE_PROGRESS_LEDGER.md")
ALLOW_PATH = Path("scripts/crate_progress_regressions.allow")
EXCLUDED_LIBRARY_CRATES = {
    "moritzbrantner-audio-analysis-test-support",
    "moenarch-audio-analysis-test-support",
    "moritzbrantner-runtime-core",
    "moenarch-runtime-core",
    "moritzbrantner-runtime-onnx",
    "moenarch-runtime-onnx",
    "moritzbrantner-video-analysis-test-support",
    "moenarch-video-analysis-test-support",
}
WRAPPER_SUFFIXES = ("-cli", "-server", "-wasm")
PACKAGE_PREFIXES = ("moritzbrantner-", "moenarch-")
DEFAULT_PACKAGE_PREFIX = "moenarch-"
SCAFFOLD_STRINGS = [
    "A deterministic summary or execution plan owned by the Rust library",
    "JSON request metadata for the operation-specific package surface",
]
LEVEL_RANK = {
    "L0 Scaffolded": 0,
    "L1 Discoverable": 1,
    "L2 Executable": 2,
    "L3 Transport Complete": 3,
    "L4 Usable": 4,
}
SHARED_EXACT_PATHS = {
    ".github/workflows/workspace-ci.yml",
    "Cargo.toml",
    "Cargo.lock",
    "package.json",
    "bun.lock",
    "docs/CRATE_PROGRESS_POLICY.md",
    "docs/CRATE_PROGRESS_LEDGER.md",
    "docs/PACKAGE_SURFACE_MATRIX.md",
    "docs/runtime-surfaces.md",
    "docs/API_CONTRACTS.md",
    "scripts/audit_crate_progress.py",
    "scripts/audit_package_surfaces.py",
    "scripts/check-fast.sh",
    "scripts/check-preflight.sh",
    "scripts/check.sh",
    "scripts/generated_snapshots.allow",
    "scripts/crate_progress_regressions.allow",
}
SHARED_PREFIXES = (
    ".github/workflows/",
    "crates/runtime/runtime-core/",
    "packages/video-analysis-ui/src/package-surface/",
)
DEBUG_KEYWORDS = (
    "describe",
    "plan",
    "inspect",
    "validate",
    "catalog",
    "schema",
    "models",
    "defaults",
    "reference",
    "inventory",
    "providers",
)


@dataclass(frozen=True)
class LibraryPackage:
    name: str
    manifest_path: Path

    @property
    def base(self) -> str:
        return companion_package_base_name(self.name)

    @property
    def relative_manifest(self) -> str:
        return self.manifest_path.as_posix()

    @property
    def relative_dir(self) -> str:
        return self.manifest_path.parent.as_posix()

    @property
    def domain(self) -> str:
        parts = self.manifest_path.parts
        if len(parts) >= 3 and parts[0] == "crates":
            return parts[1]
        return "unknown"


@dataclass
class ProgressRecord:
    library: str
    domain: str
    path: str
    level: str
    score: int
    workflow_operations: list[str]
    debug_operations: list[str]
    parity: dict[str, bool]
    readme_quickstart: bool
    primary_workflow_test: bool
    app_default_status: str
    known_gaps: list[str] = field(default_factory=list)

    def to_json(self) -> dict:
        return {
            "library": self.library,
            "domain": self.domain,
            "path": self.path,
            "level": self.level,
            "score": self.score,
            "workflow_operations": self.workflow_operations,
            "debug_operations": self.debug_operations,
            "parity": self.parity,
            "readme_quickstart": self.readme_quickstart,
            "primary_workflow_test": self.primary_workflow_test,
            "app_default_status": self.app_default_status,
            "known_gaps": self.known_gaps,
        }


@dataclass(frozen=True)
class RegressionAllow:
    crate: str
    metric: str
    expires: date
    reason: str


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true", help="rewrite docs/CRATE_PROGRESS_LEDGER.md")
    parser.add_argument("--check", action="store_true", help="fail if docs/CRATE_PROGRESS_LEDGER.md differs")
    parser.add_argument("--changed", action="store_true", help="fail if touched crates regressed from --base")
    parser.add_argument("--compare-base", help="compare all audited crates against this base ref")
    parser.add_argument("--base", default="origin/main", help="base ref for --changed")
    parser.add_argument("--only", help="limit audit to one library crate")
    parser.add_argument("--root", default=str(DEFAULT_ROOT), help=argparse.SUPPRESS)
    parser.add_argument("--dump-json", action="store_true", help=argparse.SUPPRESS)
    args = parser.parse_args()

    selected_modes = [args.write, args.check, args.changed, bool(args.compare_base), args.dump_json]
    if sum(bool(mode) for mode in selected_modes) > 1:
        parser.error("--write, --check, --changed, --compare-base, and --dump-json are mutually exclusive")

    root = Path(args.root).resolve()
    if args.changed:
        return changed_regression_check(root, args.base, args.only)
    if args.compare_base:
        return compare_base(root, args.compare_base, args.only)

    records = audit_records(root, args.only)
    if args.dump_json:
        print(json.dumps([record.to_json() for record in records], indent=2, sort_keys=True))
        return 0

    content = render_ledger(records)
    ledger_path = root / LEDGER_PATH
    if args.write:
        ledger_path.write_text(content, encoding="utf-8")
        return 0
    if args.check:
        existing = ledger_path.read_text(encoding="utf-8")
        if existing != content:
            print(
                f"{LEDGER_PATH.as_posix()} is out of date; run scripts/audit_crate_progress.py --write",
                file=sys.stderr,
            )
            return 1
        return 0

    print(content, end="")
    return 0


def audit_records(root: Path, only: str | None = None) -> list[ProgressRecord]:
    packages = library_packages(root, only)
    return [audit_package(root, package) for package in packages]


def library_packages(root: Path, only: str | None = None) -> list[LibraryPackage]:
    metadata = run_json(root, ["cargo", "metadata", "--format-version", "1", "--no-deps"])
    packages: list[LibraryPackage] = []
    for package in metadata["packages"]:
        name = package["name"]
        manifest = Path(package["manifest_path"])
        relative = manifest.relative_to(root)
        if not str(relative).startswith("crates/"):
            continue
        if str(relative).startswith("crates/bindings/"):
            continue
        if name in EXCLUDED_LIBRARY_CRATES or name.endswith(WRAPPER_SUFFIXES):
            continue
        if not any("lib" in target.get("kind", []) for target in package["targets"]):
            continue
        if only and name != only:
            continue
        packages.append(LibraryPackage(name=name, manifest_path=relative))
    packages.sort(key=lambda package: package.name)
    return packages


def audit_package(root: Path, package: LibraryPackage) -> ProgressRecord:
    gaps: list[str] = []
    operations = read_cli_operations(root, package)
    operation_ids = [operation.get("id") for operation in operations if isinstance(operation.get("id"), str)]
    app_workflow_group = app_workflow_operations(root, package)
    workflow_operations = [
        operation_id
        for operation_id in operation_ids
        if operation_id != "describe"
        and (
            (
                operation_id in app_workflow_group
                and not operation_curation_role(find_operation(operations, operation_id))
            )
            or classify_operation(find_operation(operations, operation_id)) == "workflow"
        )
    ]
    debug_operations = [
        operation_id
        for operation_id in operation_ids
        if operation_id == "describe"
        or (
            operation_id not in app_workflow_group
            and classify_operation(find_operation(operations, operation_id)) == "debug"
        )
    ]

    source_path = root / package.manifest_path.parent / "src" / "surface.rs"
    surface_source = read_text(source_path)
    surface_present = source_path.is_file() and "package_surface" in surface_source
    discoverable = surface_present and "describe" in operation_ids and len([op for op in operation_ids if op != "describe"]) >= 2
    metadata_complete = bool(operations) and all(operation_metadata_complete(operation) for operation in operations)
    no_scaffold = not any(scaffold in surface_source for scaffold in SCAFFOLD_STRINGS)

    parity = companion_parity(root, package)
    app_status = app_default_status(root, package, operations, operation_ids, workflow_operations)
    readme = readme_has_quickstart(root, package, operation_ids)
    tests = has_primary_workflow_test(root, package, workflow_operations)

    if not surface_present:
        gaps.append("missing library-owned surface module")
    if not discoverable:
        gaps.append("surface is not discoverable")
    if not metadata_complete:
        gaps.append("operation metadata is incomplete")
    if not no_scaffold:
        gaps.append("surface still contains scaffold text")
    for key, present in parity.items():
        if not present:
            gaps.append(f"missing {key}")
    if app_status != "workflow default":
        gaps.append(app_status)
    if not readme:
        gaps.append("README missing package-surface quickstart")
    if not tests:
        gaps.append("primary workflow test not found")
    if not workflow_operations:
        gaps.append("no workflow operation classified")

    score = progress_score(
        surface_present=surface_present,
        discoverable=discoverable,
        metadata_complete=metadata_complete,
        no_scaffold=no_scaffold,
        parity=parity,
        app_status=app_status,
        readme=readme,
        tests=tests,
        has_workflow=bool(workflow_operations),
    )
    level = maturity_level(
        surface_present=surface_present,
        discoverable=discoverable,
        metadata_complete=metadata_complete,
        no_scaffold=no_scaffold,
        parity=parity,
        app_status=app_status,
        readme=readme,
        tests=tests,
        has_workflow=bool(workflow_operations),
    )

    return ProgressRecord(
        library=package.name,
        domain=package.domain,
        path=package.relative_dir,
        level=level,
        score=score,
        workflow_operations=workflow_operations,
        debug_operations=debug_operations,
        parity=parity,
        readme_quickstart=readme,
        primary_workflow_test=tests,
        app_default_status=app_status,
        known_gaps=gaps,
    )


def read_cli_operations(root: Path, package: LibraryPackage) -> list[dict]:
    completed = subprocess.run(
        [
            "cargo",
            "run",
            "--quiet",
            "-p",
            public_package_name(f"{package.base}-cli", package.name),
            "--",
            "operations",
            "--json",
        ],
        cwd=root,
        env=cargo_env(),
        text=True,
        capture_output=True,
    )
    if completed.returncode != 0:
        return []
    try:
        payload = json.loads(completed.stdout)
    except json.JSONDecodeError:
        return []
    if not isinstance(payload, list):
        return []
    return [operation for operation in payload if isinstance(operation, dict)]


def operation_metadata_complete(operation: dict) -> bool:
    if not isinstance(operation.get("id"), str) or not operation.get("id", "").strip():
        return False
    if not isinstance(operation.get("name"), str) or not operation.get("name", "").strip():
        return False
    if not isinstance(operation.get("description"), str) or not operation.get("description", "").strip():
        return False
    for field in ["inputSchema", "outputSchema", "exampleRequest"]:
        if not isinstance(operation.get(field), dict):
            return False
    for field in ["wasmSupported", "serverSupported"]:
        if not isinstance(operation.get(field), bool):
            return False
    return True


def companion_parity(root: Path, package: LibraryPackage) -> dict[str, bool]:
    crate_parent = root / package.manifest_path.parent.parent
    return {
        "cli": (crate_parent / f"{package.base}-cli" / "Cargo.toml").is_file(),
        "server": (crate_parent / f"{package.base}-server" / "Cargo.toml").is_file(),
        "rust_wasm": (root / "crates" / "bindings" / f"{package.base}-wasm" / "Cargo.toml").is_file(),
        "bun_wasm": (root / "packages" / f"{package.base}-wasm" / "package.json").is_file(),
        "app": (root / "packages" / f"{package.base}-app" / "package.json").is_file(),
    }


def app_default_status(
    root: Path,
    package: LibraryPackage,
    operations: list[dict],
    operation_ids: list[str],
    workflow_operations: list[str],
) -> str:
    app_path = root / "packages" / f"{package.base}-app" / "src" / "App.tsx"
    text = read_text(app_path)
    if not text:
        return "app source missing"
    default = string_property(text, "defaultOperation")
    if not default:
        if workflow_operations and any(
            operation_curation_primary(operation) and operation_curation_role(operation) == "workflow"
            for operation in operations
        ):
            return "rust primary workflow"
        return "app default missing"
    if default not in operation_ids:
        return "app default not exposed"
    workflow_group = operations_for_group_label(text, "Workflow")
    debug_group = operations_for_group_label(text, "Debug")
    has_rust_curation = all(
        operation_curation_role(operation) in {"workflow", "debug", "support"}
        for operation in operations
    )
    if not workflow_group and not has_rust_curation:
        return "Workflow group missing"
    if not debug_group and not has_rust_curation:
        return "Debug group missing"
    if default == "describe":
        return "app defaults to describe"
    if workflow_operations and workflow_group and default not in workflow_group:
        return "app default outside Workflow group"
    featured = array_property(text, "featuredOperations")
    if featured and featured[0] == "describe" and workflow_operations:
        return "featured operations start with describe"
    return "workflow default"


def app_workflow_operations(root: Path, package: LibraryPackage) -> set[str]:
    app_path = root / "packages" / f"{package.base}-app" / "src" / "App.tsx"
    return set(operations_for_group_label(read_text(app_path), "Workflow"))


def readme_has_quickstart(root: Path, package: LibraryPackage, operation_ids: list[str]) -> bool:
    text = read_text(root / package.manifest_path.parent / "README.md").lower()
    if not text:
        return False
    if "package surface" in text and ("workflow operation" in text or "primary workflow" in text):
        return True
    if "run_surface_operation" in text or "package_surface" in text:
        return True
    return any(operation_id.lower() in text for operation_id in operation_ids if operation_id != "describe")


def has_primary_workflow_test(root: Path, package: LibraryPackage, workflow_operations: list[str]) -> bool:
    search_roots = [
        root / package.manifest_path.parent / "tests",
        root / package.manifest_path.parent / "src",
        root / package.manifest_path.parent.parent / f"{package.base}-cli" / "tests",
        root / package.manifest_path.parent.parent / f"{package.base}-server" / "tests",
        root / "crates" / "bindings" / f"{package.base}-wasm" / "src",
        root / "tests",
    ]
    needles = ["run_surface_operation", *workflow_operations[:3]]
    for search_root in search_roots:
        if not search_root.exists():
            continue
        for path in search_root.rglob("*"):
            if path.suffix not in {".rs", ".ts", ".tsx"} or not path.is_file():
                continue
            text = read_text(path)
            if package.base.replace("-", "_") in text and any(needle in text for needle in needles):
                return True
            if any(needle in text for needle in workflow_operations[:3]):
                return True
    return False


def progress_score(
    *,
    surface_present: bool,
    discoverable: bool,
    metadata_complete: bool,
    no_scaffold: bool,
    parity: dict[str, bool],
    app_status: str,
    readme: bool,
    tests: bool,
    has_workflow: bool = True,
) -> int:
    score = 0
    if surface_present:
        score += 10
    if discoverable:
        score += 15
    if metadata_complete:
        score += 15
    if no_scaffold:
        score += 10
    score += sum(4 for present in parity.values() if present)
    if app_status == "workflow default" and has_workflow:
        score += 15
    elif "Workflow group" not in app_status and "Debug group" not in app_status and "missing" not in app_status:
        score += 5
    if readme:
        score += 5
    if tests:
        score += 10
    return score


def maturity_level(
    *,
    surface_present: bool,
    discoverable: bool,
    metadata_complete: bool,
    no_scaffold: bool,
    parity: dict[str, bool],
    app_status: str,
    readme: bool,
    tests: bool,
    has_workflow: bool = True,
) -> str:
    if not surface_present or not discoverable:
        return "L0 Scaffolded"
    if not metadata_complete or not no_scaffold:
        return "L1 Discoverable"
    if not all(parity.values()):
        return "L2 Executable"
    if app_status != "workflow default" or not readme or not tests or not has_workflow:
        return "L3 Transport Complete"
    return "L4 Usable"


def render_ledger(records: list[ProgressRecord]) -> str:
    lines = [
        "# Crate Progress Ledger",
        "",
        "Generated crate maturity audit. Regenerate with `python3 scripts/audit_crate_progress.py --write`.",
        "",
        "| Library | Domain | Path | Maturity level | Score | Workflow operations | Debug operations | CLI | Server | Rust WASM | Bun WASM | App | README quickstart | Primary workflow test | App default | Known gaps |",
        "|---|---|---|---|---:|---|---|---|---|---|---|---|---|---|---|---|",
    ]
    for record in records:
        lines.append(
            "| "
            + " | ".join(
                [
                    tick(record.library),
                    record.domain,
                    tick(record.path),
                    record.level,
                    str(record.score),
                    comma_ticks(record.workflow_operations),
                    comma_ticks(record.debug_operations),
                    yes_no(record.parity["cli"]),
                    yes_no(record.parity["server"]),
                    yes_no(record.parity["rust_wasm"]),
                    yes_no(record.parity["bun_wasm"]),
                    yes_no(record.parity["app"]),
                    yes_no(record.readme_quickstart),
                    yes_no(record.primary_workflow_test),
                    record.app_default_status,
                    "; ".join(record.known_gaps) if record.known_gaps else "none",
                ]
            )
            + " |"
        )
    return "\n".join(lines) + "\n"


def changed_regression_check(root: Path, base: str, only: str | None) -> int:
    changed_paths = git_changed_paths(root, base)
    packages = library_packages(root, only)
    touched = touched_package_names(changed_paths, packages)
    if only:
        touched = {only} if only in {package.name for package in packages} else set()
    if not touched:
        print("crate progress touched audit passed: no audited crates touched")
        return 0
    return compare_records(root, base, touched)


def compare_base(root: Path, base: str, only: str | None) -> int:
    packages = library_packages(root, only)
    return compare_records(root, base, {package.name for package in packages})


def compare_records(root: Path, base: str, package_names: set[str]) -> int:
    allow_entries = read_regression_allowlist(root / ALLOW_PATH)
    expired = expired_allowlist_entries(allow_entries)
    if expired:
        for entry in expired:
            print(
                f"expired progress regression allowlist entry: {entry.crate} {entry.metric} expired {entry.expires}",
                file=sys.stderr,
            )
        return 1

    current = audit_selected_records(root, package_names)
    with base_worktree(root, base) as base_root:
        base_records = audit_selected_records(base_root, package_names)

    failures = regression_failures(current, base_records, package_names, allow_entries)
    if failures:
        print("crate progress regression audit failed:", file=sys.stderr)
        for failure in failures:
            print(f"- {failure}", file=sys.stderr)
        return 1
    print(f"crate progress regression audit passed for {len(package_names)} touched crate(s)")
    return 0


def audit_selected_records(root: Path, package_names: set[str]) -> dict[str, ProgressRecord]:
    if not package_names:
        return {}
    if len(package_names) > 8:
        return {
            record.library: record
            for record in audit_records(root)
            if record.library in package_names
        }
    return {
        record.library: record
        for package_name in sorted(package_names)
        for record in audit_records(root, package_name)
    }


def regression_failures(
    current: dict[str, ProgressRecord],
    base: dict[str, ProgressRecord],
    package_names: set[str],
    allow_entries: list[RegressionAllow],
) -> list[str]:
    failures: list[str] = []
    for package_name in sorted(package_names):
        if package_name not in current or package_name not in base:
            continue
        current_record = current[package_name]
        base_record = base[package_name]
        if LEVEL_RANK[current_record.level] < LEVEL_RANK[base_record.level] and not is_allowed(
            allow_entries, package_name, "level"
        ):
            failures.append(
                f"{package_name}: maturity regressed from {base_record.level} to {current_record.level}"
            )
        if current_record.score < base_record.score and not is_allowed(allow_entries, package_name, "score"):
            failures.append(
                f"{package_name}: score regressed from {base_record.score} to {current_record.score}"
            )
    return failures


def touched_package_names(changed_paths: list[str], packages: list[LibraryPackage]) -> set[str]:
    if any(is_shared_path(path) for path in changed_paths):
        return {package.name for package in packages}
    touched: set[str] = set()
    for path in changed_paths:
        for package in packages:
            if path.startswith(package.relative_dir + "/"):
                touched.add(package.name)
            elif path.startswith(f"crates/bindings/{package.base}-wasm/"):
                touched.add(package.name)
            elif path.startswith(f"packages/{package.base}-wasm/"):
                touched.add(package.name)
            elif path.startswith(f"packages/{package.base}-app/"):
                touched.add(package.name)
            elif path.startswith(package.manifest_path.parent.parent.as_posix() + f"/{package.base}-cli/"):
                touched.add(package.name)
            elif path.startswith(package.manifest_path.parent.parent.as_posix() + f"/{package.base}-server/"):
                touched.add(package.name)
            elif path.startswith("tests/") and (
                package.base in path or package.base.replace("-", "_") in path
            ):
                touched.add(package.name)
    return touched


def is_shared_path(path: str) -> bool:
    return path in SHARED_EXACT_PATHS or any(path.startswith(prefix) for prefix in SHARED_PREFIXES)


def git_changed_paths(root: Path, base: str) -> list[str]:
    merge_base = git_output(root, ["git", "merge-base", base, "HEAD"])
    output = git_output(root, ["git", "diff", "--name-only", f"{merge_base}...HEAD"])
    return [line.strip() for line in output.splitlines() if line.strip()]


class base_worktree:
    def __init__(self, root: Path, base: str) -> None:
        self.root = root
        self.base = base
        self.temp_dir: tempfile.TemporaryDirectory[str] | None = None
        self.worktree: Path | None = None

    def __enter__(self) -> Path:
        merge_base = git_output(self.root, ["git", "merge-base", self.base, "HEAD"])
        self.temp_dir = tempfile.TemporaryDirectory(prefix="crate-progress-base-")
        self.worktree = Path(self.temp_dir.name) / "repo"
        subprocess.check_call(
            ["git", "worktree", "add", "--detach", "--quiet", str(self.worktree), merge_base],
            cwd=self.root,
        )
        return self.worktree

    def __exit__(self, exc_type, exc, tb) -> None:
        if self.worktree and self.worktree.exists():
            subprocess.run(["git", "worktree", "remove", "--force", str(self.worktree)], cwd=self.root)
        if self.temp_dir:
            self.temp_dir.cleanup()


def read_regression_allowlist(path: Path) -> list[RegressionAllow]:
    entries: list[RegressionAllow] = []
    if not path.is_file():
        return entries
    for line_number, raw_line in enumerate(path.read_text(encoding="utf-8").splitlines(), start=1):
        line = raw_line.strip()
        if not line or line.startswith("#"):
            continue
        parts = raw_line.split("\t")
        if len(parts) != 4:
            raise SystemExit(f"{path}:{line_number}: malformed allowlist entry")
        crate, metric, expires, reason = parts
        if crate == "all" or not crate.startswith(PACKAGE_PREFIXES):
            raise SystemExit(f"{path}:{line_number}: allowlist entry must be crate-specific")
        if metric not in {"score", "level"}:
            raise SystemExit(f"{path}:{line_number}: metric must be `score` or `level`")
        entries.append(RegressionAllow(crate=crate, metric=metric, expires=date.fromisoformat(expires), reason=reason))
    return entries


def expired_allowlist_entries(entries: list[RegressionAllow]) -> list[RegressionAllow]:
    today = date.today()
    return [entry for entry in entries if entry.expires < today]


def is_allowed(entries: list[RegressionAllow], crate: str, metric: str) -> bool:
    today = date.today()
    return any(
        entry.crate == crate and entry.metric == metric and entry.expires >= today
        for entry in entries
    )


def classify_operation(operation: dict | None) -> str:
    if not operation:
        return "debug"
    curation_role = operation_curation_role(operation)
    if curation_role in {"workflow", "debug", "support"}:
        return curation_role
    schema_category = schema_category_value(operation.get("inputSchema")) or schema_category_value(operation.get("outputSchema"))
    if schema_category in {"workflow", "debug", "support"}:
        return schema_category
    operation_id = str(operation.get("id", "")).lower()
    name = str(operation.get("name", "")).lower()
    if operation_id == "describe":
        return "debug"
    if any(keyword in operation_id or keyword in name for keyword in DEBUG_KEYWORDS):
        return "debug"
    return "workflow"


def operation_curation_role(operation: dict | None) -> str | None:
    if not operation:
        return None
    curation = operation.get("curation")
    if not isinstance(curation, dict):
        return None
    role = curation.get("role")
    return role.lower() if isinstance(role, str) else None


def operation_curation_primary(operation: dict | None) -> bool:
    if not operation:
        return False
    curation = operation.get("curation")
    return isinstance(curation, dict) and curation.get("primary") is True


def schema_category_value(schema: object) -> str | None:
    if not isinstance(schema, dict):
        return None
    value = schema.get("xOperationCategory")
    if isinstance(value, str):
        return value.lower()
    for nested in schema.values():
        if isinstance(nested, dict):
            found = schema_category_value(nested)
            if found:
                return found
    return None


def find_operation(operations: list[dict], operation_id: str) -> dict | None:
    for operation in operations:
        if operation.get("id") == operation_id:
            return operation
    return None


def operations_for_group_label(text: str, label: str) -> list[str]:
    match = re.search(
        r"\{[^{}]*label:\s*[\"']" + re.escape(label) + r"[\"'][^{}]*operations:\s*\[([^\]]*)\]",
        text,
        re.DOTALL,
    )
    if not match:
        return []
    return quoted_values(match.group(1))


def string_property(text: str, property_name: str) -> str | None:
    match = re.search(rf"{re.escape(property_name)}:\s*[\"']([^\"']+)[\"']", text)
    return match.group(1) if match else None


def array_property(text: str, property_name: str) -> list[str]:
    match = re.search(rf"{re.escape(property_name)}:\s*\[([^\]]*)\]", text, re.DOTALL)
    return quoted_values(match.group(1)) if match else []


def quoted_values(text: str) -> list[str]:
    return re.findall(r"[\"']([^\"']+)[\"']", text)


def run_json(root: Path, command: list[str]) -> dict:
    output = subprocess.check_output(command, cwd=root, env=cargo_env(), text=True)
    return json.loads(output)


def cargo_env() -> dict[str, str]:
    return os.environ.copy()


def git_output(root: Path, command: list[str]) -> str:
    return subprocess.check_output(command, cwd=root, text=True).strip()


def read_text(path: Path) -> str:
    if not path.is_file():
        return ""
    return path.read_text(encoding="utf-8")


def companion_package_base_name(package_name: str) -> str:
    for prefix in PACKAGE_PREFIXES:
        if package_name.startswith(prefix):
            return package_name.removeprefix(prefix)
    return package_name


def public_package_name(package_name: str, owner_package_name: str | None = None) -> str:
    if package_name.startswith(PACKAGE_PREFIXES):
        return package_name
    return f"{package_prefix(owner_package_name)}{package_name}"


def package_prefix(package_name: str | None) -> str:
    if package_name:
        for prefix in PACKAGE_PREFIXES:
            if package_name.startswith(prefix):
                return prefix
    return DEFAULT_PACKAGE_PREFIX


def tick(value: str) -> str:
    return f"`{value}`"


def comma_ticks(values: list[str]) -> str:
    return ", ".join(tick(value) for value in values) if values else "none"


def yes_no(value: bool) -> str:
    return "yes" if value else "no"


if __name__ == "__main__":
    raise SystemExit(main())
