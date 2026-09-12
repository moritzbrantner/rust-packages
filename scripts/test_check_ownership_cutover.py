#!/usr/bin/env python3
"""Focused regression tests for the canonical ownership cutover checker."""

from __future__ import annotations

import copy
import json
import tempfile
from pathlib import Path

import check_ownership_cutover as checker


def load(path: Path) -> dict:
    with path.open(encoding="utf-8") as handle:
        return json.load(handle)


def run_with(cutover: dict) -> int:
    original_ownership = checker.OWNERSHIP
    original_cutover = checker.CUTOVER
    with tempfile.TemporaryDirectory() as directory:
        root = Path(directory)
        ownership_path = root / "package-ownership.json"
        cutover_path = root / "ownership-cutover.json"
        ownership_path.write_text(
            json.dumps(load(original_ownership)), encoding="utf-8"
        )
        cutover_path.write_text(json.dumps(cutover), encoding="utf-8")
        checker.OWNERSHIP = ownership_path
        checker.CUTOVER = cutover_path
        try:
            return checker.main()
        finally:
            checker.OWNERSHIP = original_ownership
            checker.CUTOVER = original_cutover


def main() -> int:
    baseline = load(checker.CUTOVER)
    assert run_with(copy.deepcopy(baseline)) == 0

    empty = copy.deepcopy(baseline)
    empty["families"] = []
    assert run_with(empty) == 1

    missing = copy.deepcopy(baseline)
    missing["families"] = [
        item
        for item in missing["families"]
        if item["targetRepository"] != "visual-analysis"
    ]
    assert run_with(missing) == 1

    wrong_owner = copy.deepcopy(baseline)
    for item in wrong_owner["families"]:
        if item["targetRepository"] == "nlp-stack":
            item["canonicalRepository"] = "moritzbrantner/rust-packages"
    assert run_with(wrong_owner) == 1

    duplicate = copy.deepcopy(baseline)
    duplicate["families"].append(copy.deepcopy(duplicate["families"][0]))
    assert run_with(duplicate) == 1

    print("ownership cutover checker tests: ok")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
