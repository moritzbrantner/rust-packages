# Video Analysis Model And CLI Next Steps

This document scopes the next two gaps in the video-analysis packages so follow-up
threads can pick up implementation without redoing discovery.

Current state:

- `video-analysis-models` owns model specs, Hugging Face downloads, bundle
  materialization, prediction normalization, analyzer adapters, and external
  command backends.
- `video-analysis-use-cases` can already attach external object, OCR, and text
  model commands to the `youtube-video` workflow.
- `video-analysis-cli` currently exposes scene detection/list/split plus model
  preset/download/inspect commands. It does not run model inference.
- The workspace has no ONNX Runtime, Candle, tokenizers, safetensors, ndarray,
  or image preprocessing dependency yet.
- Optional Python dependencies for model-backend experiments can be installed
  idempotently with `scripts/setup_model_external_tools.sh`.

## Plan 1: Native Model Backends

Goal: add a first native inference path that consumes `ModelBundle` and emits
the existing `RawPrediction` values through `VisionModelBackend` or
`TextModelBackend`.

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

Add a new crate:

```text
crates/video/video-analysis-onnx
```

Workspace updates:

- Add `video-analysis-onnx` to `Cargo.toml` workspace members.
- Add `video-analysis-onnx.workspace = true` under workspace dependencies.
- Add an optional root facade re-export if the crate is stable enough.

Suggested dependencies:

- `video-analysis-core`
- `video-analysis-models`
- `serde`
- `serde_json`
- Runtime dependency behind a feature:
  - `ort` or another maintained ONNX Runtime crate.
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
onnxruntime = ["dep:ort"]
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

- Can load a bundle containing one `.onnx` file and `config.json`.
- Validates there is exactly one supported ONNX model file, or returns
  `DetectError::InvalidArgument`.
- Produces deterministic `RawPrediction` values from a fake/session test double
  without network access.
- Has one ignored external test that downloads/materializes a tiny ONNX-capable
  model bundle and runs one inference on a synthetic frame.
- Does not make ONNX Runtime a required dependency for the whole workspace.

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

## Plan 2: CLI Integration Beyond Scene Detection

Goal: make `vanalyze` run more of the video-analysis stack without requiring
users to discover lower-level crates or the separate use-case binary.

Recommended sequence: add narrow, composable commands first, then promote a full
report command once model execution is real.

### Thread C: Bundle Inspection Command

Add:

```bash
vanalyze models inspect --name yolos-tiny --revision main
vanalyze models inspect --bundle-dir .video-analysis-models --name yolos-tiny
vanalyze models inspect --manifest .video-analysis-models/yolos-tiny/main/manifest.json
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

```bash
vanalyze models run \
  --manifest .video-analysis-models/yolos-tiny/main/manifest.json \
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
  --model-manifest .video-analysis-models/yolos-tiny/main/manifest.json \
  --model-backend onnx \
  --visual-sample-every 30 \
  --output analysis.json
```

Output contract:

- JSON by default for model predictions and analysis reports.
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

- Which ONNX Runtime crate/version is acceptable for this workspace?
- Should runtime downloads/binaries be allowed in normal builds, or only behind
  feature flags and external tests?
- Should the root facade re-export native backend crates, or should users import
  backend crates explicitly?
- Should `vanalyze` absorb use-case commands, or remain a primitive workflow
  CLI while `video-analysis-use-cases` stays separate?
- Which first model should be the external inference fixture? Prefer a tiny
  ONNX model that is stable on Hugging Face and small enough for CI opt-in.
