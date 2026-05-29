# math-statistics

Shared multivariate statistics for dense matrix inputs and streaming
observations.

## Highlights

- Streaming covariance accumulation
- Z-score and min/max normalizers
- Dense covariance matrix generation
- PCA-lite for small and medium dense inputs
- Deterministic power-iteration PCA with fixed iteration count

## Example

```rust,no_run
use math_linear::F32Matrix;
use math_statistics::{PrincipalComponents, RunningCovariance};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let matrix = F32Matrix::from_rows([[1.0, 0.0], [0.0, 1.0], [1.0, 1.0]])?;
    let covariance = RunningCovariance::from_matrix(&matrix.as_view())?.covariance_matrix()?;
    let pca = PrincipalComponents::fit(&matrix.as_view(), 1)?;
    assert_eq!(covariance.matrix.shape().rows, 2);
    assert_eq!(pca.components().shape().rows, 1);
    Ok(())
}
```

## Behavior

`WeightedObservation` requires at least one finite value and a positive finite
weight. `RunningCovariance` has a fixed dimensionality; every pushed
observation must match it. `count` is the number of observations pushed, while
`weight_sum` is the sum of their weights.

Covariance is reported as weighted population covariance: accumulated second
moments are divided by `weight_sum`, not by `count - 1`. `from_matrix` treats
each matrix row as one unit-weight observation.

`CovarianceMatrix::correlation_matrix` divides each covariance cell by the
product of the corresponding standard deviations. Degenerate variance terms are
clamped with `f32::EPSILON` in the denominator so the transform remains finite.

`ZScoreNormalizer` fits per-column means and standard deviations from matrix
rows. Constant columns use an epsilon-scale standard deviation, so transforming
the same fitted constant values yields zero rather than `NaN`.

`MinMaxNormalizer` fits per-column ranges and delegates normalization to
`numbers-core::NumberRange`. Degenerate ranges normalize the exact range value
to `0.0`.

`PrincipalComponents` is a small deterministic PCA helper. It extracts
components from the covariance matrix with a fixed 32-step power iteration and
simple deflation. It is intended for predictable package workflows, not as a
replacement for a full numerical linear algebra backend on ill-conditioned or
large matrices.

## Related crates

- `dense-data`
- `video-analysis-features`
- `text-embeddings`
