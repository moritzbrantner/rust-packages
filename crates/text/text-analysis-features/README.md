# text-analysis-features

Text feature extraction and analyzer adapters for `video-analysis`.

## Feature flags

- No optional feature flags today.

## Example

```rust,ignore
use text_analysis_features::{extract_keywords, KeywordOptions};

let keywords = extract_keywords(
    "Video analysis surfaces scenes, transcript events, and metrics.",
    &KeywordOptions::default(),
)?;

let _ = keywords;
```

## Related crates

- `text-analysis-core`
- `text-analysis-semantics`
- `video-analysis-core`
