# text-transcripts-server

Thin HTTP API adapter for `text-transcripts`.

Run:

```bash
cargo run -p text-transcripts-server -- --addr 127.0.0.1:3000
```

Endpoints:

- `GET /health`
- `GET /api/package`
- `GET /api/schema`
- `POST /api/run`
