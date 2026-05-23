#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
wasm-pack build "$ROOT_DIR/crates/bindings/maps-kernels-core-wasm" --target web --out-dir "$ROOT_DIR/packages/maps-kernels-core-wasm/pkg"
