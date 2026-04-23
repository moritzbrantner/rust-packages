# video-analysis-radiance-pipeline

External radiance-field pipeline orchestration for `video-analysis`.

## Feature flags

- `external-tests`: enables ignored external pipeline smoke tests

## Example

```rust,ignore
use video_analysis_radiance_pipeline::{RadiancePipeline, RadiancePipelineOptions};

let pipeline = RadiancePipeline::new(RadiancePipelineOptions::default())?;
let _ = pipeline;
```

## Related crates

- `video-analysis-radiance-io`
- `video-analysis-ffmpeg`
- `video-analysis-use-cases`
