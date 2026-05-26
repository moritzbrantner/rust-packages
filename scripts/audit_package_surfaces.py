#!/usr/bin/env python3
"""Audit and regenerate docs/PACKAGE_SURFACE_MATRIX.md."""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
MATRIX_PATH = ROOT / "docs" / "PACKAGE_SURFACE_MATRIX.md"
EXCLUDED_LIBRARY_CRATES = {"runtime-artifacts", "runtime-jobs"}
WRAPPER_SUFFIXES = ("-cli", "-server", "-wasm")


@dataclass(frozen=True)
class LibraryPackage:
    name: str
    manifest_path: Path


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true", help="rewrite docs/PACKAGE_SURFACE_MATRIX.md")
    parser.add_argument("--check", action="store_true", help="fail if docs/PACKAGE_SURFACE_MATRIX.md differs")
    parser.add_argument("--only", help="limit audit to one library crate")
    args = parser.parse_args()

    if args.write and args.check:
        parser.error("--write and --check are mutually exclusive")

    packages = library_packages(args.only)
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
    for package in packages:
        operations = operation_ids(package.name)
        lines.append(
            "| "
            + " | ".join(
                [
                    tick(package.name),
                    tick(f"{package.name}-cli"),
                    tick(f"{package.name}-server"),
                    tick(f"crates/bindings/{package.name}-wasm"),
                    tick(f"packages/{package.name}-wasm"),
                    tick(f"packages/{package.name}-app"),
                    ", ".join(tick(operation) for operation in operations),
                    yes_no((ROOT / "crates" / "bindings" / f"{package.name}-wasm").is_dir()),
                    yes_no(companion_dir(package, "server").is_dir()),
                ]
            )
            + " |"
        )
        verify_companions(package)
    return "\n".join(lines) + "\n"


def operation_ids(crate: str) -> list[str]:
    output = subprocess.check_output(
        ["cargo", "run", "--quiet", "-p", f"{crate}-cli", "--", "operations", "--json"],
        cwd=ROOT,
        text=True,
    )
    operations = json.loads(output)
    ids = [operation["id"] for operation in operations]
    return ids or ["describe"]


def verify_companions(package: LibraryPackage) -> None:
    missing = []
    if not companion_dir(package, "cli").joinpath("Cargo.toml").is_file():
        missing.append(f"{package.name}-cli")
    if not companion_dir(package, "server").joinpath("Cargo.toml").is_file():
        missing.append(f"{package.name}-server")
    if not (ROOT / "crates" / "bindings" / f"{package.name}-wasm" / "Cargo.toml").is_file():
        missing.append(f"crates/bindings/{package.name}-wasm")
    if not (ROOT / "packages" / f"{package.name}-wasm" / "package.json").is_file():
        missing.append(f"packages/{package.name}-wasm")
    if not (ROOT / "packages" / f"{package.name}-app" / "package.json").is_file():
        missing.append(f"packages/{package.name}-app")
    if missing:
        raise SystemExit(f"{package.name}: missing companion packages: {', '.join(missing)}")


def companion_dir(package: LibraryPackage, kind: str) -> Path:
    return package.manifest_path.parent.parent / f"{package.name}-{kind}"


def run_json(command: list[str]) -> dict:
    output = subprocess.check_output(command, cwd=ROOT, text=True)
    return json.loads(output)


def tick(value: str) -> str:
    return f"`{value}`"


def yes_no(value: bool) -> str:
    return "yes" if value else "no"


if __name__ == "__main__":
    raise SystemExit(main())
