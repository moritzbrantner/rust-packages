# three-d-processing-mesh-server

Thin HTTP API adapter for `three-d-processing-mesh`.

Run:

```bash
cargo run -p three-d-processing-mesh-server -- --addr 127.0.0.1:3000
```

Endpoints:

- `GET /health`
- `GET /api/package`
- `GET /api/schema`
- `POST /api/run`
