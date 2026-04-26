# math-statistics

Shared multivariate statistics for dense matrix inputs and streaming
observations.

## Highlights

- Streaming covariance accumulation
- Z-score and min/max normalizers
- Dense covariance matrix generation
- PCA-lite for small and medium dense inputs

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

## Related crates

- `dense-data`
- `video-analysis-features`
- `text-analysis-semantics`
