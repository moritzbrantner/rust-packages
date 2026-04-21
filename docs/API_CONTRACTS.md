# API Contracts

This document describes the inter-package contracts that let the Rust crates and
the `@video-analysis/ui` package work together. It is intentionally not an
exhaustive rustdoc inventory. It focuses on shared types, traits, serialized
formats, file formats, package exports, and dependency boundaries.

`video-analysis-core` owns the canonical runtime contracts for time, media
samples, scene detection, metrics, observations, analyzers, and pipelines. Other
crates should compose around those contracts instead of defining parallel types.

## Workspace Contract Map

| Package | Role | Depends on | Exposes | Consumed by |
| --- | --- | --- | --- | --- |
| `video-analysis` | Root facade crate | Library crates except CLI and use cases | Re-exports core items, detector items, and package modules | Applications that want one import surface |
| `video-analysis-core` | Canonical shared contracts and pipelines | External utility crates only | Time/frame types, media samples, detection traits/results, analyzer traits/results, observations, metrics, pipeline builders | All functional Rust crates |
| `video-analysis-data` | Online stream normalization and aggregation | `video-analysis-core` | `DataRecord`, `DataPayload`, bucket configuration, bucket summaries, stream summaries | Use cases, reporting, UI JSON generation |
| `video-analysis-detectors` | Scene detector implementations | `video-analysis-core` | `SceneDetector` implementations, scoring algorithms, composite detector contracts | CLI, use cases, applications |
| `video-analysis-ingest` | Source abstraction layer | `video-analysis-core` | Media/source metadata, source traits, source-to-pipeline adapter helpers, text line source | FFmpeg crate, use cases, applications |
| `video-analysis-ffmpeg` | FFmpeg-backed media probing and decoding | `video-analysis-core`, `video-analysis-ingest` | FFmpeg video/audio sources, metadata, probe helpers, source options | CLI, use cases, applications |
| `video-analysis-models` | Model download, backend, normalization, and external command contracts | `video-analysis-core` | Hugging Face specs/downloads, raw and normalized predictions, model analyzer adapters, external command protocol | CLI model commands, use cases, applications |
| `video-analysis-output` | Detection output writers | `video-analysis-core` | Scene CSV, stats CSV, simple HTML, combined detection writers | CLI, applications |
| `video-analysis-split` | Scene-based media splitting | `video-analysis-core` | Split options, template variables, FFmpeg split function | CLI, applications |
| `video-analysis-radiance-fields` | Shared 3D geometry, camera, ray, and volume contracts | `video-analysis-core` | Vector/color/ray types, camera intrinsics/pose, radiance field trait, rendering/grid specs | Gaussian splatting, reconstruction, applications |
| `video-analysis-gaussian-splatting` | 3D Gaussian primitive projection and CPU compositing | `video-analysis-core`, `video-analysis-radiance-fields` | Gaussian primitives, projection config/results, splat rendering helpers | Applications and future 3D workflows |
| `video-analysis-reconstruction` | Sparse reconstruction and triangulation contracts | `video-analysis-core`, `video-analysis-radiance-fields` | Camera/image/point IDs, features, matches, tracks, sparse reconstruction, triangulation/projection helpers | Applications and future 3D workflows |
| `video-analysis-cli` | `vanalyze` command-line composition | Core, detectors, FFmpeg, models, output, split | CLI commands and file outputs | End users and automation |
| `video-analysis-use-cases` | Runnable end-to-end workflows | Core, data, detectors, FFmpeg, ingest, models | `youtube-video` workflow and JSON report schema | End users, `@video-analysis/ui`, web app |
| `@video-analysis/ui` | React/Tailwind views for analysis data | React peer deps and generated report/data shapes | TypeScript report types, component subpath exports, Tailwind content export | Web apps and report viewers |

## Canonical Core Contracts

Core contracts are the common language between packages.

### Time And Frame Position

- `Timebase` stores rational seconds per tick as `num` and `den`.
- `Timestamp` stores `pts` plus a `Timebase` and converts to seconds.
- `FramePosition` binds a `frame_index` to a `Timestamp`.
- `FrameTimecode` parses and formats frame/timecode values at a known frame
  rate.

Packages that exchange frame-local or stream-local data should preserve
`FramePosition` or `Timestamp` rather than using free-floating seconds when the
frame index still matters.

