#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(git rev-parse --show-toplevel 2>/dev/null)"
if [[ -z "${ROOT_DIR:-}" ]]; then
  echo "error: not inside a git repository" >&2
  exit 1
fi

cd "$ROOT_DIR"

only_paths=()
base_ref=""
while [[ $# -gt 0 ]]; do
  case "$1" in
    --base)
      if [[ -z "${2:-}" ]]; then
        echo "error: --base requires a git revision" >&2
        exit 2
      fi
      base_ref="$2"
      shift 2
      ;;
    --only)
      if [[ -z "${2:-}" ]]; then
        echo "error: --only requires a snapshot path" >&2
        exit 2
      fi
      only_paths+=("$2")
      shift 2
      ;;
    *)
      echo "error: unknown argument: $1" >&2
      exit 2
      ;;
  esac
done

if [[ -n "$base_ref" ]]; then
  scope_file="$(mktemp -t generated-snapshot-scope.XXXXXX.json)"
  trap 'rm -f "$scope_file"' EXIT
  python3 scripts/check_changed_scope.py --base "$base_ref" > "$scope_file"
  mapfile -t scoped_paths < <(
    python3 - "$scope_file" <<'PY'
import json
import sys

with open(sys.argv[1], encoding="utf-8") as handle:
    for path in json.load(handle).get("snapshot_paths") or []:
        print(path)
PY
  )
  only_paths+=("${scoped_paths[@]}")
  if (( ${#only_paths[@]} == 0 )); then
    echo "generated snapshots: none affected"
    exit 0
  fi
fi

should_check_path() {
  local path="$1"
  if (( ${#only_paths[@]} == 0 )); then
    return 0
  fi
  local wanted
  for wanted in "${only_paths[@]}"; do
    if [[ "$path" == "$wanted" ]]; then
      return 0
    fi
  done
  return 1
}

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
  if ! should_check_path "$path"; then
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
