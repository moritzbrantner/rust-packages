#!/usr/bin/env python3
"""Validate reviewed split authority and generate its Markdown projections."""

from __future__ import annotations

import argparse
import glob
import json
import sys
from collections import Counter
from pathlib import Path

from repository_split import (
    BASELINE_PATH,
    OWNERSHIP_PATH,
    ROOT,
    TARGET_REPOSITORIES,
    cargo_metadata,
    load_json,
)

DESTINATION_MATRIX = ROOT / "docs/REPOSITORY_DESTINATION_MATRIX.md"
CONSUMERS_PATH = ROOT / "docs/repository-split/consumer-audit.json"
CONSUMER_MATRIX = ROOT / "docs/CONSUMER_RELEASE_MATRIX.md"
AUDITED_SOURCE_COMMIT = "d032ad2890c1df3c6a5b9eff024562f00d017fce"
ADAPTER_KINDS = {"CLI", "server", "WASM", "npm wrapper", "app"}


def bun_manifest_facts(root: Path = ROOT) -> dict[str, dict]:
    facts = {}
    for pattern in ("packages/*/package.json", "prototypes/web/*/package.json"):
        for raw_path in glob.glob(str(root / pattern)):
            path = Path(raw_path)
            data = json.loads(path.read_text(encoding="utf-8"))
            facts[data["name"]] = {
                "manifest_path": path.relative_to(root).as_posix(),
                "version": data.get("version"),
            }
    return facts


def validate_authority(
    authority: dict,
    metadata: dict,
    bun_facts: dict[str, dict],
    baseline: dict,
) -> list[str]:
    errors: list[str] = []
    if authority.get("source_commit") != AUDITED_SOURCE_COMMIT:
        errors.append(
            f"source_commit must be audited commit {AUDITED_SOURCE_COMMIT}"
        )
    records = authority.get("packages")
    if not isinstance(records, list):
        return errors + ["packages must be a list"]
    ids = [record.get("id") for record in records]
    duplicate_ids = sorted(item for item, count in Counter(ids).items() if count > 1)
    if duplicate_ids:
        errors.append("duplicate ownership ids: " + ", ".join(duplicate_ids))

    cargo_facts = {package["name"]: package for package in metadata.get("packages", [])}
    records_by_ecosystem = {
        ecosystem: {
            record.get("current_package_name"): record
            for record in records
            if record.get("ecosystem") == ecosystem
        }
        for ecosystem in ("cargo", "bun")
    }
    for ecosystem, facts in (("cargo", cargo_facts), ("bun", bun_facts)):
        ecosystem_records = [
            record for record in records if record.get("ecosystem") == ecosystem
        ]
        names = [record.get("current_package_name") for record in ecosystem_records]
        duplicates = sorted(
            name for name, count in Counter(names).items() if count > 1
        )
        if duplicates:
            errors.append(
                f"{ecosystem} packages classified more than once: "
                + ", ".join(duplicates)
            )
        missing = sorted(set(facts) - set(names))
        extra = sorted(set(names) - set(facts))
        if missing:
            errors.append(f"unclassified {ecosystem} packages: " + ", ".join(missing))
        if extra:
            errors.append(
                f"ownership {ecosystem} entries absent from manifests: "
                + ", ".join(extra)
            )

    cargo_records = records_by_ecosystem["cargo"]
    for record in records:
        name = record.get("current_package_name")
        ecosystem = record.get("ecosystem")
        repository = record.get("target_repository")
        if repository not in TARGET_REPOSITORIES:
            errors.append(f"{ecosystem}:{name}: unknown target repository {repository!r}")
        expected_owner = f"moritzbrantner/{repository}"
        if record.get("intended_next_release_owner") != expected_owner:
            errors.append(
                f"{ecosystem}:{name}: intended release owner must be {expected_owner}"
            )
        if ecosystem == "cargo" and name in cargo_facts:
            package = cargo_facts[name]
            actual_manifest = Path(package["manifest_path"]).relative_to(ROOT).as_posix()
            if record.get("manifest_path") != actual_manifest:
                errors.append(
                    f"cargo:{name}: manifest_path {record.get('manifest_path')!r} "
                    f"does not match {actual_manifest!r}"
                )
            if record.get("source_version") != package["version"]:
                errors.append(
                    f"cargo:{name}: source_version {record.get('source_version')!r} "
                    f"does not match {package['version']!r}"
                )
        elif ecosystem == "bun" and name in bun_facts:
            fact = bun_facts[name]
            if record.get("manifest_path") != fact["manifest_path"]:
                errors.append(f"bun:{name}: manifest_path does not match package.json")
            if record.get("source_version") != fact["version"]:
                errors.append(f"bun:{name}: source_version does not match package.json")

        wrapped = record.get("wrapped_library")
        if record.get("package_kind") in ADAPTER_KINDS:
            if not wrapped:
                errors.append(f"{ecosystem}:{name}: adapter is missing wrapped_library")
            elif wrapped not in cargo_records:
                errors.append(
                    f"{ecosystem}:{name}: wrapped library {wrapped!r} is not reviewed Cargo authority"
                )
            elif cargo_records[wrapped].get("target_repository") != repository:
                errors.append(
                    f"{ecosystem}:{name}: target differs from wrapped library {wrapped}"
                )

    expected_annotations: dict[str, list[dict]] = {}
    for violation in baseline.get("violations", []):
        expected_annotations.setdefault(violation["source_package"], []).append(
            {
                "dependency_package": violation["dependency_package"],
                "dependency_kind": violation["dependency_kind"],
                "migration_issue": violation["migration_issue"],
                "target_phase": violation["target_phase"],
            }
        )
    for record in records:
        expected = sorted(
            expected_annotations.get(record["current_package_name"], []),
            key=lambda item: (
                item["dependency_package"],
                item["dependency_kind"],
            ),
        )
        actual = sorted(
            record.get("temporary_boundary_violations", []),
            key=lambda item: (
                item.get("dependency_package", ""),
                item.get("dependency_kind", ""),
            ),
        )
        if actual != expected:
            errors.append(
                f"{record['id']}: temporary boundary annotations differ from exact baseline"
            )
    return errors


