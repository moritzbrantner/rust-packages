#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(git rev-parse --show-toplevel 2>/dev/null)"
if [[ -z "${ROOT_DIR:-}" ]]; then
  echo "error: not inside a git repository" >&2
  exit 1
fi

cd "$ROOT_DIR"

ALLOW_FILE="scripts/generated_artifacts.allow"
MAX_MEDIA_BYTES=$((5 * 1024 * 1024))
failures=0

is_allowed() {
  local path="$1"
  [[ -f "$ALLOW_FILE" ]] || return 1
  grep -Fqx -- "$path" <(grep -vE '^\s*($|#)' "$ALLOW_FILE")
}

report_failure() {
  failures=$((failures + 1))
  printf '%s\n' "$1" >&2
}

tracked_for_pathspec() {
  git ls-files -- "$@"
}

check_forbidden_pathspec() {
  local label="$1"
  shift
  local matches=()
  mapfile -t matches < <(tracked_for_pathspec "$@")
  if [[ "${#matches[@]}" -eq 0 ]]; then
    return
  fi

  local unexpected=()
  for path in "${matches[@]}"; do
    if ! is_allowed "$path"; then
      unexpected+=("$path")
    fi
  done
  if [[ "${#unexpected[@]}" -eq 0 ]]; then
    return
  fi

  report_failure "forbidden tracked generated artifact(s) under ${label}:"
  printf '  %s\n' "${unexpected[@]}" >&2
}

check_forbidden_pathspec "packages/video-analysis-ui/dist/" "packages/video-analysis-ui/dist/"
check_forbidden_pathspec "prototypes/web/video-analysis-web/dist/" "prototypes/web/video-analysis-web/dist/"
check_forbidden_pathspec "packages/*-wasm/pkg/" "packages/*-wasm/pkg/"
check_forbidden_pathspec "coverage/" "coverage/"
check_forbidden_pathspec "target/" "target/"
check_forbidden_pathspec ".cargo-target/" ".cargo-target/"
check_forbidden_pathspec ".model-runtime/" ".model-runtime/"
check_forbidden_pathspec ".test-corpora/" ".test-corpora/"
check_forbidden_pathspec ".external-test-tools/" ".external-test-tools/"
check_forbidden_pathspec ".audio-tools/" ".audio-tools/"
check_forbidden_pathspec "node_modules/" "node_modules/"
check_forbidden_pathspec "packages/*/node_modules/" "packages/*/node_modules/"
check_forbidden_pathspec "prototypes/**/node_modules/" ":(glob)prototypes/**/node_modules/**"
check_forbidden_pathspec "prototypes/web/video-analysis-web/playwright-report/" "prototypes/web/video-analysis-web/playwright-report/"
check_forbidden_pathspec "prototypes/web/video-analysis-web/test-results/" "prototypes/web/video-analysis-web/test-results/"
check_forbidden_pathspec "prototypes/web/video-analysis-web/public/workspace-architecture.json" \
  "prototypes/web/video-analysis-web/public/workspace-architecture.json"

while IFS= read -r -d '' path; do
  is_allowed "$path" && continue
  [[ -f "$path" ]] || continue
  size="$(wc -c < "$path" | tr -d ' ')"
  extension="${path##*.}"
  extension="${extension,,}"
  case "$extension" in
    mp4|mov|mkv|webm|wav|flac|ogg|mp3)
      if (( size > MAX_MEDIA_BYTES )); then
        report_failure "large tracked use-case media artifact: ${path} (${size} bytes)"
      fi
      ;;
    *)
      if (( size > MAX_MEDIA_BYTES )) && ! grep -Iq . "$path"; then
        report_failure "large tracked binary use-case artifact: ${path} (${size} bytes)"
      fi
      ;;
  esac
done < <(git ls-files -z -- 'use-case-output/**')

if (( failures > 0 )); then
  cat >&2 <<'EOF'

Generated/local artifact policy:
  - Build outputs are regenerated locally; do not commit dist/pkg outputs.
  - Test corpora belong in .test-corpora/ and can be restored with scripts/download_test_corpora.sh.
  - Model files belong in .model-runtime/ and must be materialized through model-runtime jobs.
  - Large use-case media belongs under use-case-output/ as local output; use the documented workflow to recreate it.

If a generated snapshot is intentionally reviewed, add a narrow entry to scripts/generated_artifacts.allow and document the freshness check.
EOF
  exit 1
fi

echo "generated artifact check passed"
