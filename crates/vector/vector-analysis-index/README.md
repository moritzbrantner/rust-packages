# vector-analysis-index

Exact in-memory vector search for `moritzbrantner-video-analysis`.

## Feature flags

- No optional feature flags today.

## Example

```rust,ignore
use vector_analysis_core::DenseVector;
use vector_analysis_index::VectorIndex;

let mut index = VectorIndex::new(3)?;
index.insert("a", DenseVector::new(vec![1.0, 0.0, 0.0])?)?;

let _ = index.search(DenseVector::new(vec![0.9, 0.1, 0.0])?.as_slice(), 5)?;
```

## Related crates

- `vector-analysis-core`
- `text-embeddings`
