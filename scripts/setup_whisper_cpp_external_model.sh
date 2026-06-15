#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
MODEL_STORE="${WHISPER_CPP_MODEL_STORE:-"$ROOT_DIR/.model-runtime/whisper-cpp"}"
MODEL_DIR="$MODEL_STORE/models"
MODEL_FILE="$MODEL_DIR/ggml-tiny.en.bin"
DOWNLOAD_URL="https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.en.bin"
EXPECTED_SHA256="921e4cf8686fdd993dcd081a5da5b6c365bfde1162e72b08d75ac75289920b1f"

mkdir -p "$MODEL_DIR"

sha256_file() {
  sha256sum "$1" | awk '{print $1}'
}

if [[ -f "$MODEL_FILE" ]]; then
  actual="$(sha256_file "$MODEL_FILE")"
  if [[ "$actual" == "$EXPECTED_SHA256" ]]; then
    printf 'ok: whisper.cpp tiny.en model already exists at %s\n' "$MODEL_FILE"
    exit 0
  fi
  printf 'warning: removing %s because sha256 was %s, expected %s\n' \
    "$MODEL_FILE" "$actual" "$EXPECTED_SHA256" >&2
  rm -f "$MODEL_FILE"
fi

tmp="$(mktemp "$MODEL_FILE.tmp.XXXXXX")"
cleanup() {
  rm -f "$tmp"
}
trap cleanup EXIT

if command -v curl >/dev/null 2>&1; then
  curl -L --fail --retry 3 --retry-delay 2 -o "$tmp" "$DOWNLOAD_URL"
elif command -v wget >/dev/null 2>&1; then
  wget -O "$tmp" "$DOWNLOAD_URL"
else
  printf 'error: curl or wget is required to download %s\n' "$DOWNLOAD_URL" >&2
  exit 1
fi

actual="$(sha256_file "$tmp")"
if [[ "$actual" != "$EXPECTED_SHA256" ]]; then
  printf 'error: sha256 mismatch for %s: got %s, expected %s\n' \
    "$DOWNLOAD_URL" "$actual" "$EXPECTED_SHA256" >&2
  exit 1
fi

mv "$tmp" "$MODEL_FILE"
trap - EXIT
printf 'ok: downloaded whisper.cpp tiny.en model to %s\n' "$MODEL_FILE"
