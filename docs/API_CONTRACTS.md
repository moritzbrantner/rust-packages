# API Contracts

This document describes the inter-package contracts that let the Rust crates and
the `@moritzbrantner/video-analysis-ui` package work together. It is intentionally not an
exhaustive rustdoc inventory. It focuses on shared types, traits, serialized
formats, file formats, package exports, and dependency boundaries.

`moritzbrantner-video-analysis-core` owns the canonical runtime contracts for time, media
samples, scene detection, metrics, observations, analyzers, and pipelines. Other
crates should compose around those contracts instead of defining parallel types.

## Composable Building Block Policy

Crates expose reusable domain behavior as composable building blocks. They must
not depend on graph-builder concepts such as workflow nodes, ports, edges,
sockets, or layout metadata. Workflow composition belongs in package consumers,
prototypes, CLIs, or external projects that choose to map package operations to
their own graph model.

The crate owning the most general semantic form owns the stable contract.
Specialized crates may enrich contracts for their domain, but they must preserve
conversion paths back to the general contract instead of creating unrelated
parallel DTOs.

`moritzbrantner-runtime-core` owns the shared package-surface DTOs:
`PackageSurface`, `SurfaceOperation`, `SurfaceRequest`, `SurfaceResponse`,
execution plans, side effects, artifacts, diagnostics, and runtime
capabilities. `SurfaceOperation` remains operation metadata, not workflow node
metadata.

Foundation contract owners for the first steering wave are:

| Contract family | Owner |
| --- | --- |
| Runtime surface DTOs | `moritzbrantner-runtime-core` |
| Jobs and artifacts | `moritzbrantner-jobs-core` |
| Model specs, bundles, and model lifecycle | `moritzbrantner-model-runtime` |
| Media samples, timestamps, observations, bounding boxes, analyzers | `moritzbrantner-video-analysis-core` |
| Images | `moritzbrantner-image-analysis-core` |
| Audio frames and features | `moritzbrantner-audio-analysis-core` |
| Text documents and text segments | `moritzbrantner-text-core` |
| Timed transcripts | `moritzbrantner-text-transcripts` |
| Tensors | `moritzbrantner-tensor-data` |
| Vectors | `moritzbrantner-vector-analysis-core` |
| Sparse data | `moritzbrantner-math-sparse-data` |
| Dense data | `moritzbrantner-dense-data` |
| Numbers | `moritzbrantner-numbers-core` |
| Geometry primitives | `moritzbrantner-math-geometry-2d` |
| Signal primitives | `moritzbrantner-math-signal-core` |

## Package Surface Policy

Runtime packages use the same generated surface shape, but the adapters stay
outside the reusable library crate:

- Library: a Rust `lib` target under `crates/*/*`.
- CLI: an adjacent `<crate>-cli` package.
- API: an adjacent `<crate>-server` package exposing HTTP endpoints.
- Rust WASM: a binding crate under `crates/bindings/<crate>-wasm`.
- Frontend WASM: a Bun package under `packages/<crate>-wasm`.
- UI/App: a Vite package under `packages/<crate>-app`.

Library crates should not declare generic CLI/API/UI `[[bin]]` targets. Adapter
packages depend on the library and own their runtime, transport, and webpage
code.

Text package servers and overview routes expose discovery metadata:

- `GET /api/models` returns model catalog entries with `supported`, `loadable`, `requiredFeature`, `requiredSetup`, and `smokeOperation` where applicable.
- `GET /api/benchmarks` returns benchmark scenario metadata only. It never stores or returns host-specific benchmark results.
- `POST /api/run` remains the generic operation contract.

The UI measures benchmark scenarios locally with `performance.now()` against the selected runtime mode and reports browser user agent, runtime mode, iteration counts, total time, average time, throughput, and output count.

Every non-wrapper library crate owns its operation metadata and execution in
`src/surface.rs`. Thin CLI, server, WASM, and app packages must call that
library-owned surface instead of implementing package behavior in wrapper code.
The shared `PackageSurface`, `SurfaceOperation`, `SurfaceRequest`, and
`SurfaceResponse` DTOs live in `moritzbrantner-runtime-core` and are re-exported
from `video-analysis-core::runtime` for compatibility. Generic job
envelopes and artifact storage live in `moritzbrantner-jobs-core`; model-specific artifact
metadata, bundle manifests, and Hugging Face downloads stay in `moritzbrantner-model-runtime`.
`moritzbrantner-jobs-core` owns only generic job state, cancellation, progress, diagnostics,
and artifact contracts. It must not gain model semantics, Hugging Face concepts,
ONNX/Candle/tokenizer dependencies, or domain media dependencies.

Model catalogs, model specs, preset metadata, schema validation, and deterministic
fallback planning may run synchronously. Model downloads, bundle
materialization, runtime warmup, native inference, external model commands, and
batch inference must declare execution metadata and side effects. The
server-only local ONNX defaults for text QA, image classification, and image
captioning are explicit exceptions to the no-download package-surface policy:
they use `moritzbrantner-model-runtime` to resolve or materialize bundles under
`.model-runtime`, declare filesystem/network side effects through
`xExecutionPlan`, and stay unsupported in WASM.

The current workspace-wide baseline operation is `describe`; crates should add
richer representative operations in their own surface module as library
functionality matures. See `docs/PACKAGE_SURFACE_MATRIX.md` for the generated
crate-by-crate audit.

Deterministic text, image, data, math, vector, jobs, model-runtime, and
test-support library crates are expected to expose `describe` plus at least two
crate-specific operations through the same library-owned surface. Image model
task surfaces may expose catalog, schema, import, and validation operations, but
default calls must not download models or run native inference.

Foundation package surfaces (`jobs-core`, `model-runtime`,
`video-analysis-core`, `image-analysis-core`, `audio-analysis-core`,
`text-core`, `text-transcripts`, `numbers-core`, `tensor-data`,
`vector-analysis-core`, `math-sparse-data`, `dense-data`,
`math-geometry-2d`, and `math-signal-core`) use strict release metadata for
operation schemas, preserve the structured
`operation`/`title`/`message`/`summary`/`result` response shape, and return
typed `SurfaceError` JSON strings for invalid requests, unsupported operations,
unsupported values, and resource-limit failures. Default foundation operations
remain deterministic and in-memory; they cap lifecycle scripts, numeric/tensor
payloads, vector previews, and sparse matrix entries before doing work.
Runtime/job planning metadata is exposed through `xExecutionPlan` schema
extensions, including mode, side effects, cancellation, progress units, expected
artifacts, requirements, and recommended input size.

The text crate surfaces now expose deterministic, local-first operations for
core statistics/tokenization/boundaries, lexical analysis and corpus search,
linguistic analysis/entity projection, hashed embeddings and transient search,
in-memory retrieval, transcript parsing/formatting, fallback/imported NLP
tasks, Markov/template generation, generation-from-linguistics adapters, and
runtime helpers. `runtime.onnxQaProbe`, `runtime.downloadBundle`, and
`qa.answer` without imported predictions/backend are native server-side model
workflows when built with `local-onnx`; all other default text surface calls
remain no-download unless documented otherwise. These operations must continue to return
`SurfaceResponse` values with JSON payloads and typed error strings, and they
must not silently download models, run native inference, invoke ASR commands, or
write retrieval persistence files through default surface calls outside the
documented model-backed operation exceptions.

Text package operations declare release contract metadata in their
`SurfaceOperation` schemas. Top-level request fields are explicit
(`additionalProperties: false`), outputs preserve the structured
`operation`/`title`/`message`/`summary`/`result` shape, and schema extensions
record `xOperationCategory`, `xReleaseStability`, `xContractPolicy`, resource
limits, and the typed error shape. During the release train, operation IDs and
declared fields are additive-only unless a versioned migration is documented.
Malformed JSON shapes and unknown operations should use the shared
`SurfaceError` envelope with `code`, `message`, `operation`, and `details`;
server adapters expose the same code and message in their HTTP diagnostics.
Server `/api/package` metadata also exposes `runtimeMetadata.candleDevice`,
using `null` when the package has no Candle-backed runtime preference.

Audited text and image package operations return structured JSON values with
`operation`, `title`, `message`, `summary`, and `result` fields. Concrete domain
fields remain at the top level for compatibility, while `result` carries the
same operation-owned payload for generic UI rendering.

The audio crate surfaces now follow the same parity rule. Default audio
operations expose deterministic, in-memory signal summaries, FFT/STFT features,
pitch and rhythm projections, processing summaries, spectral recognition,
speaker baselines, synthesis, MIDI rendering, fixture generation, and
non-executing I/O/separation plans. Audio transcription surfaces may run native
Candle Whisper only when built with explicit native features and caller-provided
local bundles; Candle Whisper CUDA is the primary native target and Candle CPU
is the local development fallback. External Python WhisperX is
compatibility/parity tooling only. Browser and WASM transcription package
surfaces can plan or import transcript data, but they do not run native ASR.
Default calls must reject invalid sample metadata and non-finite samples, cap
preview payloads, return `SurfaceResponse` JSON values, and avoid FFmpeg,
Demucs, model downloads, native inference, filesystem writes, or network access
unless the operation explicitly documents native/server model execution.

The first video/image/3D/ComfyUI surface parity tranche follows the same
library-owned convention. Dataset, transform, feature, storage, detector,
tracking, split, synthesis, recognition, output, image, ComfyUI, 3D, and
radiance I/O surfaces now expose deterministic summary, filter, preview,
validation, extraction, planning, and reporting operations that run in memory
and cap preview payloads. Storage and interchange surfaces use plan/preview
operations for package adapters; actual file reads/writes remain explicit
library or CLI workflows.

## Contract Ownership Rule

The crate that owns the most general semantic form owns the stable contract.
Specialized crates may add domain fields, but they must expose conversion back
to the general contract instead of creating unrelated parallel DTOs.

For the first enforced boundary, `moritzbrantner-text-core` owns generic text contracts such as
`TextDocumentContract` and `TextSegmentContract`. `moritzbrantner-text-transcripts` owns
`TranscriptSegmentContract` and `TranscriptionContract` as timed/speaker-aware
text specializations. Audio transcription surfaces consume and return those
transcript contracts through `moritzbrantner-audio-analysis-recognition`.
Speaker diarization may enrich an existing
`TranscriptionContract` with speaker labels and scores, but transcript DTOs
remain owned by `moritzbrantner-text-transcripts`.

UI and report types are projections of these contracts. A `*Report` type may
drop fields that are not needed for presentation, but shared fields should be
generated from or tested against the owning Rust contract.

## Test Surface Policy

Each package surface has a matching test layer:

- Library code owns Rust unit tests beside the implementation.
- CLI adapters own integration tests that execute the compiled binary.
- API adapters own integration tests that call the HTTP endpoints.
- UI packages own browser e2e tests that render the package through a real page.

## Feature Flag Policy

Runtime and external integration crates use a shared feature policy:

- `default = []` unless a crate is pure-Rust and intentionally unconditional.
- Runtime-enabling flags stay explicit, for example `onnx`, `onnxruntime`,
  `candle`, and `ffmpeg-tests`.
- `external-tests` always means real tools, real models, or real network
  access and is not part of the default contributor gate.
