# video-analysis-sfm-rust-backend-server

Thin HTTP API adapter for `video-analysis-sfm-rust-backend`.

Run:

```bash
cargo run -p video-analysis-sfm-rust-backend-server -- --addr 127.0.0.1:3000
```

Endpoints:

- `GET /health`
- `GET /api/package`
- `GET /api/schema`
- `POST /api/run`
