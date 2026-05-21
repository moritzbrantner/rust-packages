# maps-kernels-core

Numeric kernels for map and temporal GeoJSON processing.

The crate intentionally starts with small, deterministic `f64` kernels so
TypeScript and WASM implementations can be A/B tested against each other.
