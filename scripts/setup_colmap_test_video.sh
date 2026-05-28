#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SAMPLES_DIR="$ROOT_DIR/prototypes/web/video-analysis-web/public/samples/video"
TARGET="$SAMPLES_DIR/test-video.mp4"
VIDEO_ID="LQWH8o6kwMI"
VIDEO_URL="https://www.youtube.com/watch?v=$VIDEO_ID"
DOWNLOAD_DIR="$ROOT_DIR/.external-test-tools/downloads/colmap-test-video"

for local_bin in "$ROOT_DIR/.external-test-tools/bin" "$ROOT_DIR/.audio-tools/bin"; do
  if [[ -d "$local_bin" ]]; then
    export PATH="$local_bin:$PATH"
  fi
done

if [[ -s "$TARGET" && "${FORCE:-0}" != "1" ]]; then
  printf 'ok: COLMAP test video already exists at %s\n' "$TARGET"
  exit 0
fi

if ! command -v yt-dlp >/dev/null 2>&1; then
  cat >&2 <<EOF
error: yt-dlp is unavailable.

Install it with:
  scripts/setup_e2e_external_tools.sh yt-dlp

Then re-run:
  scripts/setup_colmap_test_video.sh
EOF
  exit 1
fi

mkdir -p "$SAMPLES_DIR"

shopt -s nullglob
existing_candidates=("$SAMPLES_DIR"/*"$VIDEO_ID"*.mp4)
shopt -u nullglob
if [[ "${#existing_candidates[@]}" -gt 0 ]]; then
  if [[ "${FORCE:-0}" == "1" ]]; then
    rm -f "$TARGET"
  fi
  mv "${existing_candidates[0]}" "$TARGET"
  printf 'ok: renamed existing download to %s\n' "$TARGET"
  exit 0
fi

rm -rf "$DOWNLOAD_DIR"
mkdir -p "$DOWNLOAD_DIR"

yt-dlp \
  --no-playlist \
  --format mp4 \
  --paths "$DOWNLOAD_DIR" \
  --output "%(title)s [%(id)s].%(ext)s" \
  "$VIDEO_URL"

shopt -s nullglob
downloaded_candidates=("$DOWNLOAD_DIR"/*"$VIDEO_ID"*.mp4 "$DOWNLOAD_DIR"/*.mp4)
shopt -u nullglob
if [[ "${#downloaded_candidates[@]}" -eq 0 ]]; then
  echo "error: yt-dlp completed but no mp4 was found in $DOWNLOAD_DIR" >&2
  exit 1
fi

if [[ "${FORCE:-0}" == "1" ]]; then
  rm -f "$TARGET"
fi
mv "${downloaded_candidates[0]}" "$TARGET"
printf 'ok: downloaded COLMAP test video to %s\n' "$TARGET"
