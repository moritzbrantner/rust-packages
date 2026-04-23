# video-analysis-split

FFmpeg-backed scene splitting utilities for `video-analysis`.

## Feature flags

- `external-tests`: enables ignored FFmpeg split tests

## Example

```rust,ignore
use video_analysis_split::SplitPlan;

let plan = SplitPlan::default();
let _ = plan.render_commands("video.mp4")?;
```

## Related crates

- `video-analysis-detectors`
- `video-analysis-ffmpeg`
- `video-analysis-cli`
