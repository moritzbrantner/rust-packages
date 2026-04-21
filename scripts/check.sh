#!/usr/bin/env bash
set -euo pipefail

cargo test --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
bun run ui:build
bun run web:build

cat <<'MSG'

Optional FFmpeg integration coverage:
  cargo test -p video-analysis-ffmpeg --features ffmpeg-tests
MSG
