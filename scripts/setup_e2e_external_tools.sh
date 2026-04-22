#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TOOLS_DIR="${EXTERNAL_TEST_TOOLS_DIR:-"$ROOT_DIR/.external-test-tools"}"
source "$ROOT_DIR/scripts/lib_external_test_tools.sh"

if [[ "$#" -eq 0 ]]; then
  set -- fast
fi

for tool in "$@"; do
  case "$tool" in
    fast)
      install_ffmpeg
      install_demucs
      install_whisper
      install_yt_dlp
      install_colmap
      install_nerfstudio
      ;;
    all)
      install_ffmpeg
      install_demucs
      install_whisper
      install_yt_dlp
      install_colmap
      NERFSTUDIO_VERIFY_TRAINING_CLI=1 install_nerfstudio
      ;;
    audio)
      install_ffmpeg
      install_demucs
      ;;
    radiance)
      install_ffmpeg
      install_colmap
      install_nerfstudio
      ;;
    ffmpeg) install_ffmpeg ;;
    demucs) install_demucs ;;
    whisper) install_whisper ;;
    yt-dlp|ytdlp) install_yt_dlp ;;
    colmap) install_colmap ;;
    nerfstudio) install_nerfstudio ;;
    conda) install_conda ;;
    *)
      fail "unknown e2e tool '$tool' (expected fast, all, audio, radiance, ffmpeg, demucs, whisper, yt-dlp, colmap, nerfstudio, or conda)"
      ;;
  esac
done
