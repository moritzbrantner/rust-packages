# External Runtime And Model Checks

This is the strict verification gate for crates that can load an external
runtime, command, local model bundle, GPU backend, or token-gated model service.
It is intentionally separate from the default local checks: default builds and
`bun run test` must stay deterministic and free of downloads, native runtimes,
GPU requirements, or service credentials.

Run the required GPU-inclusive, token-gated pass from the repository root:

```bash
scripts/check_external_runtime_models.sh --strict --gpu --token-gated \
  --report target/external-runtime-model-checks/report.json
```

The script writes:

- `target/external-runtime-model-checks/report.json`
- `target/external-runtime-model-checks/report.md`
- per-check logs under `target/external-runtime-model-checks/logs/`

The script exits non-zero unless every required crate is `pass`.

## Setup Behavior

By default, this gate verifies setup and runs smokes. It does not install tools,
download models, or repair local roots.

To provision ignored local tool/model roots before verification, pass `--setup`:

```bash
scripts/check_external_runtime_models.sh --setup --strict --gpu --token-gated \
  --report target/external-runtime-model-checks/report.json
```

Setup mode reuses the existing idempotent setup scripts:

```bash
bash scripts/setup_model_external_tools.sh all bundles
bash scripts/setup_e2e_external_tools.sh all
bash scripts/setup_whisper_cpp_external_model.sh
scripts/setup_colmap_test_video.sh
NERFSTUDIO_REQUIRE_CUDA=1 bash scripts/setup_radiance_external_tools.sh training
```

Without `--setup`, missing tools, missing bundles, missing CUDA, missing
`HF_TOKEN`, and missing `ORT_DYLIB_PATH` are reported as setup blockers.

## Statuses

- `pass`: the configured check ran and, when expected tests are declared,
  every expected test appeared as a passed test line.
- `fail`: the check ran but failed, timed out, or strict mode detected a skip or
  declared expected test evidence was missing.
- `blocked-setup`: required setup is missing, such as CUDA, `HF_TOKEN`, model
  bundles, absolute local paths, external commands, or `ORT_DYLIB_PATH`.
- `missing-coverage`: the crate is in the required matrix but the repository has
  no real ignored load-and-run smoke for that external surface yet.
- `excluded`: the crate only has checks that were intentionally omitted by the
  selected options, such as GPU rows when `--gpu` is not passed.

In `--strict`, ignored tests that print explicit skip messages are failures.
Cargo lib/doc-test harness sections that print `running 0 tests` are ignored
when all expected integration smoke tests passed. This prevents opt-in smokes
from silently passing while avoiding false failures from Cargo's empty harness
output.

Each check row records additive evidence fields in the JSON report:

- `evidence`: expected test names found in passed test lines.
- `missing_evidence`: expected test names not found in passed test lines.
- `skip_evidence`: explicit strict skip lines matched in command output.

## Required Environment

The orchestrator records non-secret environment values in the report. Secret
tokens are redacted.

Common absolute path defaults:

```bash
export SMOKE_ROOT="${SMOKE_ROOT:-$HOME/.local/share/video-analysis-smoke}"
export TRANSCRIPTION_MODEL_BUNDLE="$SMOKE_ROOT/whisper-tiny"
export TRANSCRIPTION_AUDIO_PATH="$SMOKE_ROOT/audio/native-transcription-smoke.wav"
export TRANSCRIPTION_MEDIA_PATH="$SMOKE_ROOT/audio/native-transcription-smoke.wav"
export DIARIZATION_AUDIO_PATH="$SMOKE_ROOT/audio/native-transcription-smoke.wav"
export ALIGNMENT_MODEL_DIR="$SMOKE_ROOT/models/wav2vec2-base-960h/main"
export SPEAKER_EMBEDDING_MODEL_BUNDLE="$SMOKE_ROOT/models/wespeaker-voxceleb-resnet34-LM/main"
export TEXT_CLASSIFICATION_DISTILBERT_BUNDLE="$PWD/.model-runtime/distilbert-sst2/main/manifest.json"
export TEXT_CLASSIFICATION_BART_MNLI_BUNDLE="$PWD/.model-runtime/xenova-bart-large-mnli-onnx/main/manifest.json"
export RUNTIME_ONNX_SMOKE_MODEL="$PWD/.model-runtime/roberta-base-squad2-onnx/main/files/onnx/model_quantized.onnx"
export RECOGNITION_ONNX_MODEL_BUNDLE="$PWD/.model-runtime/xenova-detr-resnet-50-onnx/main"
export COLMAP_TEST_VIDEO_PATH="$PWD/prototypes/web/video-analysis-web/public/samples/video/test-video.mp4"
export COLMAP_SPARSE_TEXT_DIR="$PWD/.external-test-tools/colmap-runs/test-video/sparse_txt"
export SPEAKER_EMBEDDING_MODEL_FILE="speaker-embedding.onnx"
export SPEAKER_EMBEDDING_DIMENSION="256"
```

