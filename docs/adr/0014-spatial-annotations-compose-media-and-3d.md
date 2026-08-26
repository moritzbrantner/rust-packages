# Spatial Annotations Compose Media And 3D

Status: accepted

`media-core` continues to own the neutral annotation envelope, exact media timing, source identity, and media-local selectors. `three-d-processing-core` continues to own canonical workspace points, vectors, quaternions, transforms, camera poses, pinhole geometry, and coordinate-convention conversions.

The spatial-annotation MVP is an interoperability module inside `three-d-processing-core`, not a new package or product layer. It composes those two existing owners without moving their contracts.

The module owns:

- stable coordinate-frame references and coordinate units;
- similarity transforms between Cartesian frames so reconstruction-relative scale is representable;
- WGS84 geographic positions and local-frame geographic anchors;
- spatial uncertainty independent from semantic annotation confidence;
- opaque domain entity references for COLMAP, Gaussian splats, or future scene formats;
- typed 3D point, box, sphere, rigid-pose, camera-pose, geographic-point, and entity selectors;
- a versioned `SpatialBinding` adapter that can preserve an existing neutral frame, region, text, or track selector while attaching a typed spatial selector.

The canonical camera pose remains `CameraPose3d`, including the existing COLMAP world-to-camera conversion. Generic object orientation remains quaternion-backed through `RigidTransform3d`. Scene-to-scene relationships use `SimilarityTransform3d` so arbitrary reconstruction scale is not silently treated as metric.

Global geographic coordinates are not treated as another Cartesian frame. `GeographicFrameAnchor` explicitly anchors a local Cartesian frame at a WGS84 position with a local ENU/NED tangent convention, orientation, and meters-per-unit scale. GIS algorithms, CRS reprojection, topology, GeoJSON processing, and map/product behavior remain outside this module and can stay owned by geo/spatial capability repositories.

COLMAP, Gaussian-splatting, image, and video crates should adapt their own domain identifiers and richer calibration models at their boundaries. The spatial core must not import their product/domain DTOs merely to make annotation interchange convenient.

This MVP intentionally does not define a universal scene graph, trajectory store, GIS database, object ontology, or product workflow. Those can be introduced later when a concrete consumer demonstrates the need.
