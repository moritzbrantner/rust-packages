# image-analysis-onnx

ONNX-backed still-image preprocessing and inference adapters for `video-analysis`.

## Runtime Surface

- Workflow operations: `image.onnx.preprocess` and
  `image.onnx.decodeDetections` run deterministic preprocessing and output
  decoding workflows.
- Debug operations: `image.onnx.preprocessing` and `describe` inspect
  preprocessing configuration and package metadata.
- Default surface operations do not load ONNX Runtime sessions. Real runtime
  integration remains behind feature-enabled adapters and external tests.

## Feature flags

- `onnxruntime`: enables native ONNX Runtime execution for supported adapters.
- `external-tests`: enables real-runtime integration tests.

## Example

```rust
# fn main() -> Result<(), Box<dyn std::error::Error>> {
use image_analysis_core::OwnedImage;
use image_analysis_onnx::{image_to_tensor, OnnxImagePreprocessing};

let image = OwnedImage::new_rgb(2, 2, vec![255; 12])?;
let tensor = image_to_tensor(&image.as_view(), &OnnxImagePreprocessing::default())?;
assert_eq!(tensor.channels, 3);
# Ok(())
# }
```

## Related crates

- `image-analysis-classification, image-analysis-embeddings, image-analysis-captioning, image-analysis-ocr, image-analysis-segmentation, or image-analysis-detection`
- `image-analysis-detection`
- `video-analysis-onnx`