### Media Samples

- `VideoFrame<'a>` is a borrowed, read-only video frame view with position,
  dimensions, pixel format, byte slice, and stride.
- `OwnedVideoFrame` owns the frame buffer and bridges source boundaries; call
  `as_frame()` when passing to borrowed consumers.
- `PixelFormat` currently supports `Rgb24` and `Bgr24`.
- `AudioFrame<'a>` is a borrowed audio frame view with timestamp, sample rate,
  channel count, and `AudioBuffer`.
- `OwnedAudioFrame` owns audio data and bridges source boundaries.
- `AudioBuffer` carries typed sample vectors: `U8`, `I16`, `I32`, or `F32`.
- `TextSegment<'a>` is a borrowed text segment view with segment index, text,
  optional timestamp/duration/language, and finality.
- `OwnedTextSegment` owns text metadata and bridges source boundaries.

Borrowed media types are read-only views. Source crates emit owned media types;
pipelines and analyzers can then borrow them without copying.

### Detection

- `SceneDetector` accepts `VideoFrame<'_>` and an optional `MetricsSink`, then
  emits zero or more `Cut` values. `finish()` can emit delayed cuts.
- `ScenePipeline` owns detector composition and converts accepted cuts into
  `Scene` values.
- `Cut` stores the position, detector name, and optional score.
- `Scene` stores inclusive start and end `FramePosition` values for a scene
  interval.
- `MetricsSink` is the write-side metric interface.
- `MetricsStore` stores metrics keyed by frame index and string metric key.
- `DetectionResult` stores scenes, cuts, metrics, and frames processed.

Pipeline state is single-run by default. After `finish_detection()` or
`finish_analysis()`, callers must call `reset()` before processing more input.

### Analyzer And Observation

- `VideoAnalyzer` accepts `VideoFrame<'_>` and emits `Observation` values.
- `AudioAnalyzer` accepts `AudioFrame<'_>` and emits `AnalysisEvent` values.
- `TextAnalyzer` accepts `TextSegment<'_>` and emits `AnalysisEvent` values.
- `Observation` is the shared video enrichment record. It can carry timestamp,
  frame, scene index, analyzer name, `ObservationKind`, label, text, score,
  region, track id, and string attributes.
- `ObservationKind` classifies observations as text, face, object, scene, or a
  custom string.
- `AnalysisEvent` is the shared audio/text event record with optional timestamp,
  analyzer name, label, and optional score.

Model analyzers, heuristic analyzers, OCR integrations, face/object detectors,
and future enrichment packages should emit these core records.

## Ingest Contracts

`video-analysis-ingest` is the source abstraction layer. It lets source
implementations feed core pipelines without coupling those sources to detectors,
output writers, splitters, or CLI code.

Key contracts:

- `SourceMode` distinguishes `Recorded` and `Live` sources.
- `MediaSourceInfo` describes an input plus optional video/audio/text stream
  metadata.
- `VideoStreamInfo` carries dimensions, optional frame rate, and pixel format.
- `AudioStreamInfo` carries sample rate, channel count, and sample format.
- `TextStreamInfo` carries text format and optional language.
- `MediaSource` yields mixed `MediaSample` values.
- `VideoFrameSource` yields `OwnedVideoFrame` values.
- `AudioFrameSource` yields `OwnedAudioFrame` values.
- `TextSegmentSource` yields `OwnedTextSegment` values.

Adapter helpers connect sources to pipelines:

- `analyze_video_source`
- `analyze_video_frames`
- `analyze_realtime_video_source`
- `analyze_audio_source`
- `analyze_text_source`

Compatibility rule: source crates should emit owned core sample types through
ingest traits and should not depend on detector, output, split, CLI, or facade
crates.

## FFmpeg Contracts

`video-analysis-ffmpeg` is an implementation crate for FFmpeg-backed media
probing and decoding.

It exposes:

- `FfmpegVideoSource`, implementing core/ingest video source contracts.
- `FfmpegAudioSource`, implementing ingest audio source contracts.
- `VideoMetadata`, with input, optional path, mode, dimensions, frame rate, and
  optional duration.
- `AudioMetadata`, with input, optional path, mode, sample rate, channels, and
  optional duration.
