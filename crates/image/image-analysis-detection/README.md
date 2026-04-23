# image-analysis-detection

Object detection built from image segmentation masks and SAM defaults for `video-analysis`.

## Feature flags

- No optional feature flags today.

## Example

```rust,ignore
use image_analysis_detection::{ImageDetectionRequest, SegmentBackedDetector};

let detector = SegmentBackedDetector::default();
let request = ImageDetectionRequest::default();
let _ = detector.detect(&request)?;
```

## Related crates

- `image-analysis-core`
- `image-analysis-segmentation`
- `video-analysis-models`
