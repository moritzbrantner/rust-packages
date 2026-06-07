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

## Package surface

Primary workflow: `threeD.pointCloud.summary`.

Workflow operations:

- `threeD.pointCloud.summary`: Summarizes point count, centroid, and bounds.
- `threeD.pointCloud.downsample`: Runs deterministic voxel downsampling over points.
- `threeD.geometry.intersections`: Computes supported 3D geometry intersections.

Debug operations:

- `describe`: inspect package metadata and runtime support.

Runtime support: library, CLI, server, and WASM wrappers expose these operations.

Run the primary workflow through the CLI:

```bash
cargo run -p moritzbrantner-three-d-processing-core-cli -- run \
  --operation threeD.pointCloud.summary \
  --json '{"points":[[0,0,0],[1,1,1]]}'
```

Successful responses use the shared package-surface shape with `operation`,
`title`, `message`, `summary`, and `result`. Default surface calls are
deterministic, local-first, and do not download models, write persistent files,
or execute external tools unless an operation explicitly documents native or
external-tool execution.

## Related crates

- `three-d-processing-mesh`
- `video-analysis-radiance-fields`
