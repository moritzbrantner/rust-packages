# image-analysis-processing

CPU image processing primitives for `video-analysis`.

## Feature flags

- No optional feature flags today.

## Example

```rust,ignore
use image_analysis_processing::{ImageOperation, ImageProcessor};

let processor = ImageProcessor::new(vec![
    ImageOperation::Grayscale,
    ImageOperation::Threshold(110),
]);

let _ = processor;
```

## Related crates

- `image-analysis-core`
- `video-analysis-editing`
