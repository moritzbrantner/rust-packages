#!/usr/bin/env python3
"""Validate an exact release plan and the real Cargo manifests it names."""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
import tomllib
from collections import Counter
from pathlib import Path
from typing import Any

from repository_split import (
    FULL_SHA_RE,
    OWNERSHIP_PATH,
    ROOT,
    SEMVER_RE,
    find_cycle,
    load_json,
)

REQUIRED_LIST_FIELDS = (
    "required_features",
    "compatibility_or_deprecation_packages",
    "required_checks",
    "expected_tags",
    "downstream_consumers",
    "required_consumer_checks",
    "dependency_order",
)
DEPENDENCY_SECTIONS = ("dependencies", "build-dependencies", "dev-dependencies")
AUTHORIZATION_BLOCK_RE = re.compile(
    r"```json\s*\n(?P<payload>.*?)\n```",
    re.DOTALL | re.IGNORECASE,
)


def load_document(path: Path) -> dict:
    with path.open("rb") as handle:
        if path.suffix == ".json":
            return json.load(handle)
        if path.suffix == ".toml":
            return tomllib.load(handle)
    raise ValueError(f"release manifest must be JSON or TOML: {path}")


def extract_release_authorization(issue_body: str) -> dict:
    for match in AUTHORIZATION_BLOCK_RE.finditer(issue_body):
        try:
            value = json.loads(match.group("payload"))
        except json.JSONDecodeError:
            continue
        if not isinstance(value, dict):
            continue
        authorization = value.get("release_authorization", value)
        if (
            isinstance(authorization, dict)
            and authorization.get("authorization") == "publish"
        ):
            return authorization
    raise ValueError(
        "release issue is missing a fenced JSON release_authorization object"
    )


def fetch_release_authorization(issue_url: str) -> dict:
    match = re.fullmatch(
        r"https://github\.com/(?P<repository>[^/]+/[^/]+)/issues/(?P<number>[1-9]\d*)",
        issue_url,
    )
    if not match:
        raise ValueError("release issue URL is not canonical")
    completed = subprocess.run(
        [
            "gh",
            "issue",
            "view",
            match.group("number"),
            "--repo",
            match.group("repository"),
            "--json",
            "body,state,url",
        ],
        text=True,
        capture_output=True,
        check=False,
    )
    if completed.returncode:
        raise ValueError(
            "cannot fetch independently controlled release issue: "
            + (completed.stderr.strip() or "gh issue view failed")
        )
    issue = json.loads(completed.stdout)
    authorization = extract_release_authorization(str(issue.get("body") or ""))
    return {
        **authorization,
        "_issue_state": issue.get("state"),
        "_issue_url": issue.get("url"),
    }


def git_sha(root: Path, *args: str) -> str:
    completed = subprocess.run(
        ["git", *args],
        cwd=root,
        text=True,
        capture_output=True,
        check=False,
    )
    if completed.returncode:
        raise ValueError(completed.stderr.strip() or f"git {' '.join(args)} failed")
    return completed.stdout.strip()


def semver_parts(version: str) -> tuple[int, int, int, str | None] | None:
    match = SEMVER_RE.fullmatch(version)
    if not match:
        return None
    prerelease = match.group(4)
    return (
        int(match.group(1)),
        int(match.group(2)),
        int(match.group(3)),
        prerelease,
    )


def is_strictly_greater(new: str, old: str) -> bool:
    new_parts = semver_parts(new)
    old_parts = semver_parts(old)
    if not new_parts or not old_parts:
        return False
    if new_parts[:3] != old_parts[:3]:
        return new_parts[:3] > old_parts[:3]
    new_pre, old_pre = new_parts[3], old_parts[3]
    if new_pre is None or old_pre is None:
        return new_pre is None and old_pre is not None
    for new_item, old_item in zip(new_pre.split("."), old_pre.split(".")):
        if new_item == old_item:
            continue
        if new_item.isdigit() and old_item.isdigit():
            return int(new_item) > int(old_item)
        if new_item.isdigit() != old_item.isdigit():
            return not new_item.isdigit()
        return new_item > old_item
    return len(new_pre.split(".")) > len(old_pre.split("."))


def inside_root(root: Path, value: str, base: Path | None = None) -> Path | None:
    root = root.resolve()
    relative_base = base.resolve() if base else root
    candidate = (
        (relative_base / value).resolve()
        if not Path(value).is_absolute()
        else Path(value).resolve()
    )
    try:
        candidate.relative_to(root)
    except ValueError:
        return None
    return candidate


