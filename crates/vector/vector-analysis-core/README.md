# vector-analysis-core

Dense vector validation and metrics for `video-analysis`.

## Feature flags

- No optional feature flags today.

## Example

```rust,ignore
use vector_analysis_core::{cosine_similarity, DenseVector};

let a = DenseVector::new(vec![1.0, 0.0, 0.0])?;
let b = DenseVector::new(vec![0.9, 0.1, 0.0])?;

let _ = cosine_similarity(a.as_slice(), b.as_slice())?;
```

## Related crates

- `vector-analysis-index`
- `text-analysis-semantics`
- `video-analysis-recognition`
