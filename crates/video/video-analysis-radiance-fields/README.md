# video-analysis-radiance-fields

Radiance-field cameras, rays, and volumes for `video-analysis`.

## Feature flags

- No optional feature flags today.

## Example

```rust,ignore
use video_analysis_radiance_fields::{CameraIntrinsics, Ray};

let intrinsics = CameraIntrinsics::default();
let ray = Ray::default();

let _ = intrinsics;
let _ = ray;
```

## Related crates

- `video-analysis-gaussian-splatting`
- `video-analysis-radiance-io`
- `video-analysis-reconstruction`

## Package surface

Workflow operations:

- `video.radiance.fieldSummary`
- `video.radiance.cameraPath`

Debug operations:

- `describe`
- `video.radiance.renderPlan`

Runtime limits:

Operations are deterministic, local-first, and side-effect free. They return inline JSON reports and do not download models, write files, or run native tools.

Invalid input returns a clear error through `run_surface_operation`; successful
responses include `operation`, `title`, `message`, `summary`, and `result` while
keeping existing top-level domain fields for compatibility.
