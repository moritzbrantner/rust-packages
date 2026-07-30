#!/usr/bin/env python3
"""Shared, standard-library-only helpers for the capability repository split."""

from __future__ import annotations

import hashlib
import json
import re
import subprocess
from collections import Counter
from pathlib import Path
from typing import Iterable

ROOT = Path(__file__).resolve().parents[1]
OWNERSHIP_PATH = ROOT / "docs/repository-split/package-ownership.json"
BASELINE_PATH = ROOT / "docs/repository-split/dependency-boundary-baseline.json"

TARGET_REPOSITORIES = {
    "moenarch-foundation",
    "nlp-stack",
    "audio-analysis",
    "visual-analysis",
    "spatial-analysis",
    "rust-packages",
}

ALLOWED_DEPENDENCIES = {
    "moenarch-foundation": set(),
    "nlp-stack": {"moenarch-foundation"},
    "audio-analysis": {"moenarch-foundation", "nlp-stack"},
    "visual-analysis": {"moenarch-foundation", "nlp-stack"},
    "spatial-analysis": {"moenarch-foundation", "visual-analysis"},
    "rust-packages": {
        "moenarch-foundation",
        "nlp-stack",
        "audio-analysis",
        "visual-analysis",
        "spatial-analysis",
    },
}

SPATIAL_VIDEO_FAMILIES = {
    "video-analysis-gaussian-splatting",
    "video-analysis-mvs",
    "video-analysis-posture",
    "video-analysis-posture-io",
    "video-analysis-radiance-fields",
    "video-analysis-radiance-io",
    "video-analysis-radiance-pipeline",
    "video-analysis-reconstruction",
    "video-analysis-sfm",
}

NON_LIBRARY_SUFFIXES = ("-cli", "-server", "-wasm", "-app")
SEMVER_RE = re.compile(
    r"^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)"
    r"(?:-([0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*))?"
    r"(?:\+([0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*))?$"
)
FULL_SHA_RE = re.compile(r"^[0-9a-f]{40}$")
AUDITED_SOURCE_COMMIT = "d032ad2890c1df3c6a5b9eff024562f00d017fce"
PHASE_A_PACKAGES_SHA256 = (
    "ddfea012979f6ee13483d4c70a9702c29a595e2a4202c3490a8350ee7f78a6bd"
)
CREATING_ISSUE_RE = re.compile(
    r"^https://github\.com/moritzbrantner/rust-packages/issues/[1-9]\d*$"
)


def load_json(path: Path) -> dict:
    with path.open(encoding="utf-8") as handle:
        return json.load(handle)


def ownership_records(authority: dict) -> list[dict]:
    baseline = authority.get("packages")
    post_baseline = authority.get("post_baseline_packages")
    baseline_records = baseline if isinstance(baseline, list) else []
    post_baseline_records = post_baseline if isinstance(post_baseline, list) else []
    return [*baseline_records, *post_baseline_records]


def boundary_violation_key(
    source_package: object,
    violation: dict,
) -> tuple[object, object, object, bool, object, object]:
    """Return the exact identity shared by annotations, baselines, and resolutions."""

    return (
        source_package,
        violation.get("dependency_package"),
        violation.get("dependency_kind"),
        bool(violation.get("optional", False)),
        violation.get("migration_issue"),
        violation.get("target_phase"),
    )


def boundary_resolution_keys(authority: dict) -> set[tuple]:
    """Return well-shaped resolution identities for current projections."""

    amendments = authority.get("resolved_boundary_violations")
    if not isinstance(amendments, list):
        return set()
    return {
        boundary_violation_key(amendment.get("source_package"), amendment)
        for amendment in amendments
        if isinstance(amendment, dict)
    }


