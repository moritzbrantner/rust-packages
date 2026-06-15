# External Test Tools

The normal test suite should stay independent from large native tools, GPUs,
models, and network downloads. Optional e2e/integration checks use local,
ignored tool roots instead:

- `.external-test-tools/` for general video/e2e/radiance tools.
- `.audio-tools/` for the audio-specific compatibility script.

## Repository Rule

Any package that needs installable external runtime assets must provide
idempotent scripts under `scripts/`. This includes Python environments, model
bundles, native CLIs, and similar non-git dependencies.

Required pattern:

- A setup script that is safe to re-run, verifies current state first, and only
  installs/repairs what is missing or invalid.
- A check script that verifies availability/integrity but does not install.
- A default local install root that is gitignored (for example
  `.external-test-tools/`, `.audio-tools/`, `.model-runtime/`).
- Usage documentation in `README.md` and/or this document.

The setup scripts are idempotent: each tool is verified first, and existing
working global or local commands are reused. Missing Python CLIs are installed
into one shared Python virtual environment under the ignored tool root, then
symlinked into `bin/`. Tools that need native packages or CUDA toolkit
management, such as COLMAP and the default Nerfstudio path, may use
Conda-compatible prefixes instead.

## General E2E Tools

Install the tools needed by `scripts/check-e2e.sh`:

```bash
bash scripts/setup_e2e_external_tools.sh
bash scripts/check-e2e.sh
```

You can install a subset:

```bash
bash scripts/setup_e2e_external_tools.sh ffmpeg whisper yt-dlp
bash scripts/setup_e2e_external_tools.sh radiance
```

The setup script publishes `.external-test-tools/bin` in GitHub Actions through
`GITHUB_PATH`. Locally, `scripts/check-e2e.sh` automatically adds
`.external-test-tools/bin` and `.audio-tools/bin` to `PATH` if those directories
exist.

## Opt-In Smoke Bundles

Use these bundles when a change touches the matching runtime surface. Check
commands are non-mutating; setup commands install or repair ignored local tool
roots only when needed.

| Bundle | Setup | Check / Run |
| --- | --- | --- |
| ONNX model smoke | `bash scripts/setup_model_external_tools.sh onnx` | `bash scripts/check_model_external_tools.sh onnx` then `scripts/check_onnx_external_smoke.sh` |
| Native transcription smoke | `bash scripts/setup_e2e_external_tools.sh whisper` plus model-bundle setup documented below | Run the env-gated native transcription commands below |
| Video scene benchmark smoke | `scripts/setup_video_scene_benchmarks.sh bbc` | `scripts/setup_video_scene_benchmarks.sh verify` then the bounded BBC evaluator command in `VIDEO_SCENE_DETECTION_PARITY.md` |
| Radiance / Nerfstudio smoke | `bash scripts/setup_radiance_external_tools.sh` | `scripts/check_e2e_external_tools.sh` or the dedicated radiance check required by the touched workflow |

Missing tools should skip only when the corresponding ignored test documents a
skip policy. If a `RUN_*` or equivalent opt-in environment variable is set,
missing setup should be treated as a failure by that smoke.

## Text Models

Default text tests do not download model files or require native runtimes. Opt-in text model tests are ignored and feature-gated:

```bash
scripts/sync_model_bundles.sh
cargo test -p moritzbrantner-text-model-runtime --features external-tests -- --ignored
cargo test -p moritzbrantner-text-linguistics --features external-tests -- --ignored
cargo test -p moritzbrantner-text-embeddings --features external-tests -- --ignored
cargo test -p moritzbrantner-text-transcripts --features native,external-tests -- --ignored
```

These tests reuse `.model-runtime` and report whether required tokenizer,
Candle, ONNX, or whisper.cpp files are locally present. The local ONNX defaults
for extractive QA, image classification, and image captioning are materialized
through `moritzbrantner-model-runtime`:

- `onnx-community/roberta-base-squad2-ONNX`
- `Xenova/vit-base-patch16-224`
- `Xenova/vit-gpt2-image-captioning`
- `Xenova/trocr-base-printed`

Live download/inference checks stay ignored and feature-gated:

```bash
cargo test -p moritzbrantner-text-question-answering --features external-tests -- --ignored
cargo test -p moritzbrantner-image-analysis-classification --features external-tests -- --ignored
cargo test -p moritzbrantner-image-analysis-captioning --features external-tests -- --ignored
cargo test -p moritzbrantner-image-analysis-ocr --features external-tests --test external_onnx_smoke -- --ignored
```

The native ONNX smoke suite can be run explicitly with:

```bash
scripts/check_onnx_external_smoke.sh
```