- Additive aliases are allowed for compatibility, but docs should prefer the
  canonical explicit feature name.

## Workspace Contract Map

| Package | Role | Depends on | Exposes | Consumed by |
| --- | --- | --- | --- | --- |
| `moritzbrantner-video-analysis` | Root facade crate | Library crates except CLI and use cases | Re-exports core items, detector items, and package modules | Applications that want one import surface |
| `moritzbrantner-comfyui-data` | ComfyUI workflow and socket typing contracts | `serde`, `serde_json`, `moritzbrantner-tensor-data`, `thiserror` | Workflow JSON nodes, links, groups, validation helpers, prompt nodes/links, normalized `ComfySocketType`, workflow socket inventories, `ConditioningItem`, `ConditioningBatch` | Applications importing, validating, inventorying, or emitting ComfyUI graphs |
| `moritzbrantner-comfyui-latents` | ComfyUI latent-space contracts | `moritzbrantner-tensor-data`, `moritzbrantner-video-analysis-core`, `serde` | `LatentBatch`, `LatentMask`, `LatentImageSize`, mask compatibility checks | Applications or integrations that need stable latent-space data contracts |
| `moritzbrantner-comfyui-models` | ComfyUI model folder, inventory, and reference contracts | `serde`, `thiserror` | Core model folder keys, default relative paths, inventory scanning, extra model paths YAML generation, `ComfyModelRole`, `ComfyModelRef` | Applications managing shared ComfyUI model libraries |
| `moritzbrantner-data-inversion-core` | Shared lossy inverse-conversion metadata | `moritzbrantner-video-analysis-core` | `InformationFidelity`, `InversionMethod`, `InversionTrace`, generated value wrappers | Synthesis crates and applications that need explicit interpolation/assumption metadata |
| `moritzbrantner-animation-core` | Shared animation timeline contracts | `moritzbrantner-three-d-processing-core`, `moritzbrantner-video-analysis-core`, `serde` | Time values, interpolation modes, keyframes, typed tracks, transform tracks, joints, skeletons, and animation clips | Future 2D/3D animation workflows, posture sequence interop, mesh/skinning work |
| `moritzbrantner-numbers-core` | Shared scalar numeric summaries and ranges | `moritzbrantner-video-analysis-core` | Running stats, weighted summaries, quantiles, histograms, numeric range helpers | `moritzbrantner-dense-data`, `moritzbrantner-video-analysis-data`, analytics workflows, and reporting utilities |
| `moritzbrantner-tensor-data` | Generic finite `f32` tensor contracts | `moritzbrantner-video-analysis-core`, `serde`, `serde_json` | `TensorShape`, `F32Tensor`, `F32TensorView`, shape/element validation, metadata | `moritzbrantner-comfyui-latents`, audio/image bridges, and future tensor-oriented interop crates |
| `moritzbrantner-finance-statistics` | Finance-oriented return and risk statistics | `moritzbrantner-video-analysis-core`, `moritzbrantner-math-statistics` | Simple/log returns, cumulative and annualized return, volatility, Sharpe, Sortino, beta/alpha, drawdown wrappers, drawdown duration, historical VaR/CVaR wrappers, tracking error, information/Calmar/Omega ratios, portfolio weights/returns/risk summaries, historical covariance, risk/return contribution, turnover, rolling windows | Finance analytics, portfolio reporting, and future market-data workflows |
| `moritzbrantner-math-geometry-2d` | Shared 2D geometry primitives | `moritzbrantner-video-analysis-core`, `serde` | Checked 2D points, vectors, rectangles, IoU/overlap ratios, normalized coordinates, segments and segment intersections, circles, polygons, bounds, affine transforms, and `BoundingBox` interop | Image/video/posture crates and UI-adjacent layout workflows |
| `moritzbrantner-math-linear` | Shared dense matrix and kernel contracts | `moritzbrantner-video-analysis-core`, `moritzbrantner-tensor-data`, `moritzbrantner-vector-analysis-core` | `F32Matrix` and `F64Matrix` shapes/views, matrix multiply, row/column helpers, centering, Gram matrices, rank estimates, QR least-squares, pure Rust SVD, pseudoinverse, LU/Cholesky/QR decomposition, determinant/condition diagnostics, Cholesky solve/log determinant, tensor/vector bridges, `Kernel1d`, `Kernel2d` | Image/video preprocessing, text model utilities, dense/statistical workflows |
| `moritzbrantner-math-signal-core` | Shared signal-domain math | `moritzbrantner-video-analysis-core`, `moritzbrantner-numbers-core` | Sample-rate/resampling descriptors, windows, frame strides, interpolation, signal level summaries, FIR kernels/application, peak normalization, and biquad design helpers | Audio crates and future time-series/video transform workflows |
| `moritzbrantner-math-sparse-data` | Shared sparse vector and matrix contracts | `moritzbrantner-video-analysis-core`, `moritzbrantner-vector-analysis-core`, `moritzbrantner-numbers-core`, `moritzbrantner-math-linear` | Sparse vectors, vector norms/add/scale/hadamard/prune/top-k, COO/CSR matrices, transpose, row/column nnz and sums, matrix summaries, row normalization, matrix-vector and matrix-matrix dense products, sparse similarities, dense bridges | Text corpus/semantic crates and future retrieval/index workflows |
| `moritzbrantner-math-statistics` | Shared scalar, pairwise, rolling, multivariate, and matrix statistics | `moritzbrantner-video-analysis-core`, `moritzbrantner-numbers-core`, `moritzbrantner-math-linear` | Series summaries, sample/population variance, changes, pairwise covariance/correlation, ranks/Spearman correlation, simple and OLS regression, OLS diagnostics, ridge regression, row-wise covariance/correlation matrices, rolling windows, z-scores, tail risk, drawdown, running covariance, covariance matrices, f64-default package normalizers, centered-SVD PCA, weighted observations | Dense-data, feature extraction, finance wrappers, and analytics workflows |
| `moritzbrantner-audio-analysis-core` | Shared audio analysis utilities | `moritzbrantner-video-analysis-core`, `moritzbrantner-tensor-data`, `moritzbrantner-math-signal-core` | Normalized sample conversion, mono mixing, shared window functions, frame iteration, streaming frame windows, feature-series contracts, level helpers, waveform batch contracts | Audio analysis crates and applications |
| `moritzbrantner-audio-analysis-fourier` | Frequency-domain audio analysis | `moritzbrantner-audio-analysis-core`, `moritzbrantner-video-analysis-core` | FFT spectra, STFT spectrograms, spectral features, mel-style band summaries, dominant-frequency analyzer | Applications and audio pipelines |
| `moritzbrantner-audio-analysis-io` | Audio input convenience facade | `moritzbrantner-audio-analysis-core`, `moritzbrantner-video-analysis-core`, `moritzbrantner-video-analysis-ingest`, `moritzbrantner-video-analysis-ffmpeg`, `hound` | Audio-named input options, FFmpeg source opening helpers, ingest re-exports, waveform batch decoding, pure WAV read/write helpers, WAV/probe plan surfaces | Applications that want audio-specific input APIs |
| `moritzbrantner-audio-analysis-pitch` | Pitch estimation | `moritzbrantner-audio-analysis-core`, `moritzbrantner-video-analysis-core` | Autocorrelation pitch detector, pitch analyzer events, note projection, chroma and pitch-class summaries | Applications and audio pipelines |
| `moritzbrantner-audio-analysis-processing` | Realtime-safe audio processing | `moritzbrantner-audio-analysis-core`, `moritzbrantner-math-signal-core`, `moritzbrantner-video-analysis-core`, `moritzbrantner-video-analysis-ingest` | Audio transform trait, processor chains, gain/clip/mono/DC/biquad/noise-gate transforms, processed sources, deterministic loudness-oriented reports | Applications, preprocessing workflows, audio pipelines |
| `moritzbrantner-audio-analysis-recognition` | Audio similarity and recognition | `moritzbrantner-audio-analysis-core`, `moritzbrantner-audio-analysis-fourier`, `moritzbrantner-video-analysis-core`, `moritzbrantner-text-transcripts` | Spectral embeddings, sample-backed reference libraries, similarity search, recognition analyzer events, and deprecated Rust transcription compatibility wrappers only | Applications, audio pipelines, reference matching workflows |
| `moritzbrantner-audio-analysis-transcription` | Native audio/video transcription orchestration | `moritzbrantner-video-analysis-core`, `moritzbrantner-text-transcripts`, optional `moritzbrantner-model-runtime`, optional `moritzbrantner-audio-analysis-speakers` | ASR/VAD/alignment provider traits, Candle Whisper CPU/CUDA execution from local bundles with timestamp-token segment timing, projected word timing, and chunk/window fallback timing, WAV native input normalization, energy VAD chunking, deterministic CTC alignment contracts plus local Candle wav2vec2 CTC execution for supported bundles, optional speaker diarization assignment, and external WhisperX compatibility import/execution | Real ASR workflows, transcript generation, alignment, diarization, and compatibility comparison |
| `moritzbrantner-audio-analysis-speakers` | Speaker analysis | `moritzbrantner-audio-analysis-core`, `moritzbrantner-audio-analysis-recognition`, `moritzbrantner-video-analysis-core` | Speaker embeddings, enrollment, thresholded identification, deterministic VAD, heuristic baseline diarization, transcript speaker assignment with majority, nearest-start, and strict-contained policies | Speaker-aware audio and transcript workflows |
| `moritzbrantner-audio-analysis-rhythm` | Rhythm and tempo analysis | `moritzbrantner-audio-analysis-core`, `moritzbrantner-video-analysis-core` | Onset envelope, onset detection, tempo estimates, rhythm analyzer events | Applications and audio pipelines |
| `moritzbrantner-audio-analysis-separation` | Instrument stem separation command wrapper | `moritzbrantner-video-analysis-core` | HTDemucs/Demucs options, command previews, opt-in Demucs execution, expected stem paths and output layouts | Applications and preprocessing workflows |
| `moritzbrantner-audio-analysis-synthesis` | Deterministic inverse audio generation | `moritzbrantner-data-inversion-core`, `moritzbrantner-video-analysis-core` | Tone specs, tone timelines, pitch/onset event to tone conversion, click-track synthesis, synthesized `OwnedAudioFrame` values | Applications prototyping audio from symbolic or analyzed events |
| `moritzbrantner-audio-analysis-test-support` | Shared audio fixtures and test helpers | `moritzbrantner-audio-analysis-core`, `moritzbrantner-video-analysis-core` | Synthetic waveform frames, deterministic audio buffers, fixture builders, assertion helpers | Audio crate tests, smoke tests, and package surface checks |
| `moritzbrantner-image-analysis-core` | Shared image contracts and statistics | `moritzbrantner-video-analysis-core`, `moritzbrantner-tensor-data` | Borrowed/owned image views, image batches, pixel formats, compacting, mean color, luma histograms, mask tensor bridge helpers | Image processing crates, applications, video frame preprocessing |
| `moritzbrantner-image-analysis-processing` | CPU image processing primitives | `moritzbrantner-image-analysis-core`, `moritzbrantner-math-geometry-2d`, `moritzbrantner-math-linear`, `moritzbrantner-video-analysis-core` | Crop, nearest resize, grayscale, invert, threshold, 3x3 convolution, processor chains, shared `RectU32`/`Kernel2d` bridges | Applications, preprocessing workflows |
| `moritzbrantner-image-analysis-ocr` | OCR presets and rich text extraction contracts | `moritzbrantner-image-analysis-core`, `moritzbrantner-video-analysis-core`, `moritzbrantner-model-runtime` | Hugging Face OCR presets, OCR technique metadata, rich text documents/blocks/lines/tokens, image and video-frame OCR backend traits | Applications extracting text from images or sampled video frames |
| `moritzbrantner-image-analysis-captioning` | Image caption model surface | `moritzbrantner-image-analysis-core`, `moritzbrantner-model-runtime`, `moritzbrantner-video-analysis-core` | Caption model presets, caption request/result DTOs, deterministic fallback captions, image caption backend traits | Applications describing images or sampled video frames |
| `moritzbrantner-image-analysis-classification` | Image classification model surface | `moritzbrantner-image-analysis-core`, `moritzbrantner-model-runtime`, `moritzbrantner-video-analysis-core` | Classification model presets, class score DTOs, deterministic fallback classifiers, image classification backend traits | Applications labeling images or sampled video frames |
| `moritzbrantner-image-analysis-embeddings` | Image embedding model surface | `moritzbrantner-image-analysis-core`, `moritzbrantner-model-runtime`, `moritzbrantner-vector-analysis-core`, `moritzbrantner-video-analysis-core` | Embedding model presets, image embedding request/result DTOs, deterministic fallback embeddings, vector bridges | Search, recognition, clustering, and multimodal retrieval workflows |
| `moritzbrantner-image-analysis-synthesis` | Deterministic inverse image generation | `moritzbrantner-data-inversion-core`, `moritzbrantner-image-analysis-core`, `moritzbrantner-video-analysis-core` | Solid images, gradients, luma-histogram expansion, region painting | Applications reconstructing approximate image buffers from summaries or regions |
| `moritzbrantner-text-analysis` | Unified text analysis orchestration | `moritzbrantner-text-core`, `moritzbrantner-text-lexical`, `moritzbrantner-text-linguistics`, `moritzbrantner-text-embeddings`, `moritzbrantner-text-retrieval`, `moritzbrantner-video-analysis-core`, optional `moritzbrantner-model-runtime` | Document and corpus analysis reports combining core text stats, lexical features, similarity, linguistic summaries, embeddings, retrieval, diagnostics, and reusable report DTOs | Text applications, transcript analysis, search prototypes, and package overview demos |
| `moritzbrantner-text-core` | Shared text analysis utilities | `moritzbrantner-video-analysis-core`, `unicode-normalization`, `unicode-segmentation` | Text document contracts, text segment bridging, whitespace normalization, span-aware tokens, Unicode word/grapheme spans, script profiles, sentences, paragraphs, counts | Text feature crates, text pipelines, applications |
| `moritzbrantner-text-lexical` | Lexical features and classical corpus statistics | `moritzbrantner-text-core`, `moritzbrantner-math-sparse-data`, `moritzbrantner-video-analysis-core`, `serde` | Stop words, keywords, n-grams, shingles, readability, stemming, extractive summaries, sentiment, reusable text analyzers, TF-IDF, BM25, sparse term matrices/vectors | Applications, text analytics, semantic indexing |
| `moritzbrantner-text-linguistics` | Local model-backed linguistic interpretation | `moritzbrantner-jobs-core`, `moritzbrantner-model-runtime`, `moritzbrantner-text-core`, `moritzbrantner-text-lexical`, `moritzbrantner-text-transcripts`, `moritzbrantner-video-analysis-core`, optional `tokenizers`/Candle crates | Language detection, tokenizer routing/alignment, lemmatization, POS/morphology, chunks, dependencies, local model named entities, jobs-backed model materialization, rule entity fallback, coreference, relations, events, discourse, topics, style, `TextAnalyzer` adapter | Applications, text pipelines, transcript analysis |
| `moritzbrantner-text-embeddings` | Embedding traits and lightweight semantic text analysis | `moritzbrantner-text-core`, `moritzbrantner-text-lexical`, `moritzbrantner-math-sparse-data`, `moritzbrantner-vector-analysis-core`, `moritzbrantner-vector-analysis-index`, `moritzbrantner-video-analysis-core`, optional `tokenizers`/`runtime-onnx`/Candle crates | `TextEmbeddingBackend`, `TextEmbedderBackend`, `EmbeddingModelInfo`, hashed dense/sparse embeddings, optional ONNX/Candle text embedders, semantic indexes, text similarity, co-occurrence graphs, related-term scoring | Retrieval, applications, semantic analysis prototypes |
| `moritzbrantner-text-retrieval` | Text ingestion, search, and persisted retrieval indexes | `moritzbrantner-text-core`, `moritzbrantner-text-lexical`, `moritzbrantner-text-embeddings`, `moritzbrantner-vector-analysis-index`, `serde`, `serde_json`, `thiserror`, `moritzbrantner-video-analysis-core` | Search documents/chunks, chunking options, full-text/semantic/hybrid query/ranking, metadata filters, search results, related-chunk lookup, manifests, chunk/vector JSONL snapshots, corpus metadata, rehydration helpers | Applications, search prototypes, local retrieval snapshots |
| `moritzbrantner-text-generation` | Deterministic text prediction and synthesis | `moritzbrantner-data-inversion-core`, `moritzbrantner-text-core`, `moritzbrantner-video-analysis-core` | Token Markov chains, next-token predictions, deterministic generation, perplexity scoring, weighted term prompts, generated text segments | Applications, text pipelines, prototyping |
| `moritzbrantner-text-generation-linguistics` | Linguistic adapters for deterministic generation | `moritzbrantner-data-inversion-core`, `moritzbrantner-text-core`, `moritzbrantner-text-generation`, `moritzbrantner-text-linguistics`, `moritzbrantner-video-analysis-core` | Linguistic-analysis term prompts, analysis-to-document synthesis, and Markov training modes for surface, normalized, lemma, and entity-aware tokens | Applications, text pipelines, prototyping |
| `moritzbrantner-text-model-runtime` | Text model runtime helper contracts | `moritzbrantner-video-analysis-core`, optional `moritzbrantner-model-runtime`/tokenizer/inference backends | Tokenization summaries, softmax helpers, text runtime request DTOs, non-executing local model helpers | Text model-backed crates, CLI model utilities, package UI runtime probes |
| `moritzbrantner-text-question-answering` | Question answering surface contracts | `moritzbrantner-text-model-runtime`, `moritzbrantner-video-analysis-core` | QA model presets, question/context request DTOs, answer span responses, deterministic lexical fallback answers | Applications adding local-first question answering over documents and transcripts |
| `moritzbrantner-text-transcripts` | Reusable transcript parsing and ASR command wrappers | `moritzbrantner-audio-analysis-core`, `moritzbrantner-audio-analysis-io`, `moritzbrantner-video-analysis-core`, `moritzbrantner-video-analysis-ingest`, `serde`, `serde_json`, `thiserror` | Transcript segment/result contracts, Whisper JSON/SRT/WebVTT/plain parsers, command transcribers, waveform-batch transcription bridge, text segment source adapter | Use cases, applications, text pipelines |
| `moritzbrantner-dense-data` | Generic dense point aggregation and clustering | `moritzbrantner-numbers-core`, `moritzbrantner-math-linear`, `moritzbrantner-math-statistics`, `moritzbrantner-video-analysis-core` | `DensePoint`, `DenseDataset`, weighted averages, per-dimension summaries, bounds, fixed-grid buckets, deterministic k-means clusters, covariance, and PCA helpers | Tables, graphs, charts, maps, media features, and analytics workflows |
| `moritzbrantner-geo-core` | Geospatial domain contracts and transforms | `moritzbrantner-video-analysis-core`, `serde` | Coordinates, bounding boxes, feature records, geometry transforms, distance and bounds utilities | Map views, location-aware reports, and future geospatial analytics workflows |
| `moritzbrantner-geo-io-geojson` | GeoJSON import/export boundary | `moritzbrantner-geo-core`, `geojson`, `serde` | GeoJSON parsing and serialization for `geo-core` geometry, feature, and collection types | File and API interchange without leaking wire-format types into algorithm crates |
| `moritzbrantner-geo-io-osm` | OpenStreetMap PBF import boundary | `moritzbrantner-geo-core`, `moritzbrantner-geo-io-geojson`, `osmpbfreader`, `regex`, optional `redb` | OSM PBF filtering, node coordinate indexing, way geometry resolution, area relation assembly, and conversion to `geo-core` feature collections | Local OSM extract workflows and map data ingestion without network fetch/cache concerns |
| `moritzbrantner-geo-clustering` | Geospatial clustering algorithms | `moritzbrantner-geo-core`, `serde` | Internal point and cluster types, viewport cluster queries | Map aggregation and analytics without exposing external GeoJSON versions |
| `moritzbrantner-geo-viz` | Geospatial visualization models | `moritzbrantner-geo-core`, `moritzbrantner-geo-io-geojson`, `moritzbrantner-geo-clustering`, `moritzbrantner-maps-kernels-core`, `rstar` | Viewport models, heat and flow features, map-oriented summaries, GeoJSON viewport output | Renderer adapters and map UI workflows |
| `moritzbrantner-vector-analysis-core` | Dense vector contracts and metrics | `moritzbrantner-video-analysis-core` | Finite vector validation, normalization, dot/cosine/L1/L2 metrics, means, summary stats | Search, recognition, clustering, analytics workflows |
| `moritzbrantner-vector-analysis-index` | Exact vector search and assignment | `moritzbrantner-vector-analysis-core`, `moritzbrantner-video-analysis-core`, `serde` | In-memory vector index, filtered search, metadata payloads, serializable vector records, search results, nearest-centroid assignment | Applications, prototypes, tests, small vector collections |
| `moritzbrantner-three-d-processing-core` | Generic 3D processing primitives | `moritzbrantner-video-analysis-core` | 3D vectors, points, bounds, transforms, quaternions, rigid transforms, line segments, rays, planes, spheres, point clouds, centroids, voxel downsampling, nearest-point lookup, and basic bounds/sphere/ray collision helpers | Mesh processing, applications, future 3D workflows |
| `moritzbrantner-three-d-processing-io` | 3D interchange formats | `moritzbrantner-three-d-processing-core`, `moritzbrantner-three-d-processing-mesh`, `moritzbrantner-video-analysis-core`, `serde_json`, `base64` | `OBJ`, `PLY`, and minimal embedded `.gltf` mesh/point-cloud I/O | Applications, CLI workflows, posture export |
| `moritzbrantner-three-d-processing-mesh` | Triangle mesh processing | `moritzbrantner-three-d-processing-core`, `moritzbrantner-video-analysis-core` | Mesh validation, topology, diagnostics, repair helpers, triangle normals, vertex normals, bounds, surface area, volume, transforms, smoothing, deterministic sampling | Applications and future 3D workflows |
| `moritzbrantner-video-analysis-core` | Canonical shared contracts, runtime DTOs, and pipelines | External utility crates only | Time/frame types, media samples, runtime diagnostics/capabilities/surface DTOs, detection traits/results, analyzer traits/results, observations, metrics, pipeline builders | All functional Rust crates and transport wrappers |
| `moritzbrantner-video-analysis-data` | Online stream normalization and aggregation | `moritzbrantner-numbers-core`, `moritzbrantner-video-analysis-core` | `DataRecord`, `DataPayload`, bucket configuration, bucket summaries, stream summaries | Use cases, reporting, UI JSON generation |
| `moritzbrantner-video-analysis-dataset` | Retained analysis dataset records | `moritzbrantner-video-analysis-core`, `moritzbrantner-video-analysis-posture`, `serde` | Serializable owned records for scenes, cuts, media metadata, observations, events, metrics, tracks, features, and structured 2D/3D pose records | Transform, feature, storage, analytics workflows |
| `moritzbrantner-video-analysis-transform` | Deterministic dataset transformations | `moritzbrantner-video-analysis-dataset` | Filtering, time windows, scene grouping, time/frame joins, dedupe, merge, numeric feature resampling | Feature extraction and applications |
| `moritzbrantner-video-analysis-features` | Reusable feature extraction over retained datasets | `moritzbrantner-video-analysis-core`, `moritzbrantner-video-analysis-dataset`, `moritzbrantner-video-analysis-transform` | Scene stats, label histograms, transcript stats, audio event stats, track summaries, vector means | Applications and downstream ML/analytics workflows |
| `moritzbrantner-video-analysis-storage` | Retained dataset persistence | `moritzbrantner-video-analysis-dataset`, `serde`, `serde_json`, `thiserror` | JSON/JSONL writers and readers plus dataset manifests | Applications and automation |
| `moritzbrantner-video-analysis-synthesis` | Deterministic inverse video frame/storyboard generation | `moritzbrantner-data-inversion-core`, `num-rational`, `moritzbrantner-video-analysis-core` | Frame synthesis specs, region outlines, observation storyboards, generated `OwnedVideoFrame` values | Applications visualizing analyzed observations as approximate frames |
| `moritzbrantner-video-analysis-detectors` | Scene detector implementations | `moritzbrantner-video-analysis-core` | `SceneDetector` implementations, scoring algorithms, composite detector contracts | CLI, use cases, applications |
| `moritzbrantner-video-analysis-editing` | Classic CPU media editing primitives | `moritzbrantner-video-analysis-core` | Frame crop, blur, grayscale, inversion, brightness/contrast, 3x3 filters, and `FrameEditor` chains | Applications, preprocessing workflows, future media export flows |
| `moritzbrantner-video-analysis-ingest` | Source abstraction layer | `moritzbrantner-video-analysis-core` | Media/source metadata, source traits, source-to-pipeline adapter helpers, text line source | FFmpeg crate, use cases, applications |
| `moritzbrantner-video-analysis-ffmpeg` | FFmpeg-backed media probing and decoding | `moritzbrantner-video-analysis-core`, `moritzbrantner-video-analysis-ingest` | FFmpeg video/audio sources, metadata, probe helpers, source options | CLI, use cases, applications |
| `moritzbrantner-model-runtime` | Model-specific specs, bundles, downloads, and validators | `moritzbrantner-jobs-core`, `moritzbrantner-video-analysis-core`, `hf-hub` | Model specs, sources, tasks, presets, bundle manifests, Hugging Face download/cache/store helpers, model metadata projection into generic artifact refs, and conformance helpers | Model-backed capability crates and CLI model commands |
| `moritzbrantner-runtime-onnx` | Domain-neutral ONNX Runtime session wrapper | optional `ort`/`ndarray`, `serde`, `thiserror` | Typed ONNX tensors, named inputs/outputs, session metadata, CPU session construction, and low-level run helpers | Task crates that own model-specific preprocessing and decoding |
| `moritzbrantner-jobs-core` | Reusable job state, progress, result envelopes, and generic artifacts | `serde`, `serde_json`, `sha2`, `moritzbrantner-video-analysis-core` | Job IDs, specs, status transitions, progress snapshots, event records, `OperationResult<T>`, `JobResult<T>`, artifact refs, memory/local stores, checksum validation, downloader/validator traits | Model materialization, asynchronous package operations, and artifact-producing workflows |
| `moritzbrantner-video-analysis-mvs` | Multi-view stereo contracts | `moritzbrantner-video-analysis-core`, `moritzbrantner-video-analysis-reconstruction`, `moritzbrantner-three-d-processing-core` | Depth maps, view-pair records, dense reconstruction options, point-cloud conversion helpers, and OpenCV MVS planning placeholder | Reconstruction, radiance, and 3D processing workflows |
| `moritzbrantner-video-analysis-tracking` | Object tracking over frame detections | `moritzbrantner-video-analysis-core` | `TrackedDetection`, `IouTracker`, tracking options, object-detection backend trait, analyzer adapter | Applications, use cases, model-backed detection pipelines |
| `moritzbrantner-video-analysis-posture` | Pose and posture estimation contracts | `moritzbrantner-video-analysis-core`, `moritzbrantner-three-d-processing-core` | 2D/3D keypoints, skeletons, pose estimates, stick figures, posture backend traits, analyzer adapter, joint angle helpers, smoothing/interpolation | Applications, use cases, model-backed posture workflows |
| `moritzbrantner-video-analysis-posture-io` | Posture interchange and preview export | `moritzbrantner-video-analysis-core`, `moritzbrantner-video-analysis-posture`, `moritzbrantner-three-d-processing-core`, `serde_json`, `base64` | COCO-style keypoint JSON, 3D stick-figure `.ply`, 3D stick-figure `.gltf` | CLI workflows, applications, dataset export |
| `moritzbrantner-video-analysis-recognition` | Reference-embedding identity matching | `moritzbrantner-video-analysis-core` | Reference libraries, normalized embeddings, recognition candidates/matches, temporal aggregation, video analyzer adapter | Applications, use cases, model-backed face/object recognition |
| `moritzbrantner-video-analysis-output` | Detection output writers | `moritzbrantner-video-analysis-core` | Scene CSV, stats CSV, simple HTML, JSON reports, EDL, FCP7 XML, FCPXML, OTIO JSON, qpfile markers, combined detection writers | CLI, applications |
| `moritzbrantner-video-analysis-split` | Scene-based media splitting | `moritzbrantner-video-analysis-core` | Split options, template variables, FFmpeg split function | CLI, applications |
| `moritzbrantner-video-analysis-radiance-fields` | Shared 3D geometry, camera, ray, and volume contracts | `moritzbrantner-video-analysis-core` | Vector/color/ray types, camera intrinsics/pose, radiance field trait, rendering/grid specs | Gaussian splatting, reconstruction, applications |
| `moritzbrantner-video-analysis-gaussian-splatting` | 3D Gaussian primitive projection and CPU compositing | `moritzbrantner-video-analysis-core`, `moritzbrantner-video-analysis-radiance-fields` | Gaussian primitives, projection config/results, splat rendering helpers | Applications and future 3D workflows |
| `moritzbrantner-video-analysis-radiance-io` | Radiance-field and 3DGS interchange formats | `moritzbrantner-video-analysis-core`, `moritzbrantner-video-analysis-radiance-fields`, `moritzbrantner-video-analysis-gaussian-splatting`, `moritzbrantner-video-analysis-reconstruction` | COLMAP text, Nerfstudio transforms, Gaussian splat PLY, preview PLY | Conversion tools and applications |
| `moritzbrantner-video-analysis-radiance-pipeline` | Radiance-field workflow composition | `moritzbrantner-video-analysis-core`, `moritzbrantner-video-analysis-radiance-fields`, `moritzbrantner-video-analysis-radiance-io`, `moritzbrantner-video-analysis-reconstruction` | Pipeline stage descriptors, reconstruction-to-radiance handoff records, preview artifact metadata | End-to-end radiance experiments and conversion tools |
| `moritzbrantner-video-analysis-reconstruction` | Sparse reconstruction and triangulation contracts | `moritzbrantner-video-analysis-core`, `moritzbrantner-video-analysis-radiance-fields` | Camera/image/point IDs, features, matches, tracks, sparse reconstruction, triangulation/projection helpers | Applications and future 3D workflows |
| `moritzbrantner-video-analysis-sfm` | Structure-from-motion workflow contracts and provider adapters | `moritzbrantner-video-analysis-core`, `moritzbrantner-video-analysis-reconstruction`, `moritzbrantner-video-analysis-radiance-fields`, `moritzbrantner-video-analysis-radiance-io` | SfM image/camera inputs, feature/match pipeline records, reconstruction summaries, backend trait adapters, COLMAP text baseline compatibility, native server-only COLMAP video reconstruction, Rust known-pose backend, and OpenCV SfM planning placeholder | Reconstruction, COLMAP/Rust provider workflows, and radiance handoff |
| `moritzbrantner-video-analysis-test-support` | Shared video workspace test helpers | `moritzbrantner-video-analysis-core`, `moritzbrantner-video-analysis-dataset` | Synthetic frames, timestamps, observations, fixture builders, assertion helpers | Video crate tests, integration smoke tests, and package surface checks |
| `moritzbrantner-video-analysis-cli` | `vanalyze` command-line composition | Core, detectors, FFmpeg, models, output, split | CLI commands, package catalog metadata, file outputs, and primitive JSON analysis reports | End users and automation |
| `moritzbrantner-video-analysis-use-cases` | Prototype runnable end-to-end workflows | Core, data, detectors, FFmpeg, ingest, models, audio/image helpers | `youtube-video`, `video-red-cars`, `audio-voice-analysis`, and `image-person-edit` workflow/report surfaces | End users, `@moritzbrantner/video-analysis-ui`, prototype web app |
| `@moritzbrantner/video-analysis-ui` | React/Tailwind views for analysis data | React peer deps and generated report/data shapes | TypeScript report types, package-surface workbench components, optional operation-group tabs, shared sample-video registry, component subpath exports, Tailwind content export | Web apps and report viewers |
| `@moritzbrantner/video-analysis-web` | Prototype app for local workflows, endpoints, and package UI | `@moritzbrantner/video-analysis-ui`, Vite, React | `/api/run-youtube-video`, `/api/workspace-architecture`, `/api/packages`, architecture and workflow pages | Developers exploring package behavior locally |

