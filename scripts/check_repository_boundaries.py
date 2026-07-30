#!/usr/bin/env python3
"""Enforce capability-repository ownership and dependency boundaries."""

from __future__ import annotations

import argparse
import json
import sys
from collections import Counter
from pathlib import Path

from repository_split import (
    ALLOWED_DEPENDENCIES,
    BASELINE_PATH,
    OWNERSHIP_PATH,
    TARGET_REPOSITORIES,
    cargo_metadata,
    find_cycle,
    internal_dependency_edges,
    load_json,
    ownership_records,
    write_json,
)


def validate(
    metadata: dict, ownership: dict, baseline: dict
) -> tuple[list[str], list[dict], list[list[str]]]:
    errors: list[str] = []
    cargo_packages = {
        package["name"]: package for package in metadata.get("packages", [])
    }
    target_dependency_document = ownership.get("target_repository_dependencies")
    if target_dependency_document is None:
        errors.append("missing target_repository_dependencies")
        target_dependency_document = {}
    target_dependencies: dict[str, set[str]] = {}
    if not isinstance(target_dependency_document, dict):
        errors.append("target_repository_dependencies must be an object")
        target_dependency_document = {}
    missing_repositories = sorted(
        TARGET_REPOSITORIES - set(target_dependency_document)
    )
    extra_repositories = sorted(
        set(target_dependency_document) - TARGET_REPOSITORIES
    )
    if missing_repositories:
        errors.append(
            "target repository graph is missing: " + ", ".join(missing_repositories)
        )
    if extra_repositories:
        errors.append(
            "target repository graph has unknown repositories: "
            + ", ".join(extra_repositories)
        )
    for source, dependencies in target_dependency_document.items():
        if source not in TARGET_REPOSITORIES:
            continue
        if not isinstance(dependencies, list) and not isinstance(dependencies, set):
            errors.append(f"{source}: target dependencies must be a list")
            continue
        unknown_dependencies = sorted(set(dependencies) - TARGET_REPOSITORIES)
        if unknown_dependencies:
            errors.append(
                f"{source}: unknown target dependencies: "
                + ", ".join(unknown_dependencies)
            )
        target_dependencies[source] = set(dependencies) & TARGET_REPOSITORIES
    if target_dependencies != ALLOWED_DEPENDENCIES:
        errors.append(
            "reviewed target repository graph does not match required "
            "directional law"
        )
    target_cycle = find_cycle(
        TARGET_REPOSITORIES,
        (
            (source, dependency)
            for source, dependencies in target_dependencies.items()
            for dependency in dependencies
        ),
    )
    target_cycles = [target_cycle] if target_cycle else []
    if target_cycle:
        errors.append(
            "reviewed target repository graph is cyclic: "
            + " -> ".join(target_cycle)
        )
    cargo_ownership_records = [
        record
        for record in ownership_records(ownership)
        if record.get("ecosystem") == "cargo"
    ]
    names = [record.get("current_package_name") for record in cargo_ownership_records]
    duplicates = sorted(name for name, count in Counter(names).items() if count > 1)
    if duplicates:
        errors.append("packages classified more than once: " + ", ".join(duplicates))
    missing = sorted(set(cargo_packages) - set(names))
    extra = sorted(set(names) - set(cargo_packages))
    if missing:
        errors.append("unclassified Cargo packages: " + ", ".join(missing))
    if extra:
        errors.append("ownership entries absent from cargo metadata: " + ", ".join(extra))

    owners: dict[str, str] = {}
    records_by_name = {}
    for record in cargo_ownership_records:
        name = record.get("current_package_name")
        repository = record.get("target_repository")
        records_by_name[name] = record
        if repository not in TARGET_REPOSITORIES:
            errors.append(f"{name}: unknown target repository {repository!r}")
        owners[name] = repository
        if record.get("package_kind") in {"CLI", "server", "WASM"}:
            wrapped = record.get("wrapped_library")
            if not wrapped:
                errors.append(f"{name}: adapter is missing wrapped_library")
            elif wrapped not in cargo_packages:
                errors.append(f"{name}: wrapped library {wrapped!r} is not a Cargo package")
            elif owners.get(wrapped, repository) != repository:
                # The final equality is checked again after all owners are loaded.
                pass
    for name, record in records_by_name.items():
        wrapped = record.get("wrapped_library")
        if wrapped and wrapped in owners and owners[wrapped] != owners[name]:
            errors.append(
                f"{name}: adapter target {owners[name]} differs from wrapped "
                f"library {wrapped} target {owners[wrapped]}"
            )

    baseline_entries = baseline.get("violations", [])
    baseline_keys: list[tuple[str, str, str, bool]] = []
    for index, entry in enumerate(baseline_entries):
        key = (
            entry.get("source_package"),
            entry.get("dependency_package"),
            entry.get("dependency_kind"),
            bool(entry.get("optional", False)),
        )
        baseline_keys.append(key)
        for field in (
            "source_package",
            "dependency_package",
            "dependency_kind",
            "reason",
            "migration_issue",
            "target_phase",
        ):
            value = entry.get(field)
            if not isinstance(value, str) or not value.strip():
                errors.append(f"baseline entry {index}: missing {field}")
            elif "*" in value:
                errors.append(f"baseline entry {index}: wildcard in {field}")
        if entry.get("dependency_kind") not in {"normal", "build", "dev"}:
            errors.append(f"baseline entry {index}: invalid dependency kind")
        if "optional" in entry and not isinstance(entry.get("optional"), bool):
            errors.append(f"baseline entry {index}: optional must be a boolean")
        issue = entry.get("migration_issue", "")
        if not issue.startswith(
            "https://github.com/moritzbrantner/rust-packages/issues/"
        ):
            errors.append(f"baseline entry {index}: invalid migration issue reference")
    duplicate_baseline = sorted(
        key for key, count in Counter(baseline_keys).items() if count > 1
    )
    if duplicate_baseline:
        errors.append(f"duplicate baseline entries: {duplicate_baseline}")

    violations = []
    for edge in internal_dependency_edges(metadata):
        source = edge["source_package"]
        dependency = edge["dependency_package"]
        if source not in owners or dependency not in owners:
            continue
        source_owner = owners[source]
        dependency_owner = owners[dependency]
        if source_owner == dependency_owner:
            continue
        kind = edge["dependency_kind"]
        optional = bool(edge.get("optional"))
        key = (source, dependency, kind, optional)
        if dependency_owner not in ALLOWED_DEPENDENCIES.get(source_owner, set()):
            violation = {
                **edge,
                "dependency_kind": kind,
                "source_repository": source_owner,
                "dependency_repository": dependency_owner,
            }
            violations.append(violation)
            if key not in baseline_keys:
                errors.append(
                    f"new forbidden edge: {source} -> {dependency} "
                    f"({kind} {'optional' if optional else 'required'}; "
                    f"{source_owner} -> {dependency_owner})"
                )

    actual_keys = {
        (
            edge["source_package"],
            edge["dependency_package"],
            edge["dependency_kind"],
            bool(edge.get("optional")),
        )
        for edge in violations
    }
    stale = sorted(set(baseline_keys) - actual_keys)
    if stale:
        errors.append(
            "stale baseline violations must be removed after the edge is fixed: "
            + ", ".join(
                f"{source}->{dependency}({kind} "
                f"{'optional' if optional else 'required'})"
                for source, dependency, kind, optional in stale
            )
        )
    return errors, violations, target_cycles


