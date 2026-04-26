# API Contracts

This document describes the inter-package contracts that let the Rust crates and
the `@video-analysis/ui` package work together. It is intentionally not an
exhaustive rustdoc inventory. It focuses on shared types, traits, serialized
formats, file formats, package exports, and dependency boundaries.

`video-analysis-core` owns the canonical runtime contracts for time, media
samples, scene detection, metrics, observations, analyzers, and pipelines. Other
crates should compose around those contracts instead of defining parallel types.

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
| `video-analysis` | Root facade crate | Library crates except CLI and use cases | Re-exports core items, detector items, and package modules | Applications that want one import surface |
| `comfyui-data` | ComfyUI workflow and prompt graph data contracts | `serde`, `serde_json` | Workflow JSON nodes, links, groups, validation helpers, API prompt nodes and links | Applications importing, validating, or emitting ComfyUI graphs |
| `comfyui-models` | ComfyUI model folder and inventory contracts | `serde`, `thiserror` | Core model folder keys, default relative paths, inventory scanning, extra model paths YAML generation | Applications managing shared ComfyUI model libraries |
| `data-inversion-core` | Shared lossy inverse-conversion metadata | `video-analysis-core` | `InformationFidelity`, `InversionMethod`, `InversionTrace`, generated value wrappers | Synthesis crates and applications that need explicit interpolation/assumption metadata |
| `numbers-core` | Shared scalar numeric summaries and ranges | `video-analysis-core` | Running stats, weighted summaries, quantiles, histograms, numeric range helpers | `dense-data`, `video-analysis-data`, analytics workflows, and reporting utilities |
| `audio-analysis-core` | Shared audio analysis utilities | `video-analysis-core` | Normalized sample conversion, mono mixing, window functions, frame iteration, streaming frame windows, level helpers | Audio analysis crates and applications |
| `audio-analysis-fourier` | Frequency-domain audio analysis | `audio-analysis-core`, `video-analysis-core` | FFT spectra, STFT spectrograms, spectral features, dominant-frequency analyzer | Applications and audio pipelines |
| `audio-analysis-io` | Audio input convenience facade | `video-analysis-core`, `video-analysis-ingest`, `video-analysis-ffmpeg` | Audio-named input options, FFmpeg source opening helpers, ingest re-exports | Applications that want audio-specific input APIs |
| `audio-analysis-pitch` | Pitch estimation | `audio-analysis-core`, `video-analysis-core` | Autocorrelation pitch detector and pitch analyzer events | Applications and audio pipelines |
| `audio-analysis-processing` | Realtime-safe audio processing | `audio-analysis-core`, `video-analysis-core`, `video-analysis-ingest` | Audio transform trait, processor chains, gain/clip/mono/DC/biquad/noise-gate transforms, processed sources | Applications, preprocessing workflows, audio pipelines |
| `audio-analysis-recognition` | Audio similarity and recognition | `audio-analysis-core`, `audio-analysis-fourier`, `video-analysis-core` | Spectral embeddings, sample-backed reference libraries, similarity search, recognition analyzer events | Applications, audio pipelines, reference matching workflows |
| `audio-analysis-rhythm` | Rhythm and tempo analysis | `audio-analysis-core`, `video-analysis-core` | Onset envelope, onset detection, tempo estimates, rhythm analyzer events | Applications and audio pipelines |
| `audio-analysis-separation` | Instrument stem separation command wrapper | `video-analysis-core` | HTDemucs/Demucs options, command execution, expected stem paths | Applications and preprocessing workflows |
| `audio-analysis-synthesis` | Deterministic inverse audio generation | `data-inversion-core`, `video-analysis-core` | Tone specs, tone timelines, pitch/onset event to tone conversion, synthesized `OwnedAudioFrame` values | Applications prototyping audio from symbolic or analyzed events |
| `image-analysis-core` | Shared image contracts and statistics | `video-analysis-core` | Borrowed/owned image views, pixel formats, compacting, mean color, luma histograms | Image processing crates, applications, video frame preprocessing |
| `image-analysis-processing` | CPU image processing primitives | `image-analysis-core`, `video-analysis-core` | Crop, nearest resize, grayscale, invert, threshold, 3x3 convolution, processor chains | Applications, preprocessing workflows |
| `image-analysis-synthesis` | Deterministic inverse image generation | `data-inversion-core`, `image-analysis-core`, `video-analysis-core` | Solid images, gradients, luma-histogram expansion, region painting | Applications reconstructing approximate image buffers from summaries or regions |
| `text-analysis-corpus` | Corpus-scale text statistics and search | `text-analysis-core`, `video-analysis-core` | Corpus options, indexed document term counts, corpus term stats, TF-IDF scores/search, BM25 ranking/search | Applications, text analytics, semantic indexing |
| `text-analysis-core` | Shared text analysis utilities | `video-analysis-core`, `unicode-normalization`, `unicode-segmentation` | Text document contracts, text segment bridging, whitespace normalization, span-aware tokens, Unicode word/grapheme spans, script profiles, sentences, paragraphs, counts | Text feature crates, text pipelines, applications |
| `text-analysis-features` | Text feature extraction | `text-analysis-core`, `video-analysis-core` | Stop words, keywords, stemming, extractive summaries, sentiment, rule entities, readability, pattern detection, reusable text analyzers, term frequencies, character/token n-grams | Applications, text pipelines, downstream text analytics |
| `text-analysis-models` | Optional model-backed text analysis | `text-analysis-semantics`, `vector-analysis-core`, `video-analysis-core`, `video-analysis-models`, optional `tokenizers`/`ort`/Candle crates | Tokenizer bundles, ONNX text classifiers/embedders with fake-runner seams, Candle classifier/embedder architecture validation | Applications that need native text model execution |
| `text-analysis-prediction` | Text prediction models | `text-analysis-core`, `video-analysis-core` | Token Markov chains, next-token predictions, deterministic generation, perplexity scoring | Applications, text pipelines, prototyping |
| `text-analysis-semantics` | Lightweight semantic text analysis | `text-analysis-core`, `text-analysis-corpus`, `vector-analysis-core`, `vector-analysis-index`, `video-analysis-core` | Hashed text embeddings, `TextEmbeddingBackend`, generic embedding search, text similarity, co-occurrence graphs, related-term scoring | Applications, search, semantic analysis prototypes |
| `text-analysis-synthesis` | Deterministic inverse text generation | `data-inversion-core`, `text-analysis-core`, `video-analysis-core` | Weighted term prompts, term/event to document generation, generated text segments | Applications turning features/events back into approximate prose |
| `text-analysis-transcription` | Reusable transcript parsing and ASR command wrappers | `video-analysis-core`, `video-analysis-ingest`, `serde`, `serde_json`, `thiserror` | Transcript segment/result contracts, Whisper JSON/SRT/WebVTT/plain parsers, command transcribers, text segment source adapter | Use cases, applications, text pipelines |
| `dense-data` | Generic dense point aggregation and clustering | `numbers-core`, `video-analysis-core` | `DensePoint`, `DenseDataset`, weighted averages, per-dimension summaries, bounds, fixed-grid buckets, deterministic k-means clusters | Tables, graphs, charts, maps, media features, and analytics workflows |
| `vector-analysis-core` | Dense vector contracts and metrics | `video-analysis-core` | Finite vector validation, normalization, dot/cosine/L1/L2 metrics, means, summary stats | Search, recognition, clustering, analytics workflows |
| `vector-analysis-index` | Exact vector search and assignment | `vector-analysis-core`, `video-analysis-core` | In-memory vector index, search results, nearest-centroid assignment | Applications, prototypes, tests, small vector collections |
| `three-d-processing-core` | Generic 3D processing primitives | `video-analysis-core` | 3D vectors, points, bounds, transforms, quaternions, rigid transforms, line segments, point clouds, centroids, voxel downsampling | Mesh processing, applications, future 3D workflows |
| `three-d-processing-io` | 3D interchange formats | `three-d-processing-core`, `three-d-processing-mesh`, `video-analysis-core`, `serde_json`, `base64` | `OBJ`, `PLY`, and minimal embedded `.gltf` mesh/point-cloud I/O | Applications, CLI workflows, posture export |
| `three-d-processing-mesh` | Triangle mesh processing | `three-d-processing-core`, `video-analysis-core` | Mesh validation, topology, triangle normals, vertex normals, bounds, surface area, volume, transforms, smoothing, deterministic sampling | Applications and future 3D workflows |
| `video-analysis-core` | Canonical shared contracts and pipelines | External utility crates only | Time/frame types, media samples, detection traits/results, analyzer traits/results, observations, metrics, pipeline builders | All functional Rust crates |
| `video-analysis-data` | Online stream normalization and aggregation | `numbers-core`, `video-analysis-core` | `DataRecord`, `DataPayload`, bucket configuration, bucket summaries, stream summaries | Use cases, reporting, UI JSON generation |
| `video-analysis-dataset` | Retained analysis dataset records | `video-analysis-core`, `video-analysis-posture`, `serde` | Serializable owned records for scenes, cuts, media metadata, observations, events, metrics, tracks, features, and structured 2D/3D pose records | Transform, feature, storage, analytics workflows |
| `video-analysis-transform` | Deterministic dataset transformations | `video-analysis-dataset` | Filtering, time windows, scene grouping, time/frame joins, dedupe, merge, numeric feature resampling | Feature extraction and applications |
| `video-analysis-features` | Reusable feature extraction over retained datasets | `video-analysis-core`, `video-analysis-dataset`, `video-analysis-transform` | Scene stats, label histograms, transcript stats, audio event stats, track summaries, vector means | Applications and downstream ML/analytics workflows |
| `video-analysis-storage` | Retained dataset persistence | `video-analysis-dataset`, `serde`, `serde_json`, `thiserror` | JSON/JSONL writers and readers plus dataset manifests | Applications and automation |
| `video-analysis-synthesis` | Deterministic inverse video frame/storyboard generation | `data-inversion-core`, `num-rational`, `video-analysis-core` | Frame synthesis specs, region outlines, observation storyboards, generated `OwnedVideoFrame` values | Applications visualizing analyzed observations as approximate frames |
| `video-analysis-detectors` | Scene detector implementations | `video-analysis-core` | `SceneDetector` implementations, scoring algorithms, composite detector contracts | CLI, use cases, applications |
| `video-analysis-editing` | Classic CPU media editing primitives | `video-analysis-core` | Frame crop, blur, grayscale, inversion, brightness/contrast, 3x3 filters, and `FrameEditor` chains | Applications, preprocessing workflows, future media export flows |
| `video-analysis-ingest` | Source abstraction layer | `video-analysis-core` | Media/source metadata, source traits, source-to-pipeline adapter helpers, text line source | FFmpeg crate, use cases, applications |
| `video-analysis-ffmpeg` | FFmpeg-backed media probing and decoding | `video-analysis-core`, `video-analysis-ingest` | FFmpeg video/audio sources, metadata, probe helpers, source options | CLI, use cases, applications |
| `video-analysis-models` | Model download, backend, normalization, and external command contracts | `video-analysis-core` | Hugging Face specs/downloads, raw and normalized predictions, model analyzer adapters, external command protocol | CLI model commands, use cases, applications |
| `video-analysis-onnx` | Optional ONNX vision model backend adapters | `video-analysis-core`, `video-analysis-models`, `video-analysis-posture`, image crates, optional `ort` | Object-detection plus posture bundle validation, image preprocessing, fake-runner seams, optional runtime execution | Native vision inference experiments and CLI feature builds |
| `video-analysis-tracking` | Object tracking over frame detections | `video-analysis-core` | `TrackedDetection`, `IouTracker`, tracking options, object-detection backend trait, analyzer adapter | Applications, use cases, model-backed detection pipelines |
| `video-analysis-posture` | Pose and posture estimation contracts | `video-analysis-core`, `three-d-processing-core` | 2D/3D keypoints, skeletons, pose estimates, stick figures, posture backend traits, analyzer adapter, joint angle helpers, smoothing/interpolation | Applications, use cases, model-backed posture workflows |
| `video-analysis-posture-io` | Posture interchange and preview export | `video-analysis-core`, `video-analysis-posture`, `three-d-processing-core`, `serde_json`, `base64` | COCO-style keypoint JSON, 3D stick-figure `.ply`, 3D stick-figure `.gltf` | CLI workflows, applications, dataset export |
| `video-analysis-recognition` | Reference-embedding identity matching | `video-analysis-core` | Reference libraries, normalized embeddings, recognition candidates/matches, temporal aggregation, video analyzer adapter | Applications, use cases, model-backed face/object recognition |
| `video-analysis-output` | Detection output writers | `video-analysis-core` | Scene CSV, stats CSV, simple HTML, combined detection writers | CLI, applications |
| `video-analysis-split` | Scene-based media splitting | `video-analysis-core` | Split options, template variables, FFmpeg split function | CLI, applications |
| `video-analysis-radiance-fields` | Shared 3D geometry, camera, ray, and volume contracts | `video-analysis-core` | Vector/color/ray types, camera intrinsics/pose, radiance field trait, rendering/grid specs | Gaussian splatting, reconstruction, applications |
| `video-analysis-gaussian-splatting` | 3D Gaussian primitive projection and CPU compositing | `video-analysis-core`, `video-analysis-radiance-fields` | Gaussian primitives, projection config/results, splat rendering helpers | Applications and future 3D workflows |
| `video-analysis-radiance-io` | Radiance-field and 3DGS interchange formats | `video-analysis-core`, `video-analysis-radiance-fields`, `video-analysis-gaussian-splatting`, `video-analysis-reconstruction` | COLMAP text, Nerfstudio transforms, Gaussian splat PLY, preview PLY | Conversion tools and applications |
| `video-analysis-reconstruction` | Sparse reconstruction and triangulation contracts | `video-analysis-core`, `video-analysis-radiance-fields` | Camera/image/point IDs, features, matches, tracks, sparse reconstruction, triangulation/projection helpers | Applications and future 3D workflows |
| `video-analysis-cli` | `vanalyze` command-line composition | Core, detectors, FFmpeg, models, output, split | CLI commands and file outputs | End users and automation |
| `video-analysis-use-cases` | Runnable end-to-end workflows | Core, data, detectors, FFmpeg, ingest, models, audio/image helpers | `youtube-video`, `video-red-cars`, `audio-voice-analysis`, and `image-person-edit` workflow/report surfaces | End users, `@video-analysis/ui`, web app |
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

