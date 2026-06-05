# math-statistics

Shared scalar, pairwise, rolling, multivariate, and matrix statistics for
finite local inputs.

## Highlights

- Streaming covariance accumulation
- Scalar series summaries, sample/population variance, and z-scores
- Pairwise covariance and correlation
- Average ranks, Spearman correlation, simple linear regression, and OLS
  regression over dense design matrices
- Difference, relative-change, and log-ratio derived series
- Rolling mean, standard deviation, min/max ranges, and correlation
- Empirical tail risk and compounded-path drawdown helpers
- Z-score and min/max normalizers
- Dense covariance matrix generation
- PCA-lite for small and medium dense inputs
- Deterministic power-iteration PCA with fixed iteration count

## Example

```rust,no_run
use math_linear::F32Matrix;
use math_statistics::{changes, summarize_series, ChangeMethod, PrincipalComponents, RunningCovariance, VarianceMode};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let returns = changes(&[100.0, 102.0, 99.0], ChangeMethod::Relative)?;
    let summary = summarize_series(&returns, VarianceMode::Sample)?;

    let matrix = F32Matrix::from_rows([[1.0, 0.0], [0.0, 1.0], [1.0, 1.0]])?;
    let covariance = RunningCovariance::from_matrix(&matrix.as_view())?.covariance_matrix()?;
    let pca = PrincipalComponents::fit(&matrix.as_view(), 1)?;
    assert_eq!(summary.count, 2);
    assert_eq!(covariance.matrix.shape().rows, 2);
    assert_eq!(pca.components().shape().rows, 1);
    Ok(())
}
```

## Behavior

Scalar series helpers reject empty or non-finite inputs. Sample variance uses
`n - 1` and requires at least two observations; population variance uses `n`.
Relative changes and log-ratios require strictly positive adjacent values.
Drawdown and compounded-return helpers require period values greater than or
equal to `-1.0`.

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

OLS uses the `math-linear` QR path for full-column-rank designs and falls back
to normal equations when QR cannot be applied.

## Related crates

- `dense-data`
- `video-analysis-features`
- `text-embeddings`