## Shared Sample Videos

The package-surface workbench exposes a shared video sample registry for video
package UIs. Small generated WebM clips are checked in under
`prototypes/web/video-analysis-web/public/samples/video/`. The COLMAP test
video is intentionally not checked in; create the ignored local
`test-video.mp4` with `bun run setup:colmap-video`. Shared sample metadata
includes both browser URLs and server-side workspace paths so client-only
operations can load `videoDataUrl` while native operations can receive
`videoPath`, `videoUrl`, and package-specific output directories.

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

## Audio Analysis Contracts

The `audio-analysis-*` crates build on the canonical `AudioFrame`,
`AudioBuffer`, `AudioAnalyzer`, and `AnalysisEvent` contracts from
`moritzbrantner-video-analysis-core`.

Audio package-surface operations return structured JSON values with `title`,
`operation`, `message`, `summary`, and `result` fields while preserving their
domain-specific top-level fields for compatibility. Workflow operations perform
in-memory analysis, processing, synthesis, or rendering. Plan, catalog, note
lookup, timestamp, model inventory, and command-preview operations are Debug UI
operations and must state when they do not scan files, decode media, write
outputs, or execute external tools.

- `moritzbrantner-audio-analysis-core` converts supported `AudioBuffer` formats into
  normalized `f32` samples, mixes interleaved channels to mono, applies common
  windows, iterates fixed-size analysis frames, and provides
  `StreamingFrameBuffer` for overlap-preserving windows across incoming chunks.
  `WindowFunction` and related window/frame-stride math are shared with
  `moritzbrantner-math-signal-core`.
