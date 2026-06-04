#!/usr/bin/env python3
"""Compare Rust and PySceneDetect scene dataset evaluator speed reports."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--rust-report", type=Path, required=True)
    parser.add_argument("--pyscenedetect-report", type=Path, required=True)
    parser.add_argument("--output", type=Path)
    return parser.parse_args()


def load_report(path: Path) -> dict[str, Any]:
    if not path.exists():
        raise SystemExit(f"report does not exist: {path}")
    return json.loads(path.read_text())


def video_ids(report: dict[str, Any]) -> list[str]:
    return [str(video.get("id", "")) for video in report.get("videos", [])]


def resize_width(report: dict[str, Any]) -> Any:
    mode = report.get("mode")
    if isinstance(mode, dict) and "resizeWidth" in mode:
        return mode["resizeWidth"]
    return report.get("configuration", {}).get("resizeWidth")


def assert_matching(rust: dict[str, Any], pyscene: dict[str, Any]) -> None:
    checks = [
        ("dataset", rust.get("dataset"), pyscene.get("dataset")),
        ("detector", rust.get("detector"), pyscene.get("detector")),
        ("video IDs", video_ids(rust), video_ids(pyscene)),
        ("resize width", resize_width(rust), resize_width(pyscene)),
        ("configuration", rust.get("configuration"), pyscene.get("configuration")),
    ]
    for name, left, right in checks:
        if left != right:
            raise SystemExit(f"mismatched {name}: rust={left!r} pyscenedetect={right!r}")


def average(videos: list[dict[str, Any]], key: str) -> float:
    values = [float(video[key]) for video in videos if video.get(key) is not None]
    return sum(values) / len(values) if values else 0.0


def ratio(numerator: float, denominator: float) -> float:
    return numerator / denominator if denominator else 0.0


def compare(
    rust_report: Path, pyscene_report: Path, rust: dict[str, Any], pyscene: dict[str, Any]
) -> dict[str, Any]:
    rust_videos = {str(video["id"]): video for video in rust.get("videos", [])}
    pyscene_videos = {str(video["id"]): video for video in pyscene.get("videos", [])}
    videos = []
    for ident in video_ids(rust):
        rust_video = rust_videos[ident]
        pyscene_video = pyscene_videos[ident]
        rust_elapsed = float(rust_video.get("elapsedMs", 0.0))
        pyscene_elapsed = float(pyscene_video.get("elapsedMs", 0.0))
        videos.append(
            {
                "id": ident,
                "rustElapsedMs": rust_elapsed,
                "pyscenedetectElapsedMs": pyscene_elapsed,
                "ratio": ratio(rust_elapsed, pyscene_elapsed),
                "rustPredicted": len(rust_video.get("predictedCuts", [])),
                "pyscenedetectPredicted": len(pyscene_video.get("predictedCuts", [])),
            }
        )

    rust_items = list(rust_videos.values())
    pyscene_items = list(pyscene_videos.values())
    rust_avg = average(rust_items, "elapsedMs")
    pyscene_avg = average(pyscene_items, "elapsedMs")
    return {
        "rustReport": str(rust_report),
        "pyscenedetectReport": str(pyscene_report),
        "aggregate": {
            "rustAvgElapsedMs": rust_avg,
            "pyscenedetectAvgElapsedMs": pyscene_avg,
            "rustVsPyscenedetectRatio": ratio(rust_avg, pyscene_avg),
            "rustDetectorAvgElapsedMs": average(rust_items, "detectorElapsedMs"),
            "pyscenedetectDetectorAvgElapsedMs": average(pyscene_items, "detectorElapsedMs"),
            "rustDecodeResizeAvgElapsedMs": average(rust_items, "decodeResizeElapsedMs"),
            "pyscenedetectDecodeResizeAvgElapsedMs": average(
                pyscene_items, "decodeResizeElapsedMs"
            ),
        },
        "videos": videos,
    }


def print_table(result: dict[str, Any]) -> None:
    print("id       rust ms    pyscenedetect ms    ratio    rust cuts    pyscene cuts")
    for video in result["videos"]:
        print(
            f"{video['id']:<8} "
            f"{video['rustElapsedMs']:>9.2f} "
            f"{video['pyscenedetectElapsedMs']:>19.2f} "
            f"{video['ratio']:>8.2f} "
            f"{video['rustPredicted']:>12} "
            f"{video['pyscenedetectPredicted']:>13}"
        )
    aggregate = result["aggregate"]
    print(
        f"average  {aggregate['rustAvgElapsedMs']:>9.2f} "
        f"{aggregate['pyscenedetectAvgElapsedMs']:>19.2f} "
        f"{aggregate['rustVsPyscenedetectRatio']:>8.2f}"
    )


def write_json(path: Path | None, data: dict[str, Any]) -> None:
    text = json.dumps(data, indent=2) + "\n"
    if path is None:
        print(text, end="")
        return
    path.parent.mkdir(parents=True, exist_ok=True)
    tmp = path.with_name(path.name + ".tmp")
    tmp.write_text(text)
    tmp.replace(path)


def main() -> int:
    args = parse_args()
    rust = load_report(args.rust_report)
    pyscene = load_report(args.pyscenedetect_report)
    assert_matching(rust, pyscene)
    result = compare(args.rust_report, args.pyscenedetect_report, rust, pyscene)
    print_table(result)
    write_json(args.output, result)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
