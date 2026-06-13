#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TOOLS_DIR="${MODEL_TOOLS_DIR:-"${EXTERNAL_TEST_TOOLS_DIR:-"$ROOT_DIR/.external-test-tools"}"}"

# shellcheck source=scripts/lib_external_test_tools.sh
source "$ROOT_DIR/scripts/lib_external_test_tools.sh"

usage() {
  cat <<'USAGE'
usage: bash scripts/setup_model_external_tools.sh [python|onnx|transformers|bundles|all]...

Installs optional model-backend Python dependencies into an ignored local venv.
The script is idempotent: existing working imports are reused, and missing
package groups are installed only as needed.

Targets:
  python        create/verify the model Python virtual environment
  onnx          install numpy, pillow, and onnxruntime
  transformers  install numpy, pillow, torch, transformers, tokenizers,
                safetensors, and huggingface_hub
  bundles       verify/download model bundles listed in scripts/model_bundles.lock.sh
  all           install onnx and transformers groups

Environment overrides:
  MODEL_TOOLS_DIR                  default: .external-test-tools
  MODEL_PYTHON_VENV                default: $MODEL_TOOLS_DIR/model-python-venv
  MODEL_ONNXRUNTIME_PIP_PACKAGE    default: onnxruntime
  MODEL_TORCH_PIP_PACKAGE          default: torch
  MODEL_TRANSFORMERS_PIP_PACKAGE   default: transformers
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
      install_model_python_target "$target"
      if [[ "$target" == "onnx" || "$target" == "all" ]]; then
        dylib="$(require_model_onnxruntime_dylib_path)"
        log "model ONNX Runtime dylib:"
        log "  $dylib"
      fi
      ;;
    bundles)
      bash "$ROOT_DIR/scripts/sync_model_bundles.sh"
      ;;
    *)
      usage >&2
      fail "unknown model dependency target '$target'"
      ;;
  esac
done

log "model Python environment:"
log "  $MODEL_PYTHON_VENV"
log "activate it with:"
log "  source \"$MODEL_PYTHON_VENV/bin/activate\""
