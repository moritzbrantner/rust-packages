# video-analysis-dataset

Serializable retained analysis records for `moritzbrantner-video-analysis`.

## Feature flags

- No optional feature flags today.

## Example

```rust,ignore
use video_analysis_dataset::AnalysisDataset;

let mut dataset = AnalysisDataset::default();
dataset.sort_records();

let _ = dataset;
```

## Related crates

- `video-analysis-core`
- `video-analysis-storage`
- `video-analysis-transform`
