# video-analysis-transform

Filtering, joins, grouping, and resampling for `moritzbrantner-video-analysis` datasets.

## Feature flags

- No optional feature flags today.

## Example

```rust,ignore
use video_analysis_transform::DatasetTransformPipeline;

let pipeline = DatasetTransformPipeline::default();
let _ = pipeline;
```

## Related crates

- `video-analysis-dataset`
- `video-analysis-features`
