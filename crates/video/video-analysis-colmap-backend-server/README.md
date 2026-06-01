# video-analysis-colmap-backend-server

Thin HTTP API adapter for `moritzbrantner-video-analysis-colmap-backend`.

Run:

```bash
cargo run -p video-analysis-colmap-backend-server -- --addr 127.0.0.1:3000
```

Endpoints:

- `GET /health`
- `GET /api/package`
- `GET /api/schema`
- `GET /api/operations`
- `POST /api/run`
- `POST /api/<operation-id>`