- `moritzbrantner-audio-analysis-fourier` provides FFT spectra, STFT spectrogram frames,
  spectral centroid/bandwidth/rolloff/flatness features, and an
  `AudioAnalyzer` that emits dominant-frequency events.
- `moritzbrantner-audio-analysis-io` re-exports the shared audio ingest traits and FFmpeg
  source types behind audio-named `AudioInput`, `AudioInputOptions`, and
  `open_audio_input` conveniences. It owns pure `hound` WAV read/write helpers
  for deterministic clips and waveform batches; FFmpeg remains opt-in for
  non-WAV decode and probe paths. It also owns `decode_audio_to_waveform_batch`
  and `write_waveform_batch_as_wav` for bridging decoded audio into portable
  waveform contracts and file-based tools.
- `moritzbrantner-audio-analysis-pitch` estimates fundamental frequency with normalized
  autocorrelation and emits pitch events when confidence crosses the configured
  threshold.
- `moritzbrantner-audio-analysis-processing` owns frame-based audio transforms and source
  adapters. Built-in transforms include gain, hard clipping, mono conversion,
  DC blocking, biquad low/high/band/notch filters, and noise gates.
  Transformed frames are emitted as `OwnedAudioFrame` values with
  `AudioBuffer::F32` payloads in the first milestone.