- `FfmpegSourceOptions`, including source mode, realtime behavior, and extra
  input args.
- `FfmpegAudioSourceOptions`, including audio chunk size and extra input args.
- `probe`, `probe_input`, `probe_audio`, and `probe_audio_input` helpers.

FFmpeg is responsible for external process interaction, probing, decoding, and
conversion. Downstream packages should consume only core and ingest contracts
such as `OwnedVideoFrame`, `OwnedAudioFrame`, `VideoFrameSource`, and
`AudioFrameSource`.

## Detector Contracts

`video-analysis-detectors` implements scene detection algorithms behind the
core `SceneDetector` trait.

Detector implementations include:

- `ContentDetector`
- `AdaptiveDetector`
- `ThresholdDetector`
- `HistogramDetector`
- `HashDetector`
- `WeightedCompositeDetector`

Scoring and composition contracts include:

- `ScoreAlgorithm`, which accepts `VideoFrame<'_>` and emits
  `AlgorithmScore`.
- `AlgorithmScore`, with frame position, raw score, and normalized score.
- `WeightedComponent`, which pairs a `ScoreAlgorithm` with a positive weight.
- `FlashFilter`, which suppresses or merges cuts that violate minimum scene
  length.

Detectors accept borrowed video frames, emit core `Cut` values, and report
optional metrics through `MetricsSink`. They should not know about FFmpeg, file
output, split plans, or CLI arguments.

## Data Aggregation Contracts

`video-analysis-data` normalizes heterogeneous streams into borrowed records and
summarizes them online.

Key contracts:

- `DataRecord<'a>` identifies a stream, sequence, optional timestamp, and
  payload.
- `DataPayload<'a>` supports video, audio, text, number, vector, and custom
  payloads.
- `DataStreamKind` classifies payload categories.
- `BucketConfig` validates aggregation settings.
- `BucketMode` supports fixed duration, record count, and estimated byte size.
- `BucketAggregator` accepts records and emits completed `DataBucket` values.
- `DataBucket` stores aggregate counts, estimated bytes, and stream summaries.
- `StreamSummary` stores per-stream counts, timestamps, payload counts, and
  media/numeric/vector summaries.

Data records do not retain original video/audio/text/vector payloads. They carry
only enough data to summarize stream shape, volume, and statistics. Fixed
duration buckets require timestamp-ordered records. Bucket summaries are stable
inputs for use-case reports and UI components.

## Model Contracts

`video-analysis-models` separates model acquisition, model-specific backend
execution, prediction normalization, and analyzer integration.

Model acquisition and identity:

- `ModelTask`
- `HuggingFaceModelSpec`
- `ModelPreset`
- `DownloadedModel`
- `HuggingFaceDownloader`

Prediction contracts:

- `RawPrediction`
- `RawBoundingBox`
- `NormalizedPrediction`
- `PredictionRepairOptions`
- `normalize_predictions`

Backend and analyzer contracts:

- `VisionModelBackend`
- `TextModelBackend`
- `ModelVideoAnalyzer`
- `ModelTextAnalyzer`
- `ExternalCommandModel`

Vision backends return raw predictions for a `VideoFrame<'_>`. Text backends
return raw predictions for a `TextSegment<'_>`. The model crate normalizes those
predictions into core `Observation` values for video and core `AnalysisEvent`
values for text.

### External Command JSON Protocol

`ExternalCommandModel` starts an executable, writes one JSON request to stdin,
and expects one JSON response on stdout.

The request contains:

- `task`: the model task as a protocol string.
- `model`: model name, repo id, revision, and downloaded file paths.
- `input`: a tagged object.

Video input is tagged as `video_frame` and includes:

- `width`
- `height`
- `pixel_format`
- `stride`
- `data_base64`

Text input is tagged as `text` and includes:

- `text`
- `language`
- `is_final`

The response shape is:

```json
{
  "predictions": []
}
```

Each prediction should match the `RawPrediction` contract. Missing prediction
fields can be repaired where supported by `PredictionRepairOptions`.

## Output And Split Contracts

`video-analysis-output` serializes detection results. It consumes only core
contracts:

- `write_scene_list_csv` writes scene rows from `&[Scene]`.
- `write_stats_csv` writes metric rows from `&MetricsStore`.
- `write_scene_list_html` writes a simple HTML scene table from `&[Scene]`.
- `write_detection_outputs` writes scenes and optional stats from
  `&DetectionResult`.

