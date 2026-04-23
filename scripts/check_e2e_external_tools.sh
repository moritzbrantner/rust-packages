#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
for local_bin in "$ROOT_DIR/.external-test-tools/bin" "$ROOT_DIR/.audio-tools/bin"; do
  if [[ -d "$local_bin" ]]; then
    export PATH="$local_bin:$PATH"
  fi
done

missing=()

check_tool() {
  local tool="$1"
  if command -v "$tool" >/dev/null 2>&1; then
    printf 'ok: %s -> %s\n' "$tool" "$(command -v "$tool")"
  else
    printf 'missing: %s\n' "$tool" >&2
    missing+=("$tool")
  fi
}

for tool in ffmpeg ffprobe demucs whisper yt-dlp colmap ns-process-data; do
  check_tool "$tool"
done

if [[ "${#missing[@]}" -ne 0 ]]; then
  cat >&2 <<'EOF'

One or more external end-to-end tools are unavailable.

Recommended remediation:
  1. Run `scripts/setup_e2e_external_tools.sh fast`
  2. Re-run `scripts/check_e2e_external_tools.sh`

If you manage tools outside the repository, make sure the missing commands are on PATH.
EOF
  exit 1
fi