The helper auto-discovers the ONNX Runtime shared library from
`.external-test-tools/model-python-venv`, canonicalizes it to an absolute path,
and exports `ORT_DYLIB_PATH` before running native ONNX tests. If
`ORT_DYLIB_PATH` is set manually, use an absolute path. Relative paths can be
resolved from the test binary directory and may hang during ORT dynamic loading.

This helper prints the expected `.model-runtime/<bundle>/main/manifest.json`
paths before running the ignored classification, captioning, detection,
embedding, OCR, and text QA/runtime-helper commands. It is opt-in only and is not
called by `bun run test`, `scripts/check-fast.sh`, or default CI. Missing
bundles should skip or fail only according to the ignored test's own semantics;
default tests do not download or execute live models.

The OCR smoke expects the printed TrOCR ONNX manifest at
`.model-runtime/trocr-base-printed-onnx/main/manifest.json` and reads
`tests/fixtures/ocr/trocr-hello.png`.

Native transcription smoke tests follow the same opt-in rule. Candle Whisper
CUDA is the primary native target and requires `candle,cuda,model-bundles`
features plus `RUN_NATIVE_TRANSCRIPTION_TESTS=1`,
`TRANSCRIPTION_MODEL_BUNDLE`, and `TRANSCRIPTION_AUDIO_PATH`. Candle CPU remains
the local development fallback. CTC alignment and native diarization smoke
tests also require their documented env vars before they run. External Python
WhisperX remains compatibility/parity tooling only, and browser/WASM package
surfaces do not run native ASR.

Set a reusable smoke root before running native transcription checks:

```bash
export SMOKE_ROOT="${SMOKE_ROOT:-$HOME/.local/share/video-analysis-smoke}"
export TRANSCRIPTION_MODEL_BUNDLE="$SMOKE_ROOT/whisper-tiny"
export TRANSCRIPTION_AUDIO_PATH="$SMOKE_ROOT/audio/native-transcription-smoke.wav"
```

On the current maintainer workstation, `/usr/local/cuda` points at CUDA 13.3, so
the passing CUDA smoke also points `RUSTFLAGS`, `LIBRARY_PATH`, and
`LD_LIBRARY_PATH` at `$SMOKE_ROOT/cuda12-libs` for CUDA 12.3 libraries. Treat
that as host-specific setup, not a default contributor requirement. Native
Whisper timing now uses timestamp-token segments when available, projected word
timing inside those segments, and chunk/window fallback timing otherwise.
wav2vec2 CTC
bundle/config/tokenizer/preprocessor validation exists, and supported local
`Wav2Vec2ForCTC` safetensors bundles execute through Candle for native CTC
alignment. Unsupported architectures or weight layouts return
`unsupported_runtime`.

The multimodal workflow smoke tests still include optional command adapters
under `scripts/`, but image/video workflows now prefer Rust-side adapters when
configured without commands:

- red-car workflows use `image-analysis-detection::ColorBlobDetector` by
  default.
- image-person-edit can use `PersonDetectorConfig::Onnx` or test-only
  `PersonDetectorConfig::Stub`.
- ComfyUI submission is available through `image-analysis-comfyui` and no
  longer requires `scripts/run_comfyui_image_edit.py`.

- `scripts/opencv_red_car_detector.py`: external-model JSON protocol adapter for
  compatibility with older red-car runs. Requires Python with `cv2` and
  `numpy`.
- `scripts/setup_image_person_edit_external_fixture.sh`: downloads or reuses
  the Me at the Zoo video, extracts a face-containing local frame under
  `.external-test-tools/image-person-edit/`, validates it with
  `scripts/opencv_person_detector.py`, and prints the `IMAGE_PERSON_EDIT_*`
  exports for `workflow-image-person-edit-real-tools`.
- `scripts/run_comfyui_image_edit.py`: image-edit JSON protocol adapter that
  submits generated workflows to a ComfyUI server when `COMFYUI_URL` is set;
  kept for compatibility with older editor-command flows.

Optional env vars used by the ignored external smoke tests:

- `AUDIO_VOICE_ANALYSIS_TRANSCRIBER`: command path for a real transcription CLI
  used by `audio-voice-analysis`.
- `IMAGE_PERSON_EDIT_INPUT`: local image path for the real image-edit smoke
  test.
- `IMAGE_PERSON_EDIT_DETECTOR_COMMAND`: detector command path for the real
  image-edit smoke test.
- `IMAGE_PERSON_EDIT_EDITOR_COMMAND`: optional editor command path for the real
  image-edit smoke test.

