#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
LOCK_FILE="${MODEL_BUNDLE_LOCK_FILE:-"$ROOT_DIR/scripts/model_bundles.lock.sh"}"
BUNDLE_DIR="${MODEL_BUNDLE_DIR:-"$ROOT_DIR/.model-runtime"}"

MODE="ensure"
WRITE_LOCK=0
REFRESH=0
declare -a FILTERS=()

usage() {
  cat <<'USAGE'
usage: bash scripts/sync_model_bundles.sh [options] [model-name|model-name@revision ...]

Idempotently verifies model bundles listed in scripts/model_bundles.lock.sh.
If a model is missing or corrupted, it is downloaded again (unless --check is used).

Options:
  --check           verify only; do not download/repair bundles
  --ensure          verify and repair bundles (default)
  --write-lock      regenerate SHA-256 lines in the lock file from local bundles
  --refresh         force re-download before lock regeneration
  --bundle-dir DIR  model bundle directory (default: .model-runtime)
  --lock-file FILE  lock file path (default: scripts/model_bundles.lock.sh)
  -h, --help        show this help

Environment:
  MODEL_CLI_BIN         command used for `models download` (default: vanalyze, fallback cargo run)
  MODEL_HF_CACHE_DIR    forwarded to `models download --cache-dir`
  MODEL_HF_TOKEN        forwarded to `models download --token`
  MODEL_HF_NO_PROGRESS=1  forwarded to `models download --no-progress`
USAGE
}

log() {
  echo "$*"
}

warn() {
  echo "warning: $*" >&2
}

fail() {
  echo "error: $*" >&2
  exit 1
}

sha256_file() {
  local file="$1"
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$file" | awk '{print $1}'
  elif command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$file" | awk '{print $1}'
  else
    fail "sha256sum or shasum is required to verify model integrity"
  fi
}

safe_bundle_segment() {
  local input="$1"
  local out=""
  local i
  local ch
  for ((i = 0; i < ${#input}; i++)); do
    ch="${input:i:1}"
    if [[ "$ch" =~ [[:alnum:]._-] ]]; then
      out+="$ch"
    else
      out+="_"
    fi
  done
  if [[ -z "$out" ]]; then
    out="_"
  fi
  printf '%s\n' "$out"
}

manifest_entries() {
  local manifest="$1"
  python3 - "$manifest" <<'PY'
import json
import sys

with open(sys.argv[1], "r", encoding="utf-8") as handle:
    data = json.load(handle)

files = data.get("files", {})
for remote_path in sorted(files.keys()):
    file_data = files[remote_path]
    local_path = file_data.get("local_path", "")
    print(f"{remote_path}\t{local_path}")
PY
}

declare -a MODEL_SPECS=()
declare -A MODEL_SHA256=()
declare -A GENERATED_SHA256=()
declare -A FILTER_MATCHED=()

model_preset() {
  local preset="${1:?model_preset requires <preset>}"
  local revision="${2:-main}"
  local name="${3:-$preset}"
  MODEL_SPECS+=("preset|$name|$preset|$revision")
}

model_custom() {
  if [[ "$#" -lt 5 ]]; then
    fail "model_custom requires <name> <repo_id> <task> <revision> <file>..."
  fi
  local name="$1"
  shift
  local repo_id="$1"
  shift
  local task="$1"
  shift
  local revision="$1"
  shift
  local files="$*"
  MODEL_SPECS+=("custom|$name|$repo_id|$task|$revision|$files")
}

model_sha256() {
  local name="${1:?model_sha256 requires <name>}"
  local revision="${2:?model_sha256 requires <revision>}"
  local remote_path="${3:?model_sha256 requires <remote_path>}"
  local sha="${4:?model_sha256 requires <sha256>}"
  MODEL_SHA256["$name|$revision|$remote_path"]="$sha"
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --check)
      MODE="check"
      ;;
    --ensure)
      MODE="ensure"
      ;;
    --write-lock)
      WRITE_LOCK=1
      ;;
    --refresh)
      REFRESH=1
      ;;
    --bundle-dir)
      shift
      [[ $# -gt 0 ]] || fail "--bundle-dir requires a path"
      BUNDLE_DIR="$1"
      ;;
    --lock-file)
      shift
      [[ $# -gt 0 ]] || fail "--lock-file requires a path"
      LOCK_FILE="$1"
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    --)
      shift
      FILTERS+=("$@")
      break
      ;;
    -*)
      fail "unknown option '$1' (use --help)"
      ;;
    *)
      FILTERS+=("$1")
      ;;
  esac
  shift
