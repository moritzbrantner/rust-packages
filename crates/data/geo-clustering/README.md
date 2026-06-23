# geo-clustering

Format-agnostic point clustering for `geo-core` coordinates.

## Legacy crate signpost

`moritzbrantner-geo-clustering` is superseded by
`moenarch-geo-clustering`. Active implementation ownership has moved to
[`moritzbrantner/geo-analysis`](https://github.com/moritzbrantner/geo-analysis).
This crate does not add an active compatibility wrapper, re-export shim, or
runtime compatibility layer for the old name. npm package migration is
deferred and is not part of this Rust-focused migration.

The public API uses internal Rust domain types and does not expose GeoJSON or
any external wire-format crate.

## Install

```toml
[dependencies]
geo-core = { package = "moenarch-geo-core", version = "0.1.0" }
geo-clustering = { package = "moenarch-geo-clustering", version = "0.1.0" }
```

```rust
use geo_clustering::{ClusterIndex, ClusterPoint};
```

## Package surface

Primary workflow: `geoCluster.viewport`.

Workflow operations:

- `geoCluster.viewport`: Returns clusters or points for a bounding box and zoom level.
- `geoCluster.bounds`: Computes bounds for finite cluster input points.

Debug operations:

- `describe`: inspect package metadata and runtime support.

Runtime support: library, CLI, server, and WASM wrappers expose these operations.

Run the primary workflow through the CLI:

```bash
cargo run -p moenarch-geo-clustering-cli -- run \
  --operation geoCluster.viewport \
  --json '{"bounds":[7.0,48.0,9.0,50.0],"points":[{"id":"a","latitude":49.0,"longitude":8.0,"properties":{}}],"zoom":8}'
```

Successful responses use the shared package-surface shape with `operation`,
`title`, `message`, `summary`, and `result`. Default surface calls are
deterministic, local-first, and do not download models, write persistent files,
or execute external tools unless an operation explicitly documents native or
external-tool execution.
