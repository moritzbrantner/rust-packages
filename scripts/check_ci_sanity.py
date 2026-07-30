#!/usr/bin/env python3
"""Lightweight changed-manifest and secret-material checks for the CI planner."""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SECRET_FILE_NAMES = {".env", ".env.local", "id_rsa", "id_ed25519"}
SECRET_FILE_SUFFIXES = {".key", ".pem", ".p12", ".pfx"}
SECRET_PATTERNS = (
    re.compile(r"-----BEGIN (?:RSA |EC |OPENSSH )?PRIVATE KEY-----"),
    re.compile(r"\bgh[pousr]_[A-Za-z0-9_]{20,}\b"),
    re.compile(r"\b(?:CARGO_REGISTRY_TOKEN|NPM_TOKEN)\s*=\s*\S+"),
)


def manifest_error(path: str, content: bytes) -> str | None:
    try:
        if path.endswith(".json"):
            json.loads(content)
        elif Path(path).name == "Cargo.toml" or path.endswith("/Cargo.toml"):
            tomllib.loads(content.decode("utf-8"))
    except (json.JSONDecodeError, UnicodeDecodeError, tomllib.TOMLDecodeError) as error:
        return f"{path}: invalid manifest ({type(error).__name__})"
    return None


def secret_findings(path: str, added_lines: list[str]) -> list[str]:
    candidate = Path(path)
    findings: list[str] = []
    if candidate.name in SECRET_FILE_NAMES or candidate.suffix.lower() in SECRET_FILE_SUFFIXES:
        findings.append(f"{path}: secret-bearing filename is not allowed")
    for line_number, line in enumerate(added_lines, start=1):
        if any(pattern.search(line) for pattern in SECRET_PATTERNS):
            findings.append(f"{path}: added secret-like material (added line {line_number})")
    return findings


def changed_paths_and_added_lines(base: str) -> tuple[list[str], dict[str, list[str]]]:
    merge_base = subprocess.check_output(
        ["git", "merge-base", base, "HEAD"], cwd=ROOT, text=True
    ).strip()
    paths = subprocess.check_output(
        ["git", "diff", "--name-only", f"{merge_base}...HEAD"], cwd=ROOT, text=True
    ).splitlines()
    patch = subprocess.check_output(
        ["git", "diff", "--unified=0", "--no-color", f"{merge_base}...HEAD"],
        cwd=ROOT,
        text=True,
    )
    added: dict[str, list[str]] = {}
    current: str | None = None
    for line in patch.splitlines():
        if line.startswith("+++ b/"):
            current = line.removeprefix("+++ b/")
            added.setdefault(current, [])
        elif current and line.startswith("+") and not line.startswith("+++"):
            added[current].append(line[1:])
    return sorted(set(paths)), added


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--base", default="origin/main")
    args = parser.parse_args()
    paths, added = changed_paths_and_added_lines(args.base)
    failures: list[str] = []
    for path in paths:
        file_path = ROOT / path
        if file_path.is_file() and (
            path.endswith(".json")
            or Path(path).name == "Cargo.toml"
            or path.endswith("/Cargo.toml")
        ):
            error = manifest_error(path, file_path.read_bytes())
            if error:
                failures.append(error)
        failures.extend(secret_findings(path, added.get(path, [])))
    if failures:
        print("\n".join(failures))
        raise SystemExit(1)
    print(f"CI sanity passed for {len(paths)} changed paths")


if __name__ == "__main__":
    main()