def validate_boundary_resolution_amendments(
    authority: dict,
    baseline: dict | None = None,
) -> list[str]:
    """Validate append-only resolutions without mutating Phase A records."""

    errors: list[str] = []
    amendments = authority.get("resolved_boundary_violations")
    if not isinstance(amendments, list):
        return ["resolved_boundary_violations must be a list"]

    immutable_keys = {
        boundary_violation_key(record.get("current_package_name"), violation)
        for record in authority.get("packages", [])
        if isinstance(record, dict)
        for violation in record.get("temporary_boundary_violations", [])
        if isinstance(violation, dict)
    }
    amendment_keys: list[tuple] = []
    required_fields = {
        "source_package",
        "dependency_package",
        "dependency_kind",
        "optional",
        "migration_issue",
        "target_phase",
        "resolved_by_issue",
    }
    for index, amendment in enumerate(amendments):
        if not isinstance(amendment, dict):
            errors.append(f"boundary resolution {index}: must be an object")
            continue
        if set(amendment) != required_fields:
            errors.append(
                f"boundary resolution {index}: fields must be "
                + ", ".join(sorted(required_fields))
            )
        key = boundary_violation_key(amendment.get("source_package"), amendment)
        amendment_keys.append(key)
        if key not in immutable_keys:
            errors.append(
                f"boundary resolution {index}: does not match an immutable Phase A annotation"
            )
        if amendment.get("dependency_kind") not in {"normal", "build", "dev"}:
            errors.append(f"boundary resolution {index}: invalid dependency kind")
        if not isinstance(amendment.get("optional"), bool):
            errors.append(f"boundary resolution {index}: optional must be a boolean")
        resolved_by_issue = amendment.get("resolved_by_issue")
        if not isinstance(resolved_by_issue, str) or not CREATING_ISSUE_RE.fullmatch(
            resolved_by_issue
        ):
            errors.append(f"boundary resolution {index}: invalid resolution issue")

    duplicates = sorted(
        (
            key
            for key, count in Counter(amendment_keys).items()
            if count > 1
        ),
        key=repr,
    )
    if duplicates:
        errors.append(f"duplicate boundary resolutions: {duplicates}")

    if baseline is not None:
        baseline_keys = {
            boundary_violation_key(entry.get("source_package"), entry)
            for entry in baseline.get("violations", [])
            if isinstance(entry, dict)
        }
        still_baselined = sorted(
            set(amendment_keys) & baseline_keys,
            key=repr,
        )
        if still_baselined:
            errors.append(
                "resolved boundary violations must be absent from the current baseline: "
                + ", ".join(
                    f"{source}->{dependency}({kind} "
                    f"{'optional' if optional else 'required'})"
                    for source, dependency, kind, optional, _, _ in still_baselined
                )
            )
    return errors


def validate_ownership_authority(
    authority: dict,
    *,
    root: Path = ROOT,
) -> list[str]:
    """Validate the immutable Phase A list and every later package's provenance."""

    errors: list[str] = []
    if authority.get("schema_version") != 2:
        errors.append("schema_version must be 2")
    if authority.get("source_commit") != AUDITED_SOURCE_COMMIT:
        errors.append(
            f"source_commit must be audited commit {AUDITED_SOURCE_COMMIT}"
        )
    baseline_records = authority.get("packages")
    if not isinstance(baseline_records, list):
        return errors + ["packages must be a list"]
    encoded_baseline = json.dumps(
        baseline_records,
        sort_keys=True,
        separators=(",", ":"),
    ).encode()
    if hashlib.sha256(encoded_baseline).hexdigest() != PHASE_A_PACKAGES_SHA256:
        errors.append(
            "immutable Phase A packages differ from the reviewed baseline"
        )
    errors.extend(validate_boundary_resolution_amendments(authority))
    post_baseline_records = authority.get("post_baseline_packages")
    if not isinstance(post_baseline_records, list):
        return errors + ["post_baseline_packages must be a list"]
    for index, record in enumerate(post_baseline_records):
        provenance = record.get("provenance") if isinstance(record, dict) else None
        if not isinstance(provenance, dict):
            errors.append(f"post-baseline package {index}: missing provenance")
            continue
        if set(provenance) != {"introduced_after_commit", "issue"}:
            errors.append(
                f"post-baseline package {index}: provenance must contain only "
                "introduced_after_commit and issue"
            )
        commit = provenance.get("introduced_after_commit")
        if not isinstance(commit, str) or not FULL_SHA_RE.fullmatch(commit):
            errors.append(
                f"post-baseline package {index}: introduced_after_commit "
                "must be an exact full commit"
            )
        elif not git_commit_exists(root, commit):
            errors.append(
                f"post-baseline package {index}: introduced_after_commit is unavailable"
            )
        elif not git_commit_is_ancestor(root, AUDITED_SOURCE_COMMIT, commit):
            errors.append(
                f"post-baseline package {index}: introduced_after_commit "
                "must not predate the Phase A audit"
            )
        elif not git_commit_is_ancestor(root, commit):
            errors.append(
                f"post-baseline package {index}: introduced_after_commit "
                "must be an ancestor of HEAD"
            )
        issue = provenance.get("issue")
        if not isinstance(issue, str) or not CREATING_ISSUE_RE.fullmatch(issue):
            errors.append(f"post-baseline package {index}: invalid creating issue")
    return errors


