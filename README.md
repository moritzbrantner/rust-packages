# Rust Multimodal Analysis Packages

This workspace contains Rust-first multimodal building-block crates for video,
audio, image, text, vector, data, math, animation, 3D, and adapter
interoperability. Package consumers integrate through foundation contracts and
audited package surfaces rather than workflow-aware node metadata.

ComfyUI is an interoperability target and useful inspiration for composition,
not the internal architecture of the crates. External projects can map package
operations into their own graph models while this repository keeps reusable
library APIs, CLI, REST, WASM, and web app surfaces aligned over the same
contracts.

The scene detection packages started as a PySceneDetect-style video analysis
implementation; the vendored
`references/pyscenedetect` directory is used only as an upstream behavior
reference.

See [AGENTS.md](AGENTS.md) for Codex/T3 agent instructions,
[docs/development.md](docs/development.md) and [CONTRIBUTING.md](CONTRIBUTING.md)
for local verification and tool setup, [SECURITY.md](SECURITY.md) for
vulnerability reporting, and the root [LICENSE-MIT](LICENSE-MIT) /
[LICENSE-APACHE](LICENSE-APACHE) files for the workspace licensing terms.

## Crates

Rust crates are grouped under `crates/` by input or integration domain:
`audio/`, `video/`, `image/`, `text/`, `vector/`, `data/`, `math/`,
`animation/`, `three-d/`, and `comfyui/`.

Start with [docs/API_CONTRACTS.md](docs/API_CONTRACTS.md) for foundation
contract ownership and [docs/PACKAGE_SURFACE_MATRIX.md](docs/PACKAGE_SURFACE_MATRIX.md)
for the audited package-surface integration map.

Package README index:

- Root facade: [`moritzbrantner-video-analysis`](src/lib.rs), [README](README.md)
- Audio: [`moritzbrantner-audio-analysis-core`](crates/audio/audio-analysis-core/README.md), [`moritzbrantner-audio-generation-midi`](crates/audio/audio-generation-midi/README.md), [`moritzbrantner-audio-analysis-fourier`](crates/audio/audio-analysis-fourier/README.md), [`moritzbrantner-audio-analysis-io`](crates/audio/audio-analysis-io/README.md), [`moritzbrantner-audio-analysis-pitch`](crates/audio/audio-analysis-pitch/README.md), [`moritzbrantner-audio-analysis-processing`](crates/audio/audio-analysis-processing/README.md), [`moritzbrantner-audio-analysis-recognition`](crates/audio/audio-analysis-recognition/README.md), [`moritzbrantner-audio-analysis-rhythm`](crates/audio/audio-analysis-rhythm/README.md), [`moritzbrantner-audio-analysis-separation`](crates/audio/audio-analysis-separation/README.md), [`moritzbrantner-audio-analysis-speakers`](crates/audio/audio-analysis-speakers/README.md), [`moritzbrantner-audio-analysis-synthesis`](crates/audio/audio-analysis-synthesis/README.md)
- ComfyUI: [`moritzbrantner-comfyui-data`](crates/comfyui/comfyui-data/README.md), [`moritzbrantner-comfyui-latents`](crates/comfyui/comfyui-latents/README.md), [`moritzbrantner-comfyui-models`](crates/comfyui/comfyui-models/README.md)
- Data: [`moritzbrantner-data-inversion-core`](crates/data/data-inversion-core/README.md), [`moritzbrantner-graph-analysis-core`](crates/data/graph-analysis-core/README.md), [`moritzbrantner-numbers-core`](crates/data/numbers-core/README.md), [`moritzbrantner-tensor-data`](crates/data/tensor-data/README.md), [`moritzbrantner-dense-data`](crates/data/dense-data/README.md)
- Animation: [`moritzbrantner-animation-core`](crates/animation/animation-core/README.md)
- Math: [`moritzbrantner-math-geometry-2d`](crates/math/math-geometry-2d/README.md), [`moritzbrantner-math-linear`](crates/math/math-linear/README.md), [`moritzbrantner-math-signal-core`](crates/math/math-signal-core/README.md), [`moritzbrantner-math-sparse-data`](crates/math/math-sparse-data/README.md), [`moritzbrantner-math-statistics`](crates/math/math-statistics/README.md)
- Image: [`moritzbrantner-image-analysis-comfyui`](crates/image/image-analysis-comfyui/README.md), [`moritzbrantner-image-analysis-core`](crates/image/image-analysis-core/README.md), [`moritzbrantner-image-analysis-detection`](crates/image/image-analysis-detection/README.md), [`moritzbrantner-image-analysis-classification`](crates/image/image-analysis-classification/README.md), [`moritzbrantner-image-analysis-embeddings`](crates/image/image-analysis-embeddings/README.md), [`moritzbrantner-image-analysis-captioning`](crates/image/image-analysis-captioning/README.md), [`moritzbrantner-image-analysis-io`](crates/image/image-analysis-io/README.md), [`moritzbrantner-image-analysis-ocr`](crates/image/image-analysis-ocr/README.md), [`moritzbrantner-image-analysis-processing`](crates/image/image-analysis-processing/README.md), [`moritzbrantner-image-analysis-segmentation`](crates/image/image-analysis-segmentation/README.md), [`moritzbrantner-image-analysis-synthesis`](crates/image/image-analysis-synthesis/README.md)
- Text: [release scope](docs/TEXT_RELEASE_SCOPE.md), [workspace guide](docs/TEXT_WORKSPACE_GUIDE.md), [corpus guide](docs/TEXT_CORPUS_GUIDE.md), [`moritzbrantner-text-analysis`](crates/text/text-analysis/README.md), [`moritzbrantner-text-core`](crates/text/text-core/README.md), [`moritzbrantner-text-lexical`](crates/text/text-lexical/README.md), [`moritzbrantner-text-linguistics`](crates/text/text-linguistics/README.md), [`moritzbrantner-text-classification`](crates/text/text-classification/README.md), [`moritzbrantner-text-question-answering`](crates/text/text-question-answering/README.md), [`moritzbrantner-text-embeddings`](crates/text/text-embeddings/README.md), [`moritzbrantner-text-index`](crates/text/text-index/README.md), [`moritzbrantner-text-retrieval`](crates/text/text-retrieval/README.md), [`moritzbrantner-text-model-runtime`](crates/text/text-model-runtime/README.md), [`moritzbrantner-text-transcripts`](crates/text/text-transcripts/README.md), [`moritzbrantner-text-generation`](crates/text/text-generation/README.md), [`moritzbrantner-text-generation-linguistics`](crates/text/text-generation-linguistics/README.md)
- Vector and 3D: [`moritzbrantner-vector-analysis-core`](crates/vector/vector-analysis-core/README.md), [`moritzbrantner-vector-analysis-index`](crates/vector/vector-analysis-index/README.md), [`moritzbrantner-three-d-processing-core`](crates/three-d/three-d-processing-core/README.md), [`moritzbrantner-three-d-processing-io`](crates/three-d/three-d-processing-io/README.md), [`moritzbrantner-three-d-processing-mesh`](crates/three-d/three-d-processing-mesh/README.md), [`moritzbrantner-three-d-scene-svg`](crates/three-d/three-d-scene-svg/README.md)
- Video: [`moritzbrantner-video-analysis-core`](crates/video/video-analysis-core/README.md), [`moritzbrantner-video-analysis-data`](crates/video/video-analysis-data/README.md), [`moritzbrantner-video-analysis-dataset`](crates/video/video-analysis-dataset/README.md), [`moritzbrantner-video-analysis-detectors`](crates/video/video-analysis-detectors/README.md), [`moritzbrantner-video-analysis-editing`](crates/video/video-analysis-editing/README.md), [`moritzbrantner-video-analysis-features`](crates/video/video-analysis-features/README.md), [`moritzbrantner-video-analysis-ffmpeg`](crates/video/video-analysis-ffmpeg/README.md), [`moritzbrantner-video-analysis-gaussian-splatting`](crates/video/video-analysis-gaussian-splatting/README.md), [`moritzbrantner-video-analysis-ingest`](crates/video/video-analysis-ingest/README.md), [`moritzbrantner-video-analysis-output`](crates/video/video-analysis-output/README.md), [`moritzbrantner-video-analysis-posture`](crates/video/video-analysis-posture/README.md), [`moritzbrantner-video-analysis-posture-io`](crates/video/video-analysis-posture-io/README.md), [`moritzbrantner-video-analysis-radiance-fields`](crates/video/video-analysis-radiance-fields/README.md), [`moritzbrantner-video-analysis-radiance-io`](crates/video/video-analysis-radiance-io/README.md), [`moritzbrantner-video-analysis-radiance-pipeline`](crates/video/video-analysis-radiance-pipeline/README.md), [`moritzbrantner-video-analysis-recognition`](crates/video/video-analysis-recognition/README.md), [`moritzbrantner-video-analysis-reconstruction`](crates/video/video-analysis-reconstruction/README.md), [`moritzbrantner-video-analysis-segmentation`](crates/video/video-analysis-segmentation/README.md), [`moritzbrantner-video-analysis-split`](crates/video/video-analysis-split/README.md), [`moritzbrantner-video-analysis-storage`](crates/video/video-analysis-storage/README.md), [`moritzbrantner-video-analysis-synthesis`](crates/video/video-analysis-synthesis/README.md), [`moritzbrantner-video-analysis-tracking`](crates/video/video-analysis-tracking/README.md), [`moritzbrantner-video-analysis-transform`](crates/video/video-analysis-transform/README.md), [`moritzbrantner-video-analysis-cli`](crates/video/video-analysis-cli/README.md)
- Prototypes: [`moritzbrantner-video-analysis-use-cases`](prototypes/rust/video-analysis-use-cases/README.md), `@moritzbrantner/video-analysis-web` in `prototypes/web/video-analysis-web`

