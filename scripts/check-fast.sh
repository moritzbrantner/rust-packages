#!/usr/bin/env bash
set -euo pipefail

cargo test --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
