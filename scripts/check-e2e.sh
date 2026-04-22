#!/usr/bin/env bash
set -euo pipefail

need() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "error: required external test command '$1' is unavailable" >&2
    echo "install '$1' or put it on PATH before running scripts/check-e2e.sh" >&2
    exit 1
  fi
}

need ffmpeg
need ffprobe
need demucs
need whisper
need yt-dlp
need colmap
need ns-process-data

FFMPEG_EXTERNAL_TESTS=1 cargo test -p audio-analysis-io --test ffmpeg_decode
cargo test -p video-analysis-ffmpeg --features ffmpeg-tests
RUN_REAL_DEMUCS_TESTS=1 cargo test -p audio-analysis-separation --features external-tests real_demucs_smoke_test_when_requested -- --ignored
cargo test -p text-analysis-transcription --features external-tests --test whisper_cli -- --ignored
cargo test -p video-analysis-models --features external-tests --test huggingface_download -- --ignored
cargo test -p video-analysis-split --features external-tests --test ffmpeg_split -- --ignored
cargo test -p video-analysis-cli --test cli_smoke vanalyze_detect_writes_scene_csv_for_generated_video -- --ignored
cargo test -p video-analysis-use-cases --test external_tools -- --ignored
cargo test -p video-analysis-radiance-pipeline --features external-tests --test external_pipeline -- --ignored
