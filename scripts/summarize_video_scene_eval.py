#!/usr/bin/env python3
"""Emit diagnostics for scene-boundary evaluation reports."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any


DISTANCE_BUCKETS = ("adjacent", "near", "medium", "far", "unmatched")


def match_cuts(predicted: list[int], truth: list[int], tolerance: int) -> tuple[set[int], set[int]]:
    remaining = set(range(len(truth)))
    matched_predicted: set[int] = set()
    matched_truth: set[int] = set()
    for predicted_index, frame in enumerate(predicted):
        best = None
        for truth_index in remaining:
            candidate = truth[truth_index]
            if abs(frame - candidate) <= tolerance and (
                best is None or abs(frame - candidate) < abs(frame - truth[best])
            ):
                best = truth_index
        if best is not None:
            remaining.remove(best)
            matched_predicted.add(predicted_index)
            matched_truth.add(best)
    return matched_predicted, matched_truth


def metrics(correct: int, predicted: int, truth: int) -> dict[str, float]:
    recall = correct / truth if truth else 0.0
    precision = correct / predicted if predicted else 0.0
    f1 = 2 * recall * precision / (recall + precision) if recall + precision else 0.0
    return {
        "recall": recall,
        "precision": precision,
        "f1": f1,
    }


def nearest_distances(frames: list[int], truth: list[int]) -> list[int | None]:
    distances: list[int | None] = []
    for frame in frames:
        if truth:
            distances.append(min(abs(frame - candidate) for candidate in truth))
        else:
            distances.append(None)
    return distances


def bucket_for_distance(distance: int | None) -> str:
    if distance is None:
        return "unmatched"
    if distance <= 2:
        return "adjacent"
    if distance <= 15:
        return "near"
    if distance <= 60:
        return "medium"
    return "far"


def distance_buckets(distances: list[int | None]) -> dict[str, int]:
    buckets = {bucket: 0 for bucket in DISTANCE_BUCKETS}
    for distance in distances:
        buckets[bucket_for_distance(distance)] += 1
    return buckets


def false_positive_clusters(frames: list[int]) -> list[list[int]]:
    clusters: list[list[int]] = []
    current: list[int] = []
    for frame in sorted(frames):
        if not current or frame - current[-1] < 30:
            current.append(frame)
            continue
        if len(current) >= 2:
            clusters.append(current)
        current = [frame]
    if len(current) >= 2:
        clusters.append(current)
    return clusters


def cluster_stats(clusters: list[list[int]]) -> tuple[int, int]:
    largest = max((len(cluster) for cluster in clusters), default=0)
    clustered_count = sum(len(cluster) for cluster in clusters if len(cluster) >= 2)
    return largest, clustered_count


def summarize_video(video: dict[str, Any], tolerance: int) -> dict[str, Any]:
    predicted = [int(value) for value in video.get("predictedCuts", [])]
    truth = [int(value) for value in video.get("groundTruthCuts", [])]
    matched_predicted, matched_truth = match_cuts(predicted, truth, tolerance)
    unmatched_predicted = [
        frame for index, frame in enumerate(predicted) if index not in matched_predicted
    ]
    unmatched_truth = [frame for index, frame in enumerate(truth) if index not in matched_truth]
    nearest_truth_distances = nearest_distances(unmatched_predicted, truth)
    clusters = false_positive_clusters(unmatched_predicted)
    largest_cluster, clustered_count = cluster_stats(clusters)
    correct = len(matched_predicted)
    result = {
        "id": str(video.get("id", "")),
        "correct": correct,
        "predicted": len(predicted),
        "groundTruth": len(truth),
        "falsePositives": len(unmatched_predicted),
        "falseNegatives": len(unmatched_truth),
        **metrics(correct, len(predicted), len(truth)),
        "elapsedMs": float(video.get("elapsedMs", 0.0)),
        "falsePositiveDistanceBuckets": distance_buckets(nearest_truth_distances),
        "falsePositiveClusters": clusters,
        "largestFalsePositiveCluster": largest_cluster,
        "clusteredFalsePositiveCount": clustered_count,
        "unmatchedPredictedCuts": unmatched_predicted,
        "unmatchedGroundTruthCuts": unmatched_truth,
        "nearestTruthDistancesForFalsePositives": nearest_truth_distances,
    }
    return result


def summarize(report: dict[str, Any], source: Path, tolerance: int) -> dict[str, Any]:
    videos = [summarize_video(video, tolerance) for video in report.get("videos", [])]
    correct = sum(video["correct"] for video in videos)
    predicted = sum(video["predicted"] for video in videos)
    truth = sum(video["groundTruth"] for video in videos)
    aggregate = {
        "videoCount": len(videos),
        "correct": correct,
        "predicted": predicted,
        "groundTruth": truth,
        "falsePositives": predicted - correct,
        "falseNegatives": truth - correct,
        **metrics(correct, predicted, truth),
        "avgElapsedMs": (
            sum(video["elapsedMs"] for video in videos) / len(videos) if videos else 0.0
        ),
        "falsePositiveDistanceBuckets": {
            bucket: sum(
                video["falsePositiveDistanceBuckets"].get(bucket, 0) for video in videos
            )
            for bucket in DISTANCE_BUCKETS
        },
        "falsePositiveClusters": [
            {"videoId": video["id"], "cuts": cluster}
            for video in videos
            for cluster in video["falsePositiveClusters"]
        ],
    }
    aggregate["largestFalsePositiveCluster"] = max(
        (len(cluster["cuts"]) for cluster in aggregate["falsePositiveClusters"]),
        default=0,
    )
    aggregate["clusteredFalsePositiveCount"] = sum(
        len(cluster["cuts"]) for cluster in aggregate["falsePositiveClusters"]
    )
    return {
        "sourceReport": str(source),
        "toleranceFrames": tolerance,
        "aggregate": aggregate,
        "videos": videos,
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("report", type=Path)
    parser.add_argument("--tolerance-frames", type=int, default=0)
    parser.add_argument("--output", type=Path, default=None)
    args = parser.parse_args()

    if args.tolerance_frames < 0:
        raise SystemExit("--tolerance-frames must be greater than or equal to 0")
    if not args.report.exists():
        raise SystemExit(f"report file does not exist: {args.report}")

    report = json.loads(args.report.read_text())
    summary = summarize(report, args.report, args.tolerance_frames)
    content = json.dumps(summary, indent=2, sort_keys=True) + "\n"
    if args.output is None:
        print(content, end="")
    else:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(content)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
