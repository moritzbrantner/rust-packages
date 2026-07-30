# ADR 0013: `scenedetect-rs` Owns Canonical Scene Detection

## Status

Accepted.

This decision is architectural only. It does not move source, add a dependency,
change a public API or serialized shape, raise an MSRV, publish a package, or
authorize a release.

## Context

Scene detection currently has overlapping ownership in two repositories.
`rust-packages` exposes published visual-analysis contracts, detector
implementations, package operations, output helpers, and split execution.
`scenedetect-rs` is a focused scene-detection product with its own public core
contracts, five detector implementations, FFmpeg frame source, CLI, serialized
artifacts, and PySceneDetect parity suite.

The overlap was audited at these immutable revisions:

- `moritzbrantner/rust-packages`
  `775fe13557d8d830a5f5b34a44aac05ee15d6437`
- `moritzbrantner/scenedetect-rs`
  `ec7e2aa6328b2e640c9be7618df6711b325a212a` (`origin/main`)

At those revisions:

- `rust-packages` has an MSRV of Rust 1.95. Its registry-visible scene-related
  packages include `moenarch-video-analysis-core` 0.1.3,
  `moenarch-video-analysis-detectors` 0.1.0, and
  `moenarch-video-analysis-split` 0.1.0.
- `scenedetect-rs` has an MSRV of Rust 1.87 and workspace version 0.1.0.
  `cargo search` did not find registry releases for `scenedetect-core`,
  `scenedetect-cli`, or `scenedetect-ffmpeg`.
- `video-analysis-core` owns `FramePosition`, `VideoFrame`, `Scene`, `Cut`,
  `MetricsStore`, `SceneDetector`, `ScenePipeline`, and one active
  `ContentDetector` implementation.
- `video-analysis-detectors` re-exports that `ContentDetector`, implements
  adaptive, threshold, histogram, hash, flash-filter, and weighted-composite
  detection, and exposes the `video.detectors.*` package operations.
- `scenedetect-core` owns serializable frame, boundary, scene-list, detection
  stats, detector configuration, and boundary-review contracts. It implements
  content, adaptive, threshold, histogram, and hash detection through bounded
  streaming APIs.
- The repositories target different PySceneDetect baselines:
  `rust-packages` documents 0.6.7.1 as its stable lane, while
  `scenedetect-rs` runs required CLI parity against 0.7.

Leaving both repositories as undifferentiated owners would make the
visual-analysis extraction preserve two active sources for the same five
algorithms and two competing scene artifact languages.

## Ownership matrix

| Surface | Current overlap | Canonical owner after migration | Retained differentiated purpose |
| --- | --- | --- | --- |
| Scene-specific frame, boundary, span, and scene-list contracts | `scenedetect-core` and `video-analysis-core` | `scenedetect-core` | `video-analysis-core` keeps its published timestamp-aware compatibility views and pipeline adapters until their own semver migration gates pass. Neutral timestamps and timebases remain foundation-owned. |
| Detection configuration and minimum-scene-length policy | Both cores and detector constructors | `scenedetect-core` | Visual constructors remain compatibility adapters. They must not become a second canonical configuration schema. |
| Detection Stats, Boundary Candidate review, and reusable Scene List artifacts | Both metrics/output stacks | `scenedetect-core` | Visual result/report DTOs remain versioned compatibility and package-surface responses. |
| Content detection | `video-analysis-core` and `scenedetect-core` | `scenedetect-core` | The visual public constructor paths adapt to the canonical implementation. |
| Adaptive detection | `video-analysis-detectors` and `scenedetect-core` | `scenedetect-core` | The visual public constructor path adapts to the canonical implementation. |
| Threshold/fade detection | `video-analysis-detectors` and `scenedetect-core` | `scenedetect-core` | The visual public constructor path adapts to the canonical implementation. |
| Histogram detection | `video-analysis-detectors` and `scenedetect-core` | `scenedetect-core` | The visual public constructor path adapts to the canonical implementation. |
| Perceptual-hash detection | `video-analysis-detectors` and `scenedetect-core` | `scenedetect-core` | The visual public constructor path adapts to the canonical implementation. |
| Flash/minimum-length filtering | Both detector stacks | `scenedetect-core` for canonical detector behavior | Visual compatibility modes remain only while required by published constructor behavior. |
| Weighted composite detection and visual score composition | Only `video-analysis-detectors` | `moenarch-video-analysis-detectors` | This is a visual-analysis extension, not a PySceneDetect-compatible canonical detector. Its components consume canonical detector scores through adapters. |
| Multi-detector visual pipeline and observation grouping | `video-analysis-core` | `moenarch-video-analysis-core` | It composes canonical detection with broader visual-analysis workflows. |
| Split planning, deterministic naming, split jobs, and execution | `video-analysis-split`; no equivalent in `scenedetect-rs` | `moenarch-video-analysis-split` | `scenedetect-core::SceneList` is an accepted input through a conversion adapter; split policy does not move into `scenedetect-core`. |
| General FFmpeg probing, ingest, audio selection, and reusable decode | `video-analysis-ffmpeg` and a narrower scene frame source | `moenarch-video-analysis-ffmpeg` | `scenedetect-ffmpeg` remains the process-backed Frame Source for the standalone CLI. It is not a general media-IO owner. |
| Dedicated PySceneDetect-compatible and native detect/render CLI | `scenedetect-cli` and broader visual CLIs | `scenedetect-cli` / `scenedetect-rs` binary | Visual CLIs retain their `vanalyze` and package-operation workflows; they do not clone the dedicated CLI. |
| PySceneDetect parity oracle and five-detector compatibility lane | Both repositories | `scenedetect-rs` | Visual tests become adapter/differential tests and retain only visual pipeline, package-surface, and split compatibility assertions. |
| Scene CSV, JSON, NDJSON, HTML, and stats artifacts | Both output stacks, with different existing shapes | `scenedetect-core` for canonical scene artifacts | Existing `video-analysis-output` shapes remain compatibility formats until a separately versioned migration. EDL, FCP7, FCPXML, OTIO, and qpfile output remain visual output ownership. |
| Runtime/package operations | Only visual-analysis exposes `video.core.*`, `video.detectors.*`, `video.output.*`, and `video.split.*` | Their existing `moenarch-video-analysis-*` package owner | Operations adapt the canonical library behavior without changing operation IDs or transport shapes. |
| Publishing | Published visual packages versus unreleased `scenedetect-*` manifests | `scenedetect-rs` releases `scenedetect-*`; visual-analysis releases `moenarch-video-analysis-*` | One repository publishes each retained package name. No package moves between publisher namespaces in this decision. |

