# video-analysis-onnx

ONNX-backed video model inference adapters for `video-analysis`.

## Feature flags

- `onnxruntime`: enables the ONNX Runtime-backed backend
- `external-tests`: enables ignored runtime smoke tests

## Example

```rust,ignore
use video_analysis_onnx::{OnnxModelBackend, OnnxObjectDetectionOptions};

let backend = OnnxModelBackend::from_bundle("bundles/yolos-tiny")?;
let options = OnnxObjectDetectionOptions::default();

let _ = backend;
let _ = options;
```

## Related crates

- `video-analysis-models`
- `image-analysis-processing`
- `video-analysis-cli`
