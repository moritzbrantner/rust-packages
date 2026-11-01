# API Contracts

This document describes the inter-package contracts that let the Rust crates and
the `@moritzbrantner/video-analysis-ui` package work together. It is intentionally not an
exhaustive rustdoc inventory. It focuses on shared types, traits, serialized
formats, file formats, package exports, and dependency boundaries.

`moenarch-media-core` owns neutral media contracts for time, sample formats,
analysis events, and shared errors. `moenarch-audio-contracts` owns audio
buffers, frames, analyzers, and pipelines. `moritzbrantner-video-analysis-core`
owns only video-specific contracts such as video frames, scene detection,
metrics, and visual observations, while preserving compatibility re-exports of
the neutral and audio contracts. Other crates should compose around these
canonical owners instead of defining parallel types.

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
capabilities. It also owns the curated landscape metadata structs used by
`xLandscape` schema extensions. `SurfaceOperation` remains operation metadata,
not workflow node metadata, and `xLandscape` is descriptive metadata rather than
execution behavior. Declared `xLandscape` metadata is optional, must be
identical on input and output schemas, and is validated only for metadata shape,
known owners, known type IDs, and port validity. Domain crates own the semantic
compatibility of the types they name.

Foundation contract owners for the first steering wave are:

| Contract family | Owner |
| --- | --- |
| Runtime surface DTOs and curated landscape metadata shape | `moritzbrantner-runtime-core` |
| Jobs and artifacts | `moritzbrantner-jobs-core` |
| Model specs, bundles, and model lifecycle | `moritzbrantner-model-runtime` |
| Neutral media time, sample formats, events, and errors | `moenarch-media-core` |
| Video frames, observations, bounding boxes, analyzers, and pipelines | `moritzbrantner-video-analysis-core` |
| Images | `moritzbrantner-image-analysis-core` |
| Audio buffers, frames, analyzers, and pipelines | `moenarch-audio-contracts` |
| Text documents and text segments | `moritzbrantner-text-core` |
| Durable text indexes, chunks, semantic facets, and index queries | `moritzbrantner-text-index` |
| Timed transcripts | `moritzbrantner-text-transcripts` |
| Tensors | `moritzbrantner-tensor-data` |
| Vectors | `moritzbrantner-vector-analysis-core` |
| Sparse data | `moritzbrantner-math-sparse-data` |
| Dense data | `moritzbrantner-dense-data` |
| Numbers | `moritzbrantner-numbers-core` |
| Geometry primitives | `moritzbrantner-math-geometry-2d` |
| Signal primitives | `moritzbrantner-math-signal-core` |
| Visual detections, visual keypoints, visual embeddings, identity match summaries | `moritzbrantner-vision-core` |

## Package Surface Policy

Package surfaces are the primary cross-runtime interface for new consumers.
Root facade re-exports are compatibility scaffolding; new workflow behavior must
be exposed and tested through the crate-owned package surface and its adapters.

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

Every runtime-capable, non-wrapper library crate owns its operation metadata
and execution in `src/surface.rs`. Thin CLI, server, WASM, and app packages must
call that library-owned surface instead of implementing package behavior in
wrapper code. The contract-only identity seams listed exhaustively in
[`contract-only-foundation-crates.json`](contract-only-foundation-crates.json)
are the sole exception: they own shared DTOs and traits, not package operations
or runtime companions. The exact enumeration keeps unrelated libraries covered
by the package-surface audits.
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
an explicit runtime storage context, declare filesystem/network side effects
through `xExecutionPlan`, and stay unsupported in WASM. Compatibility-only
surface calls may return plans without performing setup.

