# image-analysis-onnx-server

Thin HTTP API adapter for `image-analysis-onnx`.

Run:

```bash
cargo run -p image-analysis-onnx-server -- --addr 127.0.0.1:3000
```

Endpoints:

- `GET /health`
- `GET /api/package`
- `GET /api/schema`
- `POST /api/run`
