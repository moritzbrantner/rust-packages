#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TOOLS_DIR="${MODEL_TOOLS_DIR:-"${EXTERNAL_TEST_TOOLS_DIR:-"$ROOT_DIR/.external-test-tools"}"}"

# shellcheck source=scripts/lib_external_test_tools.sh
source "$ROOT_DIR/scripts/lib_external_test_tools.sh"

cd "$ROOT_DIR"

ORT_DYLIB_PATH="$(require_model_onnxruntime_dylib_path)"
export ORT_DYLIB_PATH
echo "Using ONNX Runtime dylib: $ORT_DYLIB_PATH"

echo "Expected ONNX smoke bundle manifests:"
for manifest in \
  ".model-runtime/vit-base-patch16-224/main/manifest.json" \
  ".model-runtime/vit-gpt2-image-captioning/main/manifest.json" \
  ".model-runtime/vit-gpt2-image-captioning-onnx/main/manifest.json" \
  ".model-runtime/xenova-detr-resnet-50-onnx/main/manifest.json" \
  ".model-runtime/xenova-clip-vit-base-patch32-onnx/main/manifest.json" \
  ".model-runtime/trocr-base-printed-onnx/main/manifest.json" \
  ".model-runtime/roberta-base-squad2-onnx/main/manifest.json"
do
  if [[ -f "$manifest" ]]; then
    echo "  present: $manifest"
  else
    echo "  missing: $manifest"
  fi
done

cargo test -p moenarch-image-analysis-classification --features external-tests -- --ignored
cargo test -p moenarch-image-analysis-captioning --features external-tests -- --ignored
cargo test -p moenarch-image-analysis-detection --features external-tests -- --ignored
cargo test -p moenarch-image-analysis-embeddings --features external-tests -- --ignored
cargo test -p moenarch-image-analysis-ocr --features external-tests --test external_onnx_smoke -- --ignored
cargo test -p moenarch-text-question-answering --features external-tests -- --ignored
