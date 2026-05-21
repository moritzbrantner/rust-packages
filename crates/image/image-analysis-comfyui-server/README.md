# image-analysis-comfyui-server

Thin HTTP API adapter for `image-analysis-comfyui`.

Run:

```bash
cargo run -p image-analysis-comfyui-server -- --addr 127.0.0.1:3000
```

Endpoints:

- `GET /health`
- `GET /api/package`
- `GET /api/schema`
- `POST /api/run`
