#!/usr/bin/env bash
# Shared installers for optional external integration/e2e test tools.

if [[ -z "${BASH_VERSION:-}" ]]; then
  echo "error: scripts/lib_external_test_tools.sh must be sourced from bash" >&2
  exit 1
fi

EXTERNAL_TOOLS_LIB_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="${ROOT_DIR:-"$(cd "$EXTERNAL_TOOLS_LIB_DIR/.." && pwd)"}"
TOOLS_DIR="${TOOLS_DIR:-"${EXTERNAL_TEST_TOOLS_DIR:-"$ROOT_DIR/.external-test-tools"}"}"
BIN_DIR="${BIN_DIR:-"$TOOLS_DIR/bin"}"

PYTHON_CLI_VENV="${PYTHON_CLI_VENV:-"$TOOLS_DIR/python-venv"}"
DEMUCS_VENV="${DEMUCS_VENV:-"$PYTHON_CLI_VENV"}"
DEMUCS_CONDA_PREFIX="${DEMUCS_CONDA_PREFIX:-"$TOOLS_DIR/demucs-conda"}"
WHISPER_VENV="${WHISPER_VENV:-"$PYTHON_CLI_VENV"}"
YTDLP_VENV="${YTDLP_VENV:-"$PYTHON_CLI_VENV"}"
COLMAP_CONDA_PREFIX="${COLMAP_CONDA_PREFIX:-"$TOOLS_DIR/colmap-conda"}"
NERFSTUDIO_CONDA_PREFIX="${NERFSTUDIO_CONDA_PREFIX:-"$TOOLS_DIR/nerfstudio-conda"}"
NERFSTUDIO_VENV="${NERFSTUDIO_VENV:-"$PYTHON_CLI_VENV"}"
MODEL_PYTHON_VENV="${MODEL_PYTHON_VENV:-"$TOOLS_DIR/model-python-venv"}"

mkdir -p "$BIN_DIR"

log() {
  echo "$*"
}

fail() {
  echo "error: $*" >&2
  exit 1
}

warn() {
  echo "warning: $*" >&2
}

path_for_command() {
  local name="$1"
  if [[ -x "$BIN_DIR/$name" ]]; then
    printf '%s\n' "$BIN_DIR/$name"
    return 0
  fi
  command -v "$name" 2>/dev/null
}

command_works() {
  local name="$1"
  shift
  local path
  path="$(path_for_command "$name" || true)"
  [[ -n "$path" ]] || return 1
  "$path" "$@" >/dev/null 2>&1
}

verify_command() {
  local name="$1"
  shift
  if ! command_works "$name" "$@"; then
    fail "expected '$name' to be available and runnable"
  fi
  log "verified $name"
}

publish_bin_path() {
  if [[ -n "${GITHUB_PATH:-}" ]]; then
    echo "$BIN_DIR" >> "$GITHUB_PATH"
  else
    log "add this to PATH before running external-tool tests:"
    log "  export PATH=\"$BIN_DIR:\$PATH\""
  fi
}

publish_bin_path_if_local() {
  local name="$1"
  if [[ -x "$BIN_DIR/$name" ]]; then
    publish_bin_path
  fi
}

ensure_python3() {
  command -v python3 >/dev/null 2>&1 || fail "python3 is unavailable"
}

create_venv() {
  local venv="$1"
  ensure_python3
  if [[ ! -x "$venv/bin/python" ]]; then
    python3 -m venv "$venv"
  fi
  "$venv/bin/python" -m pip install --upgrade pip setuptools wheel
}

symlink_bin() {
  local source="$1"
  local name="${2:-"$(basename "$source")"}"
  [[ -x "$source" ]] || fail "cannot publish missing executable '$source'"
  ln -sf "$source" "$BIN_DIR/$name"
}

