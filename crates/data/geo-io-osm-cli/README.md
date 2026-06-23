# geo-io-osm-cli

Thin command-line adapter for `geo-io-osm`.

## Legacy crate signpost

`moritzbrantner-geo-io-osm-cli` follows the Rust migration from
`moritzbrantner-geo-io-osm` to `moenarch-geo-io-osm`. Active implementation
ownership has moved to
[`moritzbrantner/geo-analysis`](https://github.com/moritzbrantner/geo-analysis).
This adapter does not add an active compatibility wrapper or runtime
compatibility layer for the old name. npm package migration is deferred and is
not part of this Rust-focused migration.

```bash
cargo run -p moenarch-geo-io-osm-cli -- operations --json
cargo run -p moenarch-geo-io-osm-cli -- run --operation osm.validateSpec --json '{"spec":{}}'
cargo run -p moenarch-geo-io-osm-cli -- filter --input data.osm.pbf --spec spec.json --output out.geojson
```