def workspace_manifest(root: Path) -> dict:
    path = root / "Cargo.toml"
    if not path.exists():
        return {}
    with path.open("rb") as handle:
        return tomllib.load(handle)


def package_identity(manifest: dict, workspace: dict) -> tuple[str | None, str | None]:
    package = manifest.get("package", {})
    name = package.get("name")
    version = package.get("version")
    if isinstance(version, dict) and version.get("workspace"):
        version = workspace.get("workspace", {}).get("package", {}).get("version")
    return name, version if isinstance(version, str) else None


def dependency_declarations(manifest: dict) -> list[tuple[str, Any, str]]:
    declarations: list[tuple[str, Any, str]] = []
    for section in DEPENDENCY_SECTIONS:
        for alias, spec in manifest.get(section, {}).items():
            declarations.append((alias, spec, section))
    for target in manifest.get("target", {}).values():
        if not isinstance(target, dict):
            continue
        for section in DEPENDENCY_SECTIONS:
            for alias, spec in target.get(section, {}).items():
                declarations.append((alias, spec, f"target.{section}"))
    return declarations


def resolve_workspace_dependency(
    alias: str, spec: Any, workspace: dict
) -> Any:
    if not isinstance(spec, dict) or not spec.get("workspace"):
        return spec
    inherited = workspace.get("workspace", {}).get("dependencies", {}).get(alias)
    if inherited is None:
        return spec
    if isinstance(inherited, str):
        inherited = {"version": inherited}
    merged = dict(inherited)
    merged.update({key: value for key, value in spec.items() if key != "workspace"})
    return merged


def exact_dependency_version(requirement: Any) -> str | None:
    if not isinstance(requirement, str):
        return None
    candidate = requirement[1:] if requirement.startswith("=") else requirement
    return candidate if SEMVER_RE.fullmatch(candidate) else None


