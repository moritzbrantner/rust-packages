# video-analysis-onnx

ONNX-backed video model inference adapters for `moritzbrantner-video-analysis`.

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

## Package surface

Workflow operations:

- `video.onnx.modelSummary`
- `video.onnx.decodeDetections`

Debug operations:

- `describe`
- `video.onnx.inputPlan`

Runtime limits:

The package surface reports deterministic model and tensor contracts. ONNX Runtime execution remains opt-in behind the `onnxruntime` feature.

Invalid input returns a clear error through `run_surface_operation`; successful
responses include `operation`, `title`, `message`, `summary`, and `result` while
keeping existing top-level domain fields for compatibility.
