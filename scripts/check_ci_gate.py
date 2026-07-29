#!/usr/bin/env python3
"""Fail closed when a planner-selected CI job did not run successfully."""

from __future__ import annotations

import argparse
import json


SUCCESS = "success"


def parse_assignments(values: list[str], *, boolean: bool) -> dict:
    parsed = {}
    for value in values:
        if "=" not in value:
            raise ValueError(f"expected NAME=VALUE, got {value!r}")
        name, raw = value.split("=", 1)
        if not name:
            raise ValueError("assignment name must not be empty")
        parsed[name] = raw.lower() == "true" if boolean else raw
    return parsed


def evaluate_gate(
    *,
    planner_result: str,
    selected: dict[str, bool],
    results: dict[str, str],
) -> dict:
    failures: list[str] = []
    if planner_result != SUCCESS:
        failures.append(f"planner:{planner_result}")
    for name, enabled in sorted(selected.items()):
        result = results.get(name, "missing")
        if enabled and result != SUCCESS:
            suffix = "selected-but-skipped" if result == "skipped" else result
            failures.append(f"{name}:{suffix}")
    return {
        "passed": not failures,
        "plannerResult": planner_result,
        "selected": selected,
        "results": results,
        "failures": failures,
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--planner-result", required=True)
    parser.add_argument("--selected", action="append", default=[])
    parser.add_argument("--result", action="append", default=[])
    args = parser.parse_args()
    output = evaluate_gate(
        planner_result=args.planner_result,
        selected=parse_assignments(args.selected, boolean=True),
        results=parse_assignments(args.result, boolean=False),
    )
    print(json.dumps(output, indent=2, sort_keys=True))
    if not output["passed"]:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
