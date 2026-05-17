#!/usr/bin/env bash
set -euo pipefail

cargo test --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
bun run text-wasm:test
bun run ui:build
bun run web:typecheck
bun run web:build
bun run ui:test
bun run web:test
