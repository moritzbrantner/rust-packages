# image-analysis-onnx

ONNX-backed still-image preprocessing and inference adapters for `video-analysis`.

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

- `image-analysis-tasks`
- `image-analysis-detection`
- `video-analysis-onnx`