- `moritzbrantner-video-analysis`: umbrella re-export crate.
- `moritzbrantner-comfyui-data`: serde contracts and helpers for ComfyUI workflow JSON and
  API prompt graphs, plus typed socket inventory helpers.
- `moritzbrantner-comfyui-latents`: ComfyUI-oriented latent batches, latent masks, and
  latent/image size helpers built on `moritzbrantner-tensor-data`.
- `moritzbrantner-comfyui-models`: ComfyUI model folder keys, default paths, inventory
  scanning, runtime-facing model references, and `extra_model_paths.yaml`
  generation helpers.
- `moritzbrantner-data-inversion-core`: shared trace metadata for lossy inverse conversions,
  including fidelity, confidence, assumptions, and interpolation notes.
- `moritzbrantner-graph-analysis-core`: deterministic graph and tree primitives for cycle
  detection, connected-component analysis, shortest paths, spanning trees, and
  tree validation.
- `moritzbrantner-numbers-core`: scalar numeric summaries, weighted running stats, quantiles,
  histograms, and range helpers for analytics and reporting crates.
- `moritzbrantner-tensor-data`: finite `f32` tensor shapes, borrowed/owned tensor values, and
  lightweight tensor metadata for interop contracts.
- `moritzbrantner-math-geometry-2d`: shared checked 2D points, rectangles, normalized
  coordinates, polygons, bounds, and affine transforms for image, video, and
  posture workflows.
- `moritzbrantner-math-linear`: dense matrix shapes, row/column views, matrix multiply,
  tensor/vector bridges, and shared 1D/2D kernel contracts.
- `moritzbrantner-math-signal-core`: sample-rate, window, resampling, frame-stride, FIR, and
  biquad design contracts for audio and time-series workflows.
- `moritzbrantner-math-sparse-data`: sparse vectors plus COO/CSR matrices for text, retrieval,
  and feature indexing workflows.
- `moritzbrantner-math-statistics`: streaming covariance, normalizers, covariance matrices,
  and PCA-lite utilities for dense multivariate inputs.

Finance packages moved to the sibling `finance-analysis` repository as an
Adjacent Domain Package Family. This repository keeps only deprecated
`video_analysis::finance` and `video_analysis::finance_data` doc stubs as
migration signposts.
- `moritzbrantner-audio-analysis-core`: normalized audio sample conversion, mono mixing,
  windowing, frame iteration, streaming frame windows, waveform batches, and
  level helpers for audio analysis crates.
- `moritzbrantner-audio-generation-midi`: MIDI-like note sequencing, Standard MIDI export,
  and deterministic audio rendering through the audio synthesis crate.
- `moritzbrantner-audio-analysis-fourier`: FFT, STFT/spectrogram, spectral features, and a
  dominant-frequency audio analyzer.
- `moritzbrantner-audio-analysis-io`: audio-named input conveniences over the FFmpeg-backed
  audio source and shared ingest traits.
- `moritzbrantner-audio-analysis-pitch`: autocorrelation pitch estimation and an audio
  analyzer that emits pitch events.
