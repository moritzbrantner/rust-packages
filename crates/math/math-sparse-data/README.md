# math-sparse-data

Sparse vector and matrix contracts for text, retrieval, and feature indexing.

## Highlights

- Checked sparse vectors and COO/CSR matrix formats
- Canonicalization of unsorted indices
- Sparse dot and cosine similarity helpers
- Dense and sparse conversion bridges

## Example

```rust,no_run
use math_sparse_data::{CooMatrix, SparseVector};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let vector = SparseVector::new(4, vec![0, 3], vec![1.0, 2.0])?.canonicalized()?;
    let matrix = CooMatrix::new(2, 4, vec![(0, 0, 1.0), (1, 3, 2.0)])?;
    assert_eq!(vector.to_dense(), vec![1.0, 0.0, 0.0, 2.0]);
    assert_eq!(matrix.nnz(), 2);
    Ok(())
}
```

## Related crates

- `text-analysis-corpus`
- `text-analysis-semantics`
- `vector-analysis-core`
