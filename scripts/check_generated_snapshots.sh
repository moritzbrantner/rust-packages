#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(git rev-parse --show-toplevel 2>/dev/null)"
if [[ -z "${ROOT_DIR:-}" ]]; then
  echo "error: not inside a git repository" >&2
  exit 1
fi

cd "$ROOT_DIR"

ALLOW_FILE="scripts/generated_snapshots.allow"
if [[ ! -f "$ALLOW_FILE" ]]; then
  echo "error: missing $ALLOW_FILE" >&2
  exit 1
fi

failures=0
while IFS=$'\t' read -r path check_command regenerate_command; do
  [[ -n "${path:-}" ]] || continue
  [[ "$path" =~ ^# ]] && continue
  if [[ -z "${check_command:-}" || -z "${regenerate_command:-}" ]]; then
    echo "malformed snapshot allowlist entry for $path" >&2
    failures=$((failures + 1))
    continue
  fi
  if [[ ! -f "$path" ]]; then
    echo "tracked generated snapshot missing: $path" >&2
    echo "regenerate with: $regenerate_command" >&2
    failures=$((failures + 1))
    continue
  fi
  if ! bash -lc "$check_command"; then
    echo >&2
    echo "generated snapshot is stale: $path" >&2
    echo "regenerate with: $regenerate_command" >&2
    failures=$((failures + 1))
  fi
done < "$ALLOW_FILE"

if (( failures > 0 )); then
  exit 1
fi

echo "generated snapshot check passed"
