# geo-clustering

Format-agnostic point clustering for `geo-core` coordinates.

The public API uses internal Rust domain types and does not expose GeoJSON or
any external wire-format crate.

## Install

```toml
[dependencies]
geo-core = { package = "moritzbrantner-geo-core", version = "0.1.0" }
geo-clustering = { package = "moritzbrantner-geo-clustering", version = "0.1.0" }
```

```rust
use geo_clustering::{ClusterIndex, ClusterPoint};
```