existing_conda_bin() {
  if command -v conda >/dev/null 2>&1; then
    command -v conda
  elif command -v mamba >/dev/null 2>&1; then
    command -v mamba
  elif command -v micromamba >/dev/null 2>&1; then
    command -v micromamba
  elif [[ -x "$BIN_DIR/micromamba" ]]; then
    printf '%s\n' "$BIN_DIR/micromamba"
  elif [[ -x "$TOOLS_DIR/bin/micromamba" ]]; then
    printf '%s\n' "$TOOLS_DIR/bin/micromamba"
  else
    return 1
  fi
}

install_conda() {
  if existing_conda_bin >/dev/null 2>&1; then
    log "verified conda-compatible installer"
    return
  fi

  command -v curl >/dev/null 2>&1 || fail "cannot install micromamba because curl is unavailable"

  local installer="$TOOLS_DIR/micromamba.tar.bz2"
  local os
  local arch
  os="$(uname | tr '[:upper:]' '[:lower:]')"
  arch="$(uname -m)"
  case "$arch" in
    x86_64|amd64) arch="64" ;;
    aarch64|arm64) arch="aarch64" ;;
    *) fail "unsupported micromamba architecture $arch" ;;
  esac

  curl -Ls "https://micro.mamba.pm/api/micromamba/$os-$arch/latest" -o "$installer"
  tar -xjf "$installer" -C "$TOOLS_DIR" bin/micromamba
  symlink_bin "$TOOLS_DIR/bin/micromamba" micromamba
  "$BIN_DIR/micromamba" --version >/dev/null
  publish_bin_path
  log "verified micromamba at $BIN_DIR/micromamba"
}

conda_bin() {
  existing_conda_bin || {
    install_conda
    existing_conda_bin
  }
}

create_conda_prefix() {
  local prefix="$1"
  local python_version="$2"
  local conda
  conda="$(conda_bin)"
  if [[ -x "$prefix/bin/python" ]]; then
    log "verified existing conda prefix at $prefix"
    return
  fi
  MAMBA_ROOT_PREFIX="${MAMBA_ROOT_PREFIX:-"$TOOLS_DIR/micromamba-root"}" \
    "$conda" create -y -p "$prefix" "python=$python_version" pip
}

conda_install_prefix() {
  local prefix="$1"
  shift
  local conda
  conda="$(conda_bin)"
  MAMBA_ROOT_PREFIX="${MAMBA_ROOT_PREFIX:-"$TOOLS_DIR/micromamba-root"}" \
    "$conda" install -y -p "$prefix" "$@"
}

install_ffmpeg() {
  if command_works ffmpeg -version && command_works ffprobe -version; then
    log "verified ffmpeg and ffprobe"
    return
  fi

  if command -v apt-get >/dev/null 2>&1; then
    command -v sudo >/dev/null 2>&1 || fail "apt-get install requires sudo"
    sudo apt-get update
    sudo apt-get install -y ffmpeg
  elif command -v brew >/dev/null 2>&1; then
    brew install ffmpeg
  else
    fail "cannot install ffmpeg automatically; install ffmpeg and ffprobe manually"
  fi

  verify_command ffmpeg -version
  verify_command ffprobe -version
}

install_python_cli_package() {
  local name="$1"
  local venv="$2"
  local package="$3"
  shift 3
  if command_works "$name" "$@"; then
    log "verified $name"
    return
  fi

  create_venv "$venv"
  "$venv/bin/python" -m pip install "$package"
  symlink_bin "$venv/bin/$name" "$name"
  verify_command "$name" "$@"
  publish_bin_path
}

install_yt_dlp() {
  install_python_cli_package yt-dlp "$YTDLP_VENV" "${YTDLP_PIP_PACKAGE:-yt-dlp}" --version
}

install_whisper() {
  install_python_cli_package whisper "$WHISPER_VENV" "${WHISPER_PIP_PACKAGE:-openai-whisper}" --help
}

