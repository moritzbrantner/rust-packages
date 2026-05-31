# geo-io-osm-cli

Thin command-line adapter for `geo-io-osm`.

```bash
cargo run -p geo-io-osm-cli -- operations --json
cargo run -p geo-io-osm-cli -- run --operation osm.validateSpec --json '{"spec":{}}'
cargo run -p geo-io-osm-cli -- filter --input data.osm.pbf --spec spec.json --output out.geojson
```
