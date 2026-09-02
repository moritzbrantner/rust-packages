#!/usr/bin/env bash
set -euo pipefail

readonly version="0.14.0"

if command -v wasm-pack >/dev/null 2>&1 && [[ "$(wasm-pack --version | awk '{print $2}')" == "$version" ]]; then
  exit 0
fi

for attempt in 1 2 3; do
  if cargo install wasm-pack --locked --version "$version"; then
    exit 0
  fi

  if [[ "$attempt" == "3" ]]; then
    printf 'failed to provision wasm-pack %s after %s attempts\n' "$version" "$attempt" >&2
    exit 1
  fi

  sleep "$((attempt * 5))"
done
