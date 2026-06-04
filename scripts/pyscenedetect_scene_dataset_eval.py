#!/usr/bin/env python3
"""Evaluate PySceneDetect detectors on a local scene dataset subset."""

from __future__ import annotations

import argparse
import json
import sys
import time
from pathlib import Path
from typing import Any


REPO_ROOT = Path(__file__).resolve().parents[1]
PYSCENEDETECT_ROOT = REPO_ROOT / "references" / "pyscenedetect"
if str(PYSCENEDETECT_ROOT) not in sys.path:
    sys.path.insert(0, str(PYSCENEDETECT_ROOT))


VIDEO_EXTENSIONS = {".avi", ".mkv", ".mov", ".mp4", ".webm"}
ANNOTATION_EXTENSIONS = {".csv", ".tsv", ".txt"}


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--dataset", required=True)
    parser.add_argument("--root", type=Path, required=True)
    parser.add_argument(
        "--detector",
        choices=["content", "adaptive", "threshold", "histogram", "hash"],
        default="content",
    )
    parser.add_argument("--video-id", action="append", default=[])
    parser.add_argument("--limit", type=int)
    parser.add_argument("--resize-width", type=int)
    parser.add_argument("--content-threshold", type=float, default=27.0)
    parser.add_argument("--min-scene-len", type=int, default=15)
    parser.add_argument("--filter-mode", choices=["merge", "suppress"], default="merge")
    parser.add_argument("--adaptive-threshold", type=float, default=3.0)
    parser.add_argument("--adaptive-window-width", type=int, default=2)
    parser.add_argument("--adaptive-min-content-val", type=float, default=15.0)
    parser.add_argument("--tolerance-frames", type=int, default=0)
    parser.add_argument("--output", type=Path)
    parser.add_argument("--progress", action="store_true")
    args = parser.parse_args()
    if not args.root.exists():
        raise SystemExit(f"dataset root does not exist: {args.root}")
    if args.limit is not None and args.limit < 0:
        raise SystemExit("--limit must be greater than or equal to 0")
    if args.resize_width is not None and args.resize_width <= 0:
        raise SystemExit("--resize-width must be greater than 0")
    if args.min_scene_len <= 0:
        raise SystemExit("--min-scene-len must be greater than 0")
    if args.adaptive_window_width <= 0:
        raise SystemExit("--adaptive-window-width must be greater than 0")
    args.video_id = unique(args.video_id)
    return args


def unique(values: list[str]) -> list[str]:
    seen = set()
    output = []
    for value in values:
        if value not in seen:
            seen.add(value)
            output.append(value)
    return output


def collect_files(root: Path) -> list[Path]:
    return sorted(path for path in root.rglob("*") if path.is_file())


def video_id(path: Path) -> str:
    return path.stem


def video_files(root: Path) -> list[Path]:
    return [path for path in collect_files(root) if path.suffix.lower() in VIDEO_EXTENSIONS]


def annotation_map(root: Path) -> dict[str, Path]:
    annotations: dict[str, Path] = {}
    for path in collect_files(root):
        if path.suffix.lower() not in ANNOTATION_EXTENSIONS:
            continue
        stem = path.stem
        annotations.setdefault(stem, path)
        prefix = stem.split("-", 1)[0]
        annotations.setdefault(prefix, path)
        if prefix.isdigit():
            annotations.setdefault(f"bbc_{prefix}", path)
    return annotations


def select_videos(videos: list[Path], requested: list[str], limit: int | None) -> list[Path]:
    by_id = {video_id(path): path for path in videos}
    if requested:
        missing = [value for value in requested if value not in by_id]
        if missing:
            raise SystemExit(f"requested video IDs were not found: {', '.join(missing)}")
        selected = [by_id[value] for value in requested]
    else:
        selected = sorted(by_id.values())
    if limit is not None:
        selected = selected[:limit]
    return selected


def load_annotation(path: Path) -> list[int]:
    cuts: list[int] = []
    for raw_line in path.read_text().splitlines():
        line = raw_line.strip()
        if not line or line.startswith("#"):
            continue
        fields = [field for field in line.replace("\t", " ").replace(",", " ").split(" ") if field]
        if not fields:
            continue
        value = fields[1] if len(fields) > 1 else fields[0]
        try:
            cuts.append(int(value) + 1)
        except ValueError:
            continue
    return cuts