install_demucs_with_venv() {
  if command_works demucs --help; then
    log "verified demucs"
    return
  fi
  if ! command -v python3 >/dev/null 2>&1; then
    warn "python3 is unavailable; cannot use venv installer"
    return 1
  fi
  create_venv "$DEMUCS_VENV" || return 1
  "$DEMUCS_VENV/bin/python" -m pip install "${DEMUCS_PIP_PACKAGE:-demucs}" || return 1
  symlink_bin "$DEMUCS_VENV/bin/demucs" demucs || return 1
  command_works demucs --help || return 1
  log "verified demucs in venv at $DEMUCS_VENV"
}

install_demucs_with_conda() {
  if command_works demucs --help; then
    log "verified demucs"
    return
  fi
  create_conda_prefix "$DEMUCS_CONDA_PREFIX" "${DEMUCS_PYTHON_VERSION:-3.11}"
  "$DEMUCS_CONDA_PREFIX/bin/python" -m pip install --upgrade pip setuptools wheel
  "$DEMUCS_CONDA_PREFIX/bin/python" -m pip install "${DEMUCS_PIP_PACKAGE:-demucs}"
  symlink_bin "$DEMUCS_CONDA_PREFIX/bin/demucs" demucs
  verify_command demucs --help
  log "verified demucs in conda env at $BIN_DIR/demucs"
}

install_demucs() {
  local mode="${AUDIO_DEMUCS_INSTALLER:-auto}"
  case "$mode" in
    auto)
      if install_demucs_with_venv; then
        publish_bin_path_if_local demucs
        return
      fi
      warn "venv Demucs install failed; falling back to conda"
      install_demucs_with_conda
      ;;
    venv)
      install_demucs_with_venv
      ;;
    conda)
      install_demucs_with_conda
      ;;
    *)
      fail "AUDIO_DEMUCS_INSTALLER must be auto, venv, or conda"
      ;;
  esac
  publish_bin_path_if_local demucs
}

install_colmap_with_system_package() {
  if command_works colmap -h; then
    log "verified colmap"
    return
  fi

  if command -v apt-get >/dev/null 2>&1; then
    command -v sudo >/dev/null 2>&1 || return 1
    sudo apt-get update || return 1
    sudo apt-get install -y colmap || return 1
  elif command -v brew >/dev/null 2>&1; then
    brew install colmap || return 1
  else
    return 1
  fi

  command_works colmap -h || return 1
  log "verified colmap"
}

install_colmap_with_conda() {
  if command_works colmap -h; then
    log "verified colmap"
    return
  fi
  create_conda_prefix "$COLMAP_CONDA_PREFIX" "${COLMAP_PYTHON_VERSION:-3.11}"
  conda_install_prefix "$COLMAP_CONDA_PREFIX" -c conda-forge colmap
  symlink_bin "$COLMAP_CONDA_PREFIX/bin/colmap" colmap
  verify_command colmap -h
  publish_bin_path
}

install_colmap() {
  local mode="${EXTERNAL_COLMAP_INSTALLER:-auto}"
  case "$mode" in
    auto)
      if command_works colmap -h; then
        log "verified colmap"
        return
      fi
      if install_colmap_with_system_package; then
        return
      fi
      warn "system COLMAP install unavailable; falling back to a local conda prefix"
      install_colmap_with_conda
      ;;
    system)
      install_colmap_with_system_package || fail "cannot install COLMAP with the system package manager"
      ;;
    conda)
      install_colmap_with_conda
      ;;
    *)
      fail "EXTERNAL_COLMAP_INSTALLER must be auto, system, or conda"
      ;;
  esac
}

nerfstudio_cli_is_ready() {
  command_works ns-process-data --help || return 1
  if [[ "${NERFSTUDIO_VERIFY_TRAINING_CLI:-0}" == "1" ]]; then
    command_works ns-train --help || return 1
    command_works ns-export --help || return 1
  fi
}

