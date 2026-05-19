# text-analysis-models

Optional model-backed text classification and embedding adapters for `video-analysis`.

## Feature flags

- `tokenizers`: tokenizer loading support
- `onnx`: ONNX-backed text execution
- `candle`: Candle-backed text execution
- `external-tests`: ignored download/bundle validation tests plus cheap runtime-adjacent smoke checks
- `slow-external-tests`: ignored ONNX runtime execution tests that are too slow for routine certification

CUDA-Oxide is opt-in through `TextRuntimeBackend::CudaOxide`,
`cuda_oxide_backend_priority`, and `CudaOxideTextRuntimeTarget`. The runtime
target carries the cuda-oxide module/kernel names and CUDA device metadata while
the kernel package itself is built with `cargo oxide`.

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
let cuda_first = text_analysis_models::cuda_oxide_backend_priority();

assert_eq!(backend, TextRuntimeBackend::Onnx);
assert_eq!(cuda_first[0], TextRuntimeBackend::CudaOxide);
let _ = (default_tokenizer, bert_tokenizer, catalog);
```

`from_default_cached` and `from_cached_source` download the tokenizer on first
use through the Hugging Face cache and reuse the cached file on subsequent
calls. By default that cache lives outside the git worktree.

For ONNX debugging, set `VIDEO_ANALYSIS_ONNX_TIMING=1` to log timing for
`NativeOnnxRunner::new`, including `Session::builder`, builder configuration,
`commit_from_file`, and the first `session.run`. The text ONNX runner currently
uses a conservative CPU-only session profile with single-threaded execution and
disabled graph optimization. The slow ONNX runtime tests also require
`RUN_SLOW_ONNX_TESTS=1` in addition to `--features slow-external-tests`.

## Related crates

- `text-analysis-semantics`
- `video-analysis-models`
- `video-analysis-onnx`