- `moritzbrantner-audio-analysis-processing`: realtime-safe audio frame transforms, including
  gain, clipping, mono conversion, DC blocking, biquad filters, noise gates, and
  processed audio sources.
- `moritzbrantner-audio-analysis-recognition`: deterministic spectral audio embeddings,
  sample-backed reference libraries, similarity search, and recognition events.
- `moritzbrantner-audio-analysis-rhythm`: onset detection, tempo estimation, and a rhythm
  analyzer that emits onset and BPM events.
- `moritzbrantner-audio-analysis-separation`: typed HTDemucs/Demucs integration for vocal and
  instrument stem separation through the external `demucs` CLI, including
  model-aware output discovery.
- `moritzbrantner-audio-analysis-speakers`: speaker-domain embeddings, enrollment,
  identification, VAD, diarization abstractions, and model-versioned profile
  snapshots.
- `moritzbrantner-audio-analysis-synthesis`: deterministic tone, onset, and pitch-event audio
  synthesis into core audio frames with inversion trace metadata.
- `moritzbrantner-image-analysis-comfyui`: ComfyUI workflow builders for text-to-image,
  image-to-image, inpaint, and upscale image-generation flows.
- `moritzbrantner-image-analysis-core`: borrowed/owned image views, image batches, RGB/BGR/gray
  pixel contracts, compacting, mean color, luma histograms, and mask tensor
  bridges.
- `moritzbrantner-image-analysis-io`: PNG/JPEG/WebP loading and saving for compact
  `OwnedImage` buffers.
- `moritzbrantner-image-analysis-classification`: image classification request/response
  contracts, catalog metadata, and backend traits.
- `moritzbrantner-image-analysis-embeddings`: image and face embedding request/response
  contracts, catalog metadata, and backend traits.
- `moritzbrantner-image-analysis-captioning`: image captioning request/response contracts,
  catalog metadata, and backend traits.
- `moritzbrantner-runtime-onnx`: domain-neutral ONNX Runtime session, tensor,
  metadata, and named input/output helpers. Domain crates own model decoding.
- `moritzbrantner-image-analysis-processing`: deterministic CPU image crop, resize, grayscale,
  inversion, thresholding, and 3x3 convolution pipelines.
- `moritzbrantner-image-analysis-segmentation`: prompt, binary-mask, and image-segment
  contracts with explicit opt-in automatic mask generation defaults.
- `moritzbrantner-image-analysis-synthesis`: deterministic solid, gradient, histogram, and
  region-based image synthesis into owned image buffers.
- `moritzbrantner-text-analysis`: orchestration reports that combine core document analysis,
  lexical sections, similarity, linguistic summaries, embeddings, and corpus
  analysis into reusable document/corpus outputs.
- `moritzbrantner-text-core`: text document contracts, text segment bridging,
  normalization, Unicode-safe tokens/graphemes/word segments with spans,
  sentence/paragraph splitting, script profiling, and text statistics.
- `moritzbrantner-text-lexical`: stop words, keywords, n-grams, shingles, readability,
  stemming, sentiment, extractive summaries, reusable analyzers, corpus
  statistics, sparse term matrices, TF-IDF scoring, and BM25 ranking.
- `moritzbrantner-text-linguistics`: heuristic-first language detection, tokenizer
  routing, token/subword alignment, lemmatization, morphology, POS tagging,
  chunking, dependency parsing, typed entities, coreference, events, discourse,
  topics, style profiles, and a `TextAnalyzer` adapter.
- `moritzbrantner-text-embeddings`: lightweight hashed text embeddings, embedding backend
  traits, dense-vector similarity, and co-occurrence/related-term analysis.
- `moritzbrantner-text-index`: durable text indexing, deterministic chunking,
  memory and SQLite storage, lexical/semantic/hybrid search, semantic facets,
  analysis attachments, filters, inspection, and snapshot planning.
- `moritzbrantner-text-retrieval`: contract ingestion, legacy retrieval
  compatibility, related-content lookup, reranking, and manifest/JSONL
  compatibility for retrieval index round trips.
- `moritzbrantner-text-model-runtime`: shared tokenizer bundle contracts, tokenized model
  inputs, and optional ONNX/Candle runtime traits. Default builds stay
  deterministic; `tokenizers`, `onnx`, `candle`, and `external-tests` are
  opt-in features.
- `moritzbrantner-text-classification`: text classification, zero-shot classification, and
  sentiment request/response contracts with deterministic fallbacks.
- `moritzbrantner-text-question-answering`: extractive question answering request/response
  contracts and imported span postprocessing.
- `moritzbrantner-text-transcripts`: transcript segment models, Whisper JSON,
  SRT/WebVTT/plain text parsing, command transcribers, and text segment source
  adapters.
- `moritzbrantner-text-generation`: deterministic token Markov chains, next-token prediction,
  perplexity scoring, prompt extraction, and weighted-term synthesis.
- `moritzbrantner-text-generation-linguistics`: linguistic-analysis adapters for deterministic
  term prompts, document synthesis, and Markov training over analyzed text.
- `moritzbrantner-vector-analysis-core`: dense vector validation, normalization, metrics,
  distances, means, and per-dimension summary statistics.
- `moritzbrantner-vector-analysis-index`: exact in-memory vector search and nearest-centroid
  assignment helpers.
- `moritzbrantner-dense-data`: dense numeric point datasets with weighted averages,
  per-dimension summaries, fixed-grid buckets, bounds, and deterministic
  k-means clustering for tables, graphs, charts, maps, and media-derived
  features.
- `moritzbrantner-animation-core`: timeline, keyframe, track, transform track, skeleton, and
  clip contracts for reusable animation workflows.
- `moritzbrantner-three-d-processing-core`: 3D points, vectors, quaternions, rigid transforms,
  line segments, rays, planes, spheres, point clouds, intersections,
  closest-point queries, voxel downsampling, and normalization helpers.
- `moritzbrantner-three-d-processing-io`: `OBJ`, `PLY`, and minimal embedded `.gltf`
  round-tripping for triangle meshes and point clouds.
- `moritzbrantner-three-d-processing-mesh`: triangle mesh validation, topology, normals,
  surface area and centroid, volume, deterministic sampling, closest-point and
  ray-intersection queries, transforms, and smoothing.
- `moritzbrantner-three-d-scene-svg`: SVG-inspired Scene Vector 3D documents, validation,
  serde JSON helpers, and deterministic SVG preview rendering for lightweight
  3D diagrams and reports.
