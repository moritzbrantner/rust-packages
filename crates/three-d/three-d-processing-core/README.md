# three-d-processing-core

Shared 3D geometry primitives, transforms, camera math, coordinate conversions,
and basic surface algorithms for `moritzbrantner-video-analysis`.

## Feature flags

- No optional feature flags today.

## Example

```rust,ignore
use three_d_processing_core::{
    Bounds3, CameraPose3, PinholeIntrinsics, Plane3, Point3, Ray3, Sphere3,
    Transform3, TrsTransform3, Vector3,
};

let point = Point3::new(0.0, 1.0, 2.0);
let offset = Vector3::new(1.0, 0.0, 0.0);
let transform = Transform3::translation(offset);

let _ = transform.apply_point(point);

let camera = CameraPose3::identity();
let intrinsics = PinholeIntrinsics::new(32, 32, 30.0, 30.0, 15.0, 15.0)?;
let _ = camera.project_point(intrinsics, Point3::new(0.0, 0.0, 3.0))?;

let trs = TrsTransform3::new(
    Vector3::new(1.0, 0.0, 0.0),
    three_d_processing_core::Quaternion::IDENTITY,
    Vector3::new(1.0, 1.0, 1.0),
)?;
let _ = trs.to_affine()?;

let plane = Plane3::from_point_normal(Point3::new(0.0, 0.0, 0.0), Vector3::new(0.0, 1.0, 0.0))?;
let ray = Ray3::new(Point3::new(0.0, 2.0, 0.0), Vector3::new(0.0, -1.0, 0.0))?;
let _ = plane.intersect_ray(ray)?;

let bounds = Bounds3::new(Point3::new(-1.0, -1.0, -1.0), Point3::new(1.0, 1.0, 1.0))?;
let sphere = Sphere3::new(Point3::new(1.5, 0.0, 0.0), 0.75)?;
let _ = bounds.intersects_sphere(sphere)?;
```

## Package surface

Primary workflow: `threeD.camera.project`.

Workflow operations:

- `threeD.pointCloud.summary`: Summarizes point count, centroid, and bounds.
- `threeD.pointCloud.downsample`: Runs deterministic voxel downsampling over points.
- `threeD.geometry.intersections`: Computes supported 3D geometry intersections.
- `threeD.transform.compose`: Composes two workspace TRS transforms.
- `threeD.transform.inverse`: Inverts a row-major affine 4x4 transform matrix.
- `threeD.transform.apply`: Applies a workspace TRS transform to a point and/or vector.
- `threeD.camera.project`: Projects a workspace point through a pinhole camera.
- `threeD.camera.pixelRay`: Builds a normalized workspace ray through a pixel.
- `threeD.camera.viewMatrix`: Builds workspace or WebGL row-major view matrices.
- `threeD.camera.projectionMatrix`: Builds OpenCV-depth or WebGL row-major projection matrices.
- `threeD.convert.colmapPose`: Converts COLMAP world-to-camera poses into the workspace convention.
- `threeD.convert.gltfMatrix`: Converts row-major workspace matrices to glTF/WebGL upload arrays.

Debug operations:

- `describe`: inspect package metadata and runtime support.
- `threeD.debug.matrixInspect`: inspect row-major 4x4 matrix layout and inverse availability.
- `threeD.debug.rotationInspect`: inspect quaternion normalization and rotation matrix output.
- `threeD.debug.transformDiagnostics`: inspect TRS validation and affine conversion.

Runtime support: library, CLI, server, and WASM wrappers expose these operations.

Run the primary workflow through the CLI:

```bash
cargo run -p moritzbrantner-three-d-processing-core-cli -- run \
  --operation threeD.camera.project \
  --json '{"pose":{"position":[0,0,0],"right":[1,0,0],"up":[0,1,0],"forward":[0,0,1]},"intrinsics":{"width":32,"height":32,"fx":30,"fy":30,"cx":15,"cy":15},"point":[0,0,3]}'
```

Successful responses use the shared package-surface shape with `operation`,
`title`, `message`, `summary`, and `result`. Default surface calls are
deterministic, local-first, and do not download models, write persistent files,
or execute external tools unless an operation explicitly documents native or
external-tool execution.

## Related crates

- `three-d-processing-mesh`
- `video-analysis-radiance-fields`
