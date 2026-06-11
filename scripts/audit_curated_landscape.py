#!/usr/bin/env python3
"""Audit and document curated landscape metadata declarations."""

from __future__ import annotations

import argparse
import difflib
import re
import sys
from dataclasses import dataclass
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
MATRIX_PATH = ROOT / "docs" / "CURATED_LANDSCAPE_MATRIX.md"
LANDSCAPE_PATH = ROOT / "crates" / "runtime" / "runtime-core" / "src" / "landscape.rs"

GROUP_ORDER = {"foundation": 0, "workflow": 1}


@dataclass(frozen=True)
class PilotDeclaration:
    group: str
    package: str
    path: str
    operation: str
    function_id: str


@dataclass(frozen=True)
class LandscapeRow:
    group: str
    package: str
    operation: str
    function_id: str
    type_ids: tuple[str, ...]
    path: str


FOUNDATION_DECLARATIONS = [
    PilotDeclaration(
        "foundation",
        "moritzbrantner-text-core",
        "crates/text/text-core/src/surface.rs",
        "text.tokenize",
        "text.core.tokenize",
    ),
    PilotDeclaration(
        "foundation",
        "moritzbrantner-text-transcripts",
        "crates/text/text-transcripts/src/surface.rs",
        "transcripts.toTextSegments",
        "text.transcripts.toTextSegments",
    ),
    PilotDeclaration(
        "foundation",
        "moritzbrantner-image-analysis-core",
        "crates/image/image-analysis-core/src/surface.rs",
        "image.core.summary",
        "image.core.summarizeImage",
    ),
    PilotDeclaration(
        "foundation",
        "moritzbrantner-audio-analysis-core",
        "crates/audio/audio-analysis-core/src/surface.rs",
        "audio.levels",
        "audio.core.levels",
    ),
    PilotDeclaration(
        "foundation",
        "moritzbrantner-vision-core",
        "crates/vision/vision-core/src/surface.rs",
        "vision.validateDetection",
        "vision.validateDetection",
    ),
    PilotDeclaration(
        "foundation",
        "moritzbrantner-vision-core",
        "crates/vision/vision-core/src/surface.rs",
        "vision.validateEmbedding",
        "vision.validateEmbedding",
    ),
    PilotDeclaration(
        "foundation",
        "moritzbrantner-vector-analysis-core",
        "crates/vector/vector-analysis-core/src/surface.rs",
        "vector.normalize",
        "vector.normalize",
    ),
    PilotDeclaration(
        "foundation",
        "moritzbrantner-tensor-data",
        "crates/data/tensor-data/src/surface.rs",
        "tensor.validate",
        "tensor.validate",
    ),
    PilotDeclaration(
        "foundation",
        "moritzbrantner-numbers-core",
        "crates/data/numbers-core/src/surface.rs",
        "numbers.summary",
        "numbers.summary",
    ),
    PilotDeclaration(
        "foundation",
        "moritzbrantner-math-geometry-2d",
        "crates/math/math-geometry-2d/src/surface.rs",
        "geometry.transform",
        "geometry.transformPoints",
    ),
]

