# Video Analysis Model And CLI Status And Next Steps

This document preserves the implementation context for video model backends and
CLI integration, then narrows the remaining decisions so follow-up threads do
not treat completed work as the next target.

## Current Next Work

The native model backend and CLI integration milestones below are implemented.
The remaining live work is:

- Choose whether `vanalyze` should absorb composed use-case commands or remain
  a primitive workflow CLI while `video-analysis-use-cases` stays separate.
- Choose the first stable, small external ONNX inference fixture for opt-in CI
  and local smoke runs.
- Decide whether the root facade should re-export native backend crates or keep
  backend imports explicit.
- Continue PySceneDetect `0.7` parity as a separate video scene lane: timestamp
  and VFR metadata first, then CLI spelling/output exporter compatibility.

## Completed History

The original plan in this document has been completed as implementation history:

- `moritzbrantner-runtime-onnx` owns domain-neutral ONNX tensor/session
  mechanics.
- Task crates own model semantics and decoding for image classification,
  captioning, embeddings, detection, video recognition, posture, and related
  adapters.
- `vanalyze models inspect` is implemented.
- `vanalyze models run` is implemented for raw RGB/BGR frames and
  DETR/YOLOS-style ONNX outputs behind feature gates.
- `vanalyze analyze` is implemented for scene/dataset reports and optional
  sampled video model inference.
- The use-case binary remains separate pending an explicit product decision.

Current state:

- `moritzbrantner-model-runtime` owns model specs, Hugging Face downloads, bundle
  materialization, prediction normalization, analyzer adapters, and external
  command backends.
- `moritzbrantner-video-analysis-use-cases` can already attach external object, OCR, and text
  model commands to the `youtube-video` workflow.
- `moritzbrantner-video-analysis-cli` currently exposes scene detection/list/split, primitive
  JSON analysis reports, plus model preset/download/inspect commands. It also
  parses `models run` for raw RGB/BGR frame inference, gated behind CLI ONNX
  features.
- Text model execution is owned by the text crates that use it:
  `moritzbrantner-text-linguistics` owns tokenizer alignment and local Candle-backed NER,
  while `moritzbrantner-text-embeddings` owns optional ONNX/Candle embedding runtimes.
- Native ONNX session mechanics live in `moritzbrantner-runtime-onnx`.
  Video object-detection adaptation is owned by
  `moritzbrantner-video-analysis-recognition`, pose adaptation is owned by
  `moritzbrantner-video-analysis-posture`, and image preprocessing/decoding is
  owned by the image task crates.
- Optional Python dependencies for model-backend experiments can be installed
  idempotently with `scripts/setup_model_external_tools.sh`.

## Implemented Plan 1: Native Model Backends

Original goal: add a first native inference path that consumes `ModelBundle`
and emits existing prediction values through task-owned backends.

Recommended first target: ONNX Runtime object detection.

Rationale:

- ONNX gives a stable runtime boundary for multiple Hugging Face vision models.
- It avoids inventing tensor execution code in this workspace.
- The existing `RawPrediction` normalization can absorb model-specific box
  format differences.
- Text models also need tokenization, vocabulary handling, special tokens, and
  pooling/classification conventions; they are better as the second native
  target.

### Thread A: Backend Crate Skeleton

Status: implemented through `moritzbrantner-runtime-onnx` plus owning task
crates.

Workspace shape:

- `runtime-onnx` owns only typed ONNX tensors, session metadata, and session
  execution.
- ONNX execution now flows from `model-runtime` bundle download/materialization,
  through task-crate bundle validation and output decoding, into
  `runtime-onnx` session execution.
- `image-analysis-detection` owns object-detection bundle validation and
  DETR/YOLOS-style decoding.
- `video-analysis-recognition` maps image detections back to `RawPrediction`
  values for video frames.
- `video-analysis-posture` owns pose model contracts and fake-runner seams.
- `runtime-onnx` remains excluded from package-surface wrapper requirements
  because it is intentionally domain-neutral and library-only; user-facing
  workflows live in the owning task crates.

