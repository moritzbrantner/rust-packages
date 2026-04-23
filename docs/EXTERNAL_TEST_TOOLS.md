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
  `.external-test-tools/`, `.audio-tools/`, `.video-analysis-models/`).
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

The radiance pipeline needs `ffmpeg`, `colmap`, and `ns-process-data` for the
current ignored external test:

```bash
bash scripts/setup_radiance_external_tools.sh
cargo test -p video-analysis-radiance-pipeline \
  --features external-tests --test external_pipeline -- --ignored
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