def destination_markdown(authority: dict) -> str:
    records = authority["packages"]
    cargo_counts = Counter(
        record["target_repository"]
        for record in records
        if record["ecosystem"] == "cargo"
    )
    lines = [
        "# Repository Destination Matrix",
        "",
        "<!-- Generated by scripts/generate_repository_split_inventory.py; do not edit by hand. -->",
        "",
        f"Source: `docs/repository-split/package-ownership.json` at `{authority['source_commit']}`.",
        "",
        "## Rust ownership totals",
        "",
        "| Target repository | Cargo packages |",
        "| --- | ---: |",
    ]
    for repository in sorted(cargo_counts):
        lines.append(f"| `moritzbrantner/{repository}` | {cargo_counts[repository]} |")
    lines.extend(
        [
            "",
            "## Package destinations",
            "",
            "| Ecosystem | Package | Kind | Contract owner | Wrapped library | Target | Phase | Publish class |",
            "| --- | --- | --- | --- | --- | --- | --- | --- |",
        ]
    )
    for record in records:
        wrapped = f"`{record['wrapped_library']}`" if record["wrapped_library"] else "—"
        lines.append(
            f"| {record['ecosystem']} | `{record['current_package_name']}` | "
            f"{record['package_kind']} | `{record['contract_owner']}` | {wrapped} | "
            f"`moritzbrantner/{record['target_repository']}` | "
            f"{record['extraction_phase']} | {record['publication_class']} |"
        )
    return "\n".join(lines) + "\n"


