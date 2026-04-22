#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TOOLS_DIR="${RADIANCE_TOOLS_DIR:-"${EXTERNAL_TEST_TOOLS_DIR:-"$ROOT_DIR/.external-test-tools"}"}"
source "$ROOT_DIR/scripts/lib_external_test_tools.sh"

if [[ "$#" -eq 0 ]]; then
  set -- ffmpeg colmap nerfstudio
fi

for tool in "$@"; do
  case "$tool" in
    ffmpeg) install_ffmpeg ;;
    colmap) install_colmap ;;
    nerfstudio) install_nerfstudio ;;
    training)
      NERFSTUDIO_VERIFY_TRAINING_CLI=1 install_nerfstudio
      ;;
    conda) install_conda ;;
    *)
      fail "unknown radiance tool '$tool' (expected ffmpeg, colmap, nerfstudio, training, or conda)"
      ;;
  esac
done
