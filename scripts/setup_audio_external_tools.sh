#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TOOLS_DIR="${AUDIO_TOOLS_DIR:-"$ROOT_DIR/.audio-tools"}"
source "$ROOT_DIR/scripts/lib_external_test_tools.sh"

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
