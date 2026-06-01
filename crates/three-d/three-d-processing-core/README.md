# three-d-processing-core

Shared 3D geometry primitives, transforms, and basic surface algorithms for
`moritzbrantner-video-analysis`.

## Feature flags

- No optional feature flags today.

## Example

```rust,ignore
use three_d_processing_core::{Bounds3, Plane3, Point3, Ray3, Sphere3, Transform3, Vector3};

let point = Point3::new(0.0, 1.0, 2.0);
let offset = Vector3::new(1.0, 0.0, 0.0);
let transform = Transform3::translation(offset);

let _ = transform.apply_point(point);

let plane = Plane3::from_point_normal(Point3::new(0.0, 0.0, 0.0), Vector3::new(0.0, 1.0, 0.0))?;
let ray = Ray3::new(Point3::new(0.0, 2.0, 0.0), Vector3::new(0.0, -1.0, 0.0))?;
let _ = plane.intersect_ray(ray)?;

let bounds = Bounds3::new(Point3::new(-1.0, -1.0, -1.0), Point3::new(1.0, 1.0, 1.0))?;
let sphere = Sphere3::new(Point3::new(1.5, 0.0, 0.0), 0.75)?;
let _ = bounds.intersects_sphere(sphere)?;
```

## Related crates

- `three-d-processing-mesh`
- `video-analysis-radiance-fields`
