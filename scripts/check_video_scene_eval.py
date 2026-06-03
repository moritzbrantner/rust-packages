#!/usr/bin/env python3
"""Summarize scene-boundary evaluation reports from scene_dataset_eval."""

from __future__ import annotations

import argparse
import json
from pathlib import Path


def matched_count(predicted: list[int], truth: list[int], tolerance: int) -> int:
    remaining = set(truth)
    matched = 0
    for frame in predicted:
        best = None
        for candidate in remaining:
            if abs(frame - candidate) <= tolerance and (
                best is None or abs(frame - candidate) < abs(frame - best)
            ):
                best = candidate
        if best is not None:
            remaining.remove(best)
            matched += 1
    return matched


def evaluate(report: dict, tolerance: int) -> dict:
    total_correct = 0
    total_pred = 0
    total_gt = 0
    elapsed = []
    for video in report.get("videos", []):
        predicted = [int(value) for value in video.get("predictedCuts", [])]
        truth = [int(value) for value in video.get("groundTruthCuts", [])]
        total_correct += matched_count(predicted, truth, tolerance)
        total_pred += len(predicted)
        total_gt += len(truth)
        elapsed.append(float(video.get("elapsedMs", 0.0)))

    recall = total_correct / total_gt if total_gt else 0.0
    precision = total_correct / total_pred if total_pred else 0.0
    f1 = 2 * recall * precision / (recall + precision) if recall + precision else 0.0
    summary = {
        "recall": recall,
        "precision": precision,
        "f1": f1,
        "avgElapsedMs": sum(elapsed) / len(elapsed) if elapsed else 0.0,
        "correct": total_correct,
        "predicted": total_pred,
        "groundTruth": total_gt,
        "videoCount": len(report.get("videos", [])),
    }
    mode = report.get("mode")
    if isinstance(mode, dict) and "complete" in mode:
        summary["complete"] = bool(mode["complete"])
    return summary


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("report", type=Path)
    parser.add_argument("--tolerance-frames", type=int, default=0)
    parser.add_argument("--min-f1", type=float, default=None)
    parser.add_argument("--allow-partial", action="store_true")
    args = parser.parse_args()

    if not args.report.exists():
        print(f"error: report file does not exist: {args.report}")
        return 2

    report = json.loads(args.report.read_text())
    summary = evaluate(report, args.tolerance_frames)
    if (
        not args.allow_partial
        and isinstance(report.get("mode"), dict)
        and report["mode"].get("complete") is False
    ):
        raise SystemExit("report is partial; pass --allow-partial to summarize it")
    print(json.dumps(summary, indent=2, sort_keys=True))
    if args.min_f1 is not None and summary["f1"] < args.min_f1:
        raise SystemExit(f"f1 {summary['f1']:.4f} is below required {args.min_f1:.4f}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
