# text-model-runtime

Shared tokenizer input types and native text runtime traits for `moritzbrantner-video-analysis`.

Default builds are deterministic and do not execute native model runtimes. Enable
`tokenizers`, `onnx`, or `candle` only where native runtime support is required.

## Stable contract

The stable surface is tokenizer input DTOs, runtime backend traits, raw
prediction records, softmax helpers, model load/run conformance reports, and
bundle validation without downloads.

## Quality and limits

This crate centralizes runtime contracts; it is not a model acquisition policy
or hosted inference client. Feature gates expose native paths, but callers must
explicitly select and configure them.

## Candle Devices

Candle-backed text execution defaults to CPU. Native server binaries can request
CUDA at startup when they are built with the opt-in `cuda` feature:

```bash
cargo run -p text-analysis-server -- --addr 127.0.0.1:3000
cargo run -p text-analysis-server --features cuda -- --cuda --cuda-device-index 0
```

CUDA is native-server only. It requires a CUDA-capable host plus Candle's CUDA
build prerequisites, and startup fails before binding if the requested CUDA
device cannot be initialized. WASM Candle bindings remain CPU-only.

## Package surface

- Primary workflow: `runtime.tokenizeSummary` builds a deterministic
  whitespace-token summary without downloads or native model execution.
- Workflow operations: `runtime.tokenizeSummary`, `runtime.bundleCheck`, and
  `runtime.tokenizerProbe`.
- Support operations: `runtime.softmax` exposes the shared stable softmax
  helper.
- Debug operations: `describe` inspects package metadata and operation support.
- Runtime support: pure Rust package-surface helpers are available through
  library, CLI, server, and WASM wrappers.
- Sample output includes `title`, `message`, `summary`, `result`, and
  operation-specific fields such as `tokens`, `tokenized`, readiness reports,
  or `probabilities`.
- The package surface does not load tokenizers or execute ONNX/Candle runtimes;
  those paths remain feature-gated library/server concerns. Readiness workflows
  inspect local paths only and never download model files.

## Model Conformance

The crate exposes `TextModelLoadReport`, `TextModelRunReport`, and
`TextModelCapability` for text packages that need to show whether a model is
loadable or reference-only. `validate_text_model_bundle` checks local bundle
presence without downloads; `validate_tokenizer_bundle` additionally attempts a
feature-gated tokenizer load and sample tokenization.

Opt-in checks reuse `.model-runtime`:

```bash
scripts/sync_model_bundles.sh
cargo test -p text-model-runtime --features external-tests -- --ignored
```
