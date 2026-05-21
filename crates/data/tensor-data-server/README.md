# tensor-data-server

Thin HTTP API adapter for `tensor-data`.

Run:

```bash
cargo run -p tensor-data-server -- --addr 127.0.0.1:3000
```

Endpoints:

- `GET /health`
- `GET /api/package`
- `GET /api/schema`
- `POST /api/run`
