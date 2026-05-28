#!/usr/bin/env bash
set -euo pipefail

python3 scripts/generate_dependency_chart.py --check
python3 scripts/audit_package_surfaces.py --check
cargo test --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
bun run text-wasm:test
bun run audio-wasm:test
bun run video-wasm:test
bun run ui:build
bun run web:typecheck
bun run web:build
bun run ui:test
bun run web:test
