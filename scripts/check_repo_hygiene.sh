#!/usr/bin/env bash
set -uo pipefail

ROOT_DIR="$(git rev-parse --show-toplevel 2>/dev/null)"
if [[ -z "${ROOT_DIR:-}" ]]; then
  echo "error: not inside a git repository" >&2
  exit 1
fi

cd "$ROOT_DIR"

section() {
  printf '\n== %s ==\n' "$1"
}

section "Git Status"
status_output="$(git status --short)"
if [[ -n "$status_output" ]]; then
  printf '%s\n' "$status_output"
else
  echo "clean"
fi

section "Untracked Files"
untracked_output="$(git ls-files -o --exclude-standard)"
if [[ -n "$untracked_output" ]]; then
  printf '%s\n' "$untracked_output"
else
  echo "none"
fi

section "Upstream"
if upstream="$(git rev-parse --abbrev-ref --symbolic-full-name '@{u}' 2>/dev/null)"; then
  echo "$upstream"
  read -r behind ahead < <(git rev-list --left-right --count "${upstream}...HEAD")
  echo "behind: $behind"
  echo "ahead: $ahead"
else
  echo "missing upstream branch"
fi

section "Tracked Generated Directory Candidates"
unexpected_tracked=(
  "target/"
  ".cargo-target/"
  "node_modules/"
  ".external-test-tools/"
  ".audio-tools/"
  ".video-analysis-models/"
  "coverage/"
  ".nyc_output/"
)

found_unexpected=0
for path in "${unexpected_tracked[@]}"; do
  count="$(git ls-files "$path" | wc -l | tr -d ' ')"
  if [[ "$count" != "0" ]]; then
    echo "tracked: $path ($count files)"
    found_unexpected=1
  fi
done

if [[ "$found_unexpected" == "0" ]]; then
  echo "none"
fi

section "Known Checked-In Generated Outputs"
found_known=0
for path in "packages/video-analysis-ui/dist/" "prototypes/web/video-analysis-web/dist/" "packages/text-core-wasm/pkg/"; do
  count="$(git ls-files "$path" | wc -l | tr -d ' ')"
  if [[ "$count" != "0" ]]; then
    echo "$path ($count tracked files; regenerate with the package build flow, do not edit by hand)"
    found_known=1
  fi
done

if [[ "$found_known" == "0" ]]; then
  echo "none"
fi

section "Local-Only Ignore Coverage"
local_only=(
  ".cargo-target/"
  "target/"
  "node_modules/"
  "packages/video-analysis-ui/node_modules/"
  "prototypes/web/video-analysis-web/node_modules/"
  ".external-test-tools/"
  ".audio-tools/"
  ".test-corpora/"
  ".video-analysis-models/"
  ".env"
  ".env.local"
  "coverage/"
  "packages/video-analysis-ui/test-results/"
  "prototypes/web/video-analysis-web/test-results/"
  "prototypes/web/video-analysis-web/playwright-report/"
)

missing_ignore=0
for path in "${local_only[@]}"; do
  if git check-ignore -q "$path"; then
    echo "ignored: $path"
  else
    tracked_count="$(git ls-files "$path" | wc -l | tr -d ' ')"
    if [[ "$tracked_count" == "0" ]]; then
      echo "not ignored: $path"
      missing_ignore=1
    fi
  fi
done

if [[ "$missing_ignore" != "0" ]]; then
  echo
  echo "warning: one or more local-only paths are not covered by .gitignore"
fi