verify_nerfstudio_cuda() {
  local python="$1"
  if [[ "${NERFSTUDIO_REQUIRE_CUDA:-0}" != "1" ]]; then
    return
  fi
  "$python" - <<'PY'
import torch
if not torch.cuda.is_available():
    raise SystemExit("torch.cuda.is_available() is false")
print(f"verified torch CUDA {torch.version.cuda}")
PY
}

install_tiny_cuda_nn_if_needed() {
  local python="$1"
  if [[ "${NERFSTUDIO_INSTALL_TINY_CUDA_NN:-1}" != "1" ]]; then
    return
  fi
  if "$python" - <<'PY' >/dev/null 2>&1
import tinycudann
PY
  then
    log "verified tiny-cuda-nn"
    return
  fi
  "$python" -m pip install ninja
  "$python" -m pip install 'git+https://github.com/NVlabs/tiny-cuda-nn/#subdirectory=bindings/torch'
}

install_nerfstudio_pip_packages() {
  local python="$1"
  "$python" -m pip install --upgrade pip setuptools wheel
  "$python" -m pip install \
    "torch==${NERFSTUDIO_TORCH_VERSION:-2.1.2+cu118}" \
    "torchvision==${NERFSTUDIO_TORCHVISION_VERSION:-0.16.2+cu118}" \
    --extra-index-url "${NERFSTUDIO_TORCH_INDEX_URL:-https://download.pytorch.org/whl/cu118}"
  install_tiny_cuda_nn_if_needed "$python"
  "$python" -m pip install "${NERFSTUDIO_PIP_PACKAGE:-nerfstudio}"
}

publish_nerfstudio_bins() {
  local prefix="$1"
  symlink_bin "$prefix/bin/ns-process-data" ns-process-data
  if [[ -x "$prefix/bin/ns-train" ]]; then
    symlink_bin "$prefix/bin/ns-train" ns-train
  fi
  if [[ -x "$prefix/bin/ns-export" ]]; then
    symlink_bin "$prefix/bin/ns-export" ns-export
  fi
  if [[ -x "$prefix/bin/ns-install-cli" ]]; then
    symlink_bin "$prefix/bin/ns-install-cli" ns-install-cli
  fi
}

install_nerfstudio_with_conda() {
  if nerfstudio_cli_is_ready; then
    log "verified Nerfstudio CLI"
    return
  fi

  create_conda_prefix "$NERFSTUDIO_CONDA_PREFIX" "${NERFSTUDIO_PYTHON_VERSION:-3.8}"
  if [[ "${NERFSTUDIO_INSTALL_CUDA_TOOLKIT:-1}" == "1" ]] && [[ ! -x "$NERFSTUDIO_CONDA_PREFIX/bin/nvcc" ]]; then
    conda_install_prefix "$NERFSTUDIO_CONDA_PREFIX" \
      -c "${NERFSTUDIO_CUDA_CHANNEL:-nvidia/label/cuda-11.8.0}" \
      "${NERFSTUDIO_CUDA_TOOLKIT_PACKAGE:-cuda-toolkit}"
  fi
  install_nerfstudio_pip_packages "$NERFSTUDIO_CONDA_PREFIX/bin/python"
  publish_nerfstudio_bins "$NERFSTUDIO_CONDA_PREFIX"
  verify_nerfstudio_cuda "$NERFSTUDIO_CONDA_PREFIX/bin/python"
  verify_command ns-process-data --help
  publish_bin_path
}

install_nerfstudio_with_venv() {
  if nerfstudio_cli_is_ready; then
    log "verified Nerfstudio CLI"
    return
  fi
  if [[ "${NERFSTUDIO_INSTALL_TINY_CUDA_NN:-1}" == "1" ]] && ! command -v nvcc >/dev/null 2>&1; then
    fail "NERFSTUDIO_INSTALLER=venv needs nvcc on PATH for tiny-cuda-nn; use conda or set NERFSTUDIO_INSTALL_TINY_CUDA_NN=0"
  fi
  create_venv "$NERFSTUDIO_VENV"
  install_nerfstudio_pip_packages "$NERFSTUDIO_VENV/bin/python"
  publish_nerfstudio_bins "$NERFSTUDIO_VENV"
  verify_nerfstudio_cuda "$NERFSTUDIO_VENV/bin/python"
  verify_command ns-process-data --help
  log "verified Nerfstudio CLI in venv at $NERFSTUDIO_VENV"
  publish_bin_path
}