def git_commit_exists(root: Path, commit: str) -> bool:
    if not FULL_SHA_RE.fullmatch(commit):
        return False
    return (
        subprocess.run(
            ["git", "cat-file", "-e", f"{commit}^{{commit}}"],
            cwd=root,
            check=False,
            capture_output=True,
        ).returncode
        == 0
    )


def git_commit_is_ancestor(root: Path, ancestor: str, descendant: str = "HEAD") -> bool:
    return (
        subprocess.run(
            ["git", "merge-base", "--is-ancestor", ancestor, descendant],
            cwd=root,
            check=False,
            capture_output=True,
        ).returncode
        == 0
    )


def write_json(path: Path, value: object) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def cargo_metadata(root: Path = ROOT) -> dict:
    completed = subprocess.run(
        ["cargo", "metadata", "--format-version", "1", "--no-deps"],
        cwd=root,
        check=True,
        capture_output=True,
        text=True,
    )
    return json.loads(completed.stdout)


def strip_publisher(name: str) -> str:
    for prefix in ("moenarch-", "moritzbrantner-"):
        if name.startswith(prefix):
            return name[len(prefix) :]
    return name.split("/", 1)[-1]


def wrapped_library_name(name: str) -> str:
    base = strip_publisher(name)
    for suffix in NON_LIBRARY_SUFFIXES:
        if base.endswith(suffix):
            return base[: -len(suffix)]
    return base


def target_repository(name: str) -> str:
    base = wrapped_library_name(name)
    if base in SPATIAL_VIDEO_FAMILIES or base.startswith(("three-d-", "animation-")):
        return "spatial-analysis"
    if base.startswith("text-"):
        return "nlp-stack"
    if base.startswith("audio-"):
        return "audio-analysis"
    if base == "image-analysis-comfyui" or base.startswith("comfyui-"):
        return "rust-packages"
    if base.startswith(("image-", "vision-", "video-")) and name not in {
        "moenarch-video-analysis",
        "moenarch-video-analysis-cli",
        "moenarch-video-analysis-test-support",
        "moenarch-video-analysis-use-cases",
    }:
        return "visual-analysis"
    if base.startswith(
        (
            "data-",
            "dense-",
            "graph-",
            "jobs-",
            "math-",
            "model-",
            "numbers-",
            "runtime-",
            "tensor-",
            "vector-",
        )
    ):
        return "moenarch-foundation"
    return "rust-packages"


def package_kind(name: str, manifest_path: str, cargo: bool) -> str:
    base = strip_publisher(name)
    if base.endswith("-cli"):
        return "CLI"
    if base.endswith("-server"):
        return "server"
    if base.endswith("-wasm"):
        return "WASM" if cargo else "npm wrapper"
    if base.endswith("-app") or manifest_path.startswith("prototypes/"):
        return "app" if not manifest_path.startswith("prototypes/") else "prototype"
    if "test-support" in base or "benchmarks" in base:
        return "test support"
    if name == "moenarch-video-analysis":
        return "facade"
    return "library"


