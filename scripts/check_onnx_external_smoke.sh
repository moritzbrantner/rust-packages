#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

echo "Expected ONNX smoke bundle manifests:"
for manifest in \
  ".model-runtime/vit-base-patch16-224/main/manifest.json" \
  ".model-runtime/vit-gpt2-image-captioning/main/manifest.json" \
  ".model-runtime/vit-gpt2-image-captioning-onnx/main/manifest.json" \
  ".model-runtime/xenova-detr-resnet-50-onnx/main/manifest.json" \
  ".model-runtime/xenova-clip-vit-base-patch32-onnx/main/manifest.json" \
  ".model-runtime/roberta-base-squad2-onnx/main/manifest.json"
do
  if [[ -f "$manifest" ]]; then
    echo "  present: $manifest"
  else
    echo "  missing: $manifest"
  fi
done

cargo test -p moritzbrantner-image-analysis-classification --features external-tests -- --ignored
cargo test -p moritzbrantner-image-analysis-captioning --features external-tests -- --ignored
cargo test -p moritzbrantner-image-analysis-detection --features external-tests -- --ignored
cargo test -p moritzbrantner-image-analysis-embeddings --features external-tests -- --ignored
cargo test -p moritzbrantner-text-question-answering --features external-tests -- --ignored