- `moritzbrantner-video-analysis-core`: timecodes, video/audio/text sample types, metrics, analyzer traits, observations, and realtime pipelines.
- `moritzbrantner-video-analysis-data`: stream record normalization plus online aggregation and
  bucketing for video, audio, text, numeric, and vector data.
- `moritzbrantner-video-analysis-dataset`: retained, serializable analysis records for scenes,
  frames, observations, events, metrics, tracks, features, and structured 2D
  and 3D posture records.
- `moritzbrantner-video-analysis-transform`: deterministic filtering, windowing, scene
  grouping, temporal/frame joins, dedupe, merge, and numeric resampling over
  retained dataset records.
- `moritzbrantner-video-analysis-features`: reusable feature extractors for scene stats, label
  histograms, transcripts, audio events, tracks, and vector means.
- `moritzbrantner-video-analysis-storage`: JSON, JSONL, and manifest persistence for retained
  analysis datasets.
- `moritzbrantner-video-analysis-synthesis`: deterministic storyboard/video-frame synthesis
  from frame specs and observations.
- `moritzbrantner-video-analysis-detectors`: content, adaptive, threshold, histogram, and perceptual hash detectors.
- `moritzbrantner-video-analysis-editing`: CPU frame editing primitives for cropping,
  blurring, grayscale, inversion, brightness/contrast, and 3x3 filters.
- `moritzbrantner-video-analysis-ingest`: media ingest traits plus live/file text sources.
- `moritzbrantner-video-analysis-ffmpeg`: FFmpeg-backed video and audio ingest implementations.
- `moritzbrantner-model-runtime`: generic model specs, Hugging Face downloads, bundle
  manifests, preset metadata, and runtime conformance helpers.
- `moritzbrantner-video-analysis-recognition`: reference-embedding matching and optional
  ONNX object-detection adapters through `moritzbrantner-runtime-onnx`.
- `moritzbrantner-video-analysis-tracking`: IoU-based object tracking contracts and a
  `VideoAnalyzer` adapter that emits tracked object observations.
- `moritzbrantner-video-analysis-posture`: 2D/3D pose contracts, COCO-17 skeleton helpers,
  joint-angle and bone-length math, stick figures, posture lifting contracts,
  and a posture analyzer adapter.
- `moritzbrantner-video-analysis-posture-io`: COCO-style posture JSON plus `.ply` and `.gltf`
  export for 3D stick figures.
- `moritzbrantner-video-analysis-recognition`: reference-embedding matching for face/object
  recognition, including temporal track aggregation and analyzer adapters.
- `moritzbrantner-video-analysis-radiance-fields`: camera, ray, grid, and volume rendering
  contracts for radiance-field style scene representations.
- `moritzbrantner-video-analysis-gaussian-splatting`: 3D Gaussian primitive validation,
  projection, sorting, and CPU compositing helpers for Gaussian splatting.
- `moritzbrantner-video-analysis-radiance-io`: COLMAP text, Nerfstudio transforms, and
  GraphDeco/Nerfstudio Gaussian splat PLY import/export helpers.
- `moritzbrantner-video-analysis-radiance-pipeline`: library-first loading, validation,
  summary, and CPU preview rendering across the radiance crates.
- `moritzbrantner-video-analysis-output`: scene/stats CSV, simple HTML, JSON,
  EDL, FCP, OTIO, and qpfile output helpers.
- `moritzbrantner-video-analysis-split`: ffmpeg CLI based scene splitting.
- `moritzbrantner-video-analysis-cli`: `vanalyze` command-line tool.
- `moritzbrantner-video-analysis-use-cases`: prototype runnable end-to-end use-case pipelines.
- `@moritzbrantner/video-analysis-ui`: React + TailwindCSS component packs for viewing
  analysis results in an application UI.
- `@moritzbrantner/video-analysis-web`: prototype Vite app for trying workflows, reports, and
  package integrations locally.

## Repository Layout

- `crates/`: reusable Rust packages intended to stay focused and composable.
- `packages/`: reusable frontend packages such as `@moritzbrantner/video-analysis-ui`.
- `prototypes/`: executable experiments and testbeds, including use-case
  workflows, the local web app, and future search/recommendation prototypes.

## Frontend Component Packs

The `packages/video-analysis-ui` npm package mirrors the Rust package
boundaries with subpath exports:

- `@moritzbrantner/video-analysis-ui/core`: scene timelines, scene tables, observations,
  audio/text events, and video summary cards.
- `@moritzbrantner/video-analysis-ui/cli`: command status, arguments, and generated output
  file summaries for `vanalyze` workflows.
- `@moritzbrantner/video-analysis-ui/data`: bucket and stream summary views.
- `@moritzbrantner/video-analysis-ui/detectors`: detection and cut summary views.
- `@moritzbrantner/video-analysis-ui/ffmpeg`: media metadata panels.
- `@moritzbrantner/video-analysis-ui/ingest`: source and asset summaries.
- `@moritzbrantner/video-analysis-ui/models`: capability and scored model observation views.
- `@moritzbrantner/video-analysis-ui/output`: report shell and JSON report loader.
- `@moritzbrantner/video-analysis-ui/split`: scene split plan preview.
- `@moritzbrantner/video-analysis-ui/use-cases`: composed dashboards, including
  `YoutubeVideoReportView` for the YouTube video use-case JSON report.
- `@moritzbrantner/video-analysis-ui`: the root facade that re-exports all frontend packs.

```tsx
import { YoutubeVideoReportView } from "@moritzbrantner/video-analysis-ui/use-cases";
import type { YoutubeVideoReport } from "@moritzbrantner/video-analysis-ui";

export function ReportPage({ report }: { report: YoutubeVideoReport }) {
  return <YoutubeVideoReportView report={report} />;
}
```

Add the package output to Tailwind's content list:

```js
import videoAnalysisContent from "@moritzbrantner/video-analysis-ui/tailwind-content";

export default {
  content: ["./src/**/*.{ts,tsx}", ...videoAnalysisContent],
};
```

## Use-Case Website

`prototypes/web/video-analysis-web` is a Vite React app for trying the available
use-case workflows and viewing generated reports with the component packs.

```bash
bun install
bun run ui:build
bun run web:dev
```

The app includes the YouTube video use case, a command builder for
`video-analysis-use-cases`, sample report data, JSON report loading, and
dashboard views for scenes, observations, transcript, events, buckets, and split
plans.

## Package Surfaces

Runtime packages should keep the reusable library crate separate from adapters.
When a library needs a command line, HTTP API, or browser UI, add adjacent
adapter packages under the library directory:

