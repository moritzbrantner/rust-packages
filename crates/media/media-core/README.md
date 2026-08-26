# media-core

Neutral media contracts shared across audio, video, text, and future media
consumers.

The crate owns only:

- `Timebase`, the rational duration of one timestamp tick, with additive
  validation for new boundary code;
- `Timestamp`, presentation ticks paired with their timebase, plus exact
  chronological comparison and lossless rescaling helpers;
- `MediaRange`, a validated half-open `[start, end)` range whose endpoints may
  use different timebases;
- `AnalysisEvent`, a domain-neutral labeled result with optional time and score;
- the `annotations` module, which provides a neutral annotation envelope,
  source/selectors/provenance, exact temporal queries, and JSON/JSONL storage;
- `PixelFormat` and `AudioSampleFormat`, compact stream-format identifiers
  without frame or buffer ownership;
- `DetectError` and `Result`, the shared media error identity used across
  foundation and capability contracts.

The annotation layer deliberately does not define scenes, transcript segments,
visual detections, poses, tracks, or audio features. Those remain domain-owned
and are adapted into `annotations::MediaAnnotation` at domain boundaries. This
keeps `media-core` useful as an interoperability foundation without turning it
into a cross-domain ontology.

The legacy `Timebase::new` and `Timestamp::new` constructors remain available
for compatibility. New source, serialization, and annotation boundaries should
prefer the validated constructors and chronological helpers rather than relying
on structural `Ord` or unchecked floating-point conversion.

`moenarch-video-analysis-core` re-exports the existing neutral types to preserve
its public API and type identity while consumers migrate. New neutral contracts
may be consumed directly from `moenarch-media-core` until compatibility
re-exports are deliberately expanded.

## Time semantics

A valid `Timebase` has a positive numerator and denominator. `Timestamp`
comparison across different timebases is performed exactly with rational
integer arithmetic through `chronological_cmp`; it does not convert through
`f64` seconds. `rescale_exact` succeeds only when the destination timebase can
represent the same instant with an integral PTS value.

`MediaRange` uses half-open semantics: the start is included and the end is
excluded. Empty ranges are valid. Range construction, containment, overlap, and
duration validate their timestamps and compare endpoints chronologically across
timebases.

## Annotation interoperability

`annotations::MediaAnnotation` is the common envelope for findings that need to
coexist across media domains. It carries a stable id and kind plus optional
label, exact instant or `MediaRange`, source/stream identity, source selector,
finite score, provenance, structured value, and adapter-specific attributes.
Selectors cover frames, text segments and spans, 2D regions, tracks, and custom
structured selectors without importing domain-specific types.

`annotations::AnnotationDataset` validates unique annotation ids and supports
exact chronological sorting, point-in-time queries, half-open range overlap
queries, kind filtering, and duplicate-safe merging. JSON preserves dataset
metadata and exact PTS/timebase values. JSONL is an annotation-stream format and
therefore intentionally omits dataset-level metadata.

The legacy `AnalysisEvent` converts directly into this envelope. Richer domain
models should be adapted from their owning crate rather than moved into
`media-core`; for example, `video-analysis-storage::annotations` converts the
existing retained video-analysis dataset without introducing a reverse
dependency from foundation into video.

## Ownership audit

The issue #108 contract audit kept media data in its narrowest existing domain:

- video frame and pixel-buffer contracts remain in `video-analysis-core`;
- audio buffer and audio-frame contracts remain in `audio-analysis-core`;
- image contracts remain in `image-analysis-core`;
- transcript and text contracts remain in their text crates;
- source and stream metadata remain in `video-analysis-ingest`;
- detection algorithms and model-lifecycle contracts remain with their current
  domain or foundation owners.

The format enums are metadata identifiers used by error and stream contracts;
they do not own media data. Moving the shared error identity alongside those
identifiers lets non-visual foundation crates stop depending on video ownership
without copying error DTOs or changing downstream result types.

No cross-family range contract existed at the source head audited by issue
#108, so that extraction did not invent one. `MediaRange` and the neutral
annotation envelope were added later as explicit interoperability prerequisites;
they remain intentionally separate from domain-owned media data.

## Candidate consumer audit

The issue #108 audit inspected the then-current default-branch heads of the
named candidate consumers. `video-analysis-studio` was the only candidate that
imported the original neutral Rust types directly:

| Consumer | Audited commit | Neutral contract use |
| --- | --- | --- |
| `geo-analysis` | `804f802f1459a7b1d0359cc805235715a5419b78` | none |
| `native-whisperx` | `b0ba12342fbb36b057fbe620f62d52c4fde0b36d` | none; its `video-analysis-core` use is error/domain behavior |
| `media-similarity` | `d015b36187a9c3ebd202f81175081608fb307aa3` | none; its imports are frame, scene, detection, and source contracts |
| `youtube-corpus` | `8ab21570348e7d636685a51f110f11fc2eacf363` | none |
| `video-analysis-studio` | `93ceeb1c43764be9d31c35258145604559e0a0aa` | `AnalysisEvent`, `Timebase`, and `Timestamp` |
| `stutter-tracker` | `6c68b7a343ac8470405a79f240263f9e8ca7af80` | none; its imports are video-owned errors |
| `viz-engine` | `29b85cf331701f66a796b89b5263faacf3d8998c` | none; its import is a video-owned error |

A candidate `video-analysis-studio` patch adds `moenarch-media-core`, moves
only those three imports to `media_core`, and leaves video-domain imports on
`video_analysis_core`. Its diff hash is
`40da0d918ab91c6ed2193219f3dc5983aeb1d86866c27e0a6186e9984cb55cd7`;
`git diff --check` and standalone Rust formatting checks pass. The exact-type
compatibility test in this repository proves that the patch is optional until
the consumer migration is scheduled.
