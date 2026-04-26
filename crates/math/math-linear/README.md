# math-linear

Dense matrix and kernel contracts bridging `tensor-data` and
`vector-analysis-core`.

## Highlights

- Checked small dense matrix shapes and views
- Row and column iteration with transpose views
- Matrix multiply, matrix-vector multiply, and row cosine helpers
- Shared `Kernel2d` and `Kernel1d` types for image and video processing

## Example

```rust,no_run
use math_linear::{F32Matrix, Kernel2d};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let matrix = F32Matrix::from_rows([[1.0, 0.0], [0.0, 1.0]])?;
    let product = matrix.matmul(&matrix.as_view())?;
    let kernel = Kernel2d::sharpen_3x3();
    assert_eq!(product.shape().rows, 2);
    assert_eq!(kernel.as_array_3x3()?, [0.0, -1.0, 0.0, -1.0, 5.0, -1.0, 0.0, -1.0, 0.0]);
    Ok(())
}
```

## Related crates

- `tensor-data`
- `vector-analysis-core`
- `image-analysis-processing`
