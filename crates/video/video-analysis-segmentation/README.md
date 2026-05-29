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
- `model-runtime`

## Package surface

Workflow operations:

- `video.segmentation.maskSummary`

Debug operations:

- `describe`
- `video.segmentation.promptPlan`
- `video.segmentation.trackPlan`

Runtime limits:

Operations are deterministic, local-first, and side-effect free. They return inline JSON reports and do not download models, write files, or run native tools.

Invalid input returns a clear error through `run_surface_operation`; successful
responses include `operation`, `title`, `message`, `summary`, and `result` while
keeping existing top-level domain fields for compatibility.
