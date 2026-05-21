# audio-analysis-io-server

Thin HTTP API adapter for `audio-analysis-io`.

Run:

```bash
cargo run -p audio-analysis-io-server -- --addr 127.0.0.1:3000
```

Endpoints:

- `GET /health`
- `GET /api/package`
- `GET /api/schema`
- `POST /api/run`