- `moritzbrantner-audio-analysis-recognition` turns audio samples or frames into normalized
  spectral embeddings, stores multiple sample embeddings per reference, searches
  references by cosine similarity, and provides an `AudioAnalyzer` that emits
  `audio:recognized:<reference_id>:<label>` events over streaming windows. It
  also owns audio-facing generic transcription requests, imported transcript
  normalization, and metadata-only provider plans. Native Whisper execution and
  transcript parsing/formatting remain owned by `moritzbrantner-text-transcripts`.
- `moritzbrantner-audio-analysis-rhythm` detects onset events from energy changes, estimates
  BPM from onset intervals, and can emit both onset and tempo events.
- `moritzbrantner-audio-analysis-separation` owns deterministic Demucs/HTDemucs command
  previews and expected stem-path contracts for the package surface. The
  surface preview does not decode audio, run Demucs, or write stems; the opt-in
  execution surface requires `execute=true` and reports missing tools before
  spawning the external process.

Audio analysis crates should accept borrowed core audio frames or normalized
sample slices and should emit `AnalysisEvent` values for pipeline integration.
File writing and encoded audio sinks are deferred; the current processing
surface returns processed frames for callers to analyze, stream, or encode later.

## Image Analysis Contracts

The `image-analysis-*` crates provide still-image contracts and processing
helpers without requiring video timeline semantics.

- `moritzbrantner-image-analysis-core` owns `ImageView<'_>`, `OwnedImage`,
  `ImagePixelFormat`, image compacting, mean RGB, and luma histograms.
- `ImageView::from_video_frame` and `OwnedImage::from_video_frame` bridge core
  `VideoFrame<'_>` values into still-image workflows.
- `moritzbrantner-image-analysis-io` owns PNG/JPEG/WebP file loading and saving for
  `OwnedImage` buffers. Its runtime surface exposes format support, extension
  inference, and read/write planning without touching the filesystem.
- `moritzbrantner-image-analysis-processing` owns `ImageOperation`, `ImageProcessor`,
  `ImageRegion`, crop, nearest-neighbor resize, grayscale, invert, threshold,
  convolution, and sharpen helpers. New shared geometry and kernel entrypoints
  prefer `math-geometry-2d::RectU32` and `math-linear::Kernel2d` while keeping
  `ImageRegion` and `[f32; 9]` compatibility shims.
- `moritzbrantner-image-analysis-segmentation` owns still-image prompts, binary masks,
  segments, SAM presets, and segmentation backend contracts with explicit
  opt-in automatic mask generation helpers. Its runtime surface exposes SAM
  model metadata, prompt summaries, and imported binary mask summaries without
  running SAM.
- `moritzbrantner-image-analysis-detection` owns canonical still-image detections,
  mask-proposal adapters over segmentation backends, native color-blob
  detection for simple object workflows such as red-car detection, face
  detection DTOs, face detection presets, and face detector backend traits. Its
  runtime surface additionally exposes non-executing model metadata and
  imported box summaries.
- `moritzbrantner-image-analysis-synthesis` owns deterministic, non-AI image generation from
  colors, histograms, and regions. Its runtime surface returns summary
  statistics and inversion traces, not encoded image bytes.
- Image classification, embeddings, and captioning are owned by
  `moritzbrantner-image-analysis-classification`, `moritzbrantner-image-analysis-embeddings`, and
  `moritzbrantner-image-analysis-captioning`; their runtime surfaces expose catalog/schema and
  imported value validation, not fake inference.
- `moritzbrantner-image-analysis-ocr` owns OCR model presets, rich text layout contracts, and
  image/video-frame backend traits for model, command, or heuristic recognizers.
  Its runtime surface summarizes presets, requests, and imported OCR documents
  without recognizing images.
- `moritzbrantner-image-analysis-processing` owns image model preprocessing and
  batch tensor conversion. Image classification, captioning, detection, and
  embedding crates own their ONNX adapters and use `moritzbrantner-runtime-onnx`
  only for low-level session execution.
- `moritzbrantner-image-analysis-comfyui` owns ComfyUI workflow builders and a lightweight
  HTTP client/executor for AI image generation and manipulation.
  `ImageGenerationRequest` now prefers typed `ComfyModelRef` values for
  checkpoint and upscale model selection while keeping string builder shims for
  compatibility.

Image processing outputs are compact `OwnedImage` buffers. Image crates should
not own scene timing, CLI branching, or report serialization. Pure image crates
stay classical and memory-first; AI/runtime integrations live in dedicated
image model/runtime/orchestration crates.

## Text Analysis Contracts

The `text-*` crates provide reusable text processing separate from video use
cases and model adapters.

- `moritzbrantner-text-core` owns `TextDocument<'_>`, `OwnedTextDocument`,
  `TextStats`, `TextSpan`, `Token`, `Sentence`, `Paragraph`,
  `TextProcessingOptions`, `TextBoundaryOptions`, `WordSegment`,
  `GraphemeSpan`, `ScriptProfile`, whitespace normalization, word
  tokenization, span-aware tokenization, Unicode word/grapheme segmentation,
  script profiling, sentence/paragraph splitting, and detailed stats.
- `TextDocument::from_segment` and `OwnedTextDocument::from_segment` bridge core
  `TextSegment` and `OwnedTextSegment` values into text-only workflows.
- `moritzbrantner-text-lexical` owns `TermFrequency`, `TextFeatureSummary`,
  `StopWords`, `KeywordOptions`, `Keyword`, `NgramFrequency`,
  `ReadabilitySummary`, `StemOptions`, `ExtractiveSummaryOptions`,
  `SummarySentence`, `SentimentLexicon`, `SentimentSummary`, top terms,
  keyword extraction, lexical diversity, stemming, extractive summaries,
  lexicon sentiment, pattern detection, and character/token n-grams. It also provides
  `TextStatsAnalyzer`, `KeywordAnalyzer`, `ExtractiveSummaryAnalyzer`,
  `SentimentAnalyzer`, `EntityRuleAnalyzer`, and `PatternAnalyzer` for
  `TextPipeline`. Transcript-specific pipeline analyzers live in
  `moritzbrantner-text-transcripts`.
- `moritzbrantner-text-lexical` keeps `TfIdfCorpus` stable and adds `Bm25Corpus` for
  BM25 document ranking with duplicate-id rejection and empty-query handling.
  It now also exposes optional sparse term matrices and vectors backed by
  `moritzbrantner-math-sparse-data`.
- `moritzbrantner-text-linguistics` owns language detection, tokenization policy/alignment,
  local tokenizer loading, local BERT NER execution, lemmatization,
  POS/morphology, phrase chunks, dependency trees, named entities, rule
  entities, coreference, relations, events, discourse, topics, style, and
  `TextNlpPipeline`.
- `moritzbrantner-text-embeddings` keeps `HashedTextEmbedder` and `SemanticTextIndex`
  while adding `TextEmbeddingBackend` and `EmbeddingSearchIndex<E>`. Embedding
  APIs return `DenseVector` directly instead of encoding vectors into
  `AnalysisEvent` values, and can optionally emit sparse hashed embeddings
  backed by `moritzbrantner-math-sparse-data`. Optional native embedding runtimes now live
  here through `OnnxTextEmbedder` and `CandleTextEmbedder`; model acquisition
  uses `moritzbrantner-model-runtime`.
- `moritzbrantner-text-retrieval` owns `SearchDocument`, `DocumentChunk`, `RetrievalIndex`,
  `SearchQuery`, `SearchFilter`, `HybridConfig`, `SearchResult`, retrieval
  manifests, persisted chunk/vector JSONL snapshots, and index rehydration.
- `moritzbrantner-text-transcripts` owns `TranscriptFormat`, `TranscriptSegment`,
  `TranscriptSegmentContract`, `TranscriptionResult`, `TranscriptionContract`,
  `Transcriber`, `CommandTranscriber`, `WhisperCliTranscriber`,
  `transcribe_waveform_batch`, and `TranscriptSegmentSource`. It parses Whisper
  JSON, SRT, WebVTT, and plain line transcripts, converts transcript segments
  into `TextSegmentContract` and `OwnedTextSegment` values, and bridges waveform
  batches into the existing file-based transcription path. It also owns
  transcript contract normalization, strict validation, aggregate text fallback
  helpers, and native Whisper implementation details used by audio transcription
  orchestration and transcript-aware text analysis.
- `moritzbrantner-text-generation` owns deterministic Markov prediction and deterministic
  synthesis from weighted terms and text events. `moritzbrantner-text-generation-linguistics`
  owns the adapters that turn linguistic analyses into term prompts, generated
  documents, or Markov training inputs.

Deterministic text crates should emit deterministic features and label-based
`AnalysisEvent` values. Model-backed classification and embeddings are
separate but composable through `TextModelBackend`, `ModelTextAnalyzer`, and
`TextEmbeddingBackend`.

## Vector Analysis Contracts

The `vector-analysis-*` crates standardize dense vector handling for embedding,
recognition, search, and analytics workflows.

- `moritzbrantner-vector-analysis-core` owns `DenseVector`, `VectorMetric`, finite validation,
  L2 normalization, dot product, cosine similarity, Euclidean distance,
  Manhattan distance, mean vectors, and per-dimension stats.
- `moritzbrantner-vector-analysis-index` owns `VectorRecord`, `VectorSearchIndex`,
  `SearchConfig`, `SearchResult`, exact in-memory search, and nearest-centroid
  assignment.