```bash
bash scripts/setup_image_person_edit_external_fixture.sh
export IMAGE_PERSON_EDIT_INPUT="$PWD/.external-test-tools/image-person-edit/person-frame.jpg"
export IMAGE_PERSON_EDIT_DETECTOR_COMMAND=python3
export IMAGE_PERSON_EDIT_DETECTOR_ARGS="scripts/opencv_person_detector.py"
```

ComfyUI checks require a real running server:

```bash
export COMFYUI_URL=http://127.0.0.1:8188
export COMFYUI_CHECKPOINT=<checkpoint-installed-on-that-server>
cargo test -p moritzbrantner-image-analysis-comfyui --test external_comfyui_smoke \
  comfyui_submits_generation_workflow_when_configured -- --ignored --nocapture
```

## Model Backend Python Dependencies

Native or external model-backend experiments can use an isolated Python
environment under `.external-test-tools/model-python-venv`:

```bash
bash scripts/setup_model_external_tools.sh onnx
bash scripts/check_model_external_tools.sh onnx
```

Available targets:

- `python`: create and verify the virtual environment only.
- `onnx`: install and verify `numpy`, `pillow`, and `onnxruntime`.
- `transformers`: install and verify `numpy`, `pillow`, `torch`,
  `transformers`, `tokenizers`, `safetensors`, and `huggingface_hub`.
- `all`: verify/install both package groups.

The setup script is idempotent: it creates the virtual environment if missing,
checks imports first, and installs only missing package groups. The check script
does not install anything; it fails with the matching setup command if a target
is unavailable.

## Radiance / Nerfstudio

Future radiance conversion workflows need `ffmpeg`, `colmap`, and
`ns-process-data`:

```bash
bash scripts/setup_radiance_external_tools.sh
```

By default the Nerfstudio installer creates a local Conda-compatible prefix in
`.external-test-tools/nerfstudio-conda`. It installs Python 3.8, a CUDA 11.8
toolkit package, PyTorch 2.1.2 CUDA 11.8 wheels, tiny-cuda-nn, and Nerfstudio.
This follows the current Nerfstudio installation docs and does not install a
system NVIDIA driver.

If `NERFSTUDIO_INSTALLER=venv` is selected, Nerfstudio uses the same shared
Python venv as `yt-dlp`, Whisper, and Demucs. That mode requires `nvcc` on
`PATH` unless `NERFSTUDIO_INSTALL_TINY_CUDA_NN=0` is set.

Useful overrides:

```bash
NERFSTUDIO_INSTALLER=conda bash scripts/setup_radiance_external_tools.sh nerfstudio
NERFSTUDIO_INSTALLER=venv bash scripts/setup_radiance_external_tools.sh nerfstudio
NERFSTUDIO_REQUIRE_CUDA=1 bash scripts/setup_radiance_external_tools.sh training
NERFSTUDIO_INSTALL_TINY_CUDA_NN=0 bash scripts/setup_radiance_external_tools.sh nerfstudio
```

The `training` target verifies `ns-train` and `ns-export` in addition to
`ns-process-data`; use it for dedicated GPU runners.

## Installer Controls

- `EXTERNAL_TEST_TOOLS_DIR`: override `.external-test-tools/`.
- `RADIANCE_TOOLS_DIR`: override only the radiance setup root.
- `AUDIO_TOOLS_DIR`: override `.audio-tools/` for
  `setup_audio_external_tools.sh`.
- `PYTHON_CLI_VENV`: override the shared Python virtual environment path.
- `MODEL_TOOLS_DIR`: override `.external-test-tools/` for
  `setup_model_external_tools.sh`.
- `MODEL_PYTHON_VENV`: override the model-backend Python virtual environment.
- `MODEL_ONNXRUNTIME_PIP_PACKAGE`, `MODEL_TORCH_PIP_PACKAGE`, and
  `MODEL_TRANSFORMERS_PIP_PACKAGE`: override heavyweight model packages.
- `AUDIO_DEMUCS_INSTALLER`: `auto`, `venv`, or `conda`.
- `EXTERNAL_COLMAP_INSTALLER`: `auto`, `system`, or `conda`.
- `NERFSTUDIO_INSTALLER`: `auto`, `conda`, or `venv`.
- `NERFSTUDIO_TORCH_VERSION`, `NERFSTUDIO_TORCHVISION_VERSION`, and
  `NERFSTUDIO_TORCH_INDEX_URL`: override PyTorch wheels.
- `NERFSTUDIO_CUDA_CHANNEL` and `NERFSTUDIO_CUDA_TOOLKIT_PACKAGE`: override the
  Conda CUDA toolkit source/package.