WORKFLOW_DECLARATIONS = [
    PilotDeclaration(
        "workflow",
        "moritzbrantner-text-analysis",
        "crates/text/text-analysis/src/surface.rs",
        "analysis.document",
        "text.analysis.analyzeDocument",
    ),
    PilotDeclaration(
        "workflow",
        "moritzbrantner-text-retrieval",
        "crates/text/text-retrieval/src/surface.rs",
        "retrieval.search",
        "text.retrieval.search",
    ),
    PilotDeclaration(
        "workflow",
        "moritzbrantner-audio-analysis-transcription",
        "crates/audio/audio-analysis-transcription/src/surface.rs",
        "audio.transcription.transcribe",
        "audio.transcription.transcribe",
    ),
    PilotDeclaration(
        "workflow",
        "moritzbrantner-audio-analysis-transcription",
        "crates/audio/audio-analysis-transcription/src/surface.rs",
        "audio.transcription.importWhisperX",
        "audio.transcription.importWhisperX",
    ),
    PilotDeclaration(
        "workflow",
        "moritzbrantner-image-analysis-detection",
        "crates/image/image-analysis-detection/src/surface.rs",
        "image.detection.colorBlob",
        "image.detection.detectColorBlob",
    ),
    PilotDeclaration(
        "workflow",
        "moritzbrantner-video-analysis-core",
        "crates/video/video-analysis-core/src/surface.rs",
        "video.core.timecode",
        "video.core.parseTimecode",
    ),
    PilotDeclaration(
        "workflow",
        "moritzbrantner-video-analysis-detectors",
        "crates/video/video-analysis-detectors/src/surface.rs",
        "video.detectors.compositePlan",
        "video.detectors.planCompositeDetector",
    ),
    PilotDeclaration(
        "workflow",
        "moritzbrantner-video-analysis-output",
        "crates/video/video-analysis-output/src/surface.rs",
        "video.output.csvPlan",
        "video.output.planSceneListCsv",
    ),
    PilotDeclaration(
        "workflow",
        "moritzbrantner-video-analysis-sfm",
        "crates/video/video-analysis-sfm/src/surface.rs",
        "video.sfm.matchPlan",
        "video.sfm.planMatches",
    ),
    PilotDeclaration(
        "workflow",
        "moritzbrantner-video-analysis-radiance-fields",
        "crates/video/video-analysis-radiance-fields/src/surface.rs",
        "video.radiance.cameraPath",
        "video.radiance.planCameraPath",
    ),
    PilotDeclaration(
        "workflow",
        "moritzbrantner-video-analysis-radiance-pipeline",
        "crates/video/video-analysis-radiance-pipeline/src/surface.rs",
        "video.radiancePipeline.assetCheck",
        "video.radiancePipeline.checkAssets",
    ),
]


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true", help="rewrite docs/CURATED_LANDSCAPE_MATRIX.md")
    parser.add_argument("--check", action="store_true", help="fail if the generated matrix is stale")
    args = parser.parse_args()
    if args.write and args.check:
        parser.error("--write and --check are mutually exclusive")

    rows = collect_rows()
    content = render_matrix(rows)

    if args.write:
        MATRIX_PATH.write_text(content, encoding="utf-8")
        return 0

    if args.check:
        if not MATRIX_PATH.exists():
            print(f"{MATRIX_PATH.relative_to(ROOT)} is missing; run scripts/audit_curated_landscape.py --write", file=sys.stderr)
            return 1
        existing = MATRIX_PATH.read_text(encoding="utf-8")
        if existing != content:
            diff = "\n".join(
                difflib.unified_diff(
                    existing.splitlines(),
                    content.splitlines(),
                    fromfile=str(MATRIX_PATH.relative_to(ROOT)),
                    tofile="generated",
                    lineterm="",
                )
            )
            print(f"{MATRIX_PATH.relative_to(ROOT)} is out of date; run scripts/audit_curated_landscape.py --write", file=sys.stderr)
            print(diff, file=sys.stderr)
            return 1
        return 0

    print(content, end="")
    return 0


