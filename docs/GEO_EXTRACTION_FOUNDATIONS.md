# Geo Extraction Foundations

This note records the dependency story for issue #26 of the hybrid geo
extraction pilot.

## Boundary Decision

The geo family is extraction-ready when the active geo libraries can be copied
without depending on `moritzbrantner-video-analysis-core` for generic errors or
package-surface DTOs.

`moritzbrantner-geo-core` now owns the geo-domain `GeoError` and `Result`
contract. Geo I/O, clustering, and visualization crates use that contract for
library errors. Package-surface metadata and operation envelopes continue to use
`moritzbrantner-runtime-core`.

## Audited Dependencies

Direct normal dependencies from `cargo metadata --no-deps`:

| Crate | Dependency story |
| --- | --- |
| `moritzbrantner-geo-core` | `runtime-core`, `serde`, `serde_json`; no `video-analysis-core`. |
| `moritzbrantner-geo-io-geojson` | `geo-core`, `geojson`, `runtime-core`, `serde`, `serde_json`; no `video-analysis-core`. |
| `moritzbrantner-geo-io-osm` | `geo-core`, `geo-io-geojson`, OSM parsing/filtering dependencies, optional `geo-types`, optional disk-index dependencies, `runtime-core`, `serde`, `serde_json`; no `video-analysis-core`. |
| `moritzbrantner-geo-clustering` | `geo-core`, `runtime-core`, `serde`, `serde_json`; no `video-analysis-core`. |
| `moritzbrantner-geo-viz` | `geo-core`, `geo-io-geojson`, `geo-clustering`, `maps-kernels-core`, `runtime-core`, `rstar`, `serde`, `serde_json`; no `video-analysis-core`. |
| `moritzbrantner-maps-kernels-core` | `runtime-core`, `serde`, `serde_json`; no `video-analysis-core` or `numbers-core`. |

`runtime-core` is retained as a foundation dependency because it owns the
domain-neutral package-surface DTOs used by library, CLI, server, WASM, and app
surfaces. It does not depend on geo crates.

`maps-kernels-core` is retained only by `geo-viz`, where it supplies the flat
2D path resampling/simplification kernels used by visualization operations. It
has its own `MapsKernelError` and no longer needs `video-analysis-core` or
`numbers-core` for map-kernel validation.

## Packaging Proof

The readiness proof for this slice is local packaging of the foundations and
geo libraries, without publishing:

```bash
cargo package --no-verify -p moritzbrantner-runtime-core
cargo package --no-verify -p moritzbrantner-maps-kernels-core
cargo package --no-verify -p moritzbrantner-geo-core
cargo package --no-verify -p moritzbrantner-geo-io-geojson
cargo package --no-verify -p moritzbrantner-geo-io-osm
cargo package --no-verify -p moritzbrantner-geo-clustering
cargo package --no-verify -p moritzbrantner-geo-viz
```

Do not create `moenarch/geo-analysis`, publish crates, or depend on external
repository credentials in this slice. Later extraction slices can clean-copy the
geo family with `runtime-core` and `maps-kernels-core` treated as required
foundation crates.
