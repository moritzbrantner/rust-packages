#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TOOLS_DIR="${MODEL_TOOLS_DIR:-"${EXTERNAL_TEST_TOOLS_DIR:-"$ROOT_DIR/.external-test-tools"}"}"

# shellcheck source=scripts/lib_external_test_tools.sh
source "$ROOT_DIR/scripts/lib_external_test_tools.sh"

usage() {
  cat <<'USAGE'
usage: bash scripts/check_model_external_tools.sh [python|onnx|transformers|all]...

Verifies optional model-backend Python dependencies in the local model venv.
This script does not install packages. Run setup_model_external_tools.sh first
if a target is missing.
USAGE
}

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
  usage
  exit 0
fi

targets=("$@")
if [[ ${#targets[@]} -eq 0 ]]; then
  targets=(all)
fi

for target in "${targets[@]}"; do
  case "$target" in
    python|onnx|transformers|all)
      if verify_model_python_target "$target"; then
        log "verified model $target Python dependencies in $MODEL_PYTHON_VENV"
      else
        fail "missing model $target Python dependencies; run: bash scripts/setup_model_external_tools.sh $target"
      fi
      ;;
    *)
      usage >&2
      fail "unknown model dependency target '$target'"
      ;;
  esac
done
