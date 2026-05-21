# audio-analysis-pitch-server

Thin HTTP API adapter for `audio-analysis-pitch`.

Run:

```bash
cargo run -p audio-analysis-pitch-server -- --addr 127.0.0.1:3000
```

Endpoints:

- `GET /health`
- `GET /api/package`
- `GET /api/schema`
- `POST /api/run`