install_nerfstudio() {
  local mode="${NERFSTUDIO_INSTALLER:-auto}"
  case "$mode" in
    auto)
      if nerfstudio_cli_is_ready; then
        log "verified Nerfstudio CLI"
        return
      fi
      if existing_conda_bin >/dev/null 2>&1 || command -v curl >/dev/null 2>&1; then
        install_nerfstudio_with_conda
      else
        install_nerfstudio_with_venv
      fi
      ;;
    conda)
      install_nerfstudio_with_conda
      ;;
    venv)
      install_nerfstudio_with_venv
      ;;
    *)
      fail "NERFSTUDIO_INSTALLER must be auto, conda, or venv"
      ;;
  esac
}

python_imports_work() {
  local python="$1"
  shift
  "$python" - "$@" <<'PY' >/dev/null 2>&1
import importlib
import sys

missing = []
for name in sys.argv[1:]:
    try:
        importlib.import_module(name)
    except Exception as error:
        missing.append(f"{name}: {error}")

if missing:
    raise SystemExit("\n".join(missing))
PY
}

verify_model_python_target() {
  local target="$1"
  local python="${2:-"$MODEL_PYTHON_VENV/bin/python"}"
  [[ -x "$python" ]] || return 1
  case "$target" in
    python)
      "$python" --version >/dev/null 2>&1
      ;;
    onnx)
      python_imports_work "$python" numpy PIL onnxruntime
      ;;
    transformers)
      python_imports_work "$python" numpy PIL torch transformers tokenizers safetensors huggingface_hub
      ;;
    all)
      verify_model_python_target onnx "$python" &&
        verify_model_python_target transformers "$python"
      ;;
    *)
      fail "unknown model Python dependency target '$target'"
      ;;
  esac
}

install_model_python_target() {
  local target="$1"
  if verify_model_python_target "$target"; then
    log "verified model $target Python dependencies in $MODEL_PYTHON_VENV"
    return
  fi

  create_venv "$MODEL_PYTHON_VENV"
  case "$target" in
    python)
      ;;
    onnx)
      "$MODEL_PYTHON_VENV/bin/python" -m pip install \
        "${MODEL_NUMPY_PIP_PACKAGE:-numpy}" \
        "${MODEL_PILLOW_PIP_PACKAGE:-pillow}" \
        "${MODEL_ONNXRUNTIME_PIP_PACKAGE:-onnxruntime}"
      ;;
    transformers)
      "$MODEL_PYTHON_VENV/bin/python" -m pip install \
        "${MODEL_NUMPY_PIP_PACKAGE:-numpy}" \
        "${MODEL_PILLOW_PIP_PACKAGE:-pillow}" \
        "${MODEL_TORCH_PIP_PACKAGE:-torch}" \
        "${MODEL_TRANSFORMERS_PIP_PACKAGE:-transformers}" \
        "${MODEL_TOKENIZERS_PIP_PACKAGE:-tokenizers}" \
        "${MODEL_SAFETENSORS_PIP_PACKAGE:-safetensors}" \
        "${MODEL_HUGGINGFACE_HUB_PIP_PACKAGE:-huggingface_hub}"
      ;;
    all)
      install_model_python_target onnx
      install_model_python_target transformers
      return
      ;;
    *)
      fail "unknown model Python dependency target '$target'"
      ;;
  esac
  verify_model_python_target "$target"
  log "verified model $target Python dependencies in $MODEL_PYTHON_VENV"
}