def baseline_entry(edge: dict) -> dict:
    source = edge["source_package"]
    dependency = edge["dependency_package"]
    if source == "moenarch-video-analysis-detectors":
        issue, phase, reason = 107, "phase-a", "test support remains with the compatibility repository"
    elif edge["source_repository"] == "visual-analysis" and edge["dependency_repository"] == "spatial-analysis":
        issue, phase, reason = 118, "visual", "visual implementation still imports spatial contracts"
    elif source == "moenarch-text-transcripts":
        issue, phase, reason = 112, "nlp", "transcript contracts still import audio or video implementation"
    elif dependency in {
        "moenarch-video-analysis-ffmpeg",
        "moenarch-video-analysis-ingest",
    } and edge["source_repository"] == "audio-analysis":
        issue, phase, reason = 74, "foundation", "audio decoding still imports visual media IO"
    else:
        issue, phase, reason = 109, "foundation", "inappropriate dependency on video-analysis-core remains"
    return {
        "source_package": source,
        "dependency_package": dependency,
        "dependency_kind": edge["dependency_kind"],
        "optional": bool(edge.get("optional")),
        "reason": reason,
        "migration_issue": f"https://github.com/moritzbrantner/rust-packages/issues/{issue}",
        "target_phase": phase,
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--check", action="store_true")
    parser.add_argument("--write-baseline", action="store_true")
    parser.add_argument("--metadata", type=Path)
    parser.add_argument("--ownership", type=Path, default=OWNERSHIP_PATH)
    parser.add_argument("--baseline", type=Path, default=BASELINE_PATH)
    args = parser.parse_args()
    metadata = load_json(args.metadata) if args.metadata else cargo_metadata()
    ownership = load_json(args.ownership)
    if args.write_baseline:
        empty = {"schema_version": 1, "violations": []}
        _, violations, _ = validate(metadata, ownership, empty)
        document = {
            "schema_version": 1,
            "repository": "moritzbrantner/rust-packages",
            "source_commit": ownership["source_commit"],
            "violations": [baseline_entry(edge) for edge in violations],
        }
        write_json(args.baseline, document)
        print(f"wrote {len(violations)} exact baseline violations")
        return 0
    baseline = load_json(args.baseline)
    errors, violations, target_cycles = validate(metadata, ownership, baseline)
    for edge in violations:
        print(
            f"BASELINED {edge['source_package']} -> {edge['dependency_package']} "
            f"({edge['dependency_kind']}; {edge['source_repository']} -> "
            f"{edge['dependency_repository']})"
        )
    for cycle in target_cycles:
        print("INVALID TARGET CYCLE " + " -> ".join(cycle))
    if errors:
        for error in errors:
            print(f"error: {error}", file=sys.stderr)
        return 1
    counts = Counter(edge["dependency_kind"] for edge in violations)
    print(
        f"repository boundaries pass: {len(ownership['packages'])} ownership records, "
        f"{len(violations)} reviewed violations "
        f"({', '.join(f'{count} {kind}' for kind, count in sorted(counts.items()))})"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
