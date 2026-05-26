# text-core

Shared text documents, tokenization, spans, and statistics for `video-analysis`.

Default builds are deterministic, local-first, and do not download models or
invoke native inference/runtime tools.

## Feature flags

- No optional feature flags today.

## Example

```rust,no_run
use text_core::{build_annotation_graph, TextDocument, TextProcessingOptions};

let document = TextDocument::new("doc-1", "Rust-first multimodal analysis.");
let graph = build_annotation_graph(document.text, &TextProcessingOptions::default());

assert!(!graph.tokens.is_empty());
assert_eq!(graph.sentences.len(), 1);
```

## Related crates

- `text-lexical`
- `text-lexical`
- `video-analysis-core`