done

if [[ -f "$LOCK_FILE" ]]; then
  # shellcheck source=/dev/null
  source "$LOCK_FILE"
else
  fail "lock file '$LOCK_FILE' does not exist"
fi

if [[ -n "${MODEL_CLI_BIN:-}" ]]; then
  # shellcheck disable=SC2206
  MODEL_CLI=($MODEL_CLI_BIN)
elif command -v vanalyze >/dev/null 2>&1; then
  MODEL_CLI=(vanalyze)
else
  MODEL_CLI=(cargo run -q -p moritzbrantner-video-analysis-cli --)
fi

if [[ "${#MODEL_SPECS[@]}" -eq 0 ]]; then
  if [[ "${#FILTERS[@]}" -gt 0 ]]; then
    fail "no model specs configured in $LOCK_FILE; cannot match model filters"
  fi
  log "no model specs configured in $LOCK_FILE"
  exit 0
fi

model_selected() {
  local name="$1"
  local revision="$2"
  local filter
  if [[ "${#FILTERS[@]}" -eq 0 ]]; then
    return 0
  fi
  for filter in "${FILTERS[@]}"; do
    if [[ "$filter" == "$name" || "$filter" == "$name@$revision" ]]; then
      FILTER_MATCHED["$filter"]=1
      return 0
    fi
  done
  return 1
}

bundle_root_for() {
  local name="$1"
  local revision="$2"
  printf '%s/%s/%s\n' \
    "$BUNDLE_DIR" \
    "$(safe_bundle_segment "$name")" \
    "$(safe_bundle_segment "$revision")"
}

download_spec() {
  local spec="$1"
  local overwrite="${2:-0}"
  local kind
  local name
  local a
  local b
  local c
  local d
  IFS='|' read -r kind name a b c d <<<"$spec"

  local -a cmd=("${MODEL_CLI[@]}" models download --bundle-dir "$BUNDLE_DIR")
  if [[ "$overwrite" == "1" ]]; then
    cmd+=(--overwrite)
  fi
  if [[ -n "${MODEL_HF_CACHE_DIR:-}" ]]; then
    cmd+=(--cache-dir "$MODEL_HF_CACHE_DIR")
  fi
  if [[ -n "${MODEL_HF_TOKEN:-}" ]]; then
    cmd+=(--token "$MODEL_HF_TOKEN")
  fi
  if [[ "${MODEL_HF_NO_PROGRESS:-0}" == "1" ]]; then
    cmd+=(--no-progress)
  fi

  case "$kind" in
    preset)
      local preset="$a"
      local revision="$b"
      log "syncing model bundle $name@$revision (preset: $preset)"
      cmd+=(--preset "$preset" --revision "$revision")
      ;;
    custom)
      local repo_id="$a"
      local task="$b"
      local revision="$c"
      local files_blob="$d"
      local -a files=()
      if [[ -n "$files_blob" ]]; then
        read -r -a files <<<"$files_blob"
      fi
      [[ "${#files[@]}" -gt 0 ]] || fail "custom model $name@$revision has no files configured"
      log "syncing model bundle $name@$revision (repo: $repo_id)"
      cmd+=(--repo-id "$repo_id" --task "$task" --revision "$revision")
      local file
      for file in "${files[@]}"; do
        cmd+=(--file "$file")
      done
      ;;
    *)
      fail "unsupported model spec kind '$kind'"
      ;;
  esac

  "${cmd[@]}"
}

