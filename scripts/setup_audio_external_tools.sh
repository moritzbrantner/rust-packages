#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TOOLS_DIR="${AUDIO_TOOLS_DIR:-"$ROOT_DIR/.audio-tools"}"
BIN_DIR="$TOOLS_DIR/bin"
DEMUCS_VENV="$TOOLS_DIR/demucs-venv"
DEMUCS_CONDA_PREFIX="$TOOLS_DIR/demucs-conda"

mkdir -p "$BIN_DIR"

install_ffmpeg() {
  if command -v ffmpeg >/dev/null 2>&1 && command -v ffprobe >/dev/null 2>&1; then
    ffmpeg -version >/dev/null
    ffprobe -version >/dev/null
    echo "verified ffmpeg and ffprobe"
    return
  fi

  if command -v apt-get >/dev/null 2>&1; then
    sudo apt-get update
    sudo apt-get install -y ffmpeg
  elif command -v brew >/dev/null 2>&1; then
    brew install ffmpeg
  else
    echo "error: cannot install ffmpeg automatically; install ffmpeg and ffprobe manually" >&2
    exit 1
  fi

  ffmpeg -version >/dev/null
  ffprobe -version >/dev/null
  echo "verified ffmpeg and ffprobe"
}

publish_bin_path() {
  if [[ -n "${GITHUB_PATH:-}" ]]; then
    echo "$BIN_DIR" >> "$GITHUB_PATH"
  else
    echo "add this to PATH before running external-tool tests:"
    echo "  export PATH=\"$BIN_DIR:\$PATH\""
  fi
}

install_demucs_with_venv() {
  if ! command -v python3 >/dev/null 2>&1; then
    echo "python3 is unavailable; cannot use venv installer" >&2
    return 1
  fi
  python3 -m venv "$DEMUCS_VENV"
  "$DEMUCS_VENV/bin/python" -m pip install --upgrade pip
  "$DEMUCS_VENV/bin/python" -m pip install demucs
  ln -sf "$DEMUCS_VENV/bin/demucs" "$BIN_DIR/demucs"
  "$BIN_DIR/demucs" --help >/dev/null
  echo "verified demucs in venv at $BIN_DIR/demucs"
}

install_demucs_with_conda() {
  local conda_bin=""
  if command -v conda >/dev/null 2>&1; then
    conda_bin="$(command -v conda)"
  elif command -v mamba >/dev/null 2>&1; then
    conda_bin="$(command -v mamba)"
  elif command -v micromamba >/dev/null 2>&1; then
    conda_bin="$(command -v micromamba)"
  else
    echo "conda, mamba, or micromamba is unavailable; cannot use conda installer" >&2
    return 1
  fi

  "$conda_bin" create -y -p "$DEMUCS_CONDA_PREFIX" python=3.11 pip
  "$DEMUCS_CONDA_PREFIX/bin/python" -m pip install --upgrade pip
  "$DEMUCS_CONDA_PREFIX/bin/python" -m pip install demucs
  ln -sf "$DEMUCS_CONDA_PREFIX/bin/demucs" "$BIN_DIR/demucs"
  "$BIN_DIR/demucs" --help >/dev/null
  echo "verified demucs in conda env at $BIN_DIR/demucs"
}

install_demucs() {
  local mode="${AUDIO_DEMUCS_INSTALLER:-auto}"
  case "$mode" in
    auto)
      if install_demucs_with_venv; then
        publish_bin_path
        return
      fi
      echo "venv Demucs install failed; falling back to conda" >&2
      install_demucs_with_conda
      ;;
    venv)
      install_demucs_with_venv
      ;;
    conda)
      install_demucs_with_conda
      ;;
    *)
      echo "error: AUDIO_DEMUCS_INSTALLER must be auto, venv, or conda" >&2
      exit 1
      ;;
  esac

  publish_bin_path
}

install_conda() {
  if command -v conda >/dev/null 2>&1 || command -v mamba >/dev/null 2>&1 || command -v micromamba >/dev/null 2>&1; then
    echo "verified conda-compatible installer"
    return
  fi

  if command -v curl >/dev/null 2>&1; then
    local installer="$TOOLS_DIR/micromamba.tar.bz2"
    local os
    local arch
    os="$(uname | tr '[:upper:]' '[:lower:]')"
    arch="$(uname -m)"
    case "$arch" in
      x86_64|amd64) arch="64" ;;
      aarch64|arm64) arch="aarch64" ;;
      *) echo "error: unsupported micromamba architecture $arch" >&2; exit 1 ;;
    esac
    curl -Ls "https://micro.mamba.pm/api/micromamba/$os-$arch/latest" -o "$installer"
    tar -xjf "$installer" -C "$TOOLS_DIR" bin/micromamba
    ln -sf "$TOOLS_DIR/bin/micromamba" "$BIN_DIR/micromamba"
    "$BIN_DIR/micromamba" --version >/dev/null
    publish_bin_path
    echo "verified micromamba at $BIN_DIR/micromamba"
    return
  fi

  echo "error: cannot install micromamba because curl is unavailable" >&2
  exit 1
}

if [[ "$#" -eq 0 ]]; then
  set -- ffmpeg demucs
fi

for tool in "$@"; do
  case "$tool" in
    ffmpeg) install_ffmpeg ;;
    demucs) install_demucs ;;
    conda) install_conda ;;
    *)
      echo "error: unknown audio tool '$tool' (expected ffmpeg, conda, or demucs)" >&2
      exit 1
      ;;
  esac
done