def collect_rows() -> list[LandscapeRow]:
    known_type_helpers = collect_known_type_helpers()
    known_owners = collect_known_owner_packages()
    function_ids: set[str] = set()
    rows: list[LandscapeRow] = []
    for declaration in FOUNDATION_DECLARATIONS + WORKFLOW_DECLARATIONS:
        if declaration.group not in GROUP_ORDER:
            raise SystemExit(f"unknown curated landscape group `{declaration.group}`")
        if declaration.package not in known_owners:
            raise SystemExit(f"unknown curated owner `{declaration.package}`")
        path = ROOT / declaration.path
        source = path.read_text(encoding="utf-8")
        if declaration.operation not in source:
            raise SystemExit(f"{declaration.path} missing operation `{declaration.operation}`")
        if declaration.function_id not in source:
            raise SystemExit(f"{declaration.path} missing curated function `{declaration.function_id}`")
        if declaration.function_id in function_ids:
            raise SystemExit(f"duplicate curated function id `{declaration.function_id}`")
        function_ids.add(declaration.function_id)
        type_ids = collect_type_ids(source, declaration.function_id, known_type_helpers)
        if not type_ids:
            raise SystemExit(f"{declaration.path} `{declaration.function_id}` has no well-known curated types")
        rows.append(
            LandscapeRow(
                declaration.group,
                declaration.package,
                declaration.operation,
                declaration.function_id,
                tuple(type_ids),
                declaration.path,
            )
        )
    return sorted(rows, key=lambda row: (GROUP_ORDER[row.group], row.package, row.operation))


def collect_known_type_helpers() -> dict[str, str]:
    source = LANDSCAPE_PATH.read_text(encoding="utf-8")
    constants = dict(re.findall(r'pub const ([A-Z0-9_]+): &str = "([^"]+)";', source))
    helpers: dict[str, str] = {}
    for helper, body in re.findall(
        r"pub fn ([a-z0-9_]+)\(\) -> LandscapeTypeRef \{(.*?)\n    \}",
        source,
        flags=re.DOTALL,
    ):
        match = re.search(r"type_ref\(\s*([A-Z0-9_]+),", body)
        if not match:
            continue
        const_name = match.group(1)
        if const_name in constants:
            helpers[helper] = constants[const_name]
    return helpers


def collect_known_owner_packages() -> set[str]:
    source = LANDSCAPE_PATH.read_text(encoding="utf-8")
    return set(re.findall(r'pub const OWNER_[A-Z0-9_]+: &str\s*=\s*"([^"]+)";', source, flags=re.MULTILINE))


def collect_type_ids(source: str, function_id: str, known_type_helpers: dict[str, str]) -> list[str]:
    match = re.search(
        r"LandscapeFunction::new\(\s*\"" + re.escape(function_id) + r"\"",
        source,
        flags=re.MULTILINE,
    )
    if not match:
        return []
    next_match = re.search(r"LandscapeFunction::new\(", source[match.end() :])
    end = match.end() + next_match.start() if next_match else match.start() + 4000
    window = source[match.start() : end]
    helpers = re.findall(r"well_known::([a-z0-9_]+)\(", window)
    type_ids = []
    for helper in helpers:
        try:
            type_id = known_type_helpers[helper]
        except KeyError as error:
            raise SystemExit(f"unknown well-known type helper `{helper}` near `{function_id}`") from error
        if type_id not in type_ids:
            type_ids.append(type_id)
    return type_ids


def render_matrix(rows: list[LandscapeRow]) -> str:
    lines = [
        "# Curated Landscape Matrix",
        "",
        "<!-- Generated by scripts/audit_curated_landscape.py; do not edit by hand. -->",
        "",
        "This matrix lists operations that declare curated landscape metadata through `xLandscape` schema extensions.",
        "",
        "Regenerate it after changing landscape declarations:",
        "",
        "```bash",
        "python3 scripts/audit_curated_landscape.py --write",
        "```",
        "",
        "| Group | Package | Operation | Curated function | Curated types | Source |",
        "|---|---|---|---|---|---|",
    ]
    for row in rows:
        lines.append(
            "| "
            + " | ".join(
                [
                    row.group,
                    tick(row.package),
                    tick(row.operation),
                    tick(row.function_id),
                    ", ".join(tick(type_id) for type_id in row.type_ids),
                    tick(row.path),
                ]
            )
            + " |"
        )
    return "\n".join(lines) + "\n"


def tick(value: str) -> str:
    return f"`{value}`"


if __name__ == "__main__":
    raise SystemExit(main())
