#!/usr/bin/env bash
set -euo pipefail

WASM_PACK_VERSION="${WASM_PACK_VERSION:-0.14.0}"

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
package_dir="$(cd -- "${script_dir}/.." && pwd)"
repo_root="$(cd -- "${package_dir}/../.." && pwd)"

tool_root="${repo_root}/.cargo-target/wasm-pack-tools"
tool_bin="${tool_root}/bin/wasm-pack"

wasm_pack=""
if command -v wasm-pack >/dev/null 2>&1; then
  candidate="$(command -v wasm-pack)"
  if "${candidate}" --version | grep -qx "wasm-pack ${WASM_PACK_VERSION}"; then
    wasm_pack="${candidate}"
  fi
fi

if [ -z "${wasm_pack}" ]; then
  if [ ! -x "${tool_bin}" ] || ! "${tool_bin}" --version | grep -qx "wasm-pack ${WASM_PACK_VERSION}"; then
    cargo install wasm-pack --locked --version "${WASM_PACK_VERSION}" --root "${tool_root}" --force
  fi
  wasm_pack="${tool_bin}"
fi

exec "${wasm_pack}" build "${repo_root}/crates/bindings/video-analysis-core-wasm" \
  --target web \
  --release \
  --out-dir "${package_dir}/pkg" \
  --out-name index
