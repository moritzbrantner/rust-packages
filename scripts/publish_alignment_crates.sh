#!/usr/bin/env bash
set -euo pipefail

mode="${1:-}"

if [[ "${mode}" != "" && "${mode}" != "--publish" ]]; then
  echo "usage: scripts/publish_alignment_crates.sh [--publish]" >&2
  exit 2
fi

crates=(
  "moritzbrantner-text-transcripts"
  "moritzbrantner-audio-analysis-transcription"
)

package_crate() {
  local crate="$1"
  echo "==> Packaging ${crate}"
  cargo package -p "${crate}" --allow-dirty
}

publish_crate() {
  local crate="$1"
  echo "==> Publishing ${crate}"
  cargo publish -p "${crate}" --allow-dirty
}

wait_for_text_transcripts() {
  echo "==> Waiting for moritzbrantner-text-transcripts to resolve from crates.io"
  cargo search moritzbrantner-text-transcripts --limit 1
}

for crate in "${crates[@]}"; do
  package_crate "${crate}"

  if [[ "${mode}" == "--publish" ]]; then
    publish_crate "${crate}"

    if [[ "${crate}" == "moritzbrantner-text-transcripts" ]]; then
      wait_for_text_transcripts
    fi
  fi
done
