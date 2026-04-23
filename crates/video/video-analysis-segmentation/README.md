# video-analysis-segmentation

Video segmentation primitives and SAM 2 defaults for `video-analysis`.

## Feature flags

- No optional feature flags today.

## Example

```rust,ignore
use video_analysis_segmentation::{default_sam_video_model_spec, VideoSegmentationRequest};

let _ = default_sam_video_model_spec();
let _ = VideoSegmentationRequest::default();
```

## Related crates

- `image-analysis-segmentation`
- `video-analysis-models`
