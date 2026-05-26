# text-analysis

Unified text analysis orchestration for `video-analysis`.

The crate composes the lower-level text packages into document and corpus
reports. Defaults are deterministic, local-first, and do not download models or
invoke native inference runtimes. Local model-backed NER and embeddings are
available through explicit options and feature flags.

## Example

```rust,no_run
use text_analysis::{analyze_text, DocumentAnalysisOptions};

let report = analyze_text(
    "doc-1",
    "Alice presented the tokenizer roadmap in Berlin.",
    &DocumentAnalysisOptions::default(),
)?;

assert!(!report.lexical.keywords.is_empty());
# Ok::<(), video_analysis_core::DetectError>(())
```

## Feature flags

- `tokenizers`: enables tokenizer-backed model bundle helpers.
- `candle`: enables local Candle-backed NER and embedding integration.
- `onnx`: enables local ONNX-backed embedding integration.
- `external-tests`: enables model/runtime integration test features.

## Related crates

- `text-core`
- `text-lexical`
- `text-linguistics`
- `text-embeddings`
- `text-retrieval`