### Native ONNX Compatibility Inventory

Default tests do not download bundles or execute live ONNX models. The live
compatibility checks are ignored, feature-gated, and runnable through
`scripts/check_onnx_external_smoke.sh` after bundles have been materialized
under `.model-runtime/`. `runtime-onnx` remains excluded from package-surface
wrapper requirements because it is intentionally domain-neutral and
library-only.

| Task crate | Adapter | Feature flag | Expected bundle task | Required files | Supported output shapes/names | Current ignored smoke bundle name | Known unsupported variants |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `image-analysis-classification` | `OnnxImageClassifier` | `local-onnx` / `external-tests` | `image_classification` | `config.json`, at least one `.onnx`; `preprocessor_config.json` optional | logits as `[classes]` or `[1, classes]`; first f32 output is used | `vit-base-patch16-224` | multi-output classifier heads that need non-first output selection |
| `image-analysis-embeddings` | `OnnxImageEmbedder` | `onnx` / `external-tests` | `image_embedding` | exactly one `.onnx`; `config.json` and `preprocessor_config.json` optional | preferred f32 names containing `image_embeds`, `embeds`, `embedding`, or `pooler_output`; otherwise first f32 output; vector must be non-empty, finite, and match configured size when present | `xenova-clip-vit-base-patch32-onnx` | multiple model files in one bundle; non-f32 embedding outputs |
| `image-analysis-embeddings` | `OnnxFaceEmbedder` | `onnx` | `face_embedding` or custom `face_embedding` | exactly one `.onnx`; `config.json` and `preprocessor_config.json` optional | same embedding vector selection as image embeddings | none; deterministic unit coverage only | multi-file face bundles; detector-plus-embedder combo bundles |
| `image-analysis-detection` | `OnnxFaceDetector` | `onnx` | `face_detection` or custom `face_detection` | exactly one `.onnx`; `preprocessor_config.json` optional | YuNet single `[N, 15]` or `[1, N, 15]`; split `loc/conf/iou`; split per stride with `8`, `16`, and `32` in output names | none; deterministic YuNet unit coverage only | missing stride groups; prior-count mismatches; non-YuNet face detector heads |
| `image-analysis-detection` | `OnnxObjectDetector` | `onnx` / `external-tests` | `object_detection` | `config.json`, exactly one `.onnx`; `preprocessor_config.json` optional | DETR/YOLOS-style one boxes tensor ending in 4 values plus one logits tensor; accepted ranks `[queries, dim]` and `[1, queries, dim]` | `xenova-detr-resnet-50-onnx` | YOLO grid/anchor heads; segmentation outputs; multiple model files |
| `image-analysis-captioning` | `OnnxImageCaptioner` | `local-onnx` / `external-tests` | custom `image_captioning` | `config.json`, `tokenizer.json`, encoder `.onnx`, decoder `.onnx`; `generation_config.json`, `tokenizer_config.json`, `preprocessor_config.json`, `vocab.json`, and `merges.txt` optional when tokenizer can load without them | encoder hidden states `[1, sequence, hidden]`; decoder logits named `logits` preferred, fallback first f32; logits `[1, sequence, vocab]` or `[sequence, vocab]` | `vit-gpt2-image-captioning` or `vit-gpt2-image-captioning-onnx` | beam search, cached KV decoder-only exports, BLIP/safetensors-only bundles |
| `video-analysis-posture` | `OnnxPose2dEstimator` | `onnx` | `pose_estimation_2d` | `config.json`, exactly one `.onnx`; `preprocessor_config.json` optional | pose tensor names containing `keypoints`, `poses`, or `output`; shapes `[poses, joints, 3]` or `[1, poses, joints, 3]`; optional `scores` f32 output | none; deterministic unit coverage only | batch sizes greater than one; heatmap-only outputs requiring argmax decoding |
| `video-analysis-posture` | `OnnxPoseLifter` | `onnx` | `pose_lifting_3d` | `config.json`, exactly one `.onnx`; `preprocessor_config.json` optional | output tensor names containing `keypoints`, `poses`, or `output`; shapes `[frames, joints, 3]` or `[1, frames, joints, 3]`; source pose metadata is preserved | none; deterministic unit coverage only | batch sizes greater than one; lifters requiring camera calibration inputs |
| `text-question-answering` / `runtime-onnx` | text QA/runtime helper regression | `external-tests` | `question_answering` | local QA bundle with tokenizer/config and at least one `.onnx` | helper lookup by exact name, preferred name, and fallback index for f32 outputs | `roberta-base-squad2-onnx` | full QA ONNX adapter execution is not part of the image/video consolidation |

