#!/usr/bin/env bash
set -euo pipefail

BASE_REF="${BASE_REF:-origin/main}"
CHECK_FAST_SCOPE="${CHECK_FAST_SCOPE:-changed}"
CHECK_FAST_FRONTEND="${CHECK_FAST_FRONTEND:-changed}"
CHECK_FAST_PROGRESS="${CHECK_FAST_PROGRESS:-changed}"

if [[ -n "${CHECK_FAST_RUST_JOBS:-}" ]]; then
  RUST_JOBS="$CHECK_FAST_RUST_JOBS"
elif [[ -n "${CARGO_BUILD_JOBS:-}" ]]; then
  RUST_JOBS="$CARGO_BUILD_JOBS"
elif [[ -n "${TEST_MAX_WORKERS:-}" ]]; then
  RUST_JOBS="$TEST_MAX_WORKERS"
else
  cpu_count="$(getconf _NPROCESSORS_ONLN 2>/dev/null || echo 2)"
  if (( cpu_count > 8 )); then
    RUST_JOBS=8
  else
    RUST_JOBS="$cpu_count"
  fi
fi

RUST_TEST_THREADS="${RUST_TEST_THREADS:-${TEST_MAX_WORKERS:-2}}"
export RUST_TEST_THREADS

started_at="$(date +%s)"
step_started_at="$started_at"
scope_file="$(mktemp -t check-fast-scope.XXXXXX.json)"
trap 'rm -f "$scope_file"' EXIT

log() {
  printf '[%(%H:%M:%S)T] %s\n' -1 "$*"
}

elapsed() {
  local start="$1"
  local now
  now="$(date +%s)"
  printf '%ss' "$((now - start))"
}

run_step() {
  local name="$1"
  shift
  step_started_at="$(date +%s)"
  log "start: $name"
  "$@"
  log "done: $name ($(elapsed "$step_started_at"))"
}

run_shell_step() {
  local name="$1"
  local command="$2"
  step_started_at="$(date +%s)"
  log "start: $name"
  bash -lc "$command"
  log "done: $name ($(elapsed "$step_started_at"))"
}

json_field() {
  local field="$1"
  python3 - "$scope_file" "$field" <<'PY'
import json
import sys

with open(sys.argv[1], encoding="utf-8") as handle:
    data = json.load(handle)
value = data.get(sys.argv[2])
if value is None:
    raise SystemExit(1)
if isinstance(value, bool):
    print("true" if value else "false")
elif isinstance(value, list):
    for item in value:
        print(item)
else:
    print(value)
PY
}

json_scalar() {
  local field="$1"
  json_field "$field" | sed -n '1p'
}

require_mode() {
  local name="$1"
  local value="$2"
  shift 2
  for allowed in "$@"; do
    if [[ "$value" == "$allowed" ]]; then
      return 0
    fi
  done
  echo "error: $name must be one of: $*" >&2
  exit 2
}

require_mode CHECK_FAST_SCOPE "$CHECK_FAST_SCOPE" changed workspace
require_mode CHECK_FAST_FRONTEND "$CHECK_FAST_FRONTEND" changed all none
require_mode CHECK_FAST_PROGRESS "$CHECK_FAST_PROGRESS" changed all none

step_started_at="$(date +%s)"
log "start: detect changed scope"
python3 scripts/check_changed_scope.py --base "$BASE_REF" > "$scope_file"
log "done: detect changed scope ($(elapsed "$step_started_at"))"
mapfile -t snapshot_paths < <(json_field snapshot_paths)
if [[ "$CHECK_FAST_PROGRESS" == "none" ]]; then
  filtered_snapshot_paths=()
  for snapshot_path in "${snapshot_paths[@]}"; do
    if [[ "$snapshot_path" != "docs/CRATE_PROGRESS_LEDGER.md" ]]; then
      filtered_snapshot_paths+=("$snapshot_path")
    fi
  done
  snapshot_paths=("${filtered_snapshot_paths[@]}")
fi

run_step "check changed-scope classifier tests" python3 scripts/test_check_changed_scope.py
run_step "check repository split inventory" python3 scripts/generate_repository_split_inventory.py --check
run_step "test repository split inventory authority" \
  python3 scripts/test_generate_repository_split_inventory.py
run_step "test repository boundaries" python3 scripts/test_check_repository_boundaries.py
run_step "check repository boundaries" python3 scripts/check_repository_boundaries.py --check
run_step "test release plans" python3 scripts/test_check_release_plan.py
run_step "check example release plan" \
  python3 scripts/check_release_plan.py \
    --check docs/repository-split/release-plan.example.json \
    --expected-sha d032ad2890c1df3c6a5b9eff024562f00d017fce \
    --expected-base-sha d032ad2890c1df3c6a5b9eff024562f00d017fce
