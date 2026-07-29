#!/usr/bin/env python3
"""Run the static frontend commands selected by changed-scope classification."""

from __future__ import annotations

import argparse
import json
import shlex
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def selected_commands(scope: dict, kind: str) -> list[str]:
    commands = [str(command) for command in scope.get("frontend_commands") or []]
    if kind == "wasm":
        return [command for command in commands if "wasm" in command]
    return [command for command in commands if "wasm" not in command]


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--base", default="origin/main")
    parser.add_argument("--kind", choices=["application", "wasm"], required=True)
    args = parser.parse_args()
    output = subprocess.check_output(
        [sys.executable, "scripts/check_changed_scope.py", "--base", args.base],
        cwd=ROOT,
        text=True,
    )
    commands = selected_commands(json.loads(output), args.kind)
    if not commands:
        raise SystemExit(f"planner selected {args.kind} checks but produced no commands")
    for command in commands:
        subprocess.run(shlex.split(command), cwd=ROOT, check=True)


if __name__ == "__main__":
    main()