Vector crates intentionally use exact CPU algorithms. Approximate nearest
neighbor backends can be added later behind separate implementation crates
without changing the core vector contracts.

## Numbers Contracts

`moritzbrantner-numbers-core` provides reusable scalar numeric building blocks for analytics
and reporting code that should not reimplement one-off min/max/mean, weighted
stats, quantiles, or histograms.

- `RunningStats` tracks total observations, finite/non-finite counts, weighted
  sums, mean, variance, and standard deviation.
- `NumberSummary` is the stable descriptive summary returned by scalar
  aggregators.
- `NumberRange` owns finite scalar bounds plus normalization, denormalization,
  and clamping helpers.
- `HistogramConfig`, `HistogramBin`, and `Histogram` expose deterministic
  fixed-width histograms over finite values.
- `quantile` and `quartiles` provide deterministic percentile interpolation
  over finite copied inputs.

## Analytical Math Crates Contracts

The `math-linear`, `math-statistics`, `math-sparse-data`, and
`finance-statistics` crates provide deterministic small/medium local analytical
helpers. They do not expose a separate numerical backend layer.

- `math-linear` least-squares is QR-based, requires full column rank, rejects
  non-finite inputs and invalid tolerances, and uses an automatic tolerance when
  callers pass `0.0`.
- `math-linear` SVD-class operations use pure Rust real-valued SVD by default,
  expose f64 compact diagnostics, and keep `faer`/`nalgebra` hidden behind
  reference and benchmark features.
- `math-statistics` matrix package operations default to f64. PCA uses
  centered-data SVD; OLS may use pseudoinverse for rank-deficient designs, while
  OLS diagnostics remain strict and require full rank.
- Ridge regression solves regularized normal equations deterministically and is
  not a full optimizer.
- `math-sparse-data` can summarize CSR matrices and convert or multiply sparse
  feature matrices into `math-linear::F32Matrix` for downstream dense workflows.
- `finance-statistics` portfolio risk attribution is historical
  covariance-based and assumes aligned, finite, equal-length asset return
  series.

## Finance Statistics Contracts

`moritzbrantner-finance-statistics` builds on `moritzbrantner-numbers-core` for finance-specific return
analytics without adding market-data or brokerage assumptions to the generic
math crates.

- Price-to-return helpers produce simple or log returns from strictly positive
  prices.
- Return series helpers expose sample/population variance, standard deviation,
  cumulative return, annualized return, annualized volatility, Sharpe, Sortino,
  beta/alpha, tracking error, and information ratio.
- Risk helpers expose maximum drawdown and historical VaR/CVaR as positive loss
  values.
- Portfolio attribution helpers expose historical covariance, variance, risk
  contribution, return contribution, and turnover for validated weights.
- Rolling helpers expose fixed-window mean, standard deviation, and
  correlation.

## Dense Data Contracts

`moritzbrantner-dense-data` provides generic dense numeric point processing for UI and media
workflows that need the same aggregation shape across tables, graphs, charts,
maps, and feature-derived media timelines.

- `DenseDataset` keeps its summary, bounds, bucket, and k-means APIs while now
  exposing matrix, covariance, and PCA helpers built on `moritzbrantner-math-linear` and
  `moritzbrantner-math-statistics`.

- `DensePoint` stores finite coordinates, a positive weight, an optional scalar
  value, and an optional id.
- `DenseDataset` retains dimension-consistent points and exposes averages,
  summaries, bounds, buckets, and k-means clustering.
- `DenseAverages` reports weighted coordinate means and optional weighted value
  means.
- `DenseSummary` exposes weighted per-dimension `NumberSummary` values, optional
  weighted scalar value stats, total weight, and bounds.
- `BucketGrid`, `BucketKey`, and `DenseBucket` group points into deterministic
  fixed-width coordinate buckets.
- `KMeansConfig`, `DenseCluster`, and `ClusterResult` expose deterministic CPU
  k-means clustering with stable point indices.

Dense data points intentionally do not prescribe axis semantics. Callers should
map table columns, graph/layout coordinates, longitude/latitude, embedding
dimensions, or media time/features into coordinates at the boundary.

## Inversion And Synthesis Contracts

The synthesis crates cover inverse directions where analysis has discarded
detail. They should expose that loss explicitly instead of presenting generated
data as recovered source material.

- `moritzbrantner-data-inversion-core` owns `InformationFidelity`, `InversionMethod`,
  `InversionTrace`, and `Generated<T>`. Synthesis crates should attach traces
  that identify source and target types, confidence, assumptions, and fields
  that were preserved, inferred, interpolated, templated, or defaulted. Its
  runtime surface validates confidence, compares fidelity, and builds trace
  summaries from JSON inputs.
- `moritzbrantner-audio-analysis-synthesis` turns tone timelines and supported
  `AnalysisEvent` labels such as pitch and onset events into `OwnedAudioFrame`
  values, and can render deterministic click tracks from BPM or explicit beat
  positions. It uses deterministic analytic waveforms and records that samples
  are interpolated from symbolic data.
- `moritzbrantner-image-analysis-synthesis` turns colors, color stops, luma histograms, and
  regions into `OwnedImage` buffers. Histogram and region layouts are
  deterministic approximations because the original spatial detail is not
  recoverable.
- `moritzbrantner-text-generation` turns weighted terms or analyzer events into
  `OwnedTextDocument` and `OwnedTextSegment` values using deterministic
  templates. It preserves term prominence but treats syntax and term
  relationships as inferred.
- `moritzbrantner-video-analysis-synthesis` turns frame specs or observations into
  `OwnedVideoFrame` storyboards. It preserves frame positions and regions when
  available, while labels, missing regions, and pixels are heuristic visual
  encodings.

## Ingest Contracts

`moritzbrantner-video-analysis-ingest` is the source abstraction layer. It lets source
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

`moritzbrantner-video-analysis-ffmpeg` is an implementation crate for FFmpeg-backed media
probing and decoding.

It exposes:

- `FfmpegVideoSource`, implementing core/ingest video source contracts.
- `FfmpegAudioSource`, implementing ingest audio source contracts.
- `VideoMetadata`, with input, optional path, mode, dimensions, frame rate, and
  optional duration.
- `AudioMetadata`, with input, optional path, mode, sample rate, channels, and
  optional duration.
- `FfmpegRuntimeBackend` and `FfmpegRuntimeOptions`, selecting command-backed
  or native-backed runtime paths.
- `FfmpegSourceOptions`, including source mode, realtime behavior, extra input
  args, and runtime options.
- `FfmpegAudioSourceOptions`, including audio chunk size, extra input args, and
  runtime options.
- `probe`, `probe_input`, `probe_audio`, and `probe_audio_input` helpers.
- `is_ffmpeg_available` and `is_ffprobe_available` probes.

FFmpeg is responsible for media probing, decoding, and conversion. The command
runtime remains the compatibility default; `ffmpeg-native` exposes native
runtime selection and `ffmpeg-next-bindings` is reserved for system FFmpeg
probing when development packages are installed. Downstream packages should
consume only core and ingest contracts such as `OwnedVideoFrame`,
`OwnedAudioFrame`, `VideoFrameSource`, and `AudioFrameSource`.

Generated media fixture helpers are behind the `test-utils` feature. Opt-in
decode coverage is available with:

```bash
cargo test -p moritzbrantner-video-analysis-ffmpeg --features ffmpeg-tests
```

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

## Retained Dataset Processing Contracts

`video-analysis-dataset` is the owned, serializable record layer for workflows
that need to retain analysis outputs beyond online bucket summaries.

Key contracts:

- `AnalysisDataset` stores `DatasetMetadata` plus ordered `DatasetRecord`
  values.
- `DatasetRecord` can carry video frame metadata, audio frame metadata, text
  segments, scenes, cuts, observations, audio/text events, detector metrics,
  extracted features, and track summaries.
- Serializable mirror types such as `TimestampRecord`, `FramePositionRecord`,
  `BoundingBoxRecord`, `PixelFormatRecord`, `AudioSampleFormatRecord`, and
  `ObservationKindRecord` preserve core runtime values without requiring core
  types to implement serde.
- `FeatureRecord` stores named values with optional timestamp, frame, scene,
  track, scope, and string attributes.
- `FeatureValue` supports numbers, integers, booleans, text, vectors, and string
  histograms.

Dataset records intentionally do not retain raw video or audio payload bytes.
Frame and audio records store dimensions, timing, sample format, counts, and
estimated byte size only.

`video-analysis-transform` operates on retained dataset records:

- `filter_records` and `filter_dataset` retain matching records.
- `window_by_time` groups timestamped records into fixed-duration windows.
- `group_by_scene` attaches records to scene intervals by explicit
  `scene_index` first, then by frame index or timestamp bounds.
- `join_by_time` and `join_by_frame` produce paired records within configured
  tolerances.
- `dedupe_records`, `dedupe_by`, and `merge_sorted_by_time` support basic
  dataset cleanup and composition.
- `resample_numeric_features` aggregates numeric feature records by fixed time
  interval.

`video-analysis-features` emits `FeatureRecord` values from retained datasets.
Built-in extractors cover scene statistics, observation label histograms,
transcript counts, audio event summaries, track summaries, and vector means.
`FeaturePipeline` composes multiple extractors.

`video-analysis-storage` persists retained datasets:

- JSON dataset files store the full `AnalysisDataset`.
- JSONL files store one serialized `DatasetRecord` per line.
- Dataset directories contain `records.jsonl` plus `manifest.json`.
- `DatasetManifest` records schema version, optional dataset name, total record
  count, per-kind record counts, file entries, and attributes.
- Empty JSONL lines are ignored. Malformed JSONL reports the failing line.

## Model Contracts

`model-runtime` separates generic model acquisition, bundle manifests, preset
metadata, and runtime conformance helpers from capability-specific execution.
Video prediction normalization and external command backend contracts live in
`video-analysis-recognition`.

Model acquisition and identity:

- `ModelTask`
- `HuggingFaceModelSpec`
- `ModelPreset`
- `DownloadedModel`
- `HuggingFaceDownloader`
- `ModelBundleStore`
- `ModelBundle`
- `ModelBundleManifest`
- `ModelBundleFile`

`HuggingFaceDownloader` remains the low-level cache downloader. `ModelBundleStore`
materializes downloaded files into a stable bundle directory with a
`manifest.json`; `ModelBundle` can convert that manifest back to `DownloadedModel`
for compatibility with external model execution.

The `model-runtime` surface exposes preset summaries, spec validation, and
bundle manifest plans only. Surface operations must not contact Hugging Face,
download files, or materialize bundle directories.

Text model presets include ONNX-friendly Hugging Face repos:

- `XenovaDistilbertSst2Onnx` requests `config.json`, `tokenizer.json`,
  `tokenizer_config.json`, and the first available ONNX file from
  `onnx/model.onnx`, `onnx/model_quantized.onnx`, or `onnx/model_int8.onnx`.
- `XenovaMiniLmL6V2Onnx` requests `config.json`, `tokenizer.json`,
  `tokenizer_config.json`, and the first available ONNX file from
  `onnx/model.onnx` or `onnx/model_quantized.onnx`.

