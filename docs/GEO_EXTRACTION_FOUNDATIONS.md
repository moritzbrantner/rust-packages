# Geo Extraction Foundations

This note records the dependency story for issue #26 of the hybrid geo
extraction pilot. Active implementation ownership for the geo and map-kernel
family has since moved to
[`moritzbrantner/geo-analysis`](https://github.com/moritzbrantner/geo-analysis);
this repository keeps migration signposts only.

## Boundary Decision

The geo family was extraction-ready once the active geo libraries could be
copied without depending on `moritzbrantner-video-analysis-core` for generic
errors or package-surface DTOs.

`moenarch-geo-core` owns the geo-domain `GeoError` and `Result` contract in the
extracted repository. Geo I/O, clustering, visualization, and map-kernel crates
use the extracted contracts there. `moritzbrantner-runtime-core` remains in this
repository as a shared domain-neutral package-surface foundation.

## Audited Dependencies

Direct normal dependencies from the extraction-readiness audit:

| Crate | Dependency story |
| --- | --- |
| `moenarch-geo-core` | `runtime-core`, `serde`, `serde_json`; no `video-analysis-core`. |
| `moenarch-geo-io-geojson` | `geo-core`, `geojson`, `runtime-core`, `serde`, `serde_json`; no `video-analysis-core`. |
| `moenarch-geo-io-osm` | `geo-core`, `geo-io-geojson`, OSM parsing/filtering dependencies, optional `geo-types`, optional disk-index dependencies, `runtime-core`, `serde`, `serde_json`; no `video-analysis-core`. |
| `moenarch-geo-clustering` | `geo-core`, `runtime-core`, `serde`, `serde_json`; no `video-analysis-core`. |
| `moenarch-geo-viz` | `geo-core`, `geo-io-geojson`, `geo-clustering`, `maps-kernels-core`, `runtime-core`, `rstar`, `serde`, `serde_json`; no `video-analysis-core`. |
| `moenarch-maps-kernels-core` | `runtime-core`, `serde`, `serde_json`; no `video-analysis-core` or `numbers-core`. |

`runtime-core` is retained in this repository as a foundation dependency because
it owns the domain-neutral package-surface DTOs used by library, CLI, server,
WASM, and app surfaces. It does not depend on geo crates.

`maps-kernels-core` was only retained by `geo-viz` in this workspace. It moved
with the extracted geo/map family and is no longer an active package surface in
`rust-packages`.

## Packaging Proof

The readiness proof for the extraction foundation slice was local packaging of
the foundations and geo libraries, without publishing. Those package selectors
belonged to the extraction-prep state before issue #31 removed active geo and
map-kernel surfaces from this repository; run packaging checks for the extracted
geo family in `moritzbrantner/geo-analysis` instead.

Do not publish crates or depend on external repository credentials from this
repository. Active geo implementation ownership has moved to
`moritzbrantner/geo-analysis`; this repository keeps shared non-geo foundations
and legacy signposts.
