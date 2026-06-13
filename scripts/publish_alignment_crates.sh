#!/usr/bin/env bash
set -euo pipefail

mode="${1:-}"

if [[ "${mode}" != "" && "${mode}" != "--publish" ]]; then
  echo "usage: scripts/publish_alignment_crates.sh [--publish]" >&2
  exit 2
fi

crates=(
  "moritzbrantner-text-transcripts:0.1.1"
  "moritzbrantner-audio-analysis-transcription:0.1.2"
)

package_crate() {
  local crate="$1"
  echo "==> Packaging ${crate}"
  cargo package -p "${crate}" --allow-dirty
}

crate_version_exists() {
  local crate="$1"
  local version="$2"
  local body

  if ! body="$(
    curl -sS \
      -H "User-Agent: publish_alignment_crates.sh" \
      "https://crates.io/api/v1/crates/${crate}/${version}"
  )"; then
    echo "failed to query crates.io for ${crate}@${version}" >&2
    return 2
  fi

  if [[ "${body}" == *'"version":{'* && "${body}" == *"\"num\":\"${version}\""* ]]; then
    return 0
  fi

  if [[ "${body}" == *'"errors":['* && "${body}" == *"does not have a version"* ]]; then
    return 1
  fi

  echo "unexpected crates.io response for ${crate}@${version}: ${body}" >&2
  return 2
}

publish_crate() {
  local crate="$1"
  local version="$2"
  local exists_status

  if crate_version_exists "${crate}" "${version}"; then
    echo "==> Skipping ${crate}@${version}; exact version already exists on crates.io"
    return 0
  else
    exists_status=$?
  fi

  if [[ "${exists_status}" != "1" ]]; then
    return "${exists_status}"
  fi

  echo "==> Publishing ${crate}@${version}"
  cargo publish -p "${crate}" --allow-dirty
}

wait_for_crate_version() {
  local crate="$1"
  local version="$2"

  echo "==> Waiting for ${crate}@${version} to resolve from crates.io"
  for _ in {1..30}; do
    local exists_status
    if crate_version_exists "${crate}" "${version}"; then
      return 0
    else
      exists_status=$?
    fi

    if [[ "${exists_status}" != "1" ]]; then
      return "${exists_status}"
    fi
    sleep 10
  done

  echo "timed out waiting for ${crate}@${version} on crates.io" >&2
  return 1
}

for spec in "${crates[@]}"; do
  crate="${spec%%:*}"
  version="${spec##*:}"

  package_crate "${crate}"

  if [[ "${mode}" == "--publish" ]]; then
    publish_crate "${crate}" "${version}"

    if [[ "${crate}" == "moritzbrantner-text-transcripts" ]]; then
      wait_for_crate_version "${crate}" "${version}"
    fi
  fi
done

if [[ "${mode}" == "--publish" ]]; then
  wait_for_crate_version "moritzbrantner-audio-analysis-transcription" "0.1.2"
fi
