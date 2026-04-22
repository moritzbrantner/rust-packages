#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
for local_bin in "$ROOT_DIR/.external-test-tools/bin" "$ROOT_DIR/.audio-tools/bin"; do
  if [[ -d "$local_bin" ]]; then
    export PATH="$local_bin:$PATH"
  fi
done

need() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "error: required slow external test command '$1' is unavailable" >&2
    echo "run scripts/setup_radiance_external_tools.sh training or put '$1' on PATH before running scripts/check-e2e-slow.sh" >&2
    exit 1
  fi
}

need ns-train
need ns-export

cat <<'MSG'
Slow radiance training/export E2E is reserved for dedicated runners.
Add the training/export cargo test here once the workspace has a tiny stable
Nerfstudio fixture that finishes within the runner budget.
MSG
