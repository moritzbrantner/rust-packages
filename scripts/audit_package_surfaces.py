#!/usr/bin/env python3
"""Audit and regenerate docs/PACKAGE_SURFACE_MATRIX.md."""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
MATRIX_PATH = ROOT / "docs" / "PACKAGE_SURFACE_MATRIX.md"
EXCLUDED_LIBRARY_CRATES = {"runtime-artifacts", "runtime-jobs"}
WRAPPER_SUFFIXES = ("-cli", "-server", "-wasm")
COMPANION_PACKAGE_BASE_NAMES = {
    "moritzbrantner-geo-core": "geo-core",
    "moritzbrantner-geo-io-geojson": "geo-io-geojson",
    "moritzbrantner-geo-clustering": "geo-clustering",
    "moritzbrantner-geo-viz": "geo-viz",
}
SCAFFOLD_STRINGS = [
    "A deterministic summary or execution plan owned by the Rust library",
    "JSON request metadata for the operation-specific package surface",
]
LEGACY_WASM_TEST_STRING = "run default wasm operation"
LEGACY_WASM_TEST_ALLOWLIST = {
    "crates/bindings/audio-analysis-core-wasm/src/lib.rs",
    "crates/bindings/audio-analysis-fourier-wasm/src/lib.rs",
    "crates/bindings/audio-analysis-io-wasm/src/lib.rs",
    "crates/bindings/audio-analysis-pitch-wasm/src/lib.rs",
    "crates/bindings/audio-analysis-processing-wasm/src/lib.rs",
    "crates/bindings/audio-analysis-recognition-wasm/src/lib.rs",
    "crates/bindings/audio-analysis-rhythm-wasm/src/lib.rs",
    "crates/bindings/audio-analysis-separation-wasm/src/lib.rs",
    "crates/bindings/audio-analysis-speakers-wasm/src/lib.rs",
    "crates/bindings/audio-analysis-synthesis-wasm/src/lib.rs",
    "crates/bindings/audio-analysis-test-support-wasm/src/lib.rs",
    "crates/bindings/audio-generation-midi-wasm/src/lib.rs",
}
MOD_DECLARATION_RE = re.compile(r"^\s*(?:pub(?:\([^)]*\))?\s+)?mod\s+([A-Za-z_][A-Za-z0-9_]*)\s*;")


@dataclass(frozen=True)
class LibraryPackage:
    name: str
    manifest_path: Path


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true", help="rewrite docs/PACKAGE_SURFACE_MATRIX.md")
    parser.add_argument("--check", action="store_true", help="fail if docs/PACKAGE_SURFACE_MATRIX.md differs")
    parser.add_argument("--only", help="limit audit to one library crate")
    parser.add_argument(
        "--quality",
        action="store_true",
        help="run executable package-surface maturity checks instead of rendering the matrix",
    )
    args = parser.parse_args()

    if sum(bool(value) for value in [args.write, args.check, args.quality]) > 1:
        parser.error("--write, --check, and --quality are mutually exclusive")

    packages = library_packages(args.only)
    if args.quality:
        return quality_audit(packages)

    content = render_matrix(packages)

    if args.write:
        MATRIX_PATH.write_text(content, encoding="utf-8")
        return 0

    if args.check:
        existing = MATRIX_PATH.read_text(encoding="utf-8")
        if existing != content:
            print(f"{MATRIX_PATH.relative_to(ROOT)} is out of date; run scripts/audit_package_surfaces.py --write", file=sys.stderr)
            return 1
        return 0

    print(content, end="")
    return 0


def library_packages(only: str | None) -> list[LibraryPackage]:
    metadata = run_json(["cargo", "metadata", "--format-version", "1", "--no-deps"])
    packages = []
    for package in metadata["packages"]:
        name = package["name"]
        manifest = Path(package["manifest_path"])
        relative = manifest.relative_to(ROOT)
        if not str(relative).startswith("crates/"):
            continue
        if str(relative).startswith("crates/bindings/"):
            continue
        if name in EXCLUDED_LIBRARY_CRATES or name.endswith(WRAPPER_SUFFIXES):
            continue
        if not any(target.get("kind") == ["lib"] or "lib" in target.get("kind", []) for target in package["targets"]):
            continue
        if only and name != only:
            continue
        packages.append(LibraryPackage(name=name, manifest_path=manifest))
    packages.sort(key=lambda package: package.name)
    return packages


