# geo-io-osm

OpenStreetMap PBF import adapters for `geo-core`.

This crate reads local or in-memory `.osm.pbf` data, applies practical OSM
element, bbox, and tag filters, resolves way geometry through a node coordinate
index, and emits `geo-core` feature collections. It intentionally does not own
HTTP downloads or Geofabrik fetch caching.

## Install

```toml
[dependencies]
geo-io-osm = { package = "moritzbrantner-geo-io-osm", version = "0.1.0" }
```

```rust
use geo_io_osm::{collect_osm_pbf_bytes, CollectOsmBytesOptions, IndexOptions, OsmFilterSpec};

# fn run(pbf_bytes: &[u8]) -> video_analysis_core::Result<()> {
let collected = collect_osm_pbf_bytes(CollectOsmBytesOptions {
    input: pbf_bytes,
    spec: OsmFilterSpec::default(),
    index_options: IndexOptions::default(),
})?;

let geo = collected.into_geo_feature_collection();
# Ok(())
# }
```
