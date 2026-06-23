# geo-core-server

Thin HTTP API adapter for `geo-core`.

## Legacy crate signpost

`moritzbrantner-geo-core-server` follows the Rust migration from
`moritzbrantner-geo-core` to `moenarch-geo-core`. Active implementation
ownership has moved to
[`moritzbrantner/geo-analysis`](https://github.com/moritzbrantner/geo-analysis).
This adapter does not add an active compatibility wrapper or runtime
compatibility layer for the old name. npm package migration is deferred and is
not part of this Rust-focused migration.

Run:

```bash
cargo run -p moenarch-geo-core-server -- --addr 127.0.0.1:3000
```

Endpoints:

- `GET /health`
- `GET /api/package`
- `GET /api/schema`
- `GET /api/operations`
- `POST /api/run`
- `POST /api/<operation-id>`
