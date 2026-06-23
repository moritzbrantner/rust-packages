# geo-io-osm-server

Thin HTTP API adapter for `geo-io-osm`.

## Legacy crate signpost

`moritzbrantner-geo-io-osm-server` follows the Rust migration from
`moritzbrantner-geo-io-osm` to `moenarch-geo-io-osm`. Active implementation
ownership has moved to
[`moritzbrantner/geo-analysis`](https://github.com/moritzbrantner/geo-analysis).
This adapter does not add an active compatibility wrapper or runtime
compatibility layer for the old name. npm package migration is deferred and is
not part of this Rust-focused migration.

```bash
cargo run -p moenarch-geo-io-osm-server -- --addr 127.0.0.1:3000
```
