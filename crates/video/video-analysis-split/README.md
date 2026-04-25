# video-analysis-split

FFmpeg-backed scene splitting utilities for `video-analysis`.

## Feature flags

- `external-tests`: enables ignored FFmpeg split tests

## Example

```rust,ignore
use video_analysis_split::{build_split_plan, SplitOptions};
use video_analysis_core::Scene;

let scenes: Vec<Scene> = Vec::new();
let plan = build_split_plan("video.mp4", &scenes, &SplitOptions::default()).unwrap();
assert!(plan.jobs.is_empty());
```

## Related crates

- `video-analysis-detectors`
- `video-analysis-ffmpeg`
- `video-analysis-cli`