`video-analysis-split` creates scene clips from original media:

- `SplitOptions` controls output directory, filename template, optional video
  name, and FFmpeg args.
- `DEFAULT_TEMPLATE` is `$VIDEO_NAME-Scene-$SCENE_NUMBER.mp4`.
- `split_video_ffmpeg` accepts the original media path, `&[Scene]`, and
  `SplitOptions`.

Output and split packages do not own detection, source construction, detector
selection, or CLI branching.

## 3D Scene Contracts

The 3D packages interoperate through `video-analysis-radiance-fields` geometry,
camera, ray, and color primitives.

`video-analysis-radiance-fields` exposes:

- `Vec2`
- `Vec3`
- `ColorRgb`
- `Ray`
- `CameraIntrinsics`
- `CameraPose`
- `RadianceField`
- `RadianceSample`
- `VolumeRenderConfig`
- `AxisAlignedBounds`
- `RadianceGridSpec`

`video-analysis-gaussian-splatting` exposes:

- `Quaternion`
- `Covariance3`
- `Gaussian3d`
- `GaussianScene`
- `ProjectedGaussian`
- `ProjectionConfig`
- `SplatRenderConfig`
- Projection helpers such as `project_gaussian` and `project_scene`.
- Rendering helpers such as `gaussian_weight`, `composite_splats_at_pixel`,
  and `render_projected_splats`.

`video-analysis-reconstruction` exposes:

- `CameraId`
- `ImageId`
- `Point3dId`
- `ReconstructionCamera`
- `Feature2d`
- `BinaryFeature`
- `FeatureMatch`
- `ImagePairMatches`
- `Track`
- `ReconstructionImage`
- `SparsePoint3d`
- `SparseReconstruction`
- Feature matching and track helpers such as `hamming_distance`,
  `match_binary_features`, and `build_tracks`.
- Triangulation/projection helpers such as `triangulate_observation_pair`,
  `project_point`, `reprojection_error`, and `ray_angle`.

These crates should share `CameraIntrinsics`, `CameraPose`, `Vec2`, `Vec3`,
`ColorRgb`, and `Ray` instead of introducing incompatible camera or geometry
types.

## CLI And Use-Case Boundary Contracts

`video-analysis-cli` composes runtime packages behind the `vanalyze` binary.
The command contracts are:

- `vanalyze detect`: input video, detector selection, scene CSV output, optional
  stats CSV output.
- `vanalyze list`: input video and detector selection, with scene list output.
- `vanalyze split`: input video, detector selection, and scene clip output
  directory.
- `vanalyze models presets`: lists built-in model presets.
- `vanalyze models download`: downloads a preset or explicit Hugging Face model
  files.

`video-analysis-use-cases` exposes runnable workflows. The current workflow is:

- `video-analysis-use-cases youtube-video`

The YouTube video workflow accepts a URL or local video input. It can use
optional external transcriber, object, OCR, and text model commands. Its primary
interoperability output is a JSON report consumed by applications and
`@video-analysis/ui`.

## Rust-To-UI JSON Report Contracts

The use-case JSON report is the main contract between Rust output and React
components. The serialized Rust report structs in
`crates/video-analysis-use-cases/src/main.rs` align with the TypeScript
interfaces in `packages/video-analysis-ui/src/types.ts`.

Top-level report:

- `YoutubeVideoReport`
  - `use_case`
  - `source`
  - `assets`
  - `capabilities`
  - `video`
  - `transcription`
  - `audio`
  - `text`
  - `data_buckets`

Source and assets:

- `SourceReport`: optional `url`, required `local_video`.
- `AssetReport`: `work_dir`, `report_path`, optional `audio_wav`.
- `CapabilityReport`: completed and skipped capability names.

Video and scene report:

- `VideoReport`: dimensions, frame rate string, optional duration, processed
  frame count, scenes, and observations.
- `SceneReport`: index, start/end frames, start/end seconds, and observations.
- `AnalysisObservation`: optional timestamp seconds, frame index, scene index,
  label, text, score, region, track id, and attributes, plus required analyzer
  and kind strings.

Transcript, audio, and text:

