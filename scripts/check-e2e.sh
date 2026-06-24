#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-${TEST_MAX_WORKERS:-2}}"
RUST_TEST_THREADS="${RUST_TEST_THREADS:-${TEST_MAX_WORKERS:-2}}"
export RUST_TEST_THREADS

for local_bin in "$ROOT_DIR/.external-test-tools/bin" "$ROOT_DIR/.audio-tools/bin"; do
  if [[ -d "$local_bin" ]]; then
    export PATH="$local_bin:$PATH"
  fi
done

"$ROOT_DIR/scripts/check_e2e_external_tools.sh"

FFMPEG_EXTERNAL_TESTS=1 cargo test --jobs "$CARGO_BUILD_JOBS" -p moenarch-audio-analysis-io --test ffmpeg_decode
cargo test --jobs "$CARGO_BUILD_JOBS" -p moenarch-video-analysis-ffmpeg --features ffmpeg-tests
RUN_REAL_DEMUCS_TESTS=1 cargo test --jobs "$CARGO_BUILD_JOBS" -p moenarch-audio-analysis-separation --features external-tests real_demucs_smoke_test_when_requested -- --ignored
cargo test --jobs "$CARGO_BUILD_JOBS" -p moenarch-text-transcripts --features native,external-tests --test whisper_cli -- --ignored
cargo test --jobs "$CARGO_BUILD_JOBS" -p moenarch-video-analysis-split --features external-tests --test ffmpeg_split -- --ignored
cargo test --jobs "$CARGO_BUILD_JOBS" -p moenarch-video-analysis-cli --test cli_smoke vanalyze_detect_writes_scene_csv_for_generated_video -- --ignored
cargo test --jobs "$CARGO_BUILD_JOBS" -p moenarch-video-analysis-use-cases --test external_tools -- --ignored
