#!/usr/bin/env python3
"""Audit and document the active Cargo workspace crate inventory."""

from __future__ import annotations

import argparse
import difflib
import json
import re
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
INVENTORY_PATH = ROOT / "docs" / "CRATE_INVENTORY.md"
SCAN_PATHS = [
    ROOT / "scripts",
    ROOT / "docs",
    ROOT / ".github",
]
WRAPPER_SUFFIXES = ("-cli", "-server", "-wasm")
INTERNAL_TEST_PACKAGES = {
    "moenarch-audio-analysis-test-support",
    "moenarch-video-analysis-test-support",
}
NO_SURFACE_LIBRARIES = {
    "moenarch-runtime-core",
}


@dataclass(frozen=True)
class CrateRecord:
    name: str
    path: str
    domain: str
    kind: str
    publish: str
    surface_required: bool
    facade_expected: bool


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true", help="rewrite docs/CRATE_INVENTORY.md")
    parser.add_argument("--check", action="store_true", help="fail if docs/CRATE_INVENTORY.md or selector policy is stale")
    args = parser.parse_args()

    if args.write and args.check:
        parser.error("--write and --check are mutually exclusive")

    records = crate_records()
    content = render_inventory(records)

    if args.write:
        INVENTORY_PATH.write_text(content, encoding="utf-8")
        return 0

    if args.check:
        failures = stale_selector_failures(records)
        if not INVENTORY_PATH.exists():
            failures.append(f"{INVENTORY_PATH.relative_to(ROOT)} is missing; run scripts/audit_workspace_crates.py --write")
        else:
            existing = INVENTORY_PATH.read_text(encoding="utf-8")
            if existing != content:
                diff = "\n".join(
                    difflib.unified_diff(
                        existing.splitlines(),
                        content.splitlines(),
                        fromfile=str(INVENTORY_PATH.relative_to(ROOT)),
                        tofile="generated",
                        lineterm="",
                    )
                )
                failures.append(
                    f"{INVENTORY_PATH.relative_to(ROOT)} is out of date; run scripts/audit_workspace_crates.py --write\n{diff}"
                )
        if failures:
            print("workspace crate audit failed:", file=sys.stderr)
            for failure in failures:
                print(f"- {failure}", file=sys.stderr)
            return 1
        return 0

    print(content, end="")
    return 0


def crate_records() -> list[CrateRecord]:
    metadata = run_json(["cargo", "metadata", "--format-version", "1", "--no-deps"])
    workspace_members = set(metadata["workspace_members"])
    workspace_root = Path(metadata["workspace_root"])
    records = []
    for package in metadata["packages"]:
        if package["id"] not in workspace_members:
            continue
        manifest = Path(package["manifest_path"])
        relative_manifest = manifest.relative_to(workspace_root)
        relative_dir = relative_manifest.parent
        name = package["name"]
        domain = classify_domain(relative_manifest)
        kind = classify_kind(name, relative_manifest, package.get("targets", []))
        publish = classify_publish(name, package)
        surface_required = (
            kind == "library"
            and publish == "public"
            and name not in NO_SURFACE_LIBRARIES
        )
        facade_expected = (
            kind == "library"
            and publish == "public"
            and domain not in {"root", "prototype", "bindings"}
        )
        records.append(
            CrateRecord(
                name=name,
                path=str(relative_dir) if str(relative_dir) != "." else ".",
                domain=domain,
                kind=kind,
                publish=publish,
                surface_required=surface_required,
                facade_expected=facade_expected,
            )
        )
    return sorted(records, key=lambda record: (record.domain, record.kind, record.name))


def classify_domain(relative_manifest: Path) -> str:
    parts = relative_manifest.parts
    if parts == ("Cargo.toml",):
        return "root"
    if len(parts) >= 3 and parts[0] == "crates":
        return parts[1]
    if parts and parts[0] == "prototypes":
        return "prototype"
    return "root"


def classify_kind(name: str, relative_manifest: Path, targets: list[dict]) -> str:
    if relative_manifest.parts and relative_manifest.parts[0] == "prototypes":
        return "prototype"
    if name.endswith("-cli"):
        return "cli"
    if name.endswith("-server"):
        return "server"
    if name.endswith("-wasm") or str(relative_manifest).startswith("crates/bindings/"):
        return "wasm"
    if any("bin" in target.get("kind", []) for target in targets) and not any(
        "lib" in target.get("kind", []) for target in targets
    ):
        return "app"
    return "library"


