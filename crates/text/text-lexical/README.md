# text-lexical

Text feature extraction and analyzer adapters for `video-analysis`.

Default builds are deterministic, local-first, and limited to classical lexical
analysis; they do not download models or invoke native inference/runtime tools.

## Feature flags

- No optional feature flags today.

## Example

```rust,ignore
use text_core::TextProcessingOptions;
use text_lexical::{keywords, token_shingle_similarity, KeywordOptions};

let keywords = keywords(
    "Video analysis surfaces scenes, transcript events, and metrics.",
    &KeywordOptions::default(),
);

let similarity = token_shingle_similarity(
    "scene transitions follow motion",
    "scene transitions follow dialogue",
    2,
    &TextProcessingOptions::default(),
)?;

let _ = (keywords, similarity);
```

## Related crates

- `text-core`
- `text-embeddings`
- `video-analysis-core`
