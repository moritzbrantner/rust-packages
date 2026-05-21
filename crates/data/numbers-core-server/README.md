# numbers-core-server

Thin HTTP API adapter for `numbers-core`.

Run:

```bash
cargo run -p numbers-core-server -- --addr 127.0.0.1:3000
```

Endpoints:

- `GET /health`
- `GET /api/package`
- `GET /api/schema`
- `POST /api/run`