## Audio Analysis Contracts

The `audio-analysis-*` crates build on the canonical `AudioFrame`,
`AudioBuffer`, `AudioAnalyzer`, and `AnalysisEvent` contracts from
`video-analysis-core`.

- `audio-analysis-core` converts supported `AudioBuffer` formats into
  normalized `f32` samples, mixes interleaved channels to mono, applies common
  windows, iterates fixed-size analysis frames, and provides
  `StreamingFrameBuffer` for overlap-preserving windows across incoming chunks.
- `audio-analysis-fourier` provides FFT spectra, STFT spectrogram frames,
  spectral centroid/bandwidth/rolloff/flatness features, and an
  `AudioAnalyzer` that emits dominant-frequency events.
- `audio-analysis-io` re-exports the shared audio ingest traits and FFmpeg
  source types behind audio-named `AudioInput`, `AudioInputOptions`, and
  `open_audio_input` conveniences. FFmpeg remains the only default decode
  backend.
- `audio-analysis-pitch` estimates fundamental frequency with normalized
  autocorrelation and emits pitch events when confidence crosses the configured
  threshold.
- `audio-analysis-processing` owns frame-based audio transforms and source
  adapters. Built-in transforms include gain, hard clipping, mono conversion,
  DC blocking, biquad low/high/band/notch filters, and noise gates.
  Transformed frames are emitted as `OwnedAudioFrame` values with
  `AudioBuffer::F32` payloads in the first milestone.
