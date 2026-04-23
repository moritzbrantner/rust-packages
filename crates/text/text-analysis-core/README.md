# text-analysis-core

Shared text documents, tokenization, spans, and statistics for `video-analysis`.

## Feature flags

- No optional feature flags today.

## Example

```rust,ignore
use text_analysis_core::{tokenize, TextDocument, TextProcessingOptions};

let document = TextDocument::new("Rust-first multimodal analysis.");
let tokens = tokenize(document, &TextProcessingOptions::default());

assert!(!tokens.is_empty());
```

## Related crates

- `text-analysis-features`
- `text-analysis-corpus`
- `video-analysis-core`