```text
<name>/
  Cargo.toml       # library package
  src/lib.rs
  cli/             # CLI package, depends on ..
  api/             # API package, depends on ..
  ui/              # webpage package, depends on ..
```

For example, a text-processing package should expose reusable text logic from
`text-.../src/lib.rs`, while `text-.../cli`, `text-.../api`, and
`text-.../ui` own the application code
for those surfaces. The library crate should not declare generic `[[bin]]`
targets for CLI/API/UI adapters.

```bash
cargo run -p video-analysis-cli -- packages inspect video-analysis-core --json
```

The current `video-analysis-cli` package is the workspace-level `vanalyze`
adapter. The local web app still exposes the workspace catalog at
`/api/packages` and renders package surface locations in the architecture view.

## Workspace Checks

Run the full local verification baseline before publishing changes:

```bash
scripts/check.sh
```

For the default contributor gate, run:

```bash
scripts/check-fast.sh
```

The script is changed-aware for local iteration: it runs the artifact and Rust
formatting guards, checks affected reviewed generated snapshots, then scopes
Rust test/clippy, frontend package checks, and package-surface progress
comparisons to touched files when possible. Use `CHECK_FAST_SCOPE=workspace`,
`CHECK_FAST_FRONTEND=all`, or `CHECK_FAST_PROGRESS=all` to force broader coverage. Use
`scripts/check-preflight.sh` as the broad local CI mirror before handoff.
FFmpeg decode coverage is intentionally opt-in so the default suite stays
hermetic:

```bash
cargo test -p video-analysis-ffmpeg --features ffmpeg-tests
```

Validate external-tool prerequisites before the full baseline:

```bash
scripts/check_e2e_external_tools.sh
```

Release-oriented changes should also pass the documentation build:

```bash
cargo doc --workspace --no-deps
```

Package surfaces use matching test layers: Rust libraries keep unit tests close
to the implementation, CLI and API adapters use integration tests, and UI
packages use browser e2e tests.

For crate packaging and publish-order checks, use
[docs/RELEASE_CHECKLIST.md](docs/RELEASE_CHECKLIST.md).

## Feature Flag Conventions

External-runtime crates keep `default = []` and expose explicit opt-in features.
`external-tests` always means real tools, real models, or real network access
and stays outside the default contributor gate.

- `video-analysis`: `onnx` for the optional ONNX facade re-export, with
  `onnx-backend` kept as a compatibility alias.
- `video-analysis-cli`: `onnx` enables task-level ONNX adapters and
  `onnxruntime` enables native runtime execution.
- `text-model-runtime` and `text-embeddings`: `onnx`, `candle`, and
  `external-tests`.
- `text-linguistics`: `candle` and `external-tests`.
- `runtime-onnx`: `onnxruntime` and `external-tests`.
- Image and video task crates expose `local-onnx` or `onnx` features for their
  model-backed adapters.
- `video-analysis-ffmpeg`: `ffmpeg-backend`, `ffmpeg-tests`, and
  `external-tests`.
- `audio-analysis-separation`, `text-transcripts`, and
  `video-analysis-split` keep `external-tests` for real integration coverage
  only.

## External Install Rule

If a package needs installable external runtime dependencies (for example Python
environments, model bundles, or native toolchains), it must provide idempotent
scripts in `scripts/`:

- A setup script that verifies first and only installs/repairs missing or
  invalid state.
- A check script that verifies but does not install.
- Default installation paths under gitignored local directories.

## Dependency Graph

`video-analysis-core` is the foundational crate for shared contracts and pipeline
orchestration. The domain-specific crate families are organized around small
core packages: `audio-analysis-core`, `image-analysis-core`,
`text-core`, `vector-analysis-core`, and `three-d-processing-core`.
Processing, feature, index, synthesis, scalar numeric, and generic dense-data
crates build on those cores.
Most functional video crates depend on `video-analysis-core`, while
`video-analysis-gaussian-splatting` also reuses the camera and geometry
contracts from `video-analysis-radiance-fields`. `video-analysis-radiance-io`
keeps COLMAP, Nerfstudio, and PLY parsing out of those core math crates.
Composition happens in `video-analysis-cli` and the root `video-analysis`
facade crate. The
`comfyui-*` crates are standalone ComfyUI interoperability packages for
applications that need to inspect ComfyUI workflows, prompt graphs, model
folders, and extra model path configuration.

Inverse-direction crates use `data-inversion-core` to make lossy generation
explicit: generated text, audio, images, and video frames carry fidelity,
confidence, assumptions, and notes for values that were preserved, inferred, or
interpolated.

For the inter-package API contracts, serialized report shapes, package exports,
and compatibility rules, see [API Contracts](docs/API_CONTRACTS.md).

For the generated all-crate dependency chart, see
[Workspace Crate Dependency Graph](docs/DEPENDENCY_GRAPH.md). It is rendered as
Mermaid by standard Markdown viewers and is regenerated from `cargo metadata`
with:

```bash
python3 scripts/generate_dependency_chart.py
```

## Functional Pipelines

### YouTube Video Use Case

The prototype crate `prototypes/rust/video-analysis-use-cases` contains
runnable composition examples.
The first pipeline downloads or accepts a YouTube video, detects scenes,
extracts/segments transcript text, performs simple audio activity detection,
aggregates video/audio/text records into data buckets, and can call external
model commands for object/person detection, OCR, and transcript text analysis.

```bash
cargo run -p video-analysis-use-cases -- youtube-video \
  --url "https://www.youtube.com/watch?v=..." \
  --output use-case-output/youtube-video/analysis.json
```

Required local tools for the full URL workflow are `yt-dlp`, `ffmpeg`, and
`ffprobe`. Transcription uses the reusable `text-transcripts`
Whisper CLI wrapper and is skipped unless the OpenAI Whisper CLI is available as
`whisper`, or a command is supplied explicitly:

```bash
bash scripts/setup_e2e_external_tools.sh ffmpeg yt-dlp whisper
```

```bash
cargo run -p video-analysis-use-cases -- youtube-video \
  --url "https://www.youtube.com/watch?v=..." \
  --transcriber-command whisper \
  --transcriber-arg --model \
  --transcriber-arg base
```

External vision model integrations use the `video-analysis-recognition`
`ExternalCommandModel` JSON protocol with bundles supplied by `model-runtime`.
Each command receives one JSON request on stdin and returns
`{"predictions":[...]}` on stdout.

```bash
cargo run -p video-analysis-use-cases -- youtube-video \
  --input ./video.mp4 \
  --object-command ./scripts/detect-objects \
  --ocr-command ./scripts/ocr-frame \
  --text-command ./scripts/analyze-transcript \
  --visual-sample-every 30
```

