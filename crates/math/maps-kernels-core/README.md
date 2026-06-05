# maps-kernels-core

Numeric kernels for map and temporal GeoJSON processing.

The crate intentionally starts with small, deterministic `f64` kernels so
TypeScript and WASM implementations can be A/B tested against each other.

## Highlights

- Flat 2D line and ring path summaries
- Deterministic line resampling, simplification, and densification
- Bounds and path-length calculations for map-oriented geometry payloads
