# text-analysis-server

HTTP API adapter for `text-analysis`.

```bash
cargo run -p text-analysis-server -- --addr 127.0.0.1:3000
cargo run -p text-analysis-server --features cuda -- --cuda --cuda-device-index 0
```

`--cuda` is available only for native CUDA-capable builds. Default launches are
CPU-only, and WASM package surfaces remain CPU-only.
