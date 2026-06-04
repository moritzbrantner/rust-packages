#!/usr/bin/env python3
"""Run parameter sweeps for the scene dataset evaluator."""

from __future__ import annotations

import argparse
import itertools
import json
import subprocess
import sys
from functools import cmp_to_key
from datetime import datetime, timezone
from pathlib import Path
from typing import Any


CONTENT_GRID = {
    "contentThreshold": [27.0, 30.0, 33.0, 36.0, 39.0, 42.0],
    "minSceneLen": [15, 20, 25, 30, 36, 45],
    "filterMode": ["merge", "suppress"],
    "postFilterWindow": [0, 8, 12, 15, 20, 30],
}

ADAPTIVE_GRID = {
    "adaptiveThreshold": [3.0, 3.5, 4.0, 4.5, 5.0],
    "adaptiveWindowWidth": [2, 3, 4],
    "adaptiveMinContentVal": [15.0, 18.0, 21.0, 24.0],
    "minSceneLen": [15, 20, 25, 30, 36],
    "postFilterWindow": [0, 8, 12, 15, 20, 30],
}

TARGET = {
    "f1": 0.85,
    "precision": 0.83,
    "recall": 0.82,
    "complete": True,
}


def utc_now() -> str:
    return datetime.now(timezone.utc).isoformat()


def float_id(value: float) -> str:
    if float(value).is_integer():
        return str(int(value))
    return str(value).replace(".", "p")


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


def summarize_report(report: dict[str, Any], tolerance: int) -> dict[str, Any]:
    total_correct = 0
    total_predicted = 0
    total_truth = 0
    elapsed = []
    for video in report.get("videos", []):
        predicted = [int(value) for value in video.get("predictedCuts", [])]
        truth = [int(value) for value in video.get("groundTruthCuts", [])]
        total_correct += matched_count(predicted, truth, tolerance)
        total_predicted += len(predicted)
        total_truth += len(truth)
        elapsed.append(float(video.get("elapsedMs", 0.0)))
    recall = total_correct / total_truth if total_truth else 0.0
    precision = total_correct / total_predicted if total_predicted else 0.0
    f1 = 2 * recall * precision / (recall + precision) if recall + precision else 0.0
    mode = report.get("mode")
    return {
        "videoCount": len(report.get("videos", [])),
        "correct": total_correct,
        "predicted": total_predicted,
        "groundTruth": total_truth,
        "recall": recall,
        "precision": precision,
        "f1": f1,
        "avgElapsedMs": sum(elapsed) / len(elapsed) if elapsed else 0.0,
        "complete": bool(mode.get("complete")) if isinstance(mode, dict) else False,
    }


def content_trials() -> list[dict[str, Any]]:
    trials = []
    for threshold, min_scene_len, filter_mode, post_filter_window in itertools.product(
        CONTENT_GRID["contentThreshold"],
        CONTENT_GRID["minSceneLen"],
        CONTENT_GRID["filterMode"],
        CONTENT_GRID["postFilterWindow"],
    ):
        trials.append(
            {
                "trialId": (
                    f"content-th{float_id(threshold)}-min{min_scene_len}-"
                    f"{filter_mode}-post{post_filter_window}"
                ),
                "detector": "content",
                "configuration": {
                    "contentThreshold": threshold,
                    "minSceneLen": min_scene_len,
                    "filterMode": filter_mode,
                    "postFilterWindow": post_filter_window,
                },
            }
        )
    return trials


def adaptive_trials() -> list[dict[str, Any]]:
    trials = []
    for threshold, window_width, min_content_val, min_scene_len, post_filter_window in itertools.product(
        ADAPTIVE_GRID["adaptiveThreshold"],
        ADAPTIVE_GRID["adaptiveWindowWidth"],
        ADAPTIVE_GRID["adaptiveMinContentVal"],
        ADAPTIVE_GRID["minSceneLen"],
        ADAPTIVE_GRID["postFilterWindow"],
    ):
        trials.append(
            {
                "trialId": (
                    f"adaptive-th{float_id(threshold)}-win{window_width}-"
                    f"minval{float_id(min_content_val)}-min{min_scene_len}-"
                    f"post{post_filter_window}"
                ),
                "detector": "adaptive",
                "configuration": {
                    "adaptiveThreshold": threshold,
                    "adaptiveWindowWidth": window_width,
                    "adaptiveMinContentVal": min_content_val,
                    "minSceneLen": min_scene_len,
                    "postFilterWindow": post_filter_window,
                },
            }
        )
    return trials


