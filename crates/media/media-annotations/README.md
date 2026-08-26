# media-annotations

Canonical media-neutral annotations built on `moenarch-media-core` timing.

This crate is the interoperability layer for findings that need to coexist on
one media timeline without moving domain models such as visual detections,
transcripts, scenes, poses, or audio features into `media-core`.

It owns:

- `MediaAnnotation`, a stable envelope with id, kind, label, timing, source,
  selector, score, provenance, value, and attributes;
- `AnnotationTiming`, which represents either an exact instant or a validated
  half-open `MediaRange`;
- selectors for frames, text segments/spans, 2D regions, tracks, and custom
  domain selectors;
- `AnnotationDataset`, including validation, exact temporal sorting, queries,
  and duplicate-safe merging;
- JSON and JSONL readers/writers that validate the annotation model on both
  input and output;
- a lossless adapter from the existing domain-neutral `AnalysisEvent`.

## Boundary

`media-annotations` depends only on `media-core` plus serialization support. It
does not depend on audio, text, image, vision, or video crates. Domain-specific
conversion belongs on the domain side of that boundary. This keeps the common
annotation format reusable without turning it into an ontology for every
analysis result.

## Timing

Instants retain exact PTS/timebase values. Ranges use the half-open `[start,
end)` semantics defined by `MediaRange`. JSON serialization uses the same
`pts` plus rational `timebase` representation and validates it when read.
Temporal sorting, point queries, and range queries therefore compare rational
media time rather than trusting a duplicated floating-point `seconds` field.

## Storage

`write_json` / `read_json` preserve complete dataset metadata and annotations.
`write_jsonl` / `read_jsonl` provide an annotation stream; JSONL intentionally
contains annotation records only, so dataset-level metadata is not round-tripped
through that format.

## Compatibility migration

The existing `video-analysis-dataset`, `video-analysis-storage`, and
`video-analysis-transform` crates remain compatibility surfaces. The
video-owned `video-analysis-annotation-compat` adapter converts their retained
records into this canonical annotation dataset while preserving the full legacy
record as JSON payload. New cross-media consumers should use this crate instead
of introducing another timestamped finding schema.
