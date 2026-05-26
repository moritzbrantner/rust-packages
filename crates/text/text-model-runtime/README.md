# text-model-runtime

Shared tokenizer input types and native text runtime traits for `video-analysis`.

Default builds are deterministic and do not execute native model runtimes. Enable
`tokenizers`, `onnx`, or `candle` only where native runtime support is required.

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