Primary workflow operations declare typed `SurfaceOperation.curation` metadata
with `role: "workflow"` and `primary: true` for the default package workflow.
They also declare strict request schemas (`additionalProperties: false`), keep
legacy `xOperationCategory: "workflow"` schema extensions synchronized for
older consumers, include `xReleaseStability`, `xContractPolicy`, typed
error-shape metadata, and any required `xExecutionPlan` or lower-contract proof
metadata. The tracer gate
currently enforces this for `image.classification.classify`, `index.search`,
and `video.sfm.reconstruct`; workspace-wide enforcement expands only after this
pattern proves stable.

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
`video-analysis-core`, `image-analysis-core`,
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
Curated type/function metadata is exposed through optional `xLandscape` schema
extensions. These extensions let package consumers see stable inputs and outputs
for representative operations while keeping the domain contract semantics owned
by the existing foundation crates. Missing `xLandscape` means curated I/O has
not been declared yet, not that an operation is unusable or impossible to map in
a consumer. Curated type and function IDs are stable after release; additions
are allowed, while removals or renames require a documented migration. The
landscape layer is not a workflow graph, node system, port layout, scheduling
model, execution planner, or runtime placement contract.

The text crate surfaces now expose deterministic, local-first operations for
core statistics/tokenization/boundaries, lexical analysis and corpus search,
linguistic analysis/entity projection, hashed embeddings and transient search,
soft-legacy compatibility retrieval, durable memory/SQLite text indexing, transcript parsing/formatting, fallback/imported NLP
tasks, Markov/template generation, generation-from-linguistics adapters, and
runtime helpers. `runtime.onnxQaProbe`, `runtime.downloadBundle`, and
`qa.answer` without imported predictions/backend are native server-side model
workflows when built with `local-onnx`; all other default text surface calls
remain no-download unless documented otherwise. These operations must continue to return
`SurfaceResponse` values with JSON payloads and typed error strings, and they
must not silently download models, run native inference, invoke ASR commands, or
write retrieval persistence files through default surface calls outside the
documented model-backed operation exceptions.

`moritzbrantner-text-index` owns the `index.*` operation family, with
`index.search` as the primary package-surface workflow. Package examples are
transient/dry-run by default, and package surfaces are request-scoped:
CLI/server/WASM/app operations do not create durable server-side index sessions
or return open index handles. SQLite backends require a caller-provided path,
`commit: true`, and runtime context write permission; browser/WASM adapters
report SQLite persistence as unsupported diagnostics rather than performing
hidden writes.
`qa.answerWithIndex` is the primary text-index path for new cited document QA;
`qa.answerWithRetrieval` remains available as a deprecated soft-legacy
compatibility path. Both retrieval-backed QA operations delegate context
assembly and citation mapping through private Retrieved Context, which uses full
chunk text for answer extraction when available and keeps public citation
snippets stable for display.

Text package operations declare release contract metadata in their
`SurfaceOperation` schemas. Top-level request fields are explicit
(`additionalProperties: false`), outputs preserve the structured
`operation`/`title`/`message`/`summary`/`result` shape, and schema extensions
record backward-compatible `xOperationCategory`, `xReleaseStability`,
`xContractPolicy`, resource limits, and the typed error shape. First-class
operation placement belongs in `SurfaceOperation.curation`; app configs should
only override grouping, defaulting, or ordering for presentation exceptions.
During the release train, operation IDs and declared fields are additive-only
unless a versioned migration is documented.
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
text specializations. Audio transcription surfaces in `audio-analysis` consume
and return those transcript contracts.
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
| `moritzbrantner-animation-core` | Shared animation timeline contracts | `moritzbrantner-three-d-processing-core`, `moritzbrantner-video-analysis-core`, `serde` | Time values, interpolation modes, keyframes, typed tracks, transform tracks, joints, skeletons, and animation clips | Future 2D/3D animation workflows, posture sequence interop, mesh/skinning work |
| `moritzbrantner-three-d-processing-core` | Generic 3D processing primitives | `moritzbrantner-video-analysis-core` | 3D vectors, points, bounds, transforms, quaternions, rigid transforms, line segments, rays, planes, spheres, point clouds, centroids, voxel downsampling, nearest-point lookup, and basic bounds/sphere/ray collision helpers | Mesh processing, applications, future 3D workflows |
| `moritzbrantner-three-d-processing-io` | 3D interchange formats | `moritzbrantner-three-d-processing-core`, `moritzbrantner-three-d-processing-mesh`, `moritzbrantner-video-analysis-core`, `serde_json`, `base64` | `OBJ`, `PLY`, and minimal embedded `.gltf` mesh/point-cloud I/O | Applications, CLI workflows, posture export |
| `moritzbrantner-three-d-processing-mesh` | Triangle mesh processing | `moritzbrantner-three-d-processing-core`, `moritzbrantner-video-analysis-core` | Mesh validation, topology, diagnostics, repair helpers, triangle normals, vertex normals, bounds, surface area, volume, transforms, smoothing, deterministic sampling | Applications and future 3D workflows |
| `moritzbrantner-video-analysis-mvs` | Multi-view stereo contracts | `moritzbrantner-video-analysis-core`, `moritzbrantner-video-analysis-reconstruction`, `moritzbrantner-three-d-processing-core` | Depth maps, view-pair records, dense reconstruction options, point-cloud conversion helpers, and OpenCV MVS planning placeholder | Reconstruction, radiance, and 3D processing workflows |
| `moritzbrantner-video-analysis-posture` | Pose and posture estimation contracts | `moritzbrantner-video-analysis-core`, `moritzbrantner-three-d-processing-core` | 2D/3D keypoints, skeletons, pose estimates, stick figures, posture backend traits, analyzer adapter, joint angle helpers, smoothing/interpolation | Applications, use cases, model-backed posture workflows |
| `moritzbrantner-video-analysis-posture-io` | Posture interchange and preview export | `moritzbrantner-video-analysis-core`, `moritzbrantner-video-analysis-posture`, `moritzbrantner-three-d-processing-core`, `serde_json`, `base64` | COCO-style keypoint JSON, 3D stick-figure `.ply`, 3D stick-figure `.gltf` | CLI workflows, applications, dataset export |
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

