# geo-core

Format-agnostic geospatial domain types, geometry helpers, and transforms for
video-analysis.

This crate owns stable internal concepts such as coordinates, bounding boxes,
geometry, features, collections, and geometry simplification. It intentionally
does not depend on GeoJSON or expose wire-format crate types.

## Install

```toml
[dependencies]
geo-core = { package = "moritzbrantner-geo-core", version = "0.1.0" }
```

```rust
use geo_core::{Coordinate, Geometry};
```