- `audio-analysis-recognition` turns audio samples or frames into normalized
  spectral embeddings, stores multiple sample embeddings per reference, searches
  references by cosine similarity, and provides an `AudioAnalyzer` that emits
  `audio:recognized:<reference_id>:<label>` events over streaming windows.
- `audio-analysis-rhythm` detects onset events from energy changes, estimates
  BPM from onset intervals, and can emit both onset and tempo events.
- `audio-analysis-separation` wraps the external Demucs CLI with the `htdemucs`
  model by default. It does not decode audio itself; it validates command
  options, runs the process, and returns the expected separated stem paths.

Audio analysis crates should accept borrowed core audio frames or normalized
sample slices and should emit `AnalysisEvent` values for pipeline integration.
File writing and encoded audio sinks are deferred; the current processing
surface returns processed frames for callers to analyze, stream, or encode later.

## Image Analysis Contracts

The `image-analysis-*` crates provide still-image contracts and processing
helpers without requiring video timeline semantics.

- `image-analysis-core` owns `ImageView<'_>`, `OwnedImage`,
  `ImagePixelFormat`, image compacting, mean RGB, and luma histograms.
- `ImageView::from_video_frame` and `OwnedImage::from_video_frame` bridge core
  `VideoFrame<'_>` values into still-image workflows.