def classify_publish(name: str, package: dict) -> str:
    if name in INTERNAL_TEST_PACKAGES:
        return "internal-test"
    if package.get("publish") is False:
        return "private"
    return "public"


def render_inventory(records: list[CrateRecord]) -> str:
    lines = [
        "# Crate Inventory",
        "",
        "<!-- Generated by scripts/audit_workspace_crates.py; do not edit by hand. -->",
        "",
        "This inventory is generated from `cargo metadata --no-deps` and only includes active workspace members.",
        "Excluded or deleted crates do not participate in surface, facade, or layer policy.",
        "",
        "Regenerate it after changing workspace membership or package classification:",
        "",
        "```bash",
        "python3 scripts/audit_workspace_crates.py --write",
        "```",
        "",
        "Check committed inventory and stale Cargo package selectors:",
        "",
        "```bash",
        "python3 scripts/audit_workspace_crates.py --check",
        "```",
        "",
    ]
    lines.extend(render_counts(records))
    lines.extend(
        [
            "",
            "| Package | Domain | Kind | Publish | Surface required | Facade expected | Path |",
            "|---|---|---|---|---|---|---|",
        ]
    )
    for record in records:
        lines.append(
            "| "
            + " | ".join(
                [
                    tick(record.name),
                    record.domain,
                    record.kind,
                    record.publish,
                    yes_no(record.surface_required),
                    yes_no(record.facade_expected),
                    tick(record.path),
                ]
            )
            + " |"
        )
    return "\n".join(lines) + "\n"


def render_counts(records: list[CrateRecord]) -> list[str]:
    lines = ["## Counts", ""]
    lines.append(f"- Active workspace packages: {len(records)}")
    for label, attr in [("Domains", "domain"), ("Kinds", "kind"), ("Publish classes", "publish")]:
        lines.extend(["", f"### {label}", ""])
        counts: dict[str, int] = {}
        for record in records:
            key = getattr(record, attr)
            counts[key] = counts.get(key, 0) + 1
        for key in sorted(counts):
            lines.append(f"- `{key}`: {counts[key]}")
    return lines


def stale_selector_failures(records: list[CrateRecord]) -> list[str]:
    local_names = {record.name for record in records}
    unprefixed = {package_base_name(name): name for name in local_names}
    failures = []
    for path in scan_files():
        text = path.read_text(encoding="utf-8", errors="ignore")
        for line_number, line in enumerate(text.splitlines(), start=1):
            if "cargo" not in line or "-p" not in line:
                continue
            for selector in re.findall(r"(?:^|[\s'\"])-p\s+([A-Za-z0-9_-]+)", line):
                if is_active_rust_package_name(selector):
                    if selector not in local_names and "{" not in line and "<" not in line:
                        failures.append(
                            f"{path.relative_to(ROOT)}:{line_number} uses unknown package selector `-p {selector}`"
                        )
                    continue
                if selector in unprefixed:
                    failures.append(
                        f"{path.relative_to(ROOT)}:{line_number} uses `-p {selector}`; use `-p {unprefixed[selector]}`"
                    )
    return failures


def package_base_name(name: str) -> str:
    return name.removeprefix("moenarch-").removeprefix("moritzbrantner-")


def is_active_rust_package_name(name: str) -> bool:
    return name.startswith(("moenarch-", "moritzbrantner-"))


def scan_files() -> list[Path]:
    files = []
    for path in SCAN_PATHS:
        if not path.exists():
            continue
        if path.is_file():
            files.append(path)
            continue
        for file in path.rglob("*"):
            if file.is_file() and file.suffix in {".json", ".md", ".sh", ".yml", ".yaml"}:
                files.append(file)
    return sorted(files)


def run_json(command: list[str]) -> dict:
    output = subprocess.check_output(command, cwd=ROOT, text=True)
    return json.loads(output)


def tick(value: str) -> str:
    return f"`{value}`"


def yes_no(value: bool) -> str:
    return "yes" if value else "no"


if __name__ == "__main__":
    raise SystemExit(main())
