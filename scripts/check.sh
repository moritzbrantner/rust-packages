#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-${TEST_MAX_WORKERS:-2}}"
RUST_TEST_THREADS="${RUST_TEST_THREADS:-${TEST_MAX_WORKERS:-2}}"
export RUST_TEST_THREADS

"$ROOT_DIR/scripts/check-preflight.sh"
cargo test --jobs "$CARGO_BUILD_JOBS" -p moenarch-video-analysis-ffmpeg --features ffmpeg-tests
"$ROOT_DIR/scripts/check-e2e.sh"