- `image-analysis-io` owns PNG/JPEG/WebP file loading and saving for
  `OwnedImage` buffers.
- `image-analysis-processing` owns `ImageOperation`, `ImageProcessor`,
  `ImageRegion`, crop, nearest-neighbor resize, grayscale, invert, threshold,
  convolution, and sharpen helpers.
- `image-analysis-segmentation` owns still-image prompts, binary masks,
  segments, and pure segmentation backend contracts with explicit opt-in
  automatic mask generation helpers.
- `image-analysis-detection` owns canonical still-image detections plus
  mask-proposal adapters over segmentation backends.
- `image-analysis-synthesis` owns deterministic, non-AI image generation from
  colors, histograms, and regions.
- `image-analysis-models` owns image model presets and model-backed backend
  traits for segmentation, classification, embeddings, and captioning.
- `image-analysis-onnx` owns still-image ONNX preprocessing and optional
  runtime-backed image model adapters.
- `image-analysis-comfyui` owns ComfyUI workflow builders for AI image
  generation and manipulation.

Image processing outputs are compact `OwnedImage` buffers. Image crates should
not own scene timing, CLI branching, or report serialization. Pure image crates
stay classical and memory-first; AI/runtime integrations live in dedicated
image model/runtime/orchestration crates.

## Text Analysis Contracts

