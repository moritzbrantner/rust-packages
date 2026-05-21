# geo-data

GeoJSON-oriented geometry data structures, processing algorithms, and transforms for `video-analysis`.

## Highlights

- GeoJSON-shaped geometry, feature, and feature collection types
- Conversion to and from the `geojson` crate
- Bounding-box intersection helpers for points, lines, polygons, multipolygons, and collections
- Ring area, orientation normalization, point-in-ring, and multipolygon assembly helpers
- Coordinate transforms and Douglas-Peucker simplification for reusable processing pipelines

## Example

```rust,no_run
use geo_data::{
    assemble_multipolygon, normalize_ring_orientation, point_in_ring, Coordinate, GeoFeature,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
let mut outer = vec![
    Coordinate::new(0.0, 0.0)?,
    Coordinate::new(4.0, 0.0)?,
    Coordinate::new(4.0, 4.0)?,
    Coordinate::new(0.0, 4.0)?,
    Coordinate::new(0.0, 0.0)?,
];
normalize_ring_orientation(&mut outer, true);
assert!(point_in_ring(Coordinate::new(2.0, 2.0)?, &outer));

let geometry = assemble_multipolygon(vec![outer], Vec::new())?;
let feature = GeoFeature::new(Some(geometry));
let collection = geo_data::to_geojson_feature_collection(&[feature]);
assert_eq!(collection.features.len(), 1);
    Ok(())
}
```

## Related crates

- `maps-kernels-core`
- `math-geometry-2d`
- `video-analysis-data`