## Decision

Choose **Model A: `scenedetect-rs` owns detection**.

`scenedetect-core` is the canonical owner of scene-specific contracts, the
content/adaptive/threshold/histogram/hash implementations, detection stats,
scene-list artifacts, and PySceneDetect parity behavior.

The visual package family remains the owner of its broader visual pipeline,
weighted composition, stable package operations, compatibility DTOs, adapters,
general FFmpeg integration, edit-list output, and scene-based split planning
and execution.

The adapter direction is:

```text
visual frame/timestamp compatibility input
  -> visual-owned adapter
  -> scenedetect-core canonical detector
  -> visual-owned adapter
  -> existing visual result/package/output/split contracts
```

Adapters must convert explicitly. They must not copy canonical DTO
definitions. A compatibility re-export is preferred where the canonical type
can preserve the existing Rust type identity. Where the current shapes are
semantically different, a named conversion is required and neither type may be
presented as an alias for the other.

`scenedetect-core` must not depend on the visual package family. The visual
family may depend on a published `scenedetect-core` after its release and
consumer gates pass. `scenedetect-ffmpeg` and `moenarch-video-analysis-ffmpeg`
must not depend on each other merely to share command invocation.

## Package and compatibility commitments

The following names remain:

- `scenedetect-core`, `scenedetect-ffmpeg`, `scenedetect-cli`, and the
  `scenedetect-rs` binary in `moritzbrantner/scenedetect-rs`;
- `moenarch-video-analysis-core`,
  `moenarch-video-analysis-detectors`,
  `moenarch-video-analysis-output`,
  `moenarch-video-analysis-split`, and their existing focused adapters in
  visual-analysis;
- existing npm/WASM package names and every existing `video.core.*`,
  `video.detectors.*`, `video.output.*`, and `video.split.*` operation ID.

Migration must preserve:

- the type identity currently promised by
  `video_analysis_core::ContentDetector` and the
  `video_analysis_detectors::ContentDetector` re-export;
- public `SceneDetector`, `ScenePipeline`, `Scene`, `Cut`, `DetectionResult`,
  `SplitPlan`, and executor behavior until a separately approved semver change;
- current visual serialized shapes, field casing, frame-index conventions, CSV
  columns, errors, and package response envelopes;
- current `scenedetect-core` Scene List, Detection Stats, Boundary Review, CSV,
  JSON, NDJSON, and HTML shapes;
- detector defaults and fixture outputs in both compatibility lanes until the
  adapter differential tests justify an intentional change;
- downstream source compatibility for the rust-packages facade,
  `media-similarity`, visual prototypes, and direct users of the published
  visual crates.

The two existing artifact shapes are not silently unified. The
`scenedetect-core` form is canonical for new scene-detection artifacts; the
visual form is a compatibility format with an explicit adapter and its own
snapshot tests.

## MSRV and release requirements

The `scenedetect-rs` MSRV remains Rust 1.87. This decision does not raise it.
Adapter code that needs the visual workspace's Rust 1.95 floor belongs in
visual-analysis. `scenedetect-core` may not acquire a foundation dependency
whose MSRV would raise its floor; map neutral timestamps at the visual adapter
boundary unless a compatible foundation release exists.

No publication is authorized here. The required release order is:

1. Stabilize and independently verify the canonical `scenedetect-core`
   contract at Rust 1.87.