def format_dependency(dependency: dict) -> str:
    alias = dependency["alias"]
    package = dependency["package"]
    requirement = dependency.get("requirement") or "none"
    source = dependency["source_type"]
    features = ", ".join(dependency.get("features", [])) or "none"
    return (
        f"`{alias}` → `{package}` `{requirement}` ({source}; features: {features}; "
        f"evidence: `{dependency['evidence']}`)"
    )


def consumer_markdown(source: dict) -> str:
    lines = [
        "# Consumer And Release Matrix",
        "",
        "<!-- Generated by scripts/generate_repository_split_inventory.py; do not edit by hand. -->",
        "",
        "This is an audit record, not build evidence. `inspected` means manifests and",
        "documentation were read; no consumer is marked verified unless its commands ran.",
        "",
        "| Consumer | Relationship | Exact dependencies / evidence | Clean checkout | Local development | Release order | Verification | Issue |",
        "| --- | --- | --- | --- | --- | --- | --- | --- |",
    ]
    for row in source["consumers"]:
        dependencies = row.get("dependencies", [])
        details = (
            "<br>".join(format_dependency(item) for item in dependencies)
            if dependencies
            else "<br>".join(row["packages"]) or "schema/overlap only"
        )
        lines.append(
            f"| `{row['repository']}` | {row['relationship']} | {details} | "
            f"{row['clean_checkout_expectation']} | {row['local_codevelopment']} | "
            f"{' → '.join(row['release_order'])} | "
            f"`{'`; `'.join(row['verification_commands'])}` | "
            f"[#{row['migration_issue']}](https://github.com/moritzbrantner/rust-packages/issues/{row['migration_issue']}) |"
        )
    lines.extend(["", "## Second-order chains", ""])
    for chain in source["second_order_chains"]:
        lines.append(f"- {' → '.join(f'`{part}`' for part in chain)}")
    lines.extend(
        [
            "",
            "## Namespace and registry evidence",
            "",
            "- All 347 active Cargo workspace packages use the `moenarch-*` namespace.",
            "- The registry audit found 43 matching workspace libraries and did not find 304 active package names through the broad search; this is evidence, not proof that every missing name has never existed.",
            "- `moenarch-text-transcripts` is `0.1.1` in source while crates.io reports `0.1.2`.",
            "- Runtime core, jobs, video core, model runtime, text transcripts, and audio core occur under both historical `moritzbrantner-*` and current `moenarch-*` names in consumer/release material.",
            "- Repository metadata is mixed and must be corrected only by each package's release-owner issue.",
        ]
    )
    return "\n".join(lines) + "\n"


def generate() -> tuple[dict, str, str, list[str]]:
    authority = load_json(OWNERSHIP_PATH)
    baseline = load_json(BASELINE_PATH)
    consumers = load_json(CONSUMERS_PATH)
    errors = validate_authority(
        authority,
        cargo_metadata(),
        bun_manifest_facts(),
        baseline,
    )
    return (
        authority,
        destination_markdown(authority),
        consumer_markdown(consumers),
        errors,
    )


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    authority, destination, consumers, errors = generate()
    if errors:
        for error in errors:
            print(f"error: {error}", file=sys.stderr)
        return 1
    expected = {
        DESTINATION_MATRIX: destination,
        CONSUMER_MATRIX: consumers,
    }
    if args.check:
        stale = [
            path.relative_to(ROOT).as_posix()
            for path, content in expected.items()
            if not path.exists() or path.read_text(encoding="utf-8") != content
        ]
        if stale:
            print("out of date: " + ", ".join(stale), file=sys.stderr)
            return 1
        print(
            f"reviewed ownership is valid and matrices are current "
            f"({len(authority['packages'])} packages)"
        )
        return 0
    for path, content in expected.items():
        path.write_text(content, encoding="utf-8")
    print(
        f"wrote matrices from reviewed ownership for "
        f"{len(authority['packages'])} packages"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