verify_spec() {
  local spec="$1"
  local kind
  local name
  local a
  local b
  local c
  local d
  IFS='|' read -r kind name a b c d <<<"$spec"

  local revision
  case "$kind" in
    preset) revision="$b" ;;
    custom) revision="$c" ;;
    *) fail "unsupported model spec kind '$kind'" ;;
  esac

  local bundle_root
  bundle_root="$(bundle_root_for "$name" "$revision")"
  local manifest_path="$bundle_root/manifest.json"

  local ok=1
  if [[ ! -f "$manifest_path" ]]; then
    warn "missing manifest for $name@$revision at $manifest_path"
    return 1
  fi

  declare -A manifest_local_paths=()
  local remote_path
  local local_path
  while IFS=$'\t' read -r remote_path local_path; do
    if [[ -n "$remote_path" ]]; then
      manifest_local_paths["$remote_path"]="$local_path"
    fi
  done < <(manifest_entries "$manifest_path")

  if [[ "${#manifest_local_paths[@]}" -eq 0 ]]; then
    warn "manifest has no files for $name@$revision"
    return 1
  fi

  local checksum_count=0
  local key
  for key in "${!MODEL_SHA256[@]}"; do
    IFS='|' read -r key_name key_revision _ <<<"$key"
    if [[ "$key_name" == "$name" && "$key_revision" == "$revision" ]]; then
      checksum_count=$((checksum_count + 1))
    fi
  done
  if [[ "$checksum_count" -eq 0 ]]; then
    warn "no checksums configured for $name@$revision in $LOCK_FILE"
    return 1
  fi

  for remote_path in "${!manifest_local_paths[@]}"; do
    key="$name|$revision|$remote_path"
    local expected_sha="${MODEL_SHA256[$key]:-}"
    if [[ -z "$expected_sha" ]]; then
      warn "missing checksum entry for $name@$revision file '$remote_path'"
      ok=0
      continue
    fi

    local_path="${manifest_local_paths[$remote_path]}"
    local absolute_path="$bundle_root/$local_path"
    if [[ ! -f "$absolute_path" ]]; then
      warn "missing file for $name@$revision: $absolute_path"
      ok=0
      continue
    fi

    local actual_sha
    actual_sha="$(sha256_file "$absolute_path")"
    if [[ "$actual_sha" != "$expected_sha" ]]; then
      warn "checksum mismatch for $name@$revision file '$remote_path'"
      warn "  expected: $expected_sha"
      warn "  actual:   $actual_sha"
      ok=0
    fi
  done

  for key in "${!MODEL_SHA256[@]}"; do
    IFS='|' read -r key_name key_revision key_remote <<<"$key"
    if [[ "$key_name" != "$name" || "$key_revision" != "$revision" ]]; then
      continue
    fi
    if [[ -z "${manifest_local_paths[$key_remote]:-}" ]]; then
      warn "stale checksum entry for $name@$revision file '$key_remote' (not in manifest)"
      ok=0
    fi
  done

  [[ "$ok" -eq 1 ]]
}

record_spec_checksums() {
  local spec="$1"
  local kind
  local name
  local a
  local b
  local c
  local d
  IFS='|' read -r kind name a b c d <<<"$spec"

  local revision
  case "$kind" in
    preset) revision="$b" ;;
    custom) revision="$c" ;;
    *) fail "unsupported model spec kind '$kind'" ;;
  esac

  local bundle_root
  bundle_root="$(bundle_root_for "$name" "$revision")"
  local manifest_path="$bundle_root/manifest.json"

  if [[ "$REFRESH" == "1" || ! -f "$manifest_path" ]]; then
    download_spec "$spec" "$REFRESH"
  fi
  [[ -f "$manifest_path" ]] || fail "missing manifest for $name@$revision after download"

  local remote_path
  local local_path
  local count=0
  while IFS=$'\t' read -r remote_path local_path; do
    [[ -n "$remote_path" ]] || continue
    local absolute_path="$bundle_root/$local_path"
    [[ -f "$absolute_path" ]] || fail "missing file for $name@$revision: $absolute_path"
    GENERATED_SHA256["$name|$revision|$remote_path"]="$(sha256_file "$absolute_path")"
    count=$((count + 1))
  done < <(manifest_entries "$manifest_path")
  [[ "$count" -gt 0 ]] || fail "manifest for $name@$revision has no files to checksum"

  log "recorded checksums for $name@$revision ($count files)"
}

