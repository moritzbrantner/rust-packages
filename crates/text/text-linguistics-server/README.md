# text-linguistics-server

Thin HTTP API adapter for `text-linguistics`.

Run:

```bash
cargo run -p text-linguistics-server -- --addr 127.0.0.1:3000
```

Endpoints:

- `GET /health`
- `GET /api/package`
- `GET /api/schema`
- `POST /api/run`