2. Publish an exact authorized `scenedetect-core` version from
   `moritzbrantner/scenedetect-rs` and verify it from a clean registry consumer.
3. Add visual adapters and differential tests while retaining all existing
   implementations as rollback code.
4. Switch the visual detector paths to the published canonical implementation
   in a separately reviewed release.
5. Publish compatible visual package releases only when exact release issues
   and manifests authorize them.
6. Deprecate and later remove duplicate visual detector source in separate
   changes after the source-removal gates pass.

Repository movement alone is not a version bump. The later release issues must
select patch, minor, or breaking versions from the actual public change.

## Migration and deprecation sequence

1. Add explicit canonical detector types/configuration to `scenedetect-core`
   where the current config-driven API is insufficient for stable visual
   adapters. Keep bounded streaming and the Rust 1.87 floor.
2. Add a visual-owned adapter crate or module that maps borrowed visual frames
   and timestamp-aware positions into canonical detector input and maps
   canonical boundaries/stats back into the existing visual contracts.
3. Add differential tests for all five overlapping detectors on the same
   synthetic frames and options. Test frame boundaries, minimum-length
   behavior, delayed adaptive output, metrics, finalization, and errors.
4. Route the visual Content, Adaptive, Threshold, Histogram, and Hash public
   paths through the adapter. Preserve the existing ContentDetector re-export
   identity and all operation IDs.
5. Route overlapping visual scene artifact generation through explicit
   canonical-to-compatibility rendering adapters without changing existing
   bytes or JSON shapes.
6. Keep `video-analysis-split` as the split owner and accept canonical Scene
   Lists through a conversion layer. Do not add splitting to
   `scenedetect-core`.
7. Mark duplicate visual detector implementations deprecated only after the
   canonical path has shipped and downstream consumers have passed.
8. Remove duplicate implementation source only in a later source-removal PR.

There is no deprecation plan for the retained visual package names, package
operations, weighted composite detector, visual pipeline, general FFmpeg
adapter, output-only formats, or split executor.

## Source-removal gates

Duplicate detector source may leave visual-analysis only when all of the
following are true:

- a registry-verified `scenedetect-core` release exists and resolves without
  path or moving-branch Git dependencies;
- `scenedetect-rs` still passes its documented Rust 1.87 check;
- all five detector differential tests pass at the exact visual PR head;
- visual type-identity, serialization, error, default, and operation-ID tests
  pass;
- the 0.7 parity suite passes in `scenedetect-rs`, and the visual 0.6.7.1
  compatibility assertions that remain promised pass through the adapter;
- split-plan, FFmpeg, output, CLI, server, WASM, app, facade, and
  `media-similarity` consumer checks pass where affected;
- the visual-analysis extraction consumes the released canonical crate rather
  than copied source;
- rollback to the last visual implementation release is documented; and
- deprecation has shipped before removal.

If any gate fails, retain the current implementations and block source removal,
not the ownership decision.

## Visual-analysis extraction and downstream implications

The visual-analysis extraction copies visual compatibility contracts,
package-surface adapters, weighted composition, general FFmpeg integration,
output extensions, and split ownership. It must not copy the five canonical
detector implementations after the adapter migration. During the transition,
the exact duplicate source may remain in the monolith only as rollback code.

The rust-packages facade ultimately consumes released visual and
scene-detection packages. Existing visual consumers do not migrate directly to
`scenedetect-core` unless they want the canonical low-level scene API; they may
continue using the stable `moenarch-video-analysis-*` adapters.

Standalone `scenedetect-rs` users keep the focused CLI, the process-backed
`scenedetect-ffmpeg` adapter, native Detection Stats workflow, and PySceneDetect
parity behavior.

## Follow-up implementation issues

The implementation must be split into separately reviewable issues:

1. **Stabilize and release the Rust-1.87 canonical `scenedetect-core`
   contracts.** Add only adapter-required public seams, type/serialization
   tests, package metadata, and registry-consumer proof.
2. **Add five-detector differential parity between visual-analysis and
   `scenedetect-core`.** Use identical frame fixtures/options and record the
   explicit 0.6.7.1 versus 0.7 compatibility policy.
3. **Route published visual detector APIs through a visual-owned
   `scenedetect-core` adapter.** Preserve type identity, defaults, metrics,
   errors, and package operation IDs.
4. **Adapt canonical Scene Lists into visual output and split contracts.**
   Preserve existing visual bytes/JSON and keep split execution visual-owned.
5. **Deprecate and remove duplicate visual detector source after release and
   consumer proof.** Removal is last and must satisfy every source-removal gate.

These issues must not be combined with MSRV changes, package renames, visual
repository extraction, or release automation.

## Consequences

There is one long-term source of truth for scene contracts, the five common
detectors, artifacts, and parity behavior. Visual-analysis keeps its published
consumer surface and genuinely broader responsibilities. Splitting and general
FFmpeg ownership remain outside the canonical detector crate.

The transition is deliberately staged because the canonical package is not yet
published and the existing visual packages already have consumers. Duplicate
source remains temporarily, but its ownership and removal gates are explicit.
