#!/usr/bin/env python3
"""Validate the non-destructive canonical destination ownership cutover."""

from __future__ import annotations

import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
OWNERSHIP = ROOT / "docs/repository-split/package-ownership.json"
CUTOVER = ROOT / "docs/repository-split/ownership-cutover.json"


def load(path: Path) -> dict:
    with path.open(encoding="utf-8") as handle:
        return json.load(handle)


def records(authority: dict) -> list[dict]:
    return [
        *authority.get("packages", []),
        *authority.get("post_baseline_packages", []),
    ]


def main() -> int:
    authority = load(OWNERSHIP)
    cutover = load(CUTOVER)
    families = {
        item["targetRepository"]: item
        for item in cutover.get("families", [])
        if item.get("ecosystem") == "cargo"
    }

    errors: list[str] = []
    expected_authority = {"source", "tests", "issues", "versions", "releases"}
    for target, item in sorted(families.items()):
        canonical = item.get("canonicalRepository")
        if not canonical or canonical == "moritzbrantner/rust-packages":
            errors.append(f"{target}: canonical repository must be a destination repository")
        if set(item.get("authority", [])) != expected_authority:
            errors.append(f"{target}: authority must cover source/tests/issues/versions/releases")

    migrated = [
        record
        for record in records(authority)
        if record.get("ecosystem") == "cargo" and record.get("target_repository") in families
    ]
    seen_targets = {record.get("target_repository") for record in migrated}
    for target in sorted(families):
        if target not in seen_targets:
            errors.append(f"{target}: cutover target has no Cargo packages in ownership authority")

    for record in migrated:
        target = record.get("target_repository")
        intended_owner = record.get("intended_next_release_owner")
        canonical = families[target]["canonicalRepository"]
        if intended_owner is not None and intended_owner != canonical:
            errors.append(
                f"{record.get('current_package_name')}: intended release owner {intended_owner!r} "
                f"does not match canonical {canonical!r}"
            )

    if cutover.get("rustPackagesRoleAfterCutover") != "compatibility-provenance-only":
        errors.append("rustPackagesRoleAfterCutover must be compatibility-provenance-only")

    if errors:
        print("ownership cutover violations:")
        for error in errors:
            print(f"- {error}")
        return 1

    print(f"ownership cutover: ok ({len(migrated)} migrated Cargo packages)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
