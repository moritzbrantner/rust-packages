# math-linear

Dense matrix and kernel contracts bridging `moritzbrantner-tensor-data` and
`moritzbrantner-vector-analysis-core`.

## Highlights

- Checked small dense matrix shapes and views
- Row and column iteration with transpose views
- Identity, zero, transpose, add, subtract, scale, trace, and mean utilities
- Diagonal construction, Gram matrices, and row/column centering
- Matrix multiply, matrix-vector multiply, and row cosine helpers
- Pure Rust LU decomposition with partial pivoting, determinant, solve, and
  inverse helpers
- Pure Rust Cholesky and modified Gram-Schmidt QR decomposition for
  deterministic small and medium matrix workflows
- Shared `Kernel2d` and `Kernel1d` types for image and video processing
- Bridges between rank-2 tensors, dense vectors, and matrices

## Example

```rust,no_run
use math_linear::{F32Matrix, Kernel2d};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let matrix = F32Matrix::from_rows([[2.0, 1.0], [1.0, 3.0]])?;
    let product = matrix.matmul(&matrix.as_view())?;
    let solution = matrix.solve_vector(&[1.0, 2.0])?;
    let inverse = matrix.inverse()?;
    let kernel = Kernel2d::sharpen_3x3();
    assert_eq!(product.shape().rows, 2);
    assert_eq!(solution.len(), 2);
    assert_eq!(inverse.shape().cols, 2);
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

Owned utility results are emitted as row-major `F32Matrix` values. Transpose
views do not copy, while `transpose_owned()` writes the logical transpose into a
new row-major buffer. Elementwise add, subtract, and scale validate shapes and
reject non-finite scale factors before producing owned output.

Matrix multiplication requires `left.cols == right.rows`. Pairwise row dot and
pairwise row cosine require both inputs to have the same column count and return
a matrix shaped `left.rows x right.rows`.

LU decomposition is implemented in pure Rust for deterministic small and
medium-size matrix workflows. It uses partial pivoting, rejects non-square and
singular or near-singular matrices, and powers determinant, vector solve, matrix
solve, and inverse helpers. This crate is intended as an internal matrix backend
layer for workspace packages, not as a full replacement for specialized
numerical linear algebra backends.

Cholesky decomposition requires a symmetric positive definite square matrix. QR
decomposition currently computes a thin factorization and requires
`rows >= cols` with full column rank.

Future adapters for libraries such as faer or nalgebra can be added behind
private feature-gated backend modules while keeping the public API owned by
`math-linear` types and functions.

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
