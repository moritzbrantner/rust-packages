# geo-viz

Renderer-agnostic geographic visualization indexes for map views.

This crate owns the data-side map aggregation surface used by
`@moritzbrantner/viz-engine`. Rendering remains in downstream packages.

## Install

```toml
[dependencies]
geo-core = { package = "moritzbrantner-geo-core", version = "0.1.0" }
geo-io-geojson = { package = "moritzbrantner-geo-io-geojson", version = "0.1.0" }
geo-clustering = { package = "moritzbrantner-geo-clustering", version = "0.1.0" }
geo-viz = { package = "moritzbrantner-geo-viz", version = "0.1.0" }
```

```rust
use geo_viz::{GeoPointIndex, GeoVizPoint};
```
