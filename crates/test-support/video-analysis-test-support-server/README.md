# video-analysis-test-support-server

Thin HTTP API adapter for `video-analysis-test-support`.

Run:

```bash
cargo run -p video-analysis-test-support-server -- --addr 127.0.0.1:3000
```

Endpoints:

- `GET /health`
- `GET /api/package`
- `GET /api/schema`
- `GET /api/operations`
- `POST /api/run`
- `POST /api/<operation-id>`