The `text-analysis-*` crates provide reusable text processing separate from
video use cases and model adapters.

- `text-analysis-core` owns `TextDocument<'_>`, `OwnedTextDocument`,
  `TextStats`, `TextSpan`, `Token`, `Sentence`, `Paragraph`,
  `TextProcessingOptions`, `TextBoundaryOptions`, `WordSegment`,
  `GraphemeSpan`, `ScriptProfile`, whitespace normalization, word
  tokenization, span-aware tokenization, Unicode word/grapheme segmentation,
  script profiling, sentence/paragraph splitting, and detailed stats.
- `TextDocument::from_segment` and `OwnedTextDocument::from_segment` bridge core
  `TextSegment` and `OwnedTextSegment` values into text-only workflows.
- `text-analysis-features` owns `TermFrequency`, `TextFeatureSummary`,
  `StopWords`, `KeywordOptions`, `Keyword`, `NgramFrequency`,
  `ReadabilitySummary`, `StemOptions`, `ExtractiveSummaryOptions`,
  `SummarySentence`, `SentimentLexicon`, `SentimentSummary`, `EntityRuleSet`,
  `EntityMention`, top terms, keyword extraction, lexical diversity, stemming,
  extractive summaries, lexicon sentiment, rule-based entity extraction,
  pattern detection, and character/token n-grams. It also provides
  `TextStatsAnalyzer`, `KeywordAnalyzer`, `ExtractiveSummaryAnalyzer`,
  `SentimentAnalyzer`, `EntityRuleAnalyzer`, `PatternAnalyzer`, and
  `TranscriptHeuristicAnalyzer` for `TextPipeline`.
