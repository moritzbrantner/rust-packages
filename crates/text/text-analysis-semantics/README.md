# text-analysis-semantics

Lightweight semantic text embeddings and search for `video-analysis`.

## Feature flags

- No optional feature flags today.

## Example

```rust,ignore
use text_analysis_semantics::{HashedTextEmbedder, SemanticTextIndex};

let embedder = HashedTextEmbedder::default();
let mut index = SemanticTextIndex::new(embedder.dimensions())?;

index.insert("scene-1", embedder.embed("opening scene with presenter"))?;
let _ = index.search(embedder.embed("presenter on screen")?, 5)?;
```

## Related crates

- `text-analysis-core`
- `vector-analysis-index`
- `text-analysis-corpus`