def validate_package_manifest(
    *,
    package: dict,
    record: dict,
    packages_by_name: dict[str, dict],
    ownership_by_name: dict[str, dict],
    repository: str,
    root: Path,
    workspace: dict,
) -> tuple[list[str], list[tuple[str, str]]]:
    errors: list[str] = []
    edges: list[tuple[str, str]] = []
    name = package.get("name")
    manifest_value = package.get("manifest_path")
    if not isinstance(manifest_value, str) or not manifest_value:
        return [f"{name}: missing manifest_path"], edges
    if manifest_value != record.get("manifest_path"):
        errors.append(
            f"{name}: manifest_path does not match reviewed ownership "
            f"{record.get('manifest_path')!r}"
        )
    manifest_path = inside_root(root, manifest_value)
    if not manifest_path:
        return errors + [f"{name}: manifest_path resolves outside repository"], edges
    if not manifest_path.is_file():
        return errors + [f"{name}: manifest_path does not exist: {manifest_value}"], edges
    try:
        with manifest_path.open("rb") as handle:
            manifest = tomllib.load(handle)
    except (OSError, tomllib.TOMLDecodeError) as error:
        return errors + [f"{name}: cannot read Cargo manifest: {error}"], edges
    actual_name, actual_version = package_identity(manifest, workspace)
    if actual_name != name:
        errors.append(
            f"{name}: manifest package name is {actual_name!r}, not the planned package"
        )
    expected_actual_version = (
        package.get("new_version") if package.get("publish") else package.get("old_version")
    )
    if actual_version != expected_actual_version:
        errors.append(
            f"{name}: manifest version {actual_version!r} does not match "
            f"{'new' if package.get('publish') else 'old'} version "
            f"{expected_actual_version!r}"
        )

    actual_release_dependencies: set[str] = set()
    declarations = dependency_declarations(manifest) if package.get("publish") else []
    for alias, raw_spec, section in declarations:
        inherited_from_workspace = (
            isinstance(raw_spec, dict) and bool(raw_spec.get("workspace"))
        )
        spec = resolve_workspace_dependency(alias, raw_spec, workspace)
        if isinstance(spec, str):
            if alias in packages_by_name:
                actual_release_dependencies.add(alias)
                edges.append((name, alias))
                pinned = exact_dependency_version(spec)
                if pinned != packages_by_name[alias].get("new_version"):
                    errors.append(
                        f"{name}: in-plan registry dependency {alias!r} does not "
                        f"pin planned version {packages_by_name[alias].get('new_version')}"
                    )
            continue
        if not isinstance(spec, dict):
            errors.append(f"{name}: malformed {section} dependency {alias!r}")
            continue
        dependency_name = spec.get("package", alias)
        if "path" in spec:
            dependency_path = inside_root(
                root,
                spec["path"],
                root if inherited_from_workspace else manifest_path.parent,
            )
            if dependency_path is None:
                errors.append(
                    f"{name}: outside-repository path dependency {dependency_name!r}"
                )
                continue
            dependency_manifest_path = dependency_path / "Cargo.toml"
            if not dependency_manifest_path.is_file():
                errors.append(
                    f"{name}: path dependency {dependency_name!r} has no Cargo.toml"
                )
                continue
            with dependency_manifest_path.open("rb") as handle:
                dependency_manifest = tomllib.load(handle)
            actual_dependency_name, _ = package_identity(dependency_manifest, workspace)
            declared_dependency_name = dependency_name
            dependency_name = actual_dependency_name or dependency_name
            if actual_dependency_name and actual_dependency_name != declared_dependency_name:
                errors.append(
                    f"{name}: path dependency {declared_dependency_name!r} resolves "
                    f"to package {actual_dependency_name!r}"
                )
            dependency_plan = packages_by_name.get(dependency_name)
            dependency_record = ownership_by_name.get(dependency_name)
            pinned = exact_dependency_version(spec.get("version"))
            if dependency_plan is None:
                errors.append(
                    f"{name}: path dependency {dependency_name!r} is not in this release plan"
                )
            else:
                actual_release_dependencies.add(dependency_name)
                edges.append((name, dependency_name))
            if dependency_record is None:
                errors.append(
                    f"{name}: path dependency {dependency_name!r} is not in ownership"
                )
            elif f"moritzbrantner/{dependency_record['target_repository']}" != repository:
                errors.append(
                    f"{name}: cross-owner path dependency {dependency_name!r}"
                )
            elif dependency_record.get("manifest_path") != dependency_manifest_path.relative_to(root.resolve()).as_posix():
                errors.append(
                    f"{name}: path dependency {dependency_name!r} does not match "
                    "its reviewed manifest_path"
                )
            if pinned is None:
                errors.append(
                    f"{name}: path-only dependency {dependency_name!r} lacks an exact registry version"
                )
            elif dependency_plan and pinned != dependency_plan.get("new_version"):
                errors.append(
                    f"{name}: path dependency {dependency_name!r} pins {pinned}, "
                    f"not planned version {dependency_plan.get('new_version')}"
                )
        elif "git" in spec:
            revision = spec.get("rev")
            if (
                not isinstance(revision, str)
                or not FULL_SHA_RE.fullmatch(revision)
                or "branch" in spec
                or "tag" in spec
            ):
                errors.append(
                    f"{name}: non-immutable Git dependency {dependency_name!r}"
                )
        elif dependency_name in packages_by_name:
            actual_release_dependencies.add(dependency_name)
            edges.append((name, dependency_name))
            pinned = exact_dependency_version(spec.get("version"))
            if pinned != packages_by_name[dependency_name].get("new_version"):
                errors.append(
                    f"{name}: in-plan registry dependency {dependency_name!r} does "
                    f"not pin planned version {packages_by_name[dependency_name].get('new_version')}"
                )

    declared_release_dependencies = package.get("release_dependencies")
    if not isinstance(declared_release_dependencies, list):
        errors.append(f"{name}: release_dependencies must be a list")
    elif set(declared_release_dependencies) != actual_release_dependencies:
        errors.append(
            f"{name}: release_dependencies must exactly match in-plan path dependencies "
            f"{sorted(actual_release_dependencies)}"
        )
    return errors, edges


