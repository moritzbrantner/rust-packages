# Geo Extraction Foundations

This note records the dependency story for issue #26 of the hybrid geo
extraction pilot.

## Boundary Decision

The geo family is extraction-ready when the active geo libraries can be copied
without depending on `moritzbrantner-video-analysis-core` for generic errors or
package-surface DTOs.

`moenarch-geo-core` now owns the geo-domain `GeoError` and `Result`
contract. Geo I/O, clustering, and visualization crates use that contract for
library errors. Package-surface metadata and operation envelopes continue to use
`moritzbrantner-runtime-core`.

## Audited Dependencies

Direct normal dependencies from `cargo metadata --no-deps`:

| Crate | Dependency story |
| --- | --- |
| `moenarch-geo-core` | `runtime-core`, `serde`, `serde_json`; no `video-analysis-core`. |
| `moenarch-geo-io-geojson` | `geo-core`, `geojson`, `runtime-core`, `serde`, `serde_json`; no `video-analysis-core`. |
| `moenarch-geo-io-osm` | `geo-core`, `geo-io-geojson`, OSM parsing/filtering dependencies, optional `geo-types`, optional disk-index dependencies, `runtime-core`, `serde`, `serde_json`; no `video-analysis-core`. |
| `moenarch-geo-clustering` | `geo-core`, `runtime-core`, `serde`, `serde_json`; no `video-analysis-core`. |
| `moenarch-geo-viz` | `geo-core`, `geo-io-geojson`, `geo-clustering`, `maps-kernels-core`, `runtime-core`, `rstar`, `serde`, `serde_json`; no `video-analysis-core`. |
| `moenarch-maps-kernels-core` | `runtime-core`, `serde`, `serde_json`; no `video-analysis-core` or `numbers-core`. |

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
cargo package --no-verify -p moenarch-runtime-core
cargo package --no-verify -p moenarch-maps-kernels-core
cargo package --no-verify -p moenarch-geo-core
cargo package --no-verify -p moenarch-geo-io-geojson
cargo package --no-verify -p moenarch-geo-io-osm
cargo package --no-verify -p moenarch-geo-clustering
cargo package --no-verify -p moenarch-geo-viz
```

Do not publish crates or depend on external repository credentials in this
slice. Active geo implementation ownership has moved to
`moritzbrantner/geo-analysis`; this repository keeps shared foundations and
legacy signposts. Later extraction slices can keep `runtime-core` and
`maps-kernels-core` treated as required foundation crates.
