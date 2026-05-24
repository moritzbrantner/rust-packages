# video-analysis-onnx

ONNX-backed video model inference adapters for `video-analysis`.

## Feature flags

- `onnxruntime`: enables the ONNX Runtime-backed backend
- `external-tests`: enables ignored runtime smoke tests

## Example

```rust
# fn main() -> Result<(), Box<dyn std::error::Error>> {
use image_analysis_onnx::OnnxImagePreprocessing;
use num_rational::Rational64;
use video_analysis_core::{FramePosition, PixelFormat, VideoFrame};
use video_analysis_onnx::preprocess_frame;

let bytes = vec![255_u8; 2 * 2 * 3];
let frame = VideoFrame::packed(
    FramePosition::from_frame_index(0, Rational64::new(1, 1)),
    2,
    2,
    PixelFormat::Rgb24,
    &bytes,
    6,
)?;
let tensor = preprocess_frame(&frame, &OnnxImagePreprocessing::default())?;
assert_eq!(tensor.channels, 3);
# Ok(())
# }
```

## Related crates

- `model-runtime`
- `image-analysis-onnx`
- `video-analysis-cli`