- `text-analysis-corpus` keeps `TfIdfCorpus` stable and adds `Bm25Corpus` for
  BM25 document ranking with duplicate-id rejection and empty-query handling.
- `text-analysis-semantics` keeps `HashedTextEmbedder` and `SemanticTextIndex`
  while adding `TextEmbeddingBackend` and `EmbeddingSearchIndex<E>`. Embedding
  APIs return `DenseVector` directly instead of encoding vectors into
  `AnalysisEvent` values.
- `text-analysis-models` owns optional model-backed text execution surfaces:
  `TokenizerBundle`, `TokenizedText`, `OnnxTextClassifier`,
  `OnnxTextEmbedder`, `CandleTextClassifier`, and `CandleTextEmbedder`.
  The default feature set is empty. `tokenizers` enables Hugging Face tokenizer
  loading, `onnx` enables ONNX dependencies and bundle validation, `candle`
  enables Candle dependencies and architecture checks, `external-tests` opts
  into network/model tests, and `slow-external-tests` gates slow ONNX runtime
  execution checks.
- `text-analysis-transcription` owns `TranscriptFormat`, `TranscriptSegment`,
  `TranscriptionResult`, `Transcriber`, `CommandTranscriber`,
  `WhisperCliTranscriber`, and `TranscriptSegmentSource`. It parses Whisper
  JSON, SRT, WebVTT, and plain line transcripts, and converts transcript
  segments into `OwnedTextSegment` values.

Deterministic text crates should emit deterministic features and label-based
`AnalysisEvent` values. Model-backed classification and embeddings are
separate but composable through `TextModelBackend`, `ModelTextAnalyzer`, and
`TextEmbeddingBackend`.

## Vector Analysis Contracts

The `vector-analysis-*` crates standardize dense vector handling for embedding,
recognition, search, and analytics workflows.

- `vector-analysis-core` owns `DenseVector`, `VectorMetric`, finite validation,
  L2 normalization, dot product, cosine similarity, Euclidean distance,
  Manhattan distance, mean vectors, and per-dimension stats.
- `vector-analysis-index` owns `VectorRecord`, `VectorSearchIndex`,
  `SearchConfig`, `SearchResult`, exact in-memory search, and nearest-centroid
  assignment.

Vector crates intentionally use exact CPU algorithms. Approximate nearest
neighbor backends can be added later behind separate implementation crates
without changing the core vector contracts.

## Numbers Contracts

`numbers-core` provides reusable scalar numeric building blocks for analytics
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

## Dense Data Contracts

`dense-data` provides generic dense numeric point processing for UI and media
workflows that need the same aggregation shape across tables, graphs, charts,
maps, and feature-derived media timelines.

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

- `data-inversion-core` owns `InformationFidelity`, `InversionMethod`,
  `InversionTrace`, and `Generated<T>`. Synthesis crates should attach traces
  that identify source and target types, confidence, assumptions, and fields
  that were preserved, inferred, interpolated, templated, or defaulted.
- `audio-analysis-synthesis` turns tone timelines and supported
  `AnalysisEvent` labels such as pitch and onset events into `OwnedAudioFrame`
  values. It uses deterministic analytic waveforms and records that samples are
  interpolated from symbolic data.
- `image-analysis-synthesis` turns colors, color stops, luma histograms, and
  regions into `OwnedImage` buffers. Histogram and region layouts are
  deterministic approximations because the original spatial detail is not
  recoverable.
- `text-analysis-synthesis` turns weighted terms or analyzer events into
  `OwnedTextDocument` and `OwnedTextSegment` values using deterministic
  templates. It preserves term prominence but treats syntax and term
  relationships as inferred.
- `video-analysis-synthesis` turns frame specs or observations into
  `OwnedVideoFrame` storyboards. It preserves frame positions and regions when
  available, while labels, missing regions, and pixels are heuristic visual
  encodings.

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
- `is_ffmpeg_available` and `is_ffprobe_available` probes.