def command_for_trial(args: argparse.Namespace, trial: dict[str, Any], report_path: Path) -> list[str]:
    configuration = trial["configuration"]
    command = [
        "cargo",
        "run",
        "--release",
        "-p",
        "moritzbrantner-video-analysis-detectors",
        "--example",
        "scene_dataset_eval",
        "--",
        "--dataset",
        args.dataset,
        "--root",
        args.root,
        "--detector",
        trial["detector"],
        "--resize-width",
        str(args.resize_width),
        "--max-runtime-seconds",
        str(args.max_runtime_seconds),
        "--output",
        str(report_path),
    ]
    for video_id in args.video_id:
        command.extend(["--video-id", video_id])
    if args.progress:
        command.append("--progress")
    if args.resume:
        command.append("--resume")
    if trial["detector"] == "content":
        command.extend(
            [
                "--content-threshold",
                str(configuration["contentThreshold"]),
                "--min-scene-len",
                str(configuration["minSceneLen"]),
                "--filter-mode",
                configuration["filterMode"],
                "--post-filter-window",
                str(configuration["postFilterWindow"]),
            ]
        )
    else:
        command.extend(
            [
                "--adaptive-threshold",
                str(configuration["adaptiveThreshold"]),
                "--adaptive-window-width",
                str(configuration["adaptiveWindowWidth"]),
                "--adaptive-min-content-val",
                str(configuration["adaptiveMinContentVal"]),
                "--min-scene-len",
                str(configuration["minSceneLen"]),
                "--post-filter-window",
                str(configuration["postFilterWindow"]),
            ]
        )
    return command


def load_complete_report(path: Path) -> dict[str, Any] | None:
    if not path.exists():
        return None
    report = json.loads(path.read_text())
    mode = report.get("mode")
    if isinstance(mode, dict) and mode.get("complete") is True:
        return report
    return None


def compare_trials(left: dict[str, Any], right: dict[str, Any]) -> int:
    left_summary = left.get("summary", {})
    right_summary = right.get("summary", {})
    for key in ["f1", "precision", "recall"]:
        left_value = float(left_summary.get(key, 0.0))
        right_value = float(right_summary.get(key, 0.0))
        if abs(left_value - right_value) >= 0.002:
            return 1 if left_value > right_value else -1

    left_elapsed = float(left_summary.get("avgElapsedMs", 0.0))
    right_elapsed = float(right_summary.get("avgElapsedMs", 0.0))
    if left_elapsed != right_elapsed:
        return 1 if left_elapsed < right_elapsed else -1

    if left.get("detector") != right.get("detector"):
        return 1 if left.get("detector") == "content" else -1
    return 0


def meets_target(trial: dict[str, Any]) -> bool:
    summary = trial.get("summary", {})
    return (
        summary.get("complete") is True
        and float(summary.get("f1", 0.0)) >= TARGET["f1"]
        and float(summary.get("precision", 0.0)) >= TARGET["precision"]
        and float(summary.get("recall", 0.0)) >= TARGET["recall"]
    )


def best_overall_trial(trials: list[dict[str, Any]]) -> dict[str, str] | None:
    complete = [trial for trial in trials if trial.get("summary", {}).get("complete") is True]
    if not complete:
        return None
    best = max(complete, key=cmp_to_key(compare_trials))
    return {"rankBy": "f1_precision_recall_elapsed", "trialId": best["trialId"]}


def best_passing_trial(trials: list[dict[str, Any]]) -> dict[str, str] | None:
    passing = [trial for trial in trials if meets_target(trial)]
    if not passing:
        return None
    best = max(passing, key=cmp_to_key(compare_trials))
    return {
        "rankBy": "target_then_f1_precision_recall_elapsed",
        "trialId": best["trialId"],
    }


def write_sweep(path: Path, sweep: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(sweep, indent=2, sort_keys=True) + "\n")