def render_matrix(packages: list[LibraryPackage]) -> str:
    lines = [
        "# Package Surface Matrix",
        "",
        "Generated audit of library crates and their required runtime surfaces.",
        "",
        "| Library | CLI | Server | Rust WASM crate | Bun WASM package | App package | Representative operations | WASM | Server |",
        "|---|---|---|---|---|---|---|---|---|",
    ]
    missing_companions = []
    for package in packages:
        operations = operation_ids(package.name)
        companion_base = companion_package_base_name(package.name)
        lines.append(
            "| "
            + " | ".join(
                [
                    tick(package.name),
                    tick(f"{companion_base}-cli"),
                    tick(f"{companion_base}-server"),
                    tick(f"crates/bindings/{companion_base}-wasm"),
                    tick(f"packages/{companion_base}-wasm"),
                    tick(f"packages/{companion_base}-app"),
                    ", ".join(tick(operation) for operation in operations),
                    yes_no((ROOT / "crates" / "bindings" / f"{companion_base}-wasm").is_dir()),
                    yes_no(companion_dir(package, "server").is_dir()),
                ]
            )
            + " |"
        )
        missing_companions.extend(companion_failures(package))
    if missing_companions:
        raise SystemExit(
            "missing companion packages:\n"
            + "\n".join(f"- {failure}" for failure in missing_companions)
        )
    return "\n".join(lines) + "\n"


def operation_ids(crate: str) -> list[str]:
    companion_base = companion_package_base_name(crate)
    output = subprocess.check_output(
        [
            "cargo",
            "run",
            "--quiet",
            "-p",
            f"{companion_base}-cli",
            "--",
            "operations",
            "--json",
        ],
        cwd=ROOT,
        text=True,
    )
    operations = json.loads(output)
    ids = [operation["id"] for operation in operations]
    return ids or ["describe"]


def companion_failures(package: LibraryPackage) -> list[str]:
    missing = []
    companion_base = companion_package_base_name(package.name)
    if not companion_dir(package, "cli").joinpath("Cargo.toml").is_file():
        missing.append(f"{companion_base}-cli")
    if not companion_dir(package, "server").joinpath("Cargo.toml").is_file():
        missing.append(f"{companion_base}-server")
    if not (ROOT / "crates" / "bindings" / f"{companion_base}-wasm" / "Cargo.toml").is_file():
        missing.append(f"crates/bindings/{companion_base}-wasm")
    if not (ROOT / "packages" / f"{companion_base}-wasm" / "package.json").is_file():
        missing.append(f"packages/{companion_base}-wasm")
    if not (ROOT / "packages" / f"{companion_base}-app" / "package.json").is_file():
        missing.append(f"packages/{companion_base}-app")
    return [
        f"{package.name}: missing companion package `{companion}`"
        for companion in missing
    ]


def companion_dir(package: LibraryPackage, kind: str) -> Path:
    return package.manifest_path.parent.parent / f"{companion_package_base_name(package.name)}-{kind}"


def companion_package_base_name(package_name: str) -> str:
    return COMPANION_PACKAGE_BASE_NAMES.get(package_name, package_name)


def run_json(command: list[str]) -> dict:
    output = subprocess.check_output(command, cwd=ROOT, text=True)
    return json.loads(output)


def quality_audit(packages: list[LibraryPackage]) -> int:
    failures: list[str] = []
    audit_repository_quality(failures)
    for package in packages:
        audit_one_package_quality(package, failures)

    if failures:
        print("package surface quality audit failed:", file=sys.stderr)
        for failure in failures:
            print(f"- {failure}", file=sys.stderr)
        return 1

    print(f"package surface quality audit passed for {len(packages)} library crates")
    return 0


def audit_one_package_quality(package: LibraryPackage, failures: list[str]) -> None:
    companion_missing = companion_failures(package)
    failures.extend(companion_missing)
    if companion_missing:
        return
    operations = read_cli_operations(package, failures)
    if not operations:
        failures.append(f"{package.name}: CLI reported no operations")
        return

    operation_ids = []
    seen_ids = set()
    for operation in operations:
        operation_id = operation.get("id")
        if not isinstance(operation_id, str) or not operation_id.strip():
            failures.append(f"{package.name}: operation id is empty or missing")
            continue
        if operation_id in seen_ids:
            failures.append(f"{package.name}: duplicate operation id `{operation_id}`")
        seen_ids.add(operation_id)
        operation_ids.append(operation_id)
        validate_operation_metadata(package.name, operation, failures)

    for operation in operations:
        operation_id = operation.get("id")
        if not isinstance(operation_id, str) or not operation_id:
            continue
        run_operation_example(package.name, operation, failures)

    validate_cli_invalid_behavior(package.name, failures)
    validate_app_config(package.name, operation_ids, failures)