- `TranscriptionReport`: status, optional full text, segments, and optional
  message.
- `TranscriptSegmentReport`: index, optional start/end seconds, and text.
- `AudioReport`: status, frames processed, events, and optional message.
- `TextReport`: status, segments processed, events, and optional message.
- `AnalysisEvent`: optional timestamp seconds, analyzer, label, and optional
  score.

Data buckets:

- `DataBucketReport`: bucket index, records, estimated bytes, and streams.
- `StreamBucketReport`: stream id, records, estimated bytes, payload counts,
  video frame count, audio frame count, and text segment count.

Compatibility notes:

- Rust numeric fields such as `u64`, `u32`, `f32`, and `f64` become TypeScript
  `number`.
- Rust `Option<T>` appears as optional and/or nullable UI fields where currently
  modeled.
- Report fields consumed by UI components should be preserved or intentionally
  versioned when changed.

## Facade And Package Export Contracts

The Rust root crate `video-analysis` is a convenience facade. It re-exports all
core items, detector items, and package modules for data, FFmpeg, ingest,
models, output, radiance fields, Gaussian splatting, reconstruction, and split.
It does not expose CLI or use-case binaries as library modules.

The UI package exposes these subpaths:

- `@video-analysis/ui`
- `@video-analysis/ui/core`
- `@video-analysis/ui/data`
- `@video-analysis/ui/cli`
- `@video-analysis/ui/detectors`
- `@video-analysis/ui/ffmpeg`
- `@video-analysis/ui/ingest`
- `@video-analysis/ui/models`
- `@video-analysis/ui/output`
- `@video-analysis/ui/split`
- `@video-analysis/ui/use-cases`
- `@video-analysis/ui/tailwind-content`

The root UI export re-exports shared types and all component packs. Subpath
exports should remain aligned with package boundaries so applications can import
only the views they need.

## Dependency Rules

Allowed internal dependencies:

- `video-analysis-core`: external utility crates only.
- `video-analysis-data` -> `video-analysis-core`.
- `video-analysis-detectors` -> `video-analysis-core`.
- `video-analysis-ingest` -> `video-analysis-core`.
- `video-analysis-ffmpeg` -> `video-analysis-core`,
  `video-analysis-ingest`.
- `video-analysis-models` -> `video-analysis-core`.
- `video-analysis-output` -> `video-analysis-core`.
- `video-analysis-split` -> `video-analysis-core`.
- `video-analysis-radiance-fields` -> `video-analysis-core`.
- `video-analysis-gaussian-splatting` -> `video-analysis-core`,
  `video-analysis-radiance-fields`.
- `video-analysis-reconstruction` -> `video-analysis-core`,
  `video-analysis-radiance-fields`.
- `video-analysis-cli` -> crates it composes for CLI workflows.
- `video-analysis-use-cases` -> crates it composes for runnable workflows.
- `video-analysis` root facade -> library crates it re-exports.

Forbidden or discouraged internal dependencies:

- `video-analysis-core` must not depend on any workspace crate.
- Source crates should not depend on detectors, output, split, CLI, or facade
  crates.
- Detector crates should not depend on FFmpeg, output, split, CLI, or facade
  crates.
- Data, output, and split crates should not depend on detector implementations,
  source implementations, CLI, or the root facade.
- No library crate should depend on `video-analysis-cli`.
- `@video-analysis/ui` consumes generated data/report shapes and should not
  require Rust runtime packages.

## Compatibility Checklist

For new packages:

- Use core time, sample, result, detection, observation, and event types where
  possible.
- Add new media sources through `video-analysis-ingest` traits.
- Add new scene detectors through `SceneDetector`.
- Add video/audio/text enrichment through `VideoAnalyzer`, `AudioAnalyzer`, or
  `TextAnalyzer`.
- Add model integrations through `VisionModelBackend`, `TextModelBackend`, or
  the `ExternalCommandModel` JSON protocol.
- Add UI consumers by extending explicit TypeScript report types and keeping
  them aligned with Rust serialized reports.

For changes to existing packages:

- Update this document when changing shared traits, serialized report fields,
  CLI output files, file formats, or package exports.
- Preserve optional fields where possible; otherwise document breaking changes.
- Keep dependency direction consistent with the rules above.
- Prefer core contracts over package-specific duplicates.
