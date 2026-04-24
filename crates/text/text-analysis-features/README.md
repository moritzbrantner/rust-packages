# text-analysis-features

Text feature extraction and analyzer adapters for `video-analysis`.

## Feature flags

- No optional feature flags today.

## Example

```rust,ignore
use text_analysis_core::TextProcessingOptions;
use text_analysis_features::{keywords, token_shingle_similarity, KeywordOptions};

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

- `text-analysis-core`
- `text-analysis-semantics`
- `video-analysis-core`