def audit_repository_quality(failures: list[str]) -> None:
    validate_no_dangling_mod_declarations(failures)
    validate_no_new_legacy_wasm_tests(failures)


def validate_no_dangling_mod_declarations(failures: list[str]) -> None:
    for path in sorted((ROOT / "crates").rglob("*.rs")):
        relative = path.relative_to(ROOT)
        if any(part in {"target", ".cargo-target"} for part in relative.parts):
            continue
        lines = path.read_text(encoding="utf-8").splitlines()
        for line_number, line in enumerate(lines, start=1):
            match = MOD_DECLARATION_RE.match(line)
            if not match:
                continue
            if has_cfg_attribute(lines, line_number):
                continue
            module_name = match.group(1)
            if module_name in {"self", "super", "crate"}:
                continue
            if not module_file_candidates(path, module_name):
                failures.append(
                    f"{relative}:{line_number}: dangling module declaration `mod {module_name};`"
                )


def has_cfg_attribute(lines: list[str], line_number: int) -> bool:
    index = line_number - 2
    while index >= 0:
        previous = lines[index].strip()
        if not previous:
            index -= 1
            continue
        return previous.startswith("#[cfg") or previous.startswith("#[path")
    return False


def module_file_candidates(path: Path, module_name: str) -> bool:
    stem = path.stem
    if stem in {"lib", "main", "mod"}:
        base = path.parent
    else:
        base = path.parent / stem
    return (base / f"{module_name}.rs").is_file() or (base / module_name / "mod.rs").is_file()


def validate_no_new_legacy_wasm_tests(failures: list[str]) -> None:
    for path in sorted((ROOT / "crates" / "bindings").rglob("*.rs")):
        relative = path.relative_to(ROOT).as_posix()
        if relative in LEGACY_WASM_TEST_ALLOWLIST:
            continue
        text = path.read_text(encoding="utf-8")
        if LEGACY_WASM_TEST_STRING in text:
            failures.append(
                f"{relative}: replace legacy `{LEGACY_WASM_TEST_STRING}` test with operation-specific assertions"
            )


def read_cli_operations(package: LibraryPackage, failures: list[str]) -> list[dict]:
    companion_base = companion_package_base_name(package.name)
    command = [
        "cargo",
        "run",
        "--quiet",
        "-p",
        f"{companion_base}-cli",
        "--",
        "operations",
        "--json",
    ]
    completed = subprocess.run(command, cwd=ROOT, text=True, capture_output=True)
    if completed.returncode != 0:
        failures.append(
            f"{package.name}: operations CLI failed: {compact_process_output(completed)}"
        )
        return []
    try:
        payload = json.loads(completed.stdout)
    except json.JSONDecodeError as error:
        failures.append(f"{package.name}: operations CLI returned invalid JSON: {error}")
        return []
    if not isinstance(payload, list):
        failures.append(f"{package.name}: operations CLI JSON must be a list")
        return []
    return [operation for operation in payload if isinstance(operation, dict)]


def validate_operation_metadata(crate: str, operation: dict, failures: list[str]) -> None:
    operation_id = operation.get("id")
    for field in ["name", "description"]:
        value = operation.get(field)
        if not isinstance(value, str) or not value.strip():
            failures.append(f"{crate}:{operation_id}: metadata field `{field}` is empty")
    for field in ["inputSchema", "outputSchema", "exampleRequest"]:
        value = operation.get(field)
        if not isinstance(value, dict):
            failures.append(f"{crate}:{operation_id}: `{field}` must be a JSON object")
    for field in ["wasmSupported", "serverSupported"]:
        if not isinstance(operation.get(field), bool):
            failures.append(f"{crate}:{operation_id}: `{field}` must be boolean")