write_lock_file() {
  local tmp
  tmp="$(mktemp)"

  awk '
    /^# BEGIN AUTO-CHECKSUMS$/ { skip=1; next }
    /^# END AUTO-CHECKSUMS$/ { skip=0; next }
    !skip { print }
  ' "$LOCK_FILE" >"$tmp"

  {
    echo
    echo "# BEGIN AUTO-CHECKSUMS"
    echo "# Generated by scripts/sync_model_bundles.sh --write-lock on $(date -u +"%Y-%m-%dT%H:%M:%SZ")"
    local key
    while IFS= read -r key; do
      local name revision remote_path
      IFS='|' read -r name revision remote_path <<<"$key"
      printf 'model_sha256 %q %q %q %q\n' \
        "$name" \
        "$revision" \
        "$remote_path" \
        "${GENERATED_SHA256[$key]}"
    done < <(printf '%s\n' "${!GENERATED_SHA256[@]}" | sort)
    echo "# END AUTO-CHECKSUMS"
  } >>"$tmp"

  mv "$tmp" "$LOCK_FILE"
}

declare -a SELECTED_SPECS=()
local_any_selected=0
for spec in "${MODEL_SPECS[@]}"; do
  IFS='|' read -r kind name a b c d <<<"$spec"
  case "$kind" in
    preset) revision="$b" ;;
    custom) revision="$c" ;;
    *) fail "unsupported model spec kind '$kind'" ;;
  esac
  if model_selected "$name" "$revision"; then
    SELECTED_SPECS+=("$spec")
    local_any_selected=1
  fi
done

if [[ "$local_any_selected" -eq 0 ]]; then
  if [[ "${#FILTERS[@]}" -gt 0 ]]; then
    fail "no model specs matched the provided filters"
  fi
  log "no model specs selected"
  exit 0
fi

if [[ "${#FILTERS[@]}" -gt 0 ]]; then
  for filter in "${FILTERS[@]}"; do
    if [[ -z "${FILTER_MATCHED[$filter]:-}" ]]; then
      fail "model filter '$filter' did not match any configured spec"
    fi
  done
fi

mkdir -p "$BUNDLE_DIR"

if [[ "$WRITE_LOCK" -eq 1 ]]; then
  for spec in "${SELECTED_SPECS[@]}"; do
    record_spec_checksums "$spec"
  done
  write_lock_file
  log "updated model lock file: $LOCK_FILE"
  exit 0
fi

status=0
for spec in "${SELECTED_SPECS[@]}"; do
  IFS='|' read -r kind name a b c d <<<"$spec"
  case "$kind" in
    preset) revision="$b" ;;
    custom) revision="$c" ;;
    *) fail "unsupported model spec kind '$kind'" ;;
  esac

  if verify_spec "$spec"; then
    log "verified model bundle $name@$revision"
    continue
  fi

  if [[ "$MODE" == "check" ]]; then
    status=1
    continue
  fi

  warn "repairing model bundle $name@$revision"
  download_spec "$spec" 1
  if verify_spec "$spec"; then
    log "repaired model bundle $name@$revision"
  else
    warn "unable to verify model bundle $name@$revision after repair"
    status=1
  fi
done

if [[ "$status" -ne 0 ]]; then
  if [[ "$MODE" == "check" ]]; then
    fail "model bundle verification failed"
  fi
  fail "one or more model bundles are still invalid after repair"
fi

log "all selected model bundles are valid"