def run_trial(
    args: argparse.Namespace,
    trial: dict[str, Any],
    report_path: Path,
) -> tuple[dict[str, Any] | None, int | None, str | None]:
    report = load_complete_report(report_path) if args.resume else None
    if report is not None:
        return report, None, None

    command = command_for_trial(args, trial, report_path)
    completed = subprocess.run(command, text=True, capture_output=True)
    report = json.loads(report_path.read_text()) if report_path.exists() else None
    error = None
    if completed.returncode != 0:
        error = (completed.stderr or completed.stdout).strip()
    return report, completed.returncode, error


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--dataset", required=True)
    parser.add_argument("--root", required=True)
    parser.add_argument("--video-id", action="append", required=True)
    parser.add_argument("--resize-width", type=int, default=320)
    parser.add_argument("--max-runtime-seconds", type=int, default=3300)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--tolerance-frames", type=int, default=0)
    parser.add_argument("--resume", action="store_true")
    parser.add_argument("--progress", action="store_true")
    parser.add_argument(
        "--detector-kind",
        choices=["content", "adaptive", "all"],
        default="all",
    )
    parser.add_argument("--max-trials", type=int, default=None)
    parser.add_argument("--only-trial-id", action="append", default=[])
    args = parser.parse_args()

    if args.resize_width <= 0:
        raise SystemExit("--resize-width must be greater than 0")
    if args.max_runtime_seconds <= 0:
        raise SystemExit("--max-runtime-seconds must be greater than 0")
    if args.tolerance_frames < 0:
        raise SystemExit("--tolerance-frames must be greater than or equal to 0")
    if args.max_trials is not None and args.max_trials <= 0:
        raise SystemExit("--max-trials must be greater than 0")

    trials_dir = args.output.parent / "trials"
    if args.detector_kind == "content":
        definitions = content_trials()
    elif args.detector_kind == "adaptive":
        definitions = adaptive_trials()
    else:
        definitions = content_trials() + adaptive_trials()
    if args.only_trial_id:
        selected = set(args.only_trial_id)
        definitions = [
            definition for definition in definitions if definition["trialId"] in selected
        ]
        missing = sorted(selected - {definition["trialId"] for definition in definitions})
        if missing:
            raise SystemExit(f"unknown --only-trial-id value(s): {', '.join(missing)}")
    if args.max_trials is not None:
        definitions = definitions[: args.max_trials]
    sweep: dict[str, Any] = {
        "dataset": args.dataset,
        "root": args.root,
        "videoIds": args.video_id,
        "resizeWidth": args.resize_width,
        "toleranceFrames": args.tolerance_frames,
        "target": TARGET,
        "startedAt": utc_now(),
        "finishedAt": None,
        "baseline": {
            "detector": "content",
            "contentThreshold": 27.0,
            "minSceneLen": 15,
            "filterMode": "merge",
            "postFilterWindow": 0,
            "summary": {},
        },
        "best": None,
        "bestPassing": None,
        "bestOverall": None,
        "trials": [],
    }

    for definition in definitions:
        report_path = trials_dir / f"{definition['trialId']}.json"
        trial = {
            **definition,
            "reportPath": str(report_path),
            "summary": {
                "videoCount": 0,
                "correct": 0,
                "predicted": 0,
                "groundTruth": 0,
                "recall": 0.0,
                "precision": 0.0,
                "f1": 0.0,
                "avgElapsedMs": 0.0,
                "complete": False,
            },
        }
        print(f"scene sweep: {definition['trialId']}", file=sys.stderr)
        report, return_code, error = run_trial(args, definition, report_path)
        if report is not None:
            trial["summary"] = summarize_report(report, args.tolerance_frames)
        if return_code is not None:
            trial["returnCode"] = return_code
        if error:
            trial["error"] = error

        sweep["trials"].append(trial)
        if definition["trialId"] == "content-th27-min15-merge-post0":
            sweep["baseline"]["summary"] = trial["summary"]
        sweep["bestPassing"] = best_passing_trial(sweep["trials"])
        sweep["bestOverall"] = best_overall_trial(sweep["trials"])
        sweep["best"] = sweep["bestPassing"] or sweep["bestOverall"]
        sweep["finishedAt"] = utc_now()
        write_sweep(args.output, sweep)

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