run_step "check generated/local artifacts" scripts/check_generated_artifacts.sh
if (( ${#snapshot_paths[@]} > 0 )); then
  snapshot_args=()
  for snapshot_path in "${snapshot_paths[@]}"; do
    snapshot_args+=(--only "$snapshot_path")
  done
  log "generated snapshots (${#snapshot_paths[@]}): ${snapshot_paths[*]}"
  run_step "check affected generated snapshots" scripts/check_generated_snapshots.sh "${snapshot_args[@]}"
else
  log "generated snapshots: none affected"
fi
run_step "check rust formatting" cargo fmt --all --check

rust_scope="$(json_scalar rust_scope)"
workspace_reason="$(json_scalar workspace_reason || true)"
docs_only="$(json_scalar docs_only)"
mapfile -t rust_packages < <(json_field rust_packages)
mapfile -t rust_tests < <(json_field rust_tests)
mapfile -t rust_reasons < <(json_field rust_reasons)

if [[ "$CHECK_FAST_SCOPE" == "workspace" ]]; then
  rust_scope="workspace"
  workspace_reason="CHECK_FAST_SCOPE=workspace"
fi

if [[ "$rust_scope" == "workspace" ]]; then
  log "rust scope: workspace (${workspace_reason:-no reason reported})"
  for reason in "${rust_reasons[@]:0:5}"; do
    log "rust reason: $reason"
  done
  run_step "cargo test workspace lib/tests (jobs=$RUST_JOBS)" \
    cargo test --jobs "$RUST_JOBS" --workspace --lib --tests
  run_step "cargo clippy workspace lib/tests" \
    cargo clippy --workspace --lib --tests -- -D warnings
elif [[ "$rust_scope" == "changed" ]]; then
  if (( ${#rust_packages[@]} > 0 )); then
    log "rust packages (${#rust_packages[@]}): ${rust_packages[*]}"
    for reason in "${rust_reasons[@]:0:5}"; do
      log "rust reason: $reason"
    done
    for package in "${rust_packages[@]}"; do
      run_step "cargo test $package lib/tests (jobs=$RUST_JOBS)" \
        cargo test --jobs "$RUST_JOBS" -p "$package" --lib --tests
      run_step "cargo clippy $package lib/tests" \
        cargo clippy -p "$package" --lib --tests -- -D warnings
    done
  fi
  if (( ${#rust_tests[@]} > 0 )); then
    log "root integration tests (${#rust_tests[@]}): ${rust_tests[*]}"
    for test_path in "${rust_tests[@]}"; do
      test_name="$(basename "$test_path" .rs)"
      run_step "cargo test moenarch-video-analysis --test $test_name (jobs=$RUST_JOBS)" \
        cargo test --jobs "$RUST_JOBS" -p moenarch-video-analysis --test "$test_name"
    done
    run_step "cargo clippy moenarch-video-analysis lib/tests" \
      cargo clippy -p moenarch-video-analysis --lib --tests -- -D warnings
  fi
else
  if [[ "$docs_only" == "true" ]]; then
    log "rust scope: none (docs-only change)"
  else
    log "rust scope: none"
  fi
fi

progress_scope="$(json_scalar progress_scope)"
mapfile -t progress_packages < <(json_field progress_packages)
mapfile -t progress_reasons < <(json_field progress_reasons)

if [[ "$CHECK_FAST_PROGRESS" == "all" ]]; then
  progress_scope="all"
elif [[ "$CHECK_FAST_PROGRESS" == "none" ]]; then
  progress_scope="none"
fi

if [[ "$progress_scope" == "all" ]]; then
  log "progress scope: all"
  for reason in "${progress_reasons[@]:0:5}"; do
    log "progress reason: $reason"
  done
  run_step "test crate progress audit helpers" python3 scripts/test_audit_crate_progress.py
  run_step "compare all crate progress against $BASE_REF" \
    python3 scripts/audit_crate_progress.py --compare-base "$BASE_REF"
elif [[ "$progress_scope" == "changed" ]]; then
  log "progress packages (${#progress_packages[@]}): ${progress_packages[*]}"
  for reason in "${progress_reasons[@]:0:5}"; do
    log "progress reason: $reason"
  done
  run_step "test crate progress audit helpers" python3 scripts/test_audit_crate_progress.py
  for package in "${progress_packages[@]}"; do
    run_step "compare crate progress for $package against $BASE_REF" \
      python3 scripts/audit_crate_progress.py --changed --base "$BASE_REF" --only "$package"
  done
else
  log "progress scope: none"
fi

frontend_scope="$(json_scalar frontend_scope)"
mapfile -t frontend_commands < <(json_field frontend_commands)
mapfile -t frontend_reasons < <(json_field frontend_reasons)

if [[ "$CHECK_FAST_FRONTEND" == "all" ]]; then
  frontend_scope="all"
elif [[ "$CHECK_FAST_FRONTEND" == "none" ]]; then
  frontend_scope="none"
fi

if [[ "$frontend_scope" == "all" ]]; then
  log "frontend scope: all"
  for reason in "${frontend_reasons[@]:0:5}"; do
    log "frontend reason: $reason"
  done
  fixed_frontend_commands=(
    "bun run text-wasm:test"
    "bun run text-analysis-wasm:test"
    "bun run text-linguistics-wasm:test"
    "bun run audio-wasm:test"
    "bun run video-wasm:test"
    "bun run ui:typecheck"
    "bun run web:typecheck"
    "bun run ui:test:unit"
    "bun run web:test:unit"
    "bun run web:test:api"
  )
  for command in "${fixed_frontend_commands[@]}"; do
    run_shell_step "$command" "$command"
  done
elif [[ "$frontend_scope" == "changed" ]]; then
  log "frontend commands (${#frontend_commands[@]}): ${frontend_commands[*]}"
  for reason in "${frontend_reasons[@]:0:5}"; do
    log "frontend reason: $reason"
  done
  for command in "${frontend_commands[@]}"; do
    run_shell_step "$command" "$command"
  done
else
  log "frontend scope: none"
fi

log "check-fast completed ($(elapsed "$started_at"))"
