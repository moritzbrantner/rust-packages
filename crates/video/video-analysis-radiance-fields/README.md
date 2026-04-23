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
