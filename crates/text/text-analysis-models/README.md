# text-analysis-models

Optional model-backed text classification and embedding adapters for `video-analysis`.

## Feature flags

- `tokenizers`: tokenizer loading support
- `onnx`: ONNX-backed text execution
- `candle`: Candle-backed text execution
- `external-tests`: ignored integration tests for model runtimes

## Example

```rust,ignore
use text_analysis_models::{TextClassifierBundle, TokenizerBundle};

let tokenizer = TokenizerBundle::from_dir("bundles/sentiment")?;
let classifier = TextClassifierBundle::from_dir("bundles/sentiment", tokenizer)?;

let _ = classifier;
```

## Related crates

- `text-analysis-semantics`
- `video-analysis-models`
- `video-analysis-onnx`
