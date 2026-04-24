# text-analysis-models

Optional model-backed text classification and embedding adapters for `video-analysis`.

## Feature flags

- `tokenizers`: tokenizer loading support
- `onnx`: ONNX-backed text execution
- `candle`: Candle-backed text execution
- `external-tests`: ignored integration tests for model runtimes

## Example

```rust,no_run
use text_analysis_models::{
    select_text_runtime_backend, TextRuntimeBackend, TextRuntimeCatalog, TextRuntimeConfig,
    TokenizerBundle, TokenizerPreset, TokenizerSource,
};

let default_tokenizer = TokenizerBundle::from_default_cached().unwrap();
let bert_tokenizer = TokenizerBundle::from_cached_source(
    TokenizerSource::preset(TokenizerPreset::BertBaseUncased),
) .unwrap();
let runtime = TextRuntimeConfig::default();
let catalog = TextRuntimeCatalog::default();
let backend = select_text_runtime_backend(&runtime.backend_priority);

assert_eq!(backend, TextRuntimeBackend::Onnx);
let _ = (default_tokenizer, bert_tokenizer, catalog);
```

`from_default_cached` and `from_cached_source` download the tokenizer on first
use through the Hugging Face cache and reuse the cached file on subsequent
calls. By default that cache lives outside the git worktree.

## Related crates

- `text-analysis-semantics`
- `video-analysis-models`
- `video-analysis-onnx`