`video-analysis-recognition` owns native video object-detection adaptation by
composing `image-analysis-detection` ONNX adapters. `video-analysis-posture`
owns pose ONNX options and fake-runner seams. `runtime-onnx/onnxruntime` gates
the optional `ort` dependency and executes DETR/YOLOS-style ONNX sessions that
return logits plus center-format boxes. Deterministic tests inject fake runners
so normal workspace checks do not download or execute model artifacts.

## Recognition Contracts

`video-analysis-recognition` implements reference-based identity recognition for
face, object, scene, or custom candidates.

Key contracts:

- `Embedding` stores finite, non-empty, L2-normalized vectors.
- `ReferenceIdentity` groups one known identity label with one or more reference
  embeddings and optional string attributes.
- `ReferenceLibrary` stores identities with one shared embedding dimensionality
  and performs exact cosine-similarity search.
- `MatchOptions` configures score thresholds and result limits.
- `RecognitionCandidate` carries the per-frame candidate embedding plus kind,
  optional region, detector label, detector score, track id, and attributes.
- `RecognitionBackend` is the adapter point for face/object detector plus
  embedding model implementations.
- `RecognitionVideoAnalyzer` implements core `VideoAnalyzer`, matches backend
  candidates against the library, and emits core `Observation` records.
- `TemporalRecognitionOptions` and `TemporalRecognitionAggregator` can require
  repeated hits on a stable track before emitting an identity observation.

Compatibility rule: recognition backends should do model-specific detection,
alignment/cropping, tracking, and embedding extraction. The recognition package
should own identity references, vector matching, thresholds, temporal evidence,
and conversion into core observations.

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
- `PersistentExternalCommandModel`

Vision backends return raw predictions for a `VideoFrame<'_>`. Text backends
return raw predictions for a `TextSegment<'_>`. The model crate normalizes those
predictions into core `Observation` values for video and core `AnalysisEvent`
values for text.

### External Command JSON Protocol

`ExternalCommandModel` starts an executable, writes one JSON request to stdin,
and expects one JSON response on stdout.

`PersistentExternalCommandModel` uses the same request and response objects over
newline-delimited JSON. The child process is started once, receives one compact
JSON request per line on stdin, and must return one compact JSON response per
line on stdout.

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

## ComfyUI Contracts

The `comfyui-data`, `comfyui-latents`, and `comfyui-models` crates are
standalone
interoperability packages for ComfyUI data that applications may need to read or
write.

The checked-in type/ownership matrix lives in
[`COMFYUI_TYPE_MATRIX.md`](COMFYUI_TYPE_MATRIX.md).

`comfyui-data` exposes:

- `ComfyWorkflow`, `WorkflowNode`, `WorkflowInput`, `WorkflowOutput`,
  `WorkflowLink`, and `WorkflowGroup` for workflow JSON files saved by ComfyUI.
- `WorkflowNodeId`, which accepts numeric and string node ids.
- `ComfyWorkflow::validate`, which checks duplicate node/link ids and missing
  link references.
- `ComfySocketType` plus `WorkflowTypeInventory` for normalized socket-type
  inventories across workflow inputs, outputs, and links.
- `ConditioningItem` and `ConditioningBatch` for minimal tensor-backed
  conditioning payloads with validated `[T,C]` embeddings and optional pooled
  `[C]` embeddings.
- `PromptGraph`, `PromptNode`, `PromptLink`, `prompt_link`, and
  `parse_prompt_link` for ComfyUI API prompt graphs.

`comfyui-latents` exposes:

- `LatentBatch` for validated rank-4 latent tensors with optional latent masks.
- `LatentMask` for validated latent-mask tensors, compatibility checks, and
  conversion from full-resolution image masks via 8x8 max pooling.
- `LatentImageSize` for ComfyUI-style 1/8 latent-to-image size conversions.

`comfyui-models` exposes:

- `ComfyModelKind`, including ComfyUI folder keys such as `checkpoints`,
  `loras`, `vae`, `text_encoders`, `diffusion_models`, `clip_vision`,
  `controlnet`, `upscale_models`, `audio_encoders`, and legacy aliases such as
  `clip` and `unet`.
- `ComfyModelRole` and `ComfyModelRef` for stable runtime-facing references to
  typed ComfyUI model assets.
- `ComfyModelRoot` and `ComfyModelAsset` for scanning typed model folders.
- `ExtraModelPathsConfig` and `ExtraModelPathSection` for generating
  `extra_model_paths.yaml` style configuration.

ComfyUI workflow files are JSON graph documents. ComfyUI model files usually
live under typed folders below `ComfyUI/models/`, and extra search paths are
configured with `extra_model_paths.yaml` for manual/portable installs or
`extra_models_config.yaml` for ComfyUI Desktop.

## Tracking Contracts

`video-analysis-tracking` owns lightweight temporal association for object-like
video detections. It does not run detectors itself; detector/model integrations
feed it per-frame boxes through a backend trait.

Key contracts:

- `TrackedDetection` carries a candidate kind, bounding box, optional label,
  score, track hint, and string attributes.
- `TrackingOptions` configures IoU association, score filtering, and how long
  stale tracks remain active.
- `IouTracker` performs deterministic greedy association by track hint first,
  then by best IoU among compatible labels and kinds.
- `ObjectTrack` stores stable track id, current region, label, score, first and
  last frame positions, age, missed-frame count, and attributes.
- `ObjectDetectionBackend` adapts a detector or model into per-frame tracked
  detections.
- `ObjectTrackingAnalyzer` implements core `VideoAnalyzer` and emits object
  `Observation` records with `track_id` set.

Tracking should consume normalized detections from model or heuristic packages.
It should not own image decoding, model execution, identity recognition, or
media output.

## Posture Contracts

`video-analysis-posture` standardizes pose/keypoint data and analyzer output for
posture-estimation backends.

Key contracts:

- `Keypoint` stores a named x/y coordinate plus optional score and visibility.
- `Keypoint3d` stores a named 3D point plus optional score and visibility.
- `KeypointSpace` identifies pixel-space or normalized coordinates.
- `Skeleton` and `SkeletonEdge` describe expected topology, including a COCO-17
  human skeleton preset.
- `PoseEstimate` groups one pose id, label, score, optional region, keypoints,
  and string attributes.
- `Pose3dEstimate` groups one 3D pose id, label, score, keypoints, and string
  attributes.
- `StickFigure3d` binds a skeleton to 3D joints and produces line segments for
  preview/export.
- `PoseSequence<T>` stores deterministic pose windows for lifting and
  smoothing.
- `PostureOptions` configures keypoint space, score filters, and inferred pose
  regions.
- `PostureBackend` adapts a pose-estimation model into `PoseEstimate` values.
- `PostureLiftBackend` adapts 2D pose windows into `Pose3dEstimate` values.
- `PostureAnalyzer` implements core `VideoAnalyzer` and emits custom
  `posture` observations.
- `joint_angle_degrees` and `joint_angle_3d_degrees` compute simple
  three-keypoint joint angles for posture feature extraction.
- `bone_lengths`, `normalize_pose3d`, `interpolate_missing_joints`, and
  `smooth_pose_sequence` provide classic CPU posture processing helpers.

Posture observations use core `Observation` fields for timestamp, frame, score,
track id, and region. Keypoint payloads are carried in string attributes until a
shared structured report format is introduced. `video-analysis-dataset` now
also stores owned 2D and 3D pose records for retained workflows.

## Media Editing Contracts

`video-analysis-editing` provides deterministic CPU frame transforms over core
`VideoFrame<'_>` inputs and `OwnedVideoFrame` outputs.

Key contracts:

- `FrameEdit` enumerates crop, box blur, grayscale, inversion,
  brightness/contrast, and 3x3 convolution filters.
- `FrameEditor` stores an ordered edit chain and applies it to one frame.
- `crop_frame`, `box_blur_frame`, `grayscale_frame`, `invert_frame`,
  `brightness_contrast_frame`, and `filter_3x3_frame` expose individual
  operations.
- `sharpen_frame` and `edge_detect_frame` are named 3x3 filter presets.

Editing functions preserve frame position and pixel format. Outputs are compact
packed RGB/BGR buffers, even when the source stride contains padding. Editing
does not own timeline semantics, audio editing, FFmpeg command execution, or
file writing.

## Output And Split Contracts

`video-analysis-output` serializes detection results. It consumes only core
contracts:

- `write_scene_list_csv` writes scene rows from `&[Scene]`.
- `write_stats_csv` writes metric rows from `&MetricsStore`.
- `write_scene_list_html` writes a simple HTML scene table from `&[Scene]`.
- `write_detection_result_json` writes a JSON detection snapshot from
  `&DetectionResult`.
- `write_detection_outputs` writes scenes and optional stats from
  `&DetectionResult`.

`video-analysis-split` creates scene clips from original media:

- `SplitOptions` controls output directory, filename template, optional video
  name, and FFmpeg args.
- `SplitJob` describes one planned clip write.
- `SplitPlan` contains all jobs for one input media file.
- `build_split_plan` expands scene metadata into testable split jobs without
  invoking FFmpeg.
- `DEFAULT_TEMPLATE` is `$VIDEO_NAME-Scene-$SCENE_NUMBER.mp4`.
- `split_video_ffmpeg` accepts the original media path, `&[Scene]`, and
  `SplitOptions`.

Core scene frame indices are inclusive. Split jobs treat the scene end
timestamp as the exclusive media trim endpoint, so FFmpeg `-t` receives
`end_seconds - start_seconds`.

Output and split packages do not own detection, source construction, detector
selection, or CLI branching.

## 3D Scene Contracts

The workspace has two 3D layers. Generic processing crates use
`three-d-processing-*` types for point clouds and triangle meshes. Video-driven
neural rendering and reconstruction crates continue to interoperate through
`video-analysis-radiance-fields` geometry, camera, ray, and color primitives.

`three-d-processing-core` exposes:

- `Vector3`
- `Point3`
- `Bounds3`
- `Transform3`
- `Quaternion`
- `RigidTransform3`
- `LineSegment3`
- `PointCloud`
- Point distance, closest-point, ray/surface intersection, rigid-transform,
  voxel-downsampling, and center/scale helpers.

`three-d-processing-io` exposes:

- `read_mesh` / `write_mesh`
- `read_obj_mesh` / `write_obj_mesh`
- `read_ply_mesh` / `write_ply_mesh`
- `read_ply_point_cloud` / `write_ply_point_cloud`
- `read_gltf_mesh` / `write_gltf_mesh`

`three-d-processing-mesh` exposes:

- `Edge`
- `Triangle`
- `Mesh`
- `MeshTopology`
- Triangle normal, triangle area, triangle centroid, barycentric coordinates,
  surface area/centroid, face/vertex normal helpers.
- Connected-component, manifold/watertight, volume, transform, merge,
  deterministic surface sampling, closest-point/ray-intersection queries, and
  Laplacian smoothing helpers.

`video-analysis-radiance-fields` exposes:

- `Vec2`
- `Vec3`
- `ColorRgb`
- `Ray`
- `CameraIntrinsics`
- `CameraModel`
- `CameraDistortion`
- `CameraPose`
- `CameraView`
- `CameraViewSet`
- `CoordinateSystem`
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
- `SphericalHarmonicsRgb`
- `GaussianSplat3d`
- `GaussianSplatScene`
- `GaussianSceneStats`
- `SceneTransform3`
- Projection helpers such as `project_gaussian` and `project_scene`.
- Rendering helpers such as `gaussian_weight`, `composite_splats_at_pixel`,
  and `render_projected_splats`.
