# text-embeddings

Lightweight semantic text embeddings and search for `video-analysis`.

Default builds use deterministic hashed/local embedding behavior. Native model
execution and tokenizer-backed model support are opt-in through explicit feature
flags.

## Feature flags

- `tokenizers`: tokenizer loading and model bundle tokenizer support
- `onnx`: ONNX-backed text embedding runtime support
- `candle`: Candle-backed text embedding runtime support
- `external-tests`: opt-in model/runtime checks

## Example

```rust,no_run
use text_lexical::CorpusOptions;
use text_embeddings::{HashedTextEmbedder, SemanticTextIndex, TextEmbeddingConfig};

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

- `text-core`
- `vector-analysis-index`
- `text-lexical`