The output report includes local asset paths, scenes, observations, transcript
segments, audio events, text events, and data bucket summaries.

Additional runnable local-file workflows live in the same crate:

```bash
cargo run -p video-analysis-use-cases -- video-red-cars \
  --input ./traffic.mp4 \
  --vehicle-detector-command python3 \
  --vehicle-detector-arg scripts/opencv_red_car_detector.py

cargo run -p video-analysis-use-cases -- audio-voice-analysis \
  --input ./voice.wav

cargo run -p video-analysis-use-cases -- image-person-edit \
  --input ./portrait.png \
  --prompt "replace the detected person with a marble statue" \
  --model flux1-dev.safetensors \
  --person-detector-command python3 \
  --person-detector-arg scripts/opencv_person_detector.py
```

Radiance-field crates now include a library-first composition layer:
`video-analysis-radiance-fields` owns shared scene/camera contracts,
`video-analysis-radiance-io` owns COLMAP/Nerfstudio/PLY parsing, and
`video-analysis-radiance-pipeline` wires those surfaces into typed project
loading, validation, summaries, and CPU Gaussian preview rendering.

```bash
bash scripts/setup_radiance_external_tools.sh
```

This workspace does not implement native NeRF/3DGS training or a production GPU
renderer. Distorted COLMAP camera models are parsed and retained by the IO
crate, but pipeline normalization and direct ray/camera conversion are
currently limited to undistorted `SIMPLE_PINHOLE` and `PINHOLE` cameras.

### Reference Recognition

`video-analysis-recognition` adds identity matching for known faces or objects.
It stores normalized reference embeddings, compares frame candidates with cosine
similarity, and can require repeated hits on the same track before emitting an
identity observation.

```rust,ignore
use video_analysis_core::ObservationKind;
use video_analysis_recognition::{ReferenceLibrary, RecognitionVideoAnalyzer};

let mut references = ReferenceLibrary::new();
references.add_reference(
    "einstein",
    "Albert Einstein",
    ObservationKind::Face,
    face_embedding_from_reference_image,
)?;

let analyzer = RecognitionVideoAnalyzer::new(
    "face-identity",
    detect_track_and_embed_faces_backend,
    references,
);
```

The backend is responsible for detection/tracking/embedding. The recognition
package owns the reference library, similarity search, thresholding, temporal
aggregation, and conversion into core `Observation` records.

### Video Detection

```text
video-analysis-ffmpeg
  -> video-analysis-ingest::VideoFrameSource / MediaSource
  -> video-analysis-core::OwnedVideoFrame
  -> video-analysis-core::ScenePipeline
  -> video-analysis-detectors::SceneDetector impls
  -> video-analysis-core::DetectionResult
```

- `video-analysis-ffmpeg` decodes/probes input videos and yields video frames.
- `video-analysis-ingest` defines file/live source traits for video, audio, and
  text.
- `video-analysis-core` owns video/audio/text sample types, time, scene, result,
  observation, analyzer trait, and pipeline orchestration contracts.
- `video-analysis-detectors` implements scene detector algorithms.
- `video-analysis-cli` wires the source, detector choice, and pipeline execution.

### Realtime Ingest

`ScenePipeline` supports both batch and incremental processing:

```rust,ignore
use video_analysis_core::{Result, ScenePipeline};
use video_analysis_detectors::ContentDetector;
use video_analysis_ffmpeg::FfmpegVideoSource;
use video_analysis_ingest::VideoFrameSource;

fn main() -> Result<()> {
    let mut source = FfmpegVideoSource::open_live("rtsp://camera.example/stream")?;
    let mut pipeline = ScenePipeline::builder()
        .detector(ContentDetector::default())
        .start_in_scene(true)
        .build()?;

    while let Some(frame) = source.next_video_frame()? {
        let analysis = pipeline.process_frame(frame)?;
        for cut in analysis.cuts {
            println!("cut at {:.3}s", cut.position.timestamp.seconds());
        }
    }

    let _final_result = pipeline.finish_detection()?;
    Ok(())
}
```

For recorded sources, `ScenePipeline::detect(&mut source)` remains available.
For realtime sources, call `process_frame()` as frames arrive and only call
`finish_detection()` when the stream ends or the application shuts down.

### Realtime Video Enrichment

Scene detection and frame-level enrichment can run in one pass over a live
video stream. OCR, face recognition, object detection, and similar integrations
implement `VideoAnalyzer` and emit structured `Observation` values.

```rust,ignore
use video_analysis_core::{Observation, ObservationKind, RealtimeVideoPipeline, Result, VideoAnalyzer, VideoFrame};
use video_analysis_detectors::ContentDetector;
use video_analysis_ffmpeg::FfmpegVideoSource;
use video_analysis_ingest::VideoFrameSource;

struct OcrAnalyzer;

impl VideoAnalyzer for OcrAnalyzer {
    fn name(&self) -> &str {
        "ocr"
    }

    fn process_frame(&mut self, frame: &VideoFrame<'_>) -> Result<Vec<Observation>> {
        let _ = frame;
        Ok(vec![Observation::new(self.name(), ObservationKind::Text)
            .text("detected text")
            .score(0.95)])
    }
}

fn main() -> Result<()> {
    let mut source = FfmpegVideoSource::open_live("rtsp://camera.example/stream")?;
    let mut pipeline = RealtimeVideoPipeline::builder()
        .scene_detector(ContentDetector::default())
        .video_analyzer(OcrAnalyzer)
        .start_in_scene(true)
        .build()?;

    while let Some(frame) = source.next_video_frame()? {
        let analysis = pipeline.process_frame(frame)?;

        for observation in analysis.observations {
            println!("scene {:?}: {:?}", observation.scene_index, observation.kind);
        }

        for scene in analysis.completed_scenes {
            println!(
                "closed scene {} with {} observations",
                scene.scene_index,
                scene.observations.len()
            );
        }
    }

    let result = pipeline.finish_analysis()?;
    println!("{} scenes, {} observations", result.scenes.len(), result.observations.len());
    Ok(())
}
```

`RealtimeVideoPipeline` decodes each frame once, feeds the borrowed frame to
scene detectors and video analyzers, annotates observations with the active
scene index, and emits a `SceneAnalysis` as soon as a scene closes. Downstream
components can consume either frame events immediately or complete per-scene
batches. OCR observations can also be converted to `OwnedTextSegment` with
`Observation::to_text_segment(...)` when a text pipeline should process detected
on-screen text.