- Splat-scene validation, stats, transforms, opacity/bounds filtering, preview
  color conversion, and deterministic stride downsampling.

`video-analysis-radiance-io` exposes:

- `ColmapDataset`, `ColmapCamera`, `ColmapImage`, and `ColmapPoint3d`.
- `read_colmap_text_dir` and `write_colmap_text_dir` for `cameras.txt`,
  `images.txt`, and `points3D.txt`.
- `colmap_to_view_set` and `colmap_to_sparse_reconstruction`.
- `NerfstudioTransforms`, `NerfstudioFrame`,
  `read_nerfstudio_transforms`, `write_nerfstudio_transforms`, and
  `transforms_to_view_set`.
- `read_gaussian_splat_ply`, `write_gaussian_splat_ply`, and
  `write_preview_point_cloud_ply`.

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

Neural rendering and reconstruction crates should share `CameraIntrinsics`,
`CameraPose`, `CameraViewSet`, `Vec2`, `Vec3`, `ColorRgb`, and `Ray` instead of
introducing incompatible camera or geometry types. Generic 3D processing crates
should use the `three-d-processing-*` contracts unless they explicitly need
camera/ray semantics.

The first radiance-field/3DGS layer is interop-oriented. It does not implement
native NeRF/3DGS training and does not provide a production GPU renderer.
Distorted COLMAP camera models are parsed and preserved as raw camera data, but
direct conversion to `CameraIntrinsics` is limited to undistorted
`SIMPLE_PINHOLE` and `PINHOLE` cameras.

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
  files into a local bundle directory with a manifest.
- `vanalyze models inspect`: loads a model bundle by manifest path or
  name/revision and prints its identity plus materialized files.
- `vanalyze models run`: loads a raw RGB/BGR frame and emits JSON model
  predictions. ONNX execution requires building the CLI with `onnxruntime`;
  the lighter `onnx` feature only enables bundle validation and the command
  surface.

`video-analysis-use-cases` exposes runnable workflows. Current workflows are:

- `video-analysis-use-cases youtube-video`
- `video-analysis-use-cases video-red-cars`
- `video-analysis-use-cases audio-voice-analysis`
- `video-analysis-use-cases image-person-edit`

The YouTube video workflow accepts a URL or local video input. It can use
optional external transcriber, object, OCR, and text model commands. Its primary
interoperability output is a JSON report consumed by applications and
`@moritzbrantner/video-analysis-ui`.

The reusable Rust API is exposed through
`video_analysis_use_cases::youtube`:

- `YOUTUBE_VIDEO_USE_CASE`, currently `"youtube-video"`.
- `YoutubeVideoRequest`, the library request equivalent of the CLI flags.
- `run_youtube_video`, which returns a `YoutubeVideoReport` without requiring
  clap or writing files itself.
- `write_youtube_video_report`, which writes the report JSON for CLI and
  automation use.
- `VIDEO_RED_CARS_USE_CASE`, currently `"video-red-cars"`, plus
  `VideoRedCarsRequest`, `VideoRedCarsRunRequest`, `VideoRedCarsReport`,
  `run_video_red_cars`, and `write_video_red_cars_report`.
- `AUDIO_VOICE_ANALYSIS_USE_CASE`, currently `"audio-voice-analysis"`, plus
  `AudioVoiceAnalysisRequest`, `AudioVoiceAnalysisRunRequest`,
  `AudioVoiceAnalysisReport`, `run_audio_voice_analysis`, and
  `write_audio_voice_analysis_report`.
- `IMAGE_PERSON_EDIT_USE_CASE`, currently `"image-person-edit"`, plus
  `ImagePersonEditRequest`, `ImagePersonEditRunRequest`,
  `ImagePersonEditReport`, `run_image_person_edit`, and
  `write_image_person_edit_report`.

## Rust-To-UI JSON Report Contracts

The use-case JSON report is the main contract between Rust output and React
components. The serialized Rust report structs in
`prototypes/rust/video-analysis-use-cases/src/main.rs` align with the TypeScript
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

- `use_case` is canonically `"youtube-video"`.
- The additional workflow identifiers are `"video-red-cars"`,
  `"audio-voice-analysis"`, and `"image-person-edit"`.
- Rust numeric fields such as `u64`, `u32`, `f32`, and `f64` become TypeScript
  `number`.
- Rust `Option<T>` appears as optional and/or nullable UI fields where currently
  modeled.
- Report fields consumed by UI components should be preserved or intentionally
  versioned when changed.

## Facade And Package Export Contracts

The Rust root crate `video-analysis` is a convenience facade. It re-exports all
core items, detector items, and package modules for audio, image, text, vector,
3D processing, data, FFmpeg, ingest, models, output, radiance fields, Gaussian
splatting, reconstruction, and split. It does not expose CLI or use-case
binaries as library modules.

The UI package exposes these subpaths:

- `@moritzbrantner/video-analysis-ui`
- `@moritzbrantner/video-analysis-ui/core`
- `@moritzbrantner/video-analysis-ui/data`
- `@moritzbrantner/video-analysis-ui/cli`
- `@moritzbrantner/video-analysis-ui/detectors`
- `@moritzbrantner/video-analysis-ui/ffmpeg`
- `@moritzbrantner/video-analysis-ui/ingest`
- `@moritzbrantner/video-analysis-ui/models`
- `@moritzbrantner/video-analysis-ui/output`
- `@moritzbrantner/video-analysis-ui/split`
- `@moritzbrantner/video-analysis-ui/use-cases`
- `@moritzbrantner/video-analysis-ui/tailwind-content`

The root UI export re-exports shared types and all component packs. Subpath
exports should remain aligned with package boundaries so applications can import
only the views they need.

## Dependency Rules

Allowed internal dependencies:

- `comfyui-data`: `serde`, `serde_json`, `tensor-data`, `thiserror`.
- `comfyui-models`: `serde`, `thiserror`.
- `audio-analysis-core` -> `video-analysis-core`.
- `audio-analysis-fourier` -> `audio-analysis-core`,
  `video-analysis-core`.
- `audio-analysis-io` -> `audio-analysis-core`, `video-analysis-core`,
  `video-analysis-ingest`, `video-analysis-ffmpeg`, `hound`.
- `audio-analysis-pitch` -> `audio-analysis-core`,
  `video-analysis-core`.
- `audio-analysis-processing` -> `audio-analysis-core`,
  `video-analysis-core`, `video-analysis-ingest`.
- `audio-analysis-recognition` -> `audio-analysis-core`,
  `audio-analysis-fourier`, `video-analysis-core`.
- `audio-analysis-rhythm` -> `audio-analysis-core`,
  `video-analysis-core`.
- `audio-analysis-separation` -> `video-analysis-core`.
- `image-analysis-core` -> `video-analysis-core`.
- `image-analysis-processing` -> `image-analysis-core`,
  `video-analysis-core`.
- `text-analysis` -> `text-core`, `text-lexical`,
  `text-linguistics`, `text-embeddings`, `text-retrieval`,
  `video-analysis-core`.
- `text-core` -> `video-analysis-core`,
  `unicode-normalization`.
- `text-lexical` -> `text-core`,
  `math-sparse-data`, `video-analysis-core`.
- `text-linguistics` -> `text-core`, `text-lexical`,
  `text-transcripts`, `video-analysis-core`, `model-runtime`.
- `text-embeddings` -> `text-core`, `text-lexical`,
  `math-sparse-data`, `vector-analysis-core`, `vector-analysis-index`,
  `video-analysis-core`.
- `text-retrieval` -> `text-core`, `text-lexical`,
  `text-embeddings`, `vector-analysis-index`, `video-analysis-core`,
  `serde`, `serde_json`, `thiserror`.
- `text-generation` -> `data-inversion-core`, `text-core`,
  `video-analysis-core`.
- `text-generation-linguistics` -> `data-inversion-core`, `text-core`,
  `text-generation`, `text-linguistics`, `video-analysis-core`.
- `text-transcripts` -> `audio-analysis-core`,
  `audio-analysis-io`, `video-analysis-core`, `video-analysis-ingest`,
  `serde`, `serde_json`, `thiserror`.
- `vector-analysis-core` -> `video-analysis-core`.
- `vector-analysis-index` -> `vector-analysis-core`,
  `video-analysis-core`.
- `three-d-processing-core` -> `video-analysis-core`.
- `three-d-processing-mesh` -> `three-d-processing-core`,
  `video-analysis-core`.
- `video-analysis-core`: external utility crates only.
- `video-analysis-data` -> `video-analysis-core`.
- `video-analysis-dataset` -> `video-analysis-core`.
- `video-analysis-transform` -> `video-analysis-dataset`.
- `video-analysis-features` -> `video-analysis-core`,
  `video-analysis-dataset`, `video-analysis-transform`.
- `video-analysis-storage` -> `video-analysis-dataset`.
- `video-analysis-detectors` -> `video-analysis-core`.
- `video-analysis-ingest` -> `video-analysis-core`.
- `video-analysis-ffmpeg` -> `video-analysis-core`,
  `video-analysis-ingest`.
- `video-analysis-recognition` -> `video-analysis-core`, `model-runtime`,
  `video-analysis-posture`.
- `video-analysis-output` -> `video-analysis-core`.
- `video-analysis-split` -> `video-analysis-core`.
- `video-analysis-radiance-fields` -> `video-analysis-core`.
- `video-analysis-gaussian-splatting` -> `video-analysis-core`,
  `video-analysis-radiance-fields`.
- `video-analysis-radiance-io` -> `video-analysis-core`,
  `video-analysis-radiance-fields`, `video-analysis-gaussian-splatting`,
  `video-analysis-reconstruction`, `serde`, `serde_json`, `thiserror`.
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
- `@moritzbrantner/video-analysis-ui` consumes generated data/report shapes and should not
  require Rust runtime packages.

## Compatibility Checklist

For new packages:

- Use core time, sample, result, detection, observation, and event types where
  possible.
- Keep standalone integration packages, such as `comfyui-*`, free of
  `video-analysis-*` dependencies unless they directly adapt core video/audio
  contracts.
- Add new media sources through `video-analysis-ingest` traits.
- Add new scene detectors through `SceneDetector`.
- Add video/audio/text enrichment through `VideoAnalyzer`, `AudioAnalyzer`, or
  `TextAnalyzer`.
- Add model integrations through `VisionModelBackend`, `TextModelBackend`, or
  the `ExternalCommandModel` JSON protocol.
- Add UI consumers by extending explicit TypeScript report types and keeping
  them aligned with Rust serialized reports.
- If the package requires external installables (Python environments, models,
  native CLIs, datasets), add idempotent `scripts/setup_*.sh` and
  `scripts/check_*.sh` entry points. Setup must be re-runnable and repair only
  missing/invalid state; check must not install.

For changes to existing packages:

- Update this document when changing shared traits, serialized report fields,
  CLI output files, file formats, or package exports.
- Preserve optional fields where possible; otherwise document breaking changes.
- Keep dependency direction consistent with the rules above.
- Prefer core contracts over package-specific duplicates.