def scaled_even_height(input_width: int, input_height: int, output_width: int) -> int:
    scaled = (input_height * output_width + input_width // 2) // input_width
    return max(2, scaled if scaled % 2 == 0 else scaled + 1)


def make_detector(args: argparse.Namespace) -> Any:
    from scenedetect.detectors import (
        AdaptiveDetector,
        ContentDetector,
        HashDetector,
        HistogramDetector,
        ThresholdDetector,
    )
    from scenedetect.scene_detector import FlashFilter

    if args.detector == "content":
        mode = FlashFilter.Mode.MERGE if args.filter_mode == "merge" else FlashFilter.Mode.SUPPRESS
        return ContentDetector(
            threshold=args.content_threshold,
            min_scene_len=args.min_scene_len,
            filter_mode=mode,
        )
    if args.detector == "adaptive":
        return AdaptiveDetector(
            adaptive_threshold=args.adaptive_threshold,
            min_scene_len=args.min_scene_len,
            window_width=args.adaptive_window_width,
            min_content_val=args.adaptive_min_content_val,
        )
    if args.detector == "threshold":
        return ThresholdDetector(threshold=12.0, min_scene_len=args.min_scene_len)
    if args.detector == "histogram":
        return HistogramDetector(threshold=0.05, bins=256, min_scene_len=args.min_scene_len)
    if args.detector == "hash":
        return HashDetector(threshold=0.395, size=16, lowpass=2, min_scene_len=args.min_scene_len)
    raise AssertionError(args.detector)


def detect_video(path: Path, args: argparse.Namespace) -> dict[str, Any]:
    import cv2

    detector = make_detector(args)
    cap = cv2.VideoCapture(str(path))
    if not cap.isOpened():
        raise RuntimeError(f"OpenCV could not open video: {path}")

    cuts: list[int] = []
    frame_count = 0
    decode_resize_elapsed = 0.0
    detector_elapsed = 0.0
    started = time.perf_counter()
    last_frame_num: int | None = None
    try:
        while True:
            decode_started = time.perf_counter()
            ok, frame = cap.read()
            if ok and args.resize_width:
                height = scaled_even_height(frame.shape[1], frame.shape[0], args.resize_width)
                frame = cv2.resize(frame, (args.resize_width, height), interpolation=cv2.INTER_LINEAR)
            decode_resize_elapsed += time.perf_counter() - decode_started
            if not ok:
                break
            frame_num = frame_count
            frame_count += 1
            last_frame_num = frame_num
            detector_started = time.perf_counter()
            cuts.extend(int(cut) for cut in detector.process_frame(frame_num, frame))
            detector_elapsed += time.perf_counter() - detector_started
        if last_frame_num is not None and hasattr(detector, "post_process"):
            detector_started = time.perf_counter()
            cuts.extend(int(cut) for cut in detector.post_process(last_frame_num))
            detector_elapsed += time.perf_counter() - detector_started
    finally:
        cap.release()

    elapsed = time.perf_counter() - started
    cuts = suppress_nearby_cuts(sorted(set(cuts)), 0)
    return {
        "predictedCuts": cuts,
        "elapsedMs": elapsed * 1000.0,
        "decodeResizeElapsedMs": decode_resize_elapsed * 1000.0,
        "detectorElapsedMs": detector_elapsed * 1000.0,
        "frameCount": frame_count,
        "effectiveFps": frame_count / elapsed if elapsed > 0.0 else 0.0,
    }


def suppress_nearby_cuts(cuts: list[int], window: int) -> list[int]:
    if window == 0:
        return cuts
    kept: list[int] = []
    for cut in cuts:
        if not kept or cut - kept[-1] >= window:
            kept.append(cut)
    return kept


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


def summarize(videos: list[dict[str, Any]], tolerance: int) -> dict[str, float]:
    correct = 0
    predicted = 0
    truth = 0
    elapsed = 0.0
    for video in videos:
        predicted_cuts = [int(value) for value in video["predictedCuts"]]
        truth_cuts = [int(value) for value in video["groundTruthCuts"]]
        correct += matched_count(predicted_cuts, truth_cuts, tolerance)
        predicted += len(predicted_cuts)
        truth += len(truth_cuts)
        elapsed += float(video["elapsedMs"])
    recall = correct / truth if truth else 0.0
    precision = correct / predicted if predicted else 0.0
    f1 = 2.0 * recall * precision / (recall + precision) if recall + precision else 0.0
    return {
        "recall": recall,
        "precision": precision,
        "f1": f1,
        "avgElapsedMs": elapsed / len(videos) if videos else 0.0,
    }


def configuration(args: argparse.Namespace) -> dict[str, Any]:
    return {
        "resizeWidth": args.resize_width,
        "contentThreshold": args.content_threshold if args.detector == "content" else None,
        "minSceneLen": args.min_scene_len,
        "filterMode": args.filter_mode if args.detector == "content" else None,
        "adaptiveThreshold": args.adaptive_threshold if args.detector == "adaptive" else None,
        "adaptiveWindowWidth": (
            args.adaptive_window_width if args.detector == "adaptive" else None
        ),
        "adaptiveMinContentVal": (
            args.adaptive_min_content_val if args.detector == "adaptive" else None
        ),
        "postFilterWindow": 0,
    }


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
    annotations = annotation_map(args.root)
    selected = select_videos(video_files(args.root), args.video_id, args.limit)
    if not selected:
        raise SystemExit(f"no video files selected under `{args.root}`")
    selected_ids = [video_id(path) for path in selected]

    videos = []
    for path in selected:
        ident = video_id(path)
        if args.progress:
            print(f"pyscenedetect_scene_dataset_eval: start {ident}", file=sys.stderr)
        detection = detect_video(path, args)
        annotation_path = annotations.get(ident)
        videos.append(
            {
                "id": ident,
                "path": str(path),
                "annotationPath": str(annotation_path) if annotation_path else None,
                **detection,
                "groundTruthCuts": load_annotation(annotation_path) if annotation_path else [],
            }
        )
        if args.progress:
            print(f"pyscenedetect_scene_dataset_eval: finish {ident}", file=sys.stderr)

    report = {
        "dataset": args.dataset,
        "detector": args.detector,
        "implementation": "pyscenedetect",
        "configuration": configuration(args),
        "videos": videos,
        "summary": summarize(videos, args.tolerance_frames),
        "mode": {
            "videoIds": selected_ids,
            "resizeWidth": args.resize_width,
            "complete": True,
        },
    }
    write_json(args.output, report)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
