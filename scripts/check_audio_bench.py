#!/usr/bin/env python3
import json
import platform
import sys
from pathlib import Path

THRESHOLD = 1.15


def load_json(path: Path):
    with path.open("r", encoding="utf-8") as handle:
        return json.load(handle)


def find_estimates(root: Path):
    estimates = {}
    for path in root.glob("**/new/estimates.json"):
        try:
            data = load_json(path)
            median = data["median"]["point_estimate"]
        except (OSError, KeyError, json.JSONDecodeError):
            continue
        benchmark = "/".join(path.parts[path.parts.index("criterion") + 1 : -2])
        estimates[benchmark] = median
    return estimates


def main() -> int:
    machine = platform.machine() or "unknown"
    baseline = Path("benches/baselines/audio-linux-x86_64.json")
    if machine not in {"x86_64", "amd64"}:
        print(f"warning: benchmark baseline is linux-x86_64; current machine is {machine}")
    if not baseline.exists():
        print(f"warning: missing benchmark baseline {baseline}; skipping regression check")
        return 0

    expected = load_json(baseline)
    if not expected:
        print("warning: benchmark baseline is empty; generate from clean main before enforcing")
        return 0

    actual = find_estimates(Path("target/criterion"))
    failed = False
    for name, baseline_ns in sorted(expected.items()):
        measured_ns = actual.get(name)
        if measured_ns is None:
            print(f"warning: missing benchmark result for {name}")
            continue
        ratio = measured_ns / baseline_ns
        if ratio > THRESHOLD:
            print(
                f"error: {name} regressed by {(ratio - 1.0) * 100:.1f}% "
                f"({measured_ns:.0f} ns vs {baseline_ns:.0f} ns)"
            )
            failed = True
        elif ratio < 1.0:
            print(f"ok: {name} improved by {(1.0 - ratio) * 100:.1f}%")
        else:
            print(f"ok: {name} changed by {(ratio - 1.0) * 100:.1f}%")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