## Contracts Owned Elsewhere

Core media, detection, image, text, vector, numeric, data, ingest, FFmpeg,
recognition, tracking, editing, output and split contracts are documented by
their owning repositories: `moenarch-foundation`, `nlp-stack` and
`visual-analysis`. Scene detection is owned by `scenedetect-rs`.

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

## 3D Scene Contracts

The workspace has two 3D layers. `three-d-processing-core` owns the canonical
workspace 3D spatial math contract: vectors, points, rotations, transforms,
pinhole camera geometry, and coordinate-convention conversions. Video-driven
neural rendering and reconstruction crates may keep domain DTOs, but they bridge
camera, pose, vector, and quaternion semantics to the 3D core through explicit
adapter methods.

`three-d-processing-core` exposes:

- `Vector3`
- `Point3`
- `Bounds3`
- `Transform3`
- `Quaternion`
- `RigidTransform3`
- `Vector3d`
- `Point3d`
- `Quaterniond`
- `Matrix3` / `Matrix4`
- `Matrix3d` / `Matrix4d`
- `SimilarityTransform3`
- `TrsTransform3`
- `AffineTransform3`
- `RigidTransform3d`
- `SimilarityTransform3d`
- `TrsTransform3d`
- `AffineTransform3d`
- `PinholeIntrinsics`
- `PinholeIntrinsicsd`
- `CameraPose3`
- `CameraPose3d`
- `CameraRay`
- `CameraRayd`
- `LineSegment3`
- `PointCloud`
- Point distance, closest-point, ray/surface intersection, rigid-transform,
  TRS/affine transform, row-major matrix construction/conversion, pinhole
  projection/pixel-ray, workspace/WebGL view and projection matrices, COLMAP
  pose conversion, glTF/WebGL matrix conversion, voxel-downsampling, and
  center/scale helpers. Workspace camera forward is `+Z`; WebGL view/projection
  builders explicitly map that convention to graphics `-Z`. Euler extraction is
  import/export and UI-control support only; quaternion remains the primary
  rotation type.

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

The Rust root crate `video-analysis` is a compatibility umbrella facade. It
re-exports existing core items, detector items, and package modules for audio,
image, text, vector, 3D processing, data, FFmpeg, ingest, models, output,
radiance fields, Gaussian splatting, reconstruction, and split. It does not
expose CLI or use-case binaries as library modules, and new workflow behavior
should not be proven by adding root facade exports or root facade tests.

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
- `audio-contracts` -> `media-core`.
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
- `text-transcripts` -> `runtime-core`, `text-core`, `serde`, `serde_json`,
  `thiserror`.
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
