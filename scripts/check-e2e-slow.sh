#!/usr/bin/env bash
set -euo pipefail

need() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "error: required slow external test command '$1' is unavailable" >&2
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
