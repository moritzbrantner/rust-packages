# three-d-processing-core

Shared 3D geometry primitives and transforms for `video-analysis`.

## Feature flags

- No optional feature flags today.

## Example

```rust,ignore
use three_d_processing_core::{Point3, Transform3, Vector3};

let point = Point3::new(0.0, 1.0, 2.0);
let offset = Vector3::new(1.0, 0.0, 0.0);
let transform = Transform3::from_translation(offset);

let _ = transform.apply_point(point);
```

## Related crates

- `three-d-processing-mesh`
- `video-analysis-radiance-fields`