Suggested dependencies:

- `video-analysis-core`
- `model-runtime`
- `serde`
- `serde_json`
- Runtime dependency behind a feature:
  - `runtime-onnx`.
- Image preprocessing dependency behind a feature if needed:
  - prefer an existing local image crate if possible.

Before working against Python-backed fixtures or conversion helpers, verify the
local optional dependencies:

```bash
bash scripts/setup_model_external_tools.sh onnx
bash scripts/check_model_external_tools.sh onnx
```

For Transformers or safetensors experiments:

```bash
bash scripts/setup_model_external_tools.sh transformers
bash scripts/check_model_external_tools.sh transformers
```

Initial feature shape:

```toml
[features]
default = []
onnxruntime = ["runtime-onnx/onnxruntime"]
external-tests = ["onnxruntime"]
```

Public API sketch:

```rust
pub struct OnnxModelBackend {
    bundle: ModelBundle,
    session: Session,
    task: ModelTask,
}

pub struct OnnxObjectDetectionOptions {
    pub input_width: u32,
    pub input_height: u32,
    pub score_threshold: Option<f32>,
    pub label_map: BTreeMap<i64, String>,
}

impl OnnxModelBackend {
    pub fn from_bundle(bundle: ModelBundle) -> Result<Self>;
}

impl VisionModelBackend for OnnxModelBackend {
    fn task(&self) -> ModelTask;
    fn predict_frame(&mut self, frame: &VideoFrame<'_>) -> Result<Vec<RawPrediction>>;
}
```

Acceptance criteria:

- Can load a bundle containing one `.onnx` file and `config.json`. Done.
- Validates there is exactly one supported ONNX model file, or returns
  `DetectError::InvalidArgument`.
- Produces deterministic `RawPrediction` values from a fake/session test double
  without network access. Done.
- Has one ignored external test placeholder for a tiny ONNX-capable model
  bundle.
- Does not make ONNX Runtime a required dependency for the whole workspace.
  Done.

Tests to add:

- `onnx_backend_rejects_bundle_without_model_file`
- `onnx_backend_loads_label_map_from_config`
- `onnx_object_detection_converts_boxes_to_raw_predictions`
- ignored external: `onnx_backend_runs_tiny_detection_model`

Implementation notes:

- Keep runtime-specific errors mapped to `DetectError::Source`.
- Keep preprocessing explicit and configurable. Do not silently assume one
  Hugging Face processor format covers all models.
- Prefer model-specific adapters over a huge generic parser if the first target
  is YOLOS or DETR.
- If the first selected model only ships `model.safetensors`, either add an ONNX
  preset or document conversion as a separate step. Do not pretend safetensors
  can be executed without a tensor runtime.

### Thread B: Text Backend Follow-Up

After the first vision backend lands, add a text backend separately.

Recommended target:

- A small ONNX text classifier with tokenizer assets, or
- a Candle/safetensors backend if the workspace accepts that dependency family.

Required design work:

- Add tokenizer loading from bundle files.
- Define truncation, padding, attention mask, and special token behavior.
- Decide how labels are read from `config.json` (`id2label`) and how logits map
  to `RawPrediction::label`.

Acceptance criteria:

- `TextModelBackend` implementation consumes `TextSegment<'_>`.
- Emits labels and scores as `RawPrediction`.
- Tests cover unknown labels, empty input, and deterministic logits.

