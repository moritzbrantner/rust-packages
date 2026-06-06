#!/usr/bin/env python3
"""Run native Cargo benches only for workspace packages with bench targets."""

from __future__ import annotations

import argparse
import glob
import json
import os
import subprocess
from pathlib import Path


def cargo_metadata() -> dict:
    result = subprocess.run(
        ["cargo", "metadata", "--no-deps", "--format-version", "1"],
        check=True,
        stdout=subprocess.PIPE,
        text=True,
    )
    return json.loads(result.stdout)


def bench_packages(metadata: dict) -> list[str]:
    workspace_members = set(metadata["workspace_members"])
    packages: list[str] = []
    for package in metadata["packages"]:
        if package["id"] not in workspace_members:
            continue
        manifest_dir = Path(package["manifest_path"]).parent
        if glob.glob(str(manifest_dir / "benches" / "*.rs")):
            packages.append(package["name"])
    return sorted(packages)


def main() -> int:
    parser = argparse.ArgumentParser(description="Run workspace native benchmarks.")
    parser.add_argument("--no-run", action="store_true", help="compile benchmarks without executing timings")
    args = parser.parse_args()

    packages = bench_packages(cargo_metadata())
    if not packages:
        print("No workspace packages with benches/*.rs found.")
        return 0

    print("Native benchmark packages:")
    for package in packages:
        print(f"  - {package}")

    jobs = os.environ.get("CARGO_BUILD_JOBS") or os.environ.get("TEST_MAX_WORKERS") or "2"
    for package in packages:
        command = ["cargo", "bench", "--jobs", jobs, "-p", package]
        if args.no_run:
            command.append("--no-run")
        print(f"+ {' '.join(command)}", flush=True)
        subprocess.run(command, check=True)

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
