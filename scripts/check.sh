#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

"$ROOT_DIR/scripts/check-fast.sh"
cargo test -p video-analysis-ffmpeg --features ffmpeg-tests
"$ROOT_DIR/scripts/check-e2e.sh"
