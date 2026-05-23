#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
wasm-pack build "$ROOT_DIR/crates/bindings/vector-analysis-index-wasm" --target web --out-dir "$ROOT_DIR/packages/vector-analysis-index-wasm/pkg"
