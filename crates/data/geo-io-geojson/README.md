# geo-io-geojson

GeoJSON import and export adapters for `geo-core`.

This crate owns the `geojson` dependency. Algorithm and domain crates should
depend on `geo-core` types instead of exposing `geojson` wire-format types.

## Install

```toml
[dependencies]
geo-core = { package = "moritzbrantner-geo-core", version = "0.1.0" }
geo-io-geojson = { package = "moritzbrantner-geo-io-geojson", version = "0.1.0" }
```

```rust
use geo_core::Geometry;
use geo_io_geojson::{from_geojson_geometry, to_geojson_geometry};
```