## Implemented Plan 2: CLI Integration Beyond Scene Detection

Original goal: make `vanalyze` run more of the video-analysis stack without
requiring users to discover lower-level crates or the separate use-case binary.

Recommended sequence: add narrow, composable commands first, then promote a full
report command once model execution is real.

### Thread C: Bundle Inspection Command

Add:

```bash
vanalyze models inspect --name yolos-tiny --revision main
vanalyze models inspect --bundle-dir .model-runtime --name yolos-tiny
vanalyze models inspect --manifest .model-runtime/yolos-tiny/main/manifest.json
```

Purpose:

- Confirms model bundles are discoverable by humans and scripts.
- Gives the native backend thread a stable CLI fixture.
- Requires no model inference dependency.

Status: implemented in `video-analysis-cli`.

CLI behavior:

- If `--manifest` is passed, load that manifest directly.
- Otherwise load by `--bundle-dir`, `--name`, and `--revision`.
- Print name, repo id, revision, task, manifest path, and file list.

Tests:

- Parse tests for both `--manifest` and `--name`.
- Unit test the output formatting with a fake temp bundle.

### Thread D: Model Inference Command

Add after a backend crate exists:

Status: implemented for raw RGB/BGR frames and DETR/YOLOS-style ONNX outputs
behind `video-analysis-cli --features onnxruntime`. The sampled video/report
integration milestone is implemented through `vanalyze analyze`.

```bash
vanalyze models run \
  --manifest .model-runtime/yolos-tiny/main/manifest.json \
  --backend onnx \
  --input frame.rgb \
  --width 640 \
  --height 480 \
  --pixel-format rgb24
```

For video input:

```bash
vanalyze analyze \
  --input video.mp4 \
  --model-manifest .model-runtime/yolos-tiny/main/manifest.json \
  --model-backend onnx \
  --sample-every-frames 30 \
  --output analysis.json
```

Output contract:

- JSON by default for model predictions and analysis reports.
- `vanalyze analyze` reports `framesProcessed`, detection scene/cut counts, a
  retained `AnalysisDataset`, and record-count summaries. Without a model
  manifest it remains a scene/dataset report command; model inference remains
  ONNX feature-gated.
- CSV remains available for scene lists/stats.
- Use core `Observation`/`AnalysisEvent` report shapes already represented in
  `video-analysis-use-cases`.

Tests:

- Parser tests for `models run` and `analyze`.
- Non-network fake backend tests to verify frames are decoded/sampled and
  observations are serialized.
- Ignored external test for a real ONNX model bundle.

### Thread E: Use-Case Reuse In `vanalyze`

Decide whether `video-analysis-cli` should depend on `video-analysis-use-cases`.

Option 1: `vanalyze use-cases youtube-video`

- Lowest implementation cost.
- Reuses `YoutubeVideoRequest` and report writing.
- Pulls use-case/radiance dependencies into the CLI crate.

Option 2: keep `video-analysis-use-cases` as the composed binary

- Keeps `vanalyze` focused on primitives.
- Avoids a large CLI dependency graph.
- Requires documentation updates instead of code.

Recommendation:

- Add primitive commands to `vanalyze` first: `models inspect`, then
  `models run`, then `analyze`.
- Keep the use-case binary separate until there is a shared command/options
  crate or a clear product decision to merge them.

### Minimum CLI Milestone

A useful first milestone that does not require native inference:

- `vanalyze models inspect`
- `vanalyze models download` already done
- README examples showing:
  - list presets
  - download bundle
  - inspect bundle

After that, the native backend thread can wire:

- `vanalyze models run --backend onnx`
- `vanalyze analyze --model-backend onnx`

## Open Decisions

- Should the root facade re-export native backend crates, or should users import
  backend crates explicitly?
- Should `vanalyze` absorb use-case commands, or remain a primitive workflow
  CLI while `video-analysis-use-cases` stays separate?
- Which first model should be the external inference fixture? Prefer a tiny
  ONNX model that is stable on Hugging Face and small enough for CI opt-in.