### Hugging Face Model Downloads and Normalization

Common Hugging Face models can be downloaded into stable local bundles:

```bash
cargo run -p video-analysis-cli -- models presets

cargo run -p video-analysis-cli -- models download \
  --preset yolos-tiny \
  --bundle-dir .model-runtime

cargo run -p video-analysis-cli -- models inspect \
  --name yolos-tiny \
  --bundle-dir .model-runtime
```

Raw RGB/BGR frame model inference is exposed behind the CLI `onnxruntime`
feature:

```bash
cargo run -p video-analysis-cli --features onnxruntime -- models run \
  --manifest .model-runtime/yolos-tiny/main/manifest.json \
  --backend onnx \
  --input frame.rgb \
  --width 640 \
  --height 480 \
  --pixel-format rgb24
```

Custom repositories are also supported when the files are known:

```bash
vanalyze models download \
  --repo-id hf-internal-testing/tiny-random-distilbert \
  --task text-classification \
  --bundle-dir .model-runtime \
  --file config.json \
  --file tokenizer.json
```

The bundle manifest records the model name, repo id, revision, task, and local
file paths under the bundle directory. It can be converted back to
`DownloadedModel` for compatibility with external model backends:

```bash
cargo run -p video-analysis-cli -- mesh inspect --input mesh.obj
cargo run -p video-analysis-cli -- mesh convert --input mesh.obj --output mesh.gltf
cargo run -p video-analysis-cli -- posture estimate --predictions-json poses.raw.json --output poses.coco.json
cargo run -p video-analysis-cli -- posture export --input poses.coco.json --output pose.gltf
```

```rust
use model_runtime::{ModelBundleStore, ModelPreset};

# fn main() -> Result<(), Box<dyn std::error::Error>> {
let spec = ModelPreset::YolosTiny.spec();
let bundle = ModelBundleStore::new(".model-runtime").download(&spec)?;
let downloaded = bundle.to_downloaded_model();
# let _ = downloaded;
# Ok(())
# }
```

`model-runtime` owns model acquisition. `video-analysis-recognition` keeps
video-specific inference behind small backend traits:

```rust
use std::env;

use model_runtime::{HuggingFaceModelSpec, ModelBundleStore, ModelPreset};
use video_analysis_core::VideoAnalysisPipeline;
use video_analysis_recognition::{ExternalCommandModel, ModelVideoAnalyzer};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let spec = HuggingFaceModelSpec::from_preset(ModelPreset::DetrResnet50);
    let downloaded = ModelBundleStore::new(".model-runtime")
        .download(&spec)?
        .to_downloaded_model();

    let model_name = downloaded.spec.name.clone();
    let command =
        env::var("VISION_MODEL_COMMAND").unwrap_or_else(|_| "scripts/detect-objects".to_string());
    let backend = ExternalCommandModel::new(command, downloaded).persistent();

    let analyzer = ModelVideoAnalyzer::new(model_name, backend);
    let _pipeline = VideoAnalysisPipeline::builder().analyzer(analyzer).build()?;
    Ok(())
}
```

