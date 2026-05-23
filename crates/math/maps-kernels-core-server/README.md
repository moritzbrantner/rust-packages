# maps-kernels-core-server

Thin HTTP API adapter for `maps-kernels-core`.

Run:

```bash
cargo run -p maps-kernels-core-server -- --addr 127.0.0.1:3000
```

Endpoints:

- `GET /health`
- `GET /api/package`
- `GET /api/schema`
- `GET /api/operations`
- `POST /api/run`
- `POST /api/<operation-id>`
