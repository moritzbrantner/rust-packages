# image-analysis-models-server

Thin HTTP API adapter for `image-analysis-models`.

Run:

```bash
cargo run -p image-analysis-models-server -- --addr 127.0.0.1:3000
```

Endpoints:

- `GET /health`
- `GET /api/package`
- `GET /api/schema`
- `GET /api/operations`
- `POST /api/run`
- `POST /api/<operation-id>`