Running this example requires `VISION_MODEL_COMMAND` or `scripts/detect-objects`
to point at an executable implementing the
[`ExternalCommandModel` JSON protocol](docs/API_CONTRACTS.md#external-command-json-protocol).

Backends return `RawPrediction` values and `video-analysis-recognition` repairs
and normalizes common API differences: `xywh` or `xyxy` boxes, normalized or
pixel coordinates, missing labels, minimum score filtering, and same-label
non-maximum suppression. `ModelVideoAnalyzer` emits core `Observation` values;
`ModelTextAnalyzer` emits core `AnalysisEvent` values with dynamic semantic
labels.

`video-analysis-recognition` provides the native video object-detection backend
surface by composing image detection adapters. Default builds keep runtime
execution optional: deterministic tests use injected runners, while
`runtime-onnx/onnxruntime` gates native ONNX execution for models that return
DETR/YOLOS-style logits plus center-format boxes.

For model APIs that do not have a native Rust runtime yet, `ExternalCommandModel`
passes a JSON request to any executable over stdin and expects normalized JSON
predictions on stdout. This makes Python `transformers`, ONNX Runtime helpers,
or service-specific CLIs usable while keeping the package API stable.

Optional Python dependencies for model backend experiments are installed into an
ignored local virtual environment:

```bash
bash scripts/setup_model_external_tools.sh onnx
bash scripts/check_model_external_tools.sh onnx
```

Model artifacts are intentionally kept out of git under `.model-runtime`.
Use the lock-driven idempotent sync script to verify checksums and re-download
missing/corrupted files when needed:

```bash
# verify + auto-repair
bash scripts/sync_model_bundles.sh

# verify only (CI)
bash scripts/sync_model_bundles.sh --check

# after changing model specs in scripts/model_bundles.lock.sh
bash scripts/sync_model_bundles.sh --write-lock
```

The same flow is available through the existing model tool wrappers:

```bash
bash scripts/setup_model_external_tools.sh bundles
bash scripts/check_model_external_tools.sh bundles
```

### Audio Analysis

Audio follows the same shape: the FFmpeg crate decodes, the ingest trait yields
chunks, and `AudioPipeline` analyzes each chunk as it arrives.

```rust,ignore
use video_analysis_core::{AudioPipeline, Result};
use video_analysis_ffmpeg::FfmpegAudioSource;
use video_analysis_ingest::AudioFrameSource;

fn main() -> Result<()> {
    let mut source = FfmpegAudioSource::open_live("rtsp://camera.example/stream")?;
    let mut pipeline = AudioPipeline::builder()
        .analyzer(my_audio_analyzer())
        .build()?;

    while let Some(frame) = source.next_audio_frame()? {
        let analysis = pipeline.process_frame(frame)?;
        for event in analysis.events {
            println!("{:?} {}", event.timestamp, event.label);
        }
    }

    let _final_result = pipeline.finish_analysis()?;
    Ok(())
}
```

Audio analyzers implement `video_analysis_core::AudioAnalyzer` and receive
borrowed `AudioFrame` values. `FfmpegAudioSource` emits `f32` PCM chunks by
default, with configurable samples per chunk.

### Text Analysis

Text ingest is line/segment oriented so transcripts, subtitles, logs, or live
ASR output can be analyzed incrementally.
For first-release scope and crate selection guidance, see
[Text Release Scope](docs/TEXT_RELEASE_SCOPE.md). For corpus, lexical scoring,
semantic index, retrieval, and analysis report workflows, see the
[Text Corpus Guide](docs/TEXT_CORPUS_GUIDE.md).

```rust,ignore
use video_analysis_core::{Result, TextPipeline};
use video_analysis_ingest::{TextLineSource, TextSegmentSource};

fn main() -> Result<()> {
    let mut source = TextLineSource::open("transcript.txt")?;
    let mut pipeline = TextPipeline::builder()
        .analyzer(my_text_analyzer())
        .build()?;

    while let Some(segment) = source.next_text_segment()? {
        let analysis = pipeline.process_segment(segment)?;
        for event in analysis.events {
            println!("segment {} {}", analysis.segment_index, event.label);
        }
    }

    let _final_result = pipeline.finish_analysis()?;
    Ok(())
}
```

Text analyzers implement `video_analysis_core::TextAnalyzer`. For live text,
wrap any blocking `BufRead` with `TextLineSource::live(...)`.
`text-lexical` provides ready-made analyzers for stats, keywords,
patterns, and transcript heuristics. `text-transcripts` parses
Whisper JSON, SRT, WebVTT, and plain line transcripts into reusable transcript
segments or a `TextSegmentSource`. For larger document collections,
`text-lexical` provides corpus statistics, TF-IDF terms, and TF-IDF
search; `text-embeddings` adds hashed semantic embeddings,
co-occurrence graphs, related terms, and semantic search; and
`text-generation` provides Markov next-token prediction and
generation. `text-linguistics` adds heuristic-first language
detection, tokenizer routing, token/subword alignment, lemma/POS/morphology
annotation, dependency parsing, typed entities, coreference, event extraction,
discourse segmentation, topic descriptors, and style profiles.

### Data Aggregation

Large video, audio, text, numeric, and vector streams can be normalized into
borrowed `DataRecord` values and summarized online without retaining the
original payloads.

```rust
use video_analysis_core::{Result, Timestamp, Timebase};
use video_analysis_data::{BucketAggregator, BucketConfig, DataRecord};

fn main() -> Result<()> {
    let config = BucketConfig::fixed_duration_seconds(5.0)?;
    let mut buckets = BucketAggregator::new(config)?;

    let timestamp = Timestamp::new(12, Timebase::new(1, 1));
    for bucket in buckets.push(DataRecord::number(
        "telemetry:score",
        0,
        Some(timestamp),
        0.82,
    ))? {
        println!("closed bucket {} with {} records", bucket.bucket_index, bucket.records);
    }

    let embedding = [0.1, 0.2, 0.3];
    buckets.push(DataRecord::vector(
        "telemetry:embedding",
        1,
        Some(timestamp),
        &embedding,
    ))?;

    let _tail = buckets.finish();
    Ok(())
}
```

Buckets can be fixed by duration, record count, or estimated byte size. Numeric
streams keep online min/max/mean summaries. Vector streams keep norm summaries
and bounded per-dimension means so embeddings can be handled without storing
every vector.

### Retained Datasets, Transforms, and Features

For workflows that need queryable or reusable analysis data rather than online
bucket summaries, `video-analysis-dataset` stores owned dataset records for
scenes, cuts, frame/audio/text metadata, observations, events, metrics, tracks,
and extracted features. Raw media payload bytes are intentionally not retained.

`video-analysis-transform` operates over those records with filtering, fixed
time windows, scene grouping, time/frame joins, dedupe, sorted merge, and
numeric feature resampling. `video-analysis-features` builds on the same record
model to produce scene statistics, observation label histograms, transcript
counts, audio event summaries, track summaries, and vector means.

`video-analysis-storage` persists datasets as one JSONL `DatasetRecord` per
line plus a `manifest.json` containing schema version, record counts, files,
and dataset attributes.

### Output

```text
DetectionResult
  -> video-analysis-output
  -> CSV / HTML / stdout / file writers
```

- `video-analysis-output` serializes `Scene`, `MetricsStore`, and
  `DetectionResult`.
- It must not know about FFmpeg, CLI options, or detector implementations.

### Split

```text
DetectionResult.scenes
  -> video-analysis-split
  -> ffmpeg CLI scene files
```

- `video-analysis-split` consumes only `Scene` values and splits the original
  media.
- It does not perform detection or own detector/source selection.

### CLI

```text
CLI args
  -> source construction
  -> detector construction
  -> ScenePipeline::detect
  -> output or split action
```

- `video-analysis-cli` composes all runtime pieces.
- The CLI owns command selection, argument mapping, and workflow branching.

## Dependency Rules

Allowed internal dependencies:

- `video-analysis-core`: external utility crates only.
- `video-analysis-data` -> `video-analysis-core`.
- `video-analysis-detectors` -> `video-analysis-core`.
- `video-analysis-ingest` -> `video-analysis-core`.
- `video-analysis-ffmpeg` -> `video-analysis-core`, `video-analysis-ingest`.
- `video-analysis-output` -> `video-analysis-core`.
- `video-analysis-split` -> `video-analysis-core`.
- `video-analysis-cli` -> crates it composes for CLI workflows.
- `video-analysis` root facade -> all library crates except CLI.

Forbidden internal dependencies:

- `video-analysis-core` must not depend on any workspace crate.
- `video-analysis-detectors` must not depend on FFmpeg, output, split, CLI, or
  facade crates.
- `video-analysis-data` must not depend on detectors, FFmpeg, output, split,
  CLI, or facade crates.
- `video-analysis-ingest` must not depend on detectors, FFmpeg, output, split,
  CLI, or facade crates.
- `video-analysis-ffmpeg` must not depend on detectors, output, split, CLI, or
  facade crates.
- `video-analysis-output` must not depend on detectors, FFmpeg, split, CLI, or
  facade crates.
- `video-analysis-split` must not depend on detectors, FFmpeg crate, output,
  CLI, or facade crates.
- No library crate should depend on `video-analysis-cli`.

## Example

```bash
cargo run -p video-analysis-cli -- detect --input video.mp4 --detector content --output scenes.csv --stats stats.csv
cargo run -p video-analysis-cli -- detect --input video.mp4 --detectors content,adaptive,histogram,hash --combined-threshold 0.5 --detector-weight histogram=0.75 --detector-weight hash=0.75 --output scenes.csv --stats stats.csv
cargo run -p video-analysis-cli -- list --input video.mp4 --detector adaptive
cargo run -p video-analysis-cli -- split --input video.mp4 --detector content --output-dir scenes
```

The default test suite does not require FFmpeg to be installed.
