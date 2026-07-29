#!/usr/bin/env python3
"""Run repository Python tests affected by the current changed-file set."""

from __future__ import annotations

import argparse
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def selected_tests(changed_paths: list[str], root: Path = ROOT) -> list[str]:
    selected: set[str] = set()
    for raw_path in changed_paths:
        path = raw_path.strip().removeprefix("./")
        if not path.startswith("scripts/") or not path.endswith(".py"):
            continue
        candidate = Path(path)
        if candidate.name.startswith("test_"):
            selected.add(candidate.as_posix())
            continue
        test_path = candidate.with_name(f"test_{candidate.name}")
        if (root / test_path).is_file():
            selected.add(test_path.as_posix())
    return sorted(selected)


def git_changed_paths(base: str) -> list[str]:
    merge_base = subprocess.check_output(
        ["git", "merge-base", base, "HEAD"],
        cwd=ROOT,
        text=True,
    ).strip()
    outputs = [
        subprocess.check_output(
            ["git", "diff", "--name-only", f"{merge_base}...HEAD"],
            cwd=ROOT,
            text=True,
        ),
        subprocess.check_output(
            ["git", "diff", "--name-only"],
            cwd=ROOT,
            text=True,
        ),
        subprocess.check_output(
            ["git", "diff", "--name-only", "--cached"],
            cwd=ROOT,
            text=True,
        ),
    ]
    return sorted(
        {
            line.strip()
            for output in outputs
            for line in output.splitlines()
            if line.strip()
        }
    )


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--base", default="origin/main")
    args = parser.parse_args()
    tests = selected_tests(git_changed_paths(args.base))
    for test in tests:
        subprocess.run([sys.executable, test], cwd=ROOT, check=True)


if __name__ == "__main__":
    main()
