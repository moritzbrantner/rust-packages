# geo-io-geojson-cli

Thin command-line adapter for `geo-io-geojson`.

## Legacy crate signpost

`moritzbrantner-geo-io-geojson-cli` follows the Rust migration from
`moritzbrantner-geo-io-geojson` to `moenarch-geo-io-geojson`. Active
implementation ownership has moved to
[`moritzbrantner/geo-analysis`](https://github.com/moritzbrantner/geo-analysis).
This adapter does not add an active compatibility wrapper or runtime
compatibility layer for the old name. npm package migration is deferred and is
not part of this Rust-focused migration.

Run:

```bash
cargo run -p moenarch-geo-io-geojson-cli -- operations --json
cargo run -p moenarch-geo-io-geojson-cli -- run --operation describe --json '{"includeOperations":true}'
```
