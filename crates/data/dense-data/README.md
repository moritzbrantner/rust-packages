# dense-data

Deterministic dense point datasets, bucketing, and clustering for `video-analysis`.

## Highlights

- Weighted dense point summaries with per-dimension stats
- `math-linear` matrix export for dense point coordinates
- `math-statistics` covariance and PCA helpers
- Deterministic fixed-grid bucketing
- Deterministic k-means clustering
- Dataset and point-set helpers for tables, charts, and media-derived features

## Example

```rust,no_run
use dense_data::{BucketGrid, DenseDataset, DensePoint, KMeansConfig};

fn main() -> Result<(), Box<dyn std::error::Error>> {
let dataset = DenseDataset::from_points([
    DensePoint::new([0.0, 0.0])?.named("left"),
    DensePoint::new([1.0, 1.0])?
        .named("right")
        .weighted(2.0)?
        .valued(4.0)?,
])?;

let summary = dataset.summary()?;
assert_eq!(summary.dimensions, 2);
assert_eq!(summary.coordinate_stats[0].mean, Some(2.0 / 3.0));

let buckets = dataset.buckets(&BucketGrid::uniform(2, 1.0)?)?;
assert_eq!(buckets.len(), 2);

let covariance = dataset.covariance_matrix()?;
assert_eq!(covariance.matrix.shape().rows, 2);

let clusters = dataset.k_means(KMeansConfig::new(2)?)?;
assert_eq!(clusters.clusters.len(), 2);
    Ok(())
}
```

## Related crates

- `math-linear`
- `math-statistics`
- `numbers-core`
- `vector-analysis-core`
- `video-analysis-data`
