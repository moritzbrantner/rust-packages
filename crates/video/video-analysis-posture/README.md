# video-analysis-posture

Pose and skeleton helpers for `video-analysis`.

## Feature flags

- No optional feature flags today.

## Example

```rust,ignore
use video_analysis_posture::{JointName, Skeleton};

let skeleton = Skeleton::default();
let _ = skeleton.joint(JointName::Head);
```

## Related crates

- `video-analysis-recognition`
- `video-analysis-tracking`
