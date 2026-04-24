# video-analysis-posture-io

COCO-style posture JSON and 3D stick-figure export helpers for `video-analysis`.

## Feature flags

- No optional feature flags today.

## Example

```rust,ignore
use video_analysis_posture_io::{read_coco_keypoints_json, write_stick_figure_gltf};

let poses = read_coco_keypoints_json("poses.json")?;
let figure = poses[0].to_stick_figure_3d()?;
write_stick_figure_gltf("pose.gltf", &figure)?;
```

## Related crates

- `video-analysis-posture`
- `three-d-processing-io`
