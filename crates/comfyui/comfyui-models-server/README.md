# comfyui-models-server

Thin HTTP API adapter for `comfyui-models`.

Run:

```bash
cargo run -p comfyui-models-server -- --addr 127.0.0.1:3000
```

Endpoints:

- `GET /health`
- `GET /api/package`
- `GET /api/schema`
- `POST /api/run`