FFmpeg is responsible for external process interaction, probing, decoding, and
conversion. Downstream packages should consume only core and ingest contracts
such as `OwnedVideoFrame`, `OwnedAudioFrame`, `VideoFrameSource`, and
`AudioFrameSource`.

Generated media fixture helpers are behind the `test-utils` feature. Opt-in
decode coverage is available with:

```bash
cargo test -p video-analysis-ffmpeg --features ffmpeg-tests
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

`video-analysis-models` separates model acquisition, model-specific backend
execution, prediction normalization, and analyzer integration.

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

Text model presets include ONNX-friendly Hugging Face repos:

- `XenovaDistilbertSst2Onnx` requests `config.json`, `tokenizer.json`,
  `tokenizer_config.json`, and the first available ONNX file from
  `onnx/model.onnx`, `onnx/model_quantized.onnx`, or `onnx/model_int8.onnx`.
- `XenovaMiniLmL6V2Onnx` requests `config.json`, `tokenizer.json`,
  `tokenizer_config.json`, and the first available ONNX file from
  `onnx/model.onnx` or `onnx/model_quantized.onnx`.

`video-analysis-onnx` owns native vision-model bundle adaptation. Its default
build validates object-detection bundles, reads `id2label`, parses
preprocessor size/rescale/mean/std metadata, converts frames into NCHW tensors,
and decodes runner outputs into `RawPrediction::object` values. The
`onnxruntime` feature gates the optional `ort` dependency and executes
DETR/YOLOS-style ONNX sessions that return logits plus center-format boxes.
Deterministic tests inject a fake runner so normal workspace checks do not
download or execute model artifacts.

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

The `comfyui-data` and `comfyui-models` crates are standalone
interoperability packages for ComfyUI data that applications may need to read or
write.

`comfyui-data` exposes:

- `ComfyWorkflow`, `WorkflowNode`, `WorkflowInput`, `WorkflowOutput`,
  `WorkflowLink`, and `WorkflowGroup` for workflow JSON files saved by ComfyUI.
- `WorkflowNodeId`, which accepts numeric and string node ids.
- `ComfyWorkflow::validate`, which checks duplicate node/link ids and missing
  link references.
- `PromptGraph`, `PromptNode`, `PromptLink`, `prompt_link`, and
  `parse_prompt_link` for ComfyUI API prompt graphs.

`comfyui-models` exposes:

- `ComfyModelKind`, including ComfyUI folder keys such as `checkpoints`,
  `loras`, `vae`, `text_encoders`, `diffusion_models`, `clip_vision`,
  `controlnet`, `upscale_models`, `audio_encoders`, and legacy aliases such as
  `clip` and `unet`.
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
- Point distance, rigid-transform, voxel-downsampling, and center/scale helpers.

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
- Triangle normal, triangle area, surface area, face/vertex normal helpers.
- Connected-component, manifold/watertight, volume, transform, merge,
  deterministic surface sampling, and Laplacian smoothing helpers.

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
`@video-analysis/ui`.

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
`crates/video/video-analysis-use-cases/src/main.rs` align with the TypeScript
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

- `comfyui-data`: `serde`, `serde_json`, `thiserror`.
- `comfyui-models`: `serde`, `thiserror`.
- `audio-analysis-core` -> `video-analysis-core`.
- `audio-analysis-fourier` -> `audio-analysis-core`,
  `video-analysis-core`.
- `audio-analysis-io` -> `video-analysis-core`, `video-analysis-ingest`,
  `video-analysis-ffmpeg`.
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
- `text-analysis-core` -> `video-analysis-core`,
  `unicode-normalization`.
- `text-analysis-features` -> `text-analysis-core`,
  `video-analysis-core`.
- `text-analysis-transcription` -> `video-analysis-core`,
  `video-analysis-ingest`, `serde`, `serde_json`, `thiserror`.
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
- `video-analysis-models` -> `video-analysis-core`.
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
- `@video-analysis/ui` consumes generated data/report shapes and should not
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
