# text-analysis-core

Shared text documents, tokenization, spans, and statistics for `video-analysis`.

## Feature flags

- No optional feature flags today.

## Example

```rust,no_run
use text_analysis_core::{build_annotation_graph, TextDocument, TextProcessingOptions};

let document = TextDocument::new("doc-1", "Rust-first multimodal analysis.");
let graph = build_annotation_graph(document.text, &TextProcessingOptions::default());

assert!(!graph.tokens.is_empty());
assert_eq!(graph.sentences.len(), 1);
```

## Related crates

- `text-analysis-features`
- `text-analysis-corpus`
- `video-analysis-core`
