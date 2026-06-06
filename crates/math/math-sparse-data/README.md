# math-sparse-data

Sparse vector and matrix contracts for text, retrieval, and feature indexing.

## Highlights

- Checked sparse vectors and COO/CSR matrix formats
- Canonicalization of unsorted indices
- Sparse dot and cosine similarity helpers
- Sparse vector norms, scaling, addition, Hadamard product, pruning, and top-k entries
- CSR row/column counts and sums, matrix summaries, row normalization,
  matrix-vector multiply, dense matrix multiply, COO/CSR transpose, and COO round trips
- Dense and sparse conversion bridges

## Example

```rust,no_run
use math_linear::F32Matrix;
use math_sparse_data::{CooMatrix, SparseVector};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let vector = SparseVector::new(4, vec![0, 3], vec![1.0, 2.0])?.canonicalized()?;
    let matrix = CooMatrix::new(2, 4, vec![(0, 0, 1.0), (1, 3, 2.0)])?;
    let csr = matrix.to_csr()?;
    let dense = csr.to_dense_matrix()?;
    let right = F32Matrix::from_rows([[1.0], [2.0], [3.0], [4.0]])?;
    let product = csr.mul_dense_matrix(&right.as_view())?;
    assert_eq!(vector.to_dense(), vec![1.0, 0.0, 0.0, 2.0]);
    assert_eq!(matrix.nnz(), 2);
    assert_eq!(dense.shape().rows, 2);
    assert_eq!(product.shape().cols, 1);
    Ok(())
}
```

## Behavior

COO inputs are canonicalized before CSR construction, combining duplicate
coordinates and dropping exact zero stored values. Matrix summaries and dense
products are deterministic helpers for small and medium sparse feature
matrices. Dense matrix outputs use `math-linear::F32Matrix`, allowing sparse
feature workflows to move into linear algebra and statistics without adding an
external math backend.

## Runtime Surface

The package surface exposes sparse vector similarity, dense conversion, matrix
summary/statistics, vector operations, matrix-vector multiplication, and
transpose operations. Successful responses preserve sparse result fields and add
the shared `operation`, `title`, `message`, `summary`, and `result` fields.

Default surface calls are deterministic and in-memory. They reject more than
100,000 vector values or 100,000 COO matrix entries with typed
`runtime_core::SurfaceError` JSON. Unsupported sparse metrics and matrix formats
also return typed surface errors.

## Related crates

- `text-lexical`
- `text-embeddings`
- `vector-analysis-core`
