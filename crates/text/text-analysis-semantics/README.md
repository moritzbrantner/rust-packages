# text-analysis-semantics

Lightweight semantic text embeddings and search for `video-analysis`.

## Feature flags

- No optional feature flags today.

## Example

```rust,no_run
use text_analysis_corpus::CorpusOptions;
use text_analysis_semantics::{HashedTextEmbedder, SemanticTextIndex, TextEmbeddingConfig};

let embedder = HashedTextEmbedder::new(
    TextEmbeddingConfig {
        dimensions: 128,
        use_idf: true,
    },
    CorpusOptions::default(),
) .unwrap();
let mut index = SemanticTextIndex::new(embedder);

index.add_document("scene-1", "opening scene with presenter").unwrap();
let matches = index.search("presenter on screen", 5).unwrap();

assert_eq!(matches[0].metadata.model_name.as_deref(), Some("hashed-text-embedder"));
```

## Related crates

- `text-analysis-core`
- `vector-analysis-index`
- `text-analysis-corpus`
