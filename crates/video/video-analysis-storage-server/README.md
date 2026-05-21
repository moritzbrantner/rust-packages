# video-analysis-storage-server

Thin HTTP API adapter for `video-analysis-storage`.

Run:

```bash
cargo run -p video-analysis-storage-server -- --addr 127.0.0.1:3000
```

Endpoints:

- `GET /health`
- `GET /api/package`
- `GET /api/schema`
- `POST /api/run`
