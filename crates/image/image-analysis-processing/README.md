# image-analysis-processing

CPU image processing primitives for `video-analysis`.

## Feature flags

- No optional feature flags today.

## Example

```rust
# fn main() -> Result<(), Box<dyn std::error::Error>> {
use image_analysis_processing::{ImageOperation, ImageProcessor};

let processor = ImageProcessor::new()
    .operation(ImageOperation::Grayscale)
    .operation(ImageOperation::Threshold { level: 110 });

let _ = processor;
# Ok(())
# }
```

## Related crates

- `image-analysis-core`
- `video-analysis-editing`
