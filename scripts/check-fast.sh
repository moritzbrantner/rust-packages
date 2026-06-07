#!/usr/bin/env bash
set -euo pipefail

CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-${TEST_MAX_WORKERS:-2}}"
RUST_TEST_THREADS="${RUST_TEST_THREADS:-${TEST_MAX_WORKERS:-2}}"
export RUST_TEST_THREADS

scripts/check_generated_artifacts.sh
scripts/check_generated_snapshots.sh
python3 scripts/audit_crate_progress.py --check
python3 scripts/test_audit_crate_progress.py
python3 scripts/audit_crate_progress.py --changed --base "${BASE_REF:-origin/main}"
cargo test --jobs "$CARGO_BUILD_JOBS" --workspace --all-targets
python3 scripts/audit_package_surfaces.py --quality
cargo clippy --workspace --all-targets -- -D warnings
bun run text-wasm:test
bun run audio-wasm:test
bun run video-wasm:test
bun run ui:build
bun run web:typecheck
bun run web:build
bun run ui:test
bun run web:test
