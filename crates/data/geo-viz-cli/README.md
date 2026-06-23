# geo-viz-cli

Thin command-line adapter for `geo-viz`.

## Legacy crate signpost

`moritzbrantner-geo-viz-cli` follows the Rust migration from
`moritzbrantner-geo-viz` to `moenarch-geo-viz`. Active implementation
ownership has moved to
[`moritzbrantner/geo-analysis`](https://github.com/moritzbrantner/geo-analysis).
This adapter does not add an active compatibility wrapper or runtime
compatibility layer for the old name. npm package migration is deferred and is
not part of this Rust-focused migration.

Run:

```bash
cargo run -p moenarch-geo-viz-cli -- operations --json
cargo run -p moenarch-geo-viz-cli -- run --operation describe --json '{"includeOperations":true}'
```
