# dense-data

Deterministic dense point datasets, bucketing, and clustering for `video-analysis`.

## Feature flags

- No optional feature flags today.

## Example

```rust,ignore
use dense_data::{DenseDataset, DensePoint};

let dataset = DenseDataset::new(vec![
    DensePoint::new(vec![0.0, 0.0], 1.0)?,
    DensePoint::new(vec![1.0, 1.0], 2.0)?,
])?;

let _clusters = dataset.k_means(2)?;
```

## Related crates

- `vector-analysis-core`
- `video-analysis-data`
