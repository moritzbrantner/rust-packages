# finance-statistics-server

Thin HTTP API adapter for `finance-statistics`.

Run:

```bash
cargo run -p finance-statistics-server -- --addr 127.0.0.1:3000
```

Endpoints:

- `GET /health`
- `GET /api/package`
- `GET /api/schema`
- `POST /api/run`