def dependency_kind(dependency: dict) -> str:
    return dependency.get("kind") or "normal"


def internal_dependency_edges(metadata: dict) -> list[dict]:
    """Return every distinct workspace dependency edge and declaration kind."""

    packages = {package["name"]: package for package in metadata["packages"]}
    edges_by_key: dict[tuple[str, str, str, bool], dict] = {}
    for package in metadata["packages"]:
        for dependency in package["dependencies"]:
            if dependency["name"] in packages:
                kind = dependency_kind(dependency)
                optional = bool(dependency.get("optional"))
                key = (package["name"], dependency["name"], kind, optional)
                edge = edges_by_key.setdefault(
                    key,
                    {
                        "source_package": package["name"],
                        "dependency_package": dependency["name"],
                        "dependency_kind": kind,
                        "optional": optional,
                        "declaration_kinds": [kind],
                    },
                )
    return [edges_by_key[key] for key in sorted(edges_by_key)]


def find_cycle(nodes: Iterable[str], edges: Iterable[tuple[str, str]]) -> list[str] | None:
    adjacency: dict[str, set[str]] = {node: set() for node in nodes}
    for source, dependency in edges:
        adjacency.setdefault(source, set()).add(dependency)
        adjacency.setdefault(dependency, set())
    visiting: set[str] = set()
    visited: set[str] = set()
    path: list[str] = []

    def visit(node: str) -> list[str] | None:
        if node in visiting:
            start = path.index(node)
            return path[start:] + [node]
        if node in visited:
            return None
        visiting.add(node)
        path.append(node)
        for dependency in sorted(adjacency[node]):
            cycle = visit(dependency)
            if cycle:
                return cycle
        path.pop()
        visiting.remove(node)
        visited.add(node)
        return None

    for node in sorted(adjacency):
        cycle = visit(node)
        if cycle:
            return cycle
    return None


def find_cycles(
    nodes: Iterable[str], edges: Iterable[tuple[str, str]]
) -> list[list[str]]:
    """Return canonical simple directed cycles for the small repository graph."""

    adjacency: dict[str, set[str]] = {node: set() for node in nodes}
    for source, dependency in edges:
        adjacency.setdefault(source, set()).add(dependency)
        adjacency.setdefault(dependency, set())
    found: set[tuple[str, ...]] = set()

    def canonical(path: list[str]) -> tuple[str, ...]:
        cycle = path[:-1]
        rotations = [tuple(cycle[index:] + cycle[:index]) for index in range(len(cycle))]
        selected = min(rotations)
        return selected + (selected[0],)

    def walk(start: str, node: str, path: list[str]) -> None:
        for dependency in sorted(adjacency[node]):
            if dependency == start:
                found.add(canonical(path + [start]))
            elif dependency not in path:
                walk(start, dependency, path + [dependency])

    for start in sorted(adjacency):
        walk(start, start, [start])
    return [list(cycle) for cycle in sorted(found)]


def topological_order(nodes: Iterable[str], edges: Iterable[tuple[str, str]]) -> list[str]:
    nodes = list(dict.fromkeys(nodes))
    dependencies: dict[str, set[str]] = {node: set() for node in nodes}
    dependents: dict[str, set[str]] = {node: set() for node in nodes}
    for dependent, dependency in edges:
        dependencies[dependent].add(dependency)
        dependents[dependency].add(dependent)
    ready = sorted(node for node in nodes if not dependencies[node])
    result = []
    while ready:
        node = ready.pop(0)
        result.append(node)
        for dependent in sorted(dependents[node]):
            dependencies[dependent].discard(node)
            if not dependencies[dependent] and dependent not in result and dependent not in ready:
                ready.append(dependent)
                ready.sort()
    if len(result) != len(nodes):
        cycle = find_cycle(nodes, edges)
        raise ValueError(f"dependency cycle: {' -> '.join(cycle or [])}")
    return result
