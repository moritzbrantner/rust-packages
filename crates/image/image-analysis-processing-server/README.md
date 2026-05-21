# image-analysis-processing-server

Thin HTTP API adapter for `image-analysis-processing`.

Run:

```bash
cargo run -p image-analysis-processing-server -- --addr 127.0.0.1:3000
```

Endpoints:

- `GET /health`
- `GET /api/package`
- `GET /api/schema`
- `POST /api/run`