`ORT_DYLIB_PATH` must be absolute. If it is unset, the orchestrator tries to
discover the ONNX Runtime shared library under the local model or WhisperX
Python environments:

```text
.external-test-tools/model-python-venv/
.audio-tools/whisperx-venv/
```

Token-gated checks require:

```bash
export HF_TOKEN=...
```

The token must be accepted by Hugging Face and must have access to the pyannote
gated model used by the WhisperX diarization path.

GPU checks require a CUDA-capable host visible to:

```bash
nvidia-smi
```

Nerfstudio training checks also require:

```bash
ns-process-data
ns-train
ns-export
```

## Required Crate Matrix

| Area | Crates |
| --- | --- |
| Runtime/model spine | `moritzbrantner-model-runtime`, `moritzbrantner-runtime-onnx`, `moritzbrantner-video-analysis` |
| Image ONNX/models | `moritzbrantner-image-analysis-classification`, `moritzbrantner-image-analysis-captioning`, `moritzbrantner-image-analysis-detection`, `moritzbrantner-image-analysis-embeddings`, `moritzbrantner-image-analysis-ocr`, `moritzbrantner-image-analysis-comfyui` |
| Text native/models | `moritzbrantner-text-model-runtime`, `moritzbrantner-text-embeddings`, `moritzbrantner-text-linguistics`, `moritzbrantner-text-classification`, `moritzbrantner-text-question-answering`, `moritzbrantner-text-analysis`, `moritzbrantner-text-transcripts` |
| Audio native/models | `moritzbrantner-audio-analysis-io`, `moritzbrantner-audio-analysis-speakers`, `moritzbrantner-audio-analysis-transcription`, `moritzbrantner-audio-analysis-separation` |
| Video native/models | `moritzbrantner-video-analysis-ffmpeg`, `moritzbrantner-video-analysis-split`, `moritzbrantner-video-analysis-posture`, `moritzbrantner-video-analysis-recognition`, `moritzbrantner-video-analysis-cli` |
| Radiance / reconstruction | `moritzbrantner-video-analysis-radiance-fields`, `moritzbrantner-video-analysis-radiance-io`, `moritzbrantner-video-analysis-radiance-pipeline`, `moritzbrantner-video-analysis-sfm`, `moritzbrantner-video-analysis-mvs`, `moritzbrantner-video-analysis-reconstruction` |
| Workflow integration | `moritzbrantner-video-analysis-use-cases` |

The report includes a feature inventory from `cargo metadata --no-deps` for
packages exposing relevant features such as `external-tests`, `local-models`,
`local-onnx`, `onnx`, `candle`, `cuda`, `native`, `ffmpeg-*`, or
`model-bundles`.

## Current Setup-Dependent Rows

The strict gate does not mark undocumented gaps as successful, and it now
distinguishes setup blockers from missing coverage. Current setup-dependent rows
are:

- `moritzbrantner-video-analysis-posture`: requires `ORT_DYLIB_PATH` and
  `POSTURE_ONNX_MODEL_BUNDLE`, normally
  `.model-runtime/xenova-yolov8n-pose-onnx/main`.
- `moritzbrantner-video-analysis-mvs`: requires COLMAP, sparse text output
  under `COLMAP_SPARSE_TEXT_DIR`, source frames under `COLMAP_MVS_IMAGE_DIR`,
  and sparse binary output under `COLMAP_MVS_SPARSE_DIR`.
- `moritzbrantner-image-analysis-comfyui`: reports `blocked-setup` until
  `COMFYUI_URL` points at a real running server. Use:

  ```bash
  export COMFYUI_URL=http://127.0.0.1:8188
  export COMFYUI_CHECKPOINT=<checkpoint-installed-on-that-server>
  cargo test -p moritzbrantner-image-analysis-comfyui --test external_comfyui_smoke \
    comfyui_submits_generation_workflow_when_configured -- --ignored --nocapture
  ```

- `moritzbrantner-video-analysis-recognition`: reports `blocked-setup` until
  `RECOGNITION_ONNX_MODEL_BUNDLE` points at a local ONNX object-detection
  bundle.
- `moritzbrantner-video-analysis-sfm`, `moritzbrantner-video-analysis-radiance-io`,
  `moritzbrantner-video-analysis-radiance-pipeline`, and
  `moritzbrantner-video-analysis-reconstruction` report `blocked-setup` until
  COLMAP can run successfully and produces supported sparse text output under
  `COLMAP_SPARSE_TEXT_DIR`.
- `moritzbrantner-video-analysis-radiance-fields` is `excluded` in non-GPU
  reports and remains a GPU/Nerfstudio follow-up for `--gpu` runs.

Each new smoke should remain ignored, feature-gated or env-gated, and outside
the default `bun run test` path.
