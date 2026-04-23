# video-analysis-data

Normalized stream records and online aggregation for `video-analysis`.

## Feature flags

- No optional feature flags today.

## Example

```rust,ignore
use video_analysis_data::{DataBucketOptions, StreamAggregator};

let mut aggregator = StreamAggregator::new(DataBucketOptions::default())?;
let _ = aggregator.finish();
```

## Related crates

- `video-analysis-core`
- `video-analysis-dataset`
- `dense-data`
