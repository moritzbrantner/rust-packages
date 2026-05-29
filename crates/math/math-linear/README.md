# math-linear

Dense matrix and kernel contracts bridging `tensor-data` and
`vector-analysis-core`.

## Highlights

- Checked small dense matrix shapes and views
- Row and column iteration with transpose views
- Matrix multiply, matrix-vector multiply, and row cosine helpers
- Shared `Kernel2d` and `Kernel1d` types for image and video processing
- Bridges between rank-2 tensors, dense vectors, and matrices

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

## Behavior

`MatrixShape` rejects zero rows or columns and computes element counts with
checked multiplication. `F32Matrix` stores values in row-major order. A
transposed `F32MatrixView` swaps shape and layout metadata without copying the
underlying values.

Matrix construction and validation require the value count to match
`rows * cols`, and every value must be finite. Matrix/vector multiplication also
requires a finite vector whose length matches the matrix column count.

Matrix multiplication requires `left.cols == right.rows`. Pairwise row dot and
pairwise row cosine require both inputs to have the same column count and return
a matrix shaped `left.rows x right.rows`.

Row and column L2 normalization use `vector-analysis-core` normalization rules.
Rows or columns with an effectively zero norm return an error instead of
producing non-finite values.

`Kernel2d` values are stored in row-major order and are not normalized
automatically. For example, `Kernel2d::blur_3x3()` contains nine `1.0`
coefficients; callers can scale the output or coefficients when they need an
averaging blur.

Rank-2 `tensor-data::F32Tensor` values can be converted to matrices, and
matrices can be converted back to tensors with shape `[rows, cols]`.

## Related crates

- `tensor-data`
- `vector-analysis-core`
- `image-analysis-processing`
