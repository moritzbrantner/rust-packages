#!/usr/bin/env python3
"""Validate the current owner decisions for the retired spatial-analysis plan."""

from __future__ import annotations

import re
from pathlib import Path

from repository_split import OWNERSHIP_PATH, ROOT, load_json, ownership_records, wrapped_library_name

RECONCILIATION_PATH = ROOT / "docs/repository-split/spatial-reconciliation.json"
ALLOWED_CURRENT_REPOSITORIES = {
    "moritzbrantner/3d-lab",
    "moritzbrantner/video-to-3d",
    "moritzbrantner/rust-packages",
}
DECISIONS = {"canonical-destination", "retained-temporarily"}
FULL_SHA_RE = re.compile(r"^[0-9a-f]{40}$")
GITHUB_REPO_RE = re.compile(r"^moritzbrantner/[a-z0-9][a-z0-9-]*$")


def family_for(record: dict) -> str:
    return wrapped_library_name(record["current_package_name"])


def validate(authority: dict, reconciliation: dict) -> list[str]:
    errors: list[str] = []

    if reconciliation.get("schemaVersion") != 1:
        errors.append("spatial reconciliation schemaVersion must be 1")

    retired = reconciliation.get("retiredTargetRepository")
    if retired != "spatial-analysis":
        errors.append("retiredTargetRepository must be spatial-analysis")

    if reconciliation.get("issue") != "https://github.com/moritzbrantner/rust-packages/issues/179":
        errors.append("spatial reconciliation must remain anchored to issue #179")

    family_items = reconciliation.get("families")
    if not isinstance(family_items, list):
        return errors + ["families must be a list"]

    family_names = [item.get("family") for item in family_items if isinstance(item, dict)]
    if len(family_names) != len(set(family_names)):
        errors.append("spatial reconciliation contains duplicate family decisions")
    families = {
        item.get("family"): item
        for item in family_items
        if isinstance(item, dict) and isinstance(item.get("family"), str)
    }

    stale_records = [
        record
        for record in ownership_records(authority)
        if record.get("target_repository") == retired
    ]
    stale_families = {family_for(record) for record in stale_records}
    missing = sorted(stale_families - set(families))
    extra = sorted(set(families) - stale_families)
    if missing:
        errors.append("spatial families without a current decision: " + ", ".join(missing))
    if extra:
        errors.append("spatial decisions without reviewed packages: " + ", ".join(extra))

    for family in sorted(families):
        item = families[family]
        decision = item.get("decision")
        repository = item.get("currentAuthorityRepository")
        replacements = item.get("replacementSurfaces")
        role = item.get("historicalPackageRole")
        reason = item.get("reason")

        if decision not in DECISIONS:
            errors.append(f"{family}: decision must be one of {sorted(DECISIONS)}")
        if repository not in ALLOWED_CURRENT_REPOSITORIES:
            errors.append(f"{family}: unsupported current authority {repository!r}")
        if isinstance(repository, str) and not GITHUB_REPO_RE.fullmatch(repository):
            errors.append(f"{family}: malformed current authority repository")
        if repository == "moritzbrantner/spatial-analysis":
            errors.append(f"{family}: retired spatial-analysis cannot remain authoritative")
        if not isinstance(replacements, list) or not all(
            isinstance(value, str) and value.strip() for value in replacements
        ):
            errors.append(f"{family}: replacementSurfaces must be a string list")
        if not isinstance(reason, str) or not reason.strip():
            errors.append(f"{family}: reason must be non-empty")

        if decision == "canonical-destination":
            if repository == "moritzbrantner/rust-packages":
                errors.append(f"{family}: canonical destination cannot be rust-packages")
            if not replacements:
                errors.append(f"{family}: canonical destination needs a proven replacement surface")
            if role != "compatibility-provenance-only":
                errors.append(
                    f"{family}: canonical destination historical role must be compatibility-provenance-only"
                )
        elif decision == "retained-temporarily":
            if repository != "moritzbrantner/rust-packages":
                errors.append(f"{family}: retained family must remain owned by rust-packages")
            if role != "residual-compatibility-authority":
                errors.append(
                    f"{family}: retained historical role must be residual-compatibility-authority"
                )

        seams = item.get("extractedSeams", [])
        if not isinstance(seams, list):
            errors.append(f"{family}: extractedSeams must be a list when present")
            continue
        for index, seam in enumerate(seams):
            if not isinstance(seam, dict):
                errors.append(f"{family}: extracted seam {index} must be an object")
                continue
            required = {"name", "canonicalRepository", "surface", "evidence"}
            if set(seam) != required:
                errors.append(
                    f"{family}: extracted seam {index} fields must be {', '.join(sorted(required))}"
                )
                continue
            canonical = seam.get("canonicalRepository")
            if canonical not in ALLOWED_CURRENT_REPOSITORIES - {"moritzbrantner/rust-packages"}:
                errors.append(f"{family}: extracted seam {index} has unsupported canonical owner")
            if not all(
                isinstance(seam.get(field), str) and seam[field].strip()
                for field in ("name", "surface", "evidence")
            ):
                errors.append(f"{family}: extracted seam {index} must be fully described")
            evidence = seam.get("evidence")
            if isinstance(evidence, str) and not evidence.startswith("https://github.com/moritzbrantner/"):
                errors.append(f"{family}: extracted seam {index} evidence must be a GitHub URL")

    evidence = reconciliation.get("evidence")
    if not isinstance(evidence, dict):
        errors.append("evidence must be an object")
    else:
        for key in ("threeDSpatialProvider", "colmapProvider", "consumerGate"):
            item = evidence.get(key)
            if not isinstance(item, dict):
                errors.append(f"evidence.{key} must be an object")
                continue
            commit = item.get("commit")
            if not isinstance(commit, str) or not FULL_SHA_RE.fullmatch(commit):
                errors.append(f"evidence.{key}.commit must be an exact full commit")
            repository = item.get("repository")
            if not isinstance(repository, str) or not GITHUB_REPO_RE.fullmatch(repository):
                errors.append(f"evidence.{key}.repository must be a moritzbrantner repository")
            pull_request = item.get("pullRequest")
            if not isinstance(pull_request, str) or not pull_request.startswith(
                "https://github.com/moritzbrantner/"
            ):
                errors.append(f"evidence.{key}.pullRequest must be a GitHub PR URL")
        consumer = evidence.get("consumerGate")
        if isinstance(consumer, dict) and consumer.get("rustPackagesSourceDependencyRemoved") is not True:
            errors.append("consumerGate must prove the rust-packages source dependency was removed")

    # Every immutable package previously sent to the nonexistent monolith must now resolve
    # through exactly one family decision. Adapter packages inherit their wrapped library family.
    for record in stale_records:
        family = family_for(record)
        if family not in families:
            errors.append(f"{record.get('id')}: no current spatial authority")

    return errors


def main() -> int:
    authority = load_json(OWNERSHIP_PATH)
    reconciliation = load_json(RECONCILIATION_PATH)
    errors = validate(authority, reconciliation)
    if errors:
        print("spatial reconciliation violations:")
        for error in errors:
            print(f"- {error}")
        return 1

    retired = reconciliation["retiredTargetRepository"]
    records = [
        record
        for record in ownership_records(authority)
        if record.get("target_repository") == retired
    ]
    print(
        "spatial reconciliation: ok "
        f"({len(records)} historical packages, {len(reconciliation['families'])} families)"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
