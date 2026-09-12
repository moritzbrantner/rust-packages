#!/usr/bin/env python3
"""Validate the non-destructive canonical destination ownership cutover."""

from __future__ import annotations

import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
OWNERSHIP = ROOT / "docs/repository-split/package-ownership.json"
CUTOVER = ROOT / "docs/repository-split/ownership-cutover.json"

EXPECTED_FAMILIES = {
    "moenarch-foundation": "moritzbrantner/moenarch-foundation",
    "nlp-stack": "moritzbrantner/nlp-stack",
    "audio-analysis": "moritzbrantner/audio-analysis",
    "visual-analysis": "moritzbrantner/visual-analysis",
}
EXPECTED_AUTHORITY = {"source", "tests", "issues", "versions", "releases"}
EXPECTED_ROLE = "compatibility-provenance-only-for-cutover-families"


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
    family_items = [
        item for item in cutover.get("families", []) if item.get("ecosystem") == "cargo"
    ]
    targets = [item.get("targetRepository") for item in family_items]
    families = {item.get("targetRepository"): item for item in family_items}

    errors: list[str] = []
    if len(targets) != len(set(targets)):
        errors.append("cutover contains duplicate Cargo target repositories")

    expected_targets = set(EXPECTED_FAMILIES)
    actual_targets = set(families)
    for target in sorted(expected_targets - actual_targets):
        errors.append(f"{target}: required cutover family is missing")
    for target in sorted(actual_targets - expected_targets):
        errors.append(f"{target}: unexpected cutover family")

    for target, expected_canonical in sorted(EXPECTED_FAMILIES.items()):
        item = families.get(target)
        if item is None:
            continue
        canonical = item.get("canonicalRepository")
        if canonical != expected_canonical:
            errors.append(
                f"{target}: canonical repository must be {expected_canonical!r}, got {canonical!r}"
            )
        if set(item.get("authority", [])) != EXPECTED_AUTHORITY:
            errors.append(f"{target}: authority must cover source/tests/issues/versions/releases")

    migrated = [
        record
        for record in records(authority)
        if record.get("ecosystem") == "cargo"
        and record.get("target_repository") in EXPECTED_FAMILIES
    ]
    seen_targets = {record.get("target_repository") for record in migrated}
    for target in sorted(expected_targets):
        if target not in seen_targets:
            errors.append(f"{target}: cutover target has no Cargo packages in ownership authority")

    for record in migrated:
        target = record.get("target_repository")
        intended_owner = record.get("intended_next_release_owner")
        canonical = EXPECTED_FAMILIES[target]
        if intended_owner is not None and intended_owner != canonical:
            errors.append(
                f"{record.get('current_package_name')}: intended release owner {intended_owner!r} "
                f"does not match canonical {canonical!r}"
            )

    if cutover.get("rustPackagesRoleAfterCutover") != EXPECTED_ROLE:
        errors.append(f"rustPackagesRoleAfterCutover must be {EXPECTED_ROLE}")

    if errors:
        print("ownership cutover violations:")
        for error in errors:
            print(f"- {error}")
        return 1

    print(f"ownership cutover: ok ({len(migrated)} migrated Cargo packages)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
