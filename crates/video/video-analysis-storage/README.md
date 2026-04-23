# video-analysis-storage

Dataset persistence for `video-analysis`.

## Feature flags

- No optional feature flags today.

## Example

```rust,ignore
use video_analysis_storage::{load_dataset_dir, write_dataset_dir};

write_dataset_dir("output/report", &Default::default())?;
let dataset = load_dataset_dir("output/report")?;

let _ = dataset;
```

## Related crates

- `video-analysis-dataset`
- `video-analysis-output`