def run_operation_example(crate: str, operation: dict, failures: list[str]) -> None:
    operation_id = operation["id"]
    example_request = operation.get("exampleRequest")
    companion_base = companion_package_base_name(crate)
    command = [
        "cargo",
        "run",
        "--quiet",
        "-p",
        f"{companion_base}-cli",
        "--",
        "run",
        "--operation",
        operation_id,
        "--json",
        json.dumps(example_request),
    ]
    completed = subprocess.run(command, cwd=ROOT, text=True, capture_output=True)
    if completed.returncode != 0:
        failures.append(
            f"{crate}:{operation_id}: example request failed: {compact_process_output(completed)}"
        )
        return
    try:
        response = json.loads(completed.stdout)
    except json.JSONDecodeError as error:
        failures.append(f"{crate}:{operation_id}: run returned invalid JSON: {error}")
        return
    if response.get("operation") != operation_id:
        failures.append(
            f"{crate}:{operation_id}: response operation was `{response.get('operation')}`"
        )
    response_text = json.dumps(response, sort_keys=True)
    for scaffold in SCAFFOLD_STRINGS:
        if scaffold in response_text:
            failures.append(f"{crate}:{operation_id}: response still contains scaffold text")

    value = response.get("value")
    if not isinstance(value, dict):
        failures.append(f"{crate}:{operation_id}: response value must be an object")
        return
    for field in ["operation", "title", "message", "summary", "result"]:
        if field not in value:
            failures.append(f"{crate}:{operation_id}: response value missing `{field}`")
    if value.get("operation") != operation_id:
        failures.append(
            f"{crate}:{operation_id}: value operation was `{value.get('operation')}`"
        )
    if not isinstance(value.get("title"), str) or not value.get("title", "").strip():
        failures.append(f"{crate}:{operation_id}: response title is empty")
    if not isinstance(value.get("message"), str) or not value.get("message", "").strip():
        failures.append(f"{crate}:{operation_id}: response message is empty")
    if not isinstance(value.get("summary"), dict):
        failures.append(f"{crate}:{operation_id}: response summary must be an object")
    if value.get("result") is None:
        failures.append(f"{crate}:{operation_id}: response result must not be null")


def validate_cli_invalid_behavior(crate: str, failures: list[str]) -> None:
    companion_base = companion_package_base_name(crate)
    unsupported = subprocess.run(
        [
            "cargo",
            "run",
            "--quiet",
            "-p",
            f"{companion_base}-cli",
            "--",
            "run",
            "--operation",
            "__surfaceAudit.invalidOperation",
            "--json",
            "{}",
        ],
        cwd=ROOT,
        text=True,
        capture_output=True,
    )
    if unsupported.returncode == 0:
        failures.append(f"{crate}: unsupported operation unexpectedly succeeded")
    elif "unsupported operation" not in compact_process_output(unsupported).lower():
        failures.append(f"{crate}: unsupported operation error is unclear")

    malformed = subprocess.run(
        [
            "cargo",
            "run",
            "--quiet",
            "-p",
            f"{companion_base}-cli",
            "--",
            "run",
            "--operation",
            "describe",
            "--json",
            "{",
        ],
        cwd=ROOT,
        text=True,
        capture_output=True,
    )
    if malformed.returncode == 0:
        failures.append(f"{crate}: malformed JSON unexpectedly succeeded")
    elif not any(
        marker in compact_process_output(malformed).lower()
        for marker in ["json", "parse", "parsing", "eof", "error"]
    ):
        failures.append(f"{crate}: malformed JSON error is unclear")


def validate_app_config(crate: str, operation_ids: list[str], failures: list[str]) -> None:
    companion_base = companion_package_base_name(crate)
    app_path = ROOT / "packages" / f"{companion_base}-app" / "src" / "App.tsx"
    if not app_path.is_file():
        failures.append(f"{crate}: missing app source {app_path.relative_to(ROOT)}")
        return
    text = app_path.read_text(encoding="utf-8")
    for token in ["defaultOperation", "featuredOperations", "operationGroups"]:
        if token not in text:
            failures.append(f"{crate}: app config missing `{token}`")
    if 'label: "Workflow"' not in text and "label: 'Workflow'" not in text:
        failures.append(f"{crate}: app config missing Workflow operation group")
    if 'label: "Debug"' not in text and "label: 'Debug'" not in text:
        failures.append(f"{crate}: app config missing Debug operation group")

    default_match = re.search(r"defaultOperation:\s*[\"']([^\"']+)[\"']", text)
    if not default_match:
        return
    default_operation = default_match.group(1)
    if default_operation not in operation_ids:
        failures.append(
            f"{crate}: default operation `{default_operation}` is not exposed by the library"
        )
    if default_operation == "describe" and any(operation != "describe" for operation in operation_ids):
        failures.append(f"{crate}: app defaults to describe instead of the primary workflow")


def compact_process_output(completed: subprocess.CompletedProcess[str]) -> str:
    output = f"{completed.stdout}\n{completed.stderr}".strip()
    return " ".join(output.split())[:600]


def tick(value: str) -> str:
    return f"`{value}`"


def yes_no(value: bool) -> str:
    return "yes" if value else "no"


if __name__ == "__main__":
    raise SystemExit(main())