def validate_plan(
    plan: dict,
    ownership: dict,
    repository_root: Path = ROOT,
    actual_head_sha: str | None = None,
    actual_base_sha: str | None = None,
    release_authorization: dict | None = None,
) -> list[str]:
    errors: list[str] = []
    repository = plan.get("repository")
    raw_packages = plan.get("packages")
    publishes_packages = isinstance(raw_packages, list) and any(
        isinstance(package, dict) and package.get("publish") is True
        for package in raw_packages
    )
    if not isinstance(repository, str) or not repository.startswith("moritzbrantner/"):
        errors.append("repository must be an exact moritzbrantner repository")
    if publishes_packages and actual_head_sha is None:
        errors.append("actual Git head SHA is required for a publishable plan")
    if publishes_packages and actual_base_sha is None:
        errors.append("actual Git base SHA is required for a publishable plan")
    if publishes_packages and release_authorization is None:
        errors.append(
            "live release-issue authorization is required for a publishable plan"
        )
    for field in ("source_sha", "default_branch_base_sha"):
        value = plan.get(field)
        if not isinstance(value, str) or not FULL_SHA_RE.fullmatch(value):
            errors.append(f"{field} must be a full lowercase commit SHA")
        actual = actual_head_sha if field == "source_sha" else actual_base_sha
        if publishes_packages and actual and value != actual:
            errors.append(f"{field} {value!r} does not match actual Git SHA {actual}")
    issue = plan.get("release_issue")
    if not isinstance(issue, str):
        errors.append("missing exact release issue reference")
    elif not isinstance(repository, str) or not re.fullmatch(
        rf"https://github\.com/{re.escape(repository)}/issues/[1-9]\d*",
        issue,
    ):
        errors.append(
            "release issue must be a canonical numeric issue URL for "
            f"{repository}"
        )
    if plan.get("destination_registry") != "crates.io":
        errors.append("destination_registry must be crates.io")
    for field in REQUIRED_LIST_FIELDS:
        if field not in plan:
            errors.append(f"missing required field {field}")
        elif not isinstance(plan[field], list):
            errors.append(f"{field} must be a list")
    required_checks = plan.get("required_checks")
    if isinstance(required_checks, list) and any(
        not isinstance(check, str) or not check.strip()
        for check in required_checks
    ):
        errors.append("required_checks entries must be nonempty strings")
    if publishes_packages and release_authorization is not None:
        authorization_pairs = {
            field: (release_authorization.get(field), plan.get(field))
            for field in (
                "repository",
                "release_issue",
                "source_sha",
                "default_branch_base_sha",
                "required_checks",
            )
        }
        for field, (authorized, planned) in authorization_pairs.items():
            if authorized != planned:
                errors.append(
                    f"{field} does not match live release-issue authorization"
                )
        if release_authorization.get("_issue_url") != issue:
            errors.append("fetched release issue URL does not match the plan")
        if release_authorization.get("_issue_state") != "OPEN":
            errors.append("release issue must be open while publication is authorized")
        authorized_packages = sorted(
            (
                package.get("name"),
                package.get("version"),
            )
            for package in release_authorization.get("packages", [])
            if isinstance(package, dict)
        )
        planned_packages = sorted(
            (package.get("name"), package.get("new_version"))
            for package in raw_packages
            if isinstance(package, dict) and package.get("publish") is True
        )
        if authorized_packages != planned_packages:
            errors.append(
                "publishable packages and versions do not match live "
                "release-issue authorization"
            )

    ownership_by_name = {
        record["current_package_name"]: record
        for record in ownership.get("packages", [])
        if record.get("ecosystem") == "cargo"
    }
    packages = raw_packages
    if not isinstance(packages, list) or not packages:
        return errors + ["packages must be a non-empty list"]
    names = [package.get("name") for package in packages]
    duplicates = sorted(name for name, count in Counter(names).items() if count > 1)
    if duplicates:
        errors.append("duplicate package names: " + ", ".join(duplicates))
    packages_by_name = {
        package.get("name"): package for package in packages if package.get("name")
    }
    package_names = set(packages_by_name)
    dependency_edges: list[tuple[str, str]] = []
    publishes_foundation = False
    workspace = workspace_manifest(repository_root)

    for package in packages:
        name = package.get("name")
        publish = package.get("publish")
        if not isinstance(publish, bool):
            errors.append(f"{name}: publish must be a boolean")
        record = ownership_by_name.get(name)
        if not record:
            errors.append(f"package {name!r} is absent from the ownership map")
            continue
        expected_owner = f"moritzbrantner/{record['target_repository']}"
        if package.get("owner") != expected_owner or repository != expected_owner:
            errors.append(
                f"wrong owner for {name}: plan={package.get('owner')!r}, "
                f"repository={repository!r}, map={expected_owner!r}"
            )
        publishes_foundation |= bool(package.get("publish")) and record[
            "target_repository"
        ] == "moenarch-foundation"
        old = package.get("old_version")
        new = package.get("new_version")
        if not isinstance(old, str) or not SEMVER_RE.fullmatch(old):
            errors.append(f"{name}: malformed old_version")
        elif record.get("source_version") and old != record["source_version"]:
            errors.append(
                f"{name}: old_version {old} does not match reviewed source version "
                f"{record['source_version']}"
            )
        if not isinstance(new, str) or not SEMVER_RE.fullmatch(new):
            errors.append(f"{name}: malformed new_version")
        elif isinstance(old, str) and SEMVER_RE.fullmatch(old):
            if publish is True and not is_strictly_greater(new, old):
                errors.append(
                    f"{name}: new_version must be strictly greater than old_version"
                )
            elif publish is False and new != old:
                errors.append(
                    f"{name}: nonpublish entry must keep new_version equal to old_version"
                )
        manifest_errors, manifest_edges = validate_package_manifest(
            package=package,
            record=record,
            packages_by_name=packages_by_name,
            ownership_by_name=ownership_by_name,
            repository=repository,
            root=repository_root,
            workspace=workspace,
        )
        errors.extend(manifest_errors)
        dependency_edges.extend(manifest_edges)
        if publish is True:
            expected_tag = f"{name}-v{new}"
            if package.get("expected_tag") != expected_tag:
                errors.append(f"{name}: expected_tag must be {expected_tag}")
        elif publish is False and package.get("expected_tag") is not None:
            errors.append(
                f"{name}: nonpublish entry must not declare expected_tag"
            )
        declared_dependencies = package.get("release_dependencies", [])
        if isinstance(declared_dependencies, list):
            for dependency in declared_dependencies:
                if dependency not in package_names:
                    errors.append(f"{name}: unknown release dependency {dependency}")
                else:
                    dependency_edges.append((name, dependency))

    dependency_edges = sorted(set(dependency_edges))
    cycle = find_cycle(package_names, dependency_edges)
    if cycle:
        errors.append("release dependency cycle: " + " -> ".join(cycle))
    order = plan.get("dependency_order", [])
    if isinstance(order, list):
        if len(order) != len(set(order)):
            errors.append("dependency_order contains duplicates")
        if set(order) != package_names:
            errors.append("dependency_order must contain every package exactly once")
        positions = {name: index for index, name in enumerate(order)}
        for dependent, dependency in dependency_edges:
            if positions.get(dependent, -1) < positions.get(dependency, -1):
                errors.append(f"wrong dependency order: {dependent} precedes {dependency}")
    if publishes_foundation and (
        not plan.get("required_consumer_checks")
        or not plan.get("downstream_consumers")
    ):
        errors.append(
            "foundation publication requires consumer gates and downstream consumers"
        )
    expected_tag_values = plan.get("expected_tags", [])
    if isinstance(expected_tag_values, list) and len(expected_tag_values) != len(
        set(expected_tag_values)
    ):
        errors.append("expected_tags contains duplicates")
    expected_tags = (
        set(expected_tag_values)
        if isinstance(expected_tag_values, list)
        else set()
    )
    package_tags = {
        package.get("expected_tag") for package in packages if package.get("publish")
    }
    if expected_tags != package_tags:
        errors.append("expected_tags must exactly match publishable package tags")
    if not plan.get("required_checks"):
        errors.append("required_checks must not be empty")
    return errors


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--check", type=Path, required=True, metavar="MANIFEST")
    parser.add_argument("--ownership", type=Path, default=OWNERSHIP_PATH)
    parser.add_argument("--repository-root", type=Path, default=ROOT)
    parser.add_argument("--base-ref", default="origin/main")
    parser.add_argument("--print-order", action="store_true")
    args = parser.parse_args()
    try:
        plan = load_document(args.check)
    except (OSError, ValueError, json.JSONDecodeError, tomllib.TOMLDecodeError) as error:
        print(f"error: {error}", file=sys.stderr)
        return 1
    publishes_packages = any(
        isinstance(package, dict) and package.get("publish") is True
        for package in plan.get("packages", [])
    )
    try:
        actual_head_sha = (
            git_sha(args.repository_root, "rev-parse", "HEAD")
            if publishes_packages
            else None
        )
        actual_base_sha = (
            git_sha(args.repository_root, "merge-base", args.base_ref, "HEAD")
            if publishes_packages
            else None
        )
        authorization = (
            fetch_release_authorization(str(plan.get("release_issue") or ""))
            if publishes_packages
            else None
        )
    except (ValueError, json.JSONDecodeError) as error:
        print(f"error: {error}", file=sys.stderr)
        return 1
    errors = validate_plan(
        plan,
        load_json(args.ownership),
        args.repository_root,
        actual_head_sha,
        actual_base_sha,
        authorization,
    )
    if errors:
        for error in errors:
            print(f"error: {error}", file=sys.stderr)
        return 1
    print(f"release plan valid: {plan['repository']} ({len(plan['packages'])} packages)")
    if args.print_order:
        for index, package in enumerate(plan["dependency_order"], 1):
            print(f"{index}. {package}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
