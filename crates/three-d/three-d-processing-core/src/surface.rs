//! Library-owned runtime surface for `three-d-processing-core`.

use runtime_core::{
    OperationId, PackageSurface, RuntimeCapabilities, SurfaceOperation, SurfaceRequest,
    SurfaceResponse,
};
use serde::de::DeserializeOwned;
use serde::Deserialize;

use crate::{
    centroid, collision_sphere_sphere, intersect_ray_bounds, intersect_ray_sphere,
    sphere_intersects_bounds, voxel_downsample, AffineTransform3, Bounds3, CameraPose3,
    CameraPose3d, Matrix4, Matrix4d, PinholeIntrinsics, PinholeIntrinsicsd, Point3, Point3d,
    Quaternion, Ray3, Sphere3, TrsTransform3, TrsTransform3d, Vector3, Vector3d,
};

/// Returns the package surface exposed by every transport wrapper.
pub fn package_surface() -> PackageSurface {
    PackageSurface {
        library: env!("CARGO_PKG_NAME").to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        capabilities: RuntimeCapabilities::pure_rust(),
        operations: vec![
            operation(
                "describe",
                "Describe package",
                "Shared 3D geometry primitives and transforms for video-analysis.",
                serde_json::json!({"includeOperations": true}),
            ),
            operation(
                "threeD.pointCloud.summary",
                "Point cloud summary",
                "Summarizes point count, centroid, and bounds.",
                serde_json::json!({"points": [[0, 0, 0], [1, 1, 1]]}),
            ),
            operation(
                "threeD.pointCloud.downsample",
                "Point cloud downsample",
                "Runs deterministic voxel downsampling over points.",
                serde_json::json!({"points": [[0, 0, 0], [0.01, 0, 0]], "voxelSize": 0.1}),
            ),
            operation(
                "threeD.geometry.intersections",
                "Geometry intersections",
                "Computes supported 3D geometry intersections.",
                serde_json::json!({"mode": "raySphere", "ray": {"origin": [0, 0, -2], "direction": [0, 0, 1]}, "sphere": {"center": [0, 0, 0], "radius": 1}}),
            ),
            operation(
                "threeD.transform.compose",
                "Compose transforms",
                "Composes two workspace TRS transforms and returns an affine 4x4 matrix.",
                serde_json::json!({
                    "left": {"translation": [1, 0, 0], "rotation": [0, 0, 0, 1], "scale": [1, 1, 1]},
                    "right": {"translation": [0, 2, 0], "rotation": [0, 0, 0, 1], "scale": [1, 1, 1]}
                }),
            ),
            operation(
                "threeD.transform.inverse",
                "Invert transform",
                "Inverts a row-major affine 4x4 transform matrix.",
                serde_json::json!({"matrix": [[1, 0, 0, 2], [0, 1, 0, 0], [0, 0, 1, 0], [0, 0, 0, 1]]}),
            ),
            operation(
                "threeD.transform.apply",
                "Apply transform",
                "Applies a workspace TRS transform to a point and/or vector.",
                serde_json::json!({
                    "transform": {"translation": [1, 0, 0], "rotation": [0, 0, 0, 1], "scale": [1, 1, 1]},
                    "point": [0, 0, 1],
                    "vector": [0, 1, 0]
                }),
            ),
            operation(
                "threeD.camera.project",
                "Project camera point",
                "Projects a workspace point through a pinhole camera pose and intrinsics.",
                serde_json::json!({
                    "pose": {"position": [0, 0, 0], "right": [1, 0, 0], "up": [0, 1, 0], "forward": [0, 0, 1]},
                    "intrinsics": {"width": 32, "height": 32, "fx": 30, "fy": 30, "cx": 15, "cy": 15},
                    "point": [0, 0, 3]
                }),
            ),
            operation(
                "threeD.camera.pixelRay",
                "Build camera ray",
                "Builds a normalized workspace ray through a pixel.",
                serde_json::json!({
                    "pose": {"position": [0, 0, 0], "right": [1, 0, 0], "up": [0, 1, 0], "forward": [0, 0, 1]},
                    "intrinsics": {"width": 32, "height": 32, "fx": 30, "fy": 30, "cx": 15, "cy": 15},
                    "pixel": [15, 15],
                    "near": 0,
                    "far": 100
                }),
            ),
            operation(
                "threeD.camera.viewMatrix",
                "Build camera view matrix",
                "Builds a row-major camera view matrix for workspace or WebGL target conventions.",
                serde_json::json!({
                    "pose": {"position": [0, 0, 0], "right": [1, 0, 0], "up": [0, 1, 0], "forward": [0, 0, 1]},
                    "target": "workspace"
                }),
            ),
            operation(
                "threeD.camera.projectionMatrix",
                "Build camera projection matrix",
                "Builds a row-major pinhole projection matrix for OpenCV depth or WebGL targets.",
                serde_json::json!({
                    "intrinsics": {"width": 32, "height": 32, "fx": 30, "fy": 30, "cx": 15, "cy": 15},
                    "near": 0.1,
                    "far": 100,
                    "target": "webgl"
                }),
            ),
            operation(
                "threeD.convert.colmapPose",
                "Convert COLMAP pose",
                "Converts COLMAP world-to-camera quaternion and translation into the workspace camera convention.",
                serde_json::json!({"qw": 1, "qx": 0, "qy": 0, "qz": 0, "tx": 0, "ty": 0, "tz": 0}),
            ),
            operation(
                "threeD.convert.gltfMatrix",
                "Convert glTF matrix",
                "Converts between row-major workspace matrices and column-major glTF/WebGL upload arrays.",
                serde_json::json!({"matrix": [[1, 0, 0, 0], [0, 1, 0, 0], [0, 0, 1, 0], [0, 0, 0, 1]]}),
            ),
            operation(
                "threeD.debug.matrixInspect",
                "Debug matrix inspect",
                "Inspects a row-major 4x4 matrix and reports inverse availability plus column-major upload values.",
                serde_json::json!({"matrix": [[1, 0, 0, 0], [0, 1, 0, 0], [0, 0, 1, 0], [0, 0, 0, 1]]}),
            ),
            operation(
                "threeD.debug.rotationInspect",
                "Debug rotation inspect",
                "Inspects a quaternion and returns normalized rotation matrix diagnostics.",
                serde_json::json!({"quaternion": [0, 0, 0, 1]}),
            ),
            operation(
                "threeD.debug.transformDiagnostics",
                "Debug transform diagnostics",
                "Validates a TRS transform and returns its affine matrix and inverse availability.",
                serde_json::json!({"transform": {"translation": [0, 0, 0], "rotation": [0, 0, 0, 1], "scale": [1, 1, 1]}}),
            ),
        ],
    }
}

fn operation(
    id: &str,
    name: &str,
    description: &str,
    example_request: serde_json::Value,
) -> SurfaceOperation {
    SurfaceOperation {
        id: OperationId::new(id),
        name: name.to_string(),
        description: Some(description.to_string()),
        curation: runtime_core::SurfaceOperationCuration::from_operation_id(id),
        input_schema: serde_json::json!({"type": "object", "additionalProperties": true, "xOperationCategory": runtime_core::operation_category(id)}),
        output_schema: serde_json::json!({"type": "object", "xOperationCategory": runtime_core::operation_category(id)}),
        example_request,
        wasm_supported: true,
        server_supported: true,
    }
}

/// Runs one library-owned operation.
pub fn run_surface_operation(request: SurfaceRequest) -> Result<SurfaceResponse, String> {
    let operation = request.operation.clone();
    let value = match request.operation.as_str() {
        "describe" => describe_value(request.input),
        "threeD.pointCloud.summary" => point_cloud_summary_value(parse_input(request.input)?)?,
        "threeD.pointCloud.downsample" => {
            point_cloud_downsample_value(parse_input(request.input)?)?
        }
        "threeD.geometry.intersections" => intersections_value(parse_input(request.input)?)?,
        "threeD.transform.compose" => transform_compose_value(parse_input(request.input)?)?,
        "threeD.transform.inverse" => transform_inverse_value(parse_input(request.input)?)?,
        "threeD.transform.apply" => transform_apply_value(parse_input(request.input)?)?,
        "threeD.camera.project" => camera_project_value(parse_input(request.input)?)?,
        "threeD.camera.pixelRay" => camera_pixel_ray_value(parse_input(request.input)?)?,
        "threeD.camera.viewMatrix" => camera_view_matrix_value(parse_input(request.input)?)?,
        "threeD.camera.projectionMatrix" => {
            camera_projection_matrix_value(parse_input(request.input)?)?
        }
        "threeD.convert.colmapPose" => colmap_pose_value(parse_input(request.input)?)?,
        "threeD.convert.gltfMatrix" => gltf_matrix_value(parse_input(request.input)?)?,
        "threeD.debug.matrixInspect" => matrix_inspect_value(parse_input(request.input)?)?,
        "threeD.debug.rotationInspect" => rotation_inspect_value(parse_input(request.input)?)?,
        "threeD.debug.transformDiagnostics" => {
            transform_diagnostics_value(parse_input(request.input)?)?
        }
        operation => {
            return Err(format!(
                "unsupported operation `{operation}` for {}",
                env!("CARGO_PKG_NAME")
            ))
        }
    };
    Ok(surface_response(operation, value))
}

fn describe_value(input: serde_json::Value) -> serde_json::Value {
    let surface = package_surface();
    serde_json::json!({
        "library": surface.library,
        "version": surface.version,
        "operationCount": surface.operations.len(),
        "operations": surface.operations.iter().map(|operation| operation.id.as_str()).collect::<Vec<_>>(),
        "input": input
    })
}

fn surface_response(operation: OperationId, value: serde_json::Value) -> SurfaceResponse {
    SurfaceResponse {
        operation,
        value,
        diagnostics: Vec::new(),
        artifacts: Vec::new(),
    }
}

fn parse_input<T: DeserializeOwned>(input: serde_json::Value) -> Result<T, String> {
    serde_json::from_value(input).map_err(|error| format!("invalid request: {error}"))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PointsRequest {
    points: Vec<[f32; 3]>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DownsampleRequest {
    points: Vec<[f32; 3]>,
    voxel_size: f32,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "mode")]
enum IntersectionsRequest {
    #[serde(rename = "raySphere")]
    RaySphere {
        ray: RayPayload,
        sphere: SpherePayload,
    },
    #[serde(rename = "rayBounds")]
    RayBounds {
        ray: RayPayload,
        bounds: BoundsPayload,
    },
    #[serde(rename = "sphereSphere", rename_all = "camelCase")]
    SphereSphere {
        left_sphere: SpherePayload,
        right_sphere: SpherePayload,
    },
    #[serde(rename = "sphereBounds")]
    SphereBounds {
        sphere: SpherePayload,
        bounds: BoundsPayload,
    },
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RayPayload {
    origin: [f32; 3],
    direction: [f32; 3],
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SpherePayload {
    center: [f32; 3],
    radius: f32,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BoundsPayload {
    min: [f32; 3],
    max: [f32; 3],
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
enum Precision {
    #[default]
    F32,
    F64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TransformComposeRequest {
    left: TrsPayload,
    right: TrsPayload,
    #[serde(default)]
    precision: Precision,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TransformInverseRequest {
    matrix: [[f32; 4]; 4],
    #[serde(default)]
    precision: Precision,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TransformApplyRequest {
    transform: TrsPayload,
    #[serde(default)]
    point: Option<[f32; 3]>,
    #[serde(default)]
    vector: Option<[f32; 3]>,
    #[serde(default)]
    precision: Precision,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TrsPayload {
    translation: [f32; 3],
    rotation: [f32; 4],
    scale: [f32; 3],
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CameraProjectRequest {
    pose: CameraPosePayload,
    intrinsics: PinholeIntrinsicsPayload,
    point: [f32; 3],
    #[serde(default)]
    precision: Precision,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CameraPixelRayRequest {
    pose: CameraPosePayload,
    intrinsics: PinholeIntrinsicsPayload,
    pixel: [f32; 2],
    #[serde(default)]
    near: f32,
    #[serde(default = "default_far")]
    far: f32,
    #[serde(default)]
    precision: Precision,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CameraViewMatrixRequest {
    pose: CameraPosePayload,
    target: CameraViewTarget,
    #[serde(default)]
    precision: Precision,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase")]
enum CameraViewTarget {
    Workspace,
    Webgl,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CameraProjectionMatrixRequest {
    intrinsics: PinholeIntrinsicsPayload,
    near: f32,
    far: f32,
    target: CameraProjectionTarget,
    #[serde(default)]
    precision: Precision,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase")]
enum CameraProjectionTarget {
    OpencvDepth,
    Webgl,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CameraPosePayload {
    position: [f32; 3],
    right: [f32; 3],
    up: [f32; 3],
    forward: [f32; 3],
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PinholeIntrinsicsPayload {
    width: u32,
    height: u32,
    fx: f32,
    fy: f32,
    cx: f32,
    cy: f32,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ColmapPoseRequest {
    qw: f32,
    qx: f32,
    qy: f32,
    qz: f32,
    tx: f32,
    ty: f32,
    tz: f32,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MatrixRequest {
    matrix: [[f32; 4]; 4],
    #[serde(default)]
    precision: Precision,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RotationInspectRequest {
    quaternion: [f32; 4],
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TransformDiagnosticsRequest {
    transform: TrsPayload,
}

fn point_cloud_summary_value(request: PointsRequest) -> Result<serde_json::Value, String> {
    let points = parse_points(request.points);
    let bounds = Bounds3::from_points(&points).map_err(|error| error.to_string())?;
    Ok(serde_json::json!({
        "count": points.len(),
        "centroid": centroid(&points).map_err(|error| error.to_string())?,
        "bounds": bounds
    }))
}

fn point_cloud_downsample_value(request: DownsampleRequest) -> Result<serde_json::Value, String> {
    let points = parse_points(request.points);
    let downsampled =
        voxel_downsample(&points, request.voxel_size).map_err(|error| error.to_string())?;
    Ok(serde_json::json!({
        "inputCount": points.len(),
        "outputCount": downsampled.len(),
        "points": downsampled
    }))
}

fn intersections_value(request: IntersectionsRequest) -> Result<serde_json::Value, String> {
    match request {
        IntersectionsRequest::RaySphere { ray, sphere } => {
            let ray = ray_payload(ray)?;
            let sphere = sphere_payload(sphere)?;
            Ok(serde_json::json!({
                "mode": "raySphere",
                "points": intersect_ray_sphere(ray, sphere).map_err(|error| error.to_string())?
            }))
        }
        IntersectionsRequest::RayBounds { ray, bounds } => {
            let ray = ray_payload(ray)?;
            let bounds = bounds_payload(bounds)?;
            Ok(serde_json::json!({
                "mode": "rayBounds",
                "intersection": intersect_ray_bounds(ray, bounds).map_err(|error| error.to_string())?
            }))
        }
        IntersectionsRequest::SphereSphere {
            left_sphere,
            right_sphere,
        } => {
            let left = sphere_payload(left_sphere)?;
            let right = sphere_payload(right_sphere)?;
            let collision =
                collision_sphere_sphere(left, right).map_err(|error| error.to_string())?;
            Ok(serde_json::json!({
                "mode": "sphereSphere",
                "intersects": collision.is_some(),
                "collision": collision
            }))
        }
        IntersectionsRequest::SphereBounds { sphere, bounds } => {
            let sphere = sphere_payload(sphere)?;
            let bounds = bounds_payload(bounds)?;
            Ok(serde_json::json!({
                "mode": "sphereBounds",
                "intersects": sphere_intersects_bounds(sphere, bounds).map_err(|error| error.to_string())?
            }))
        }
    }
}

fn transform_compose_value(request: TransformComposeRequest) -> Result<serde_json::Value, String> {
    match request.precision {
        Precision::F32 => {
            let left = trs_payload(request.left)?;
            let right = trs_payload(request.right)?;
            let matrix = left
                .to_affine()
                .and_then(|affine| affine.compose(right.to_affine()?))
                .map_err(|error| error.to_string())?
                .matrix;
            Ok(matrix4_value(matrix))
        }
        Precision::F64 => {
            let left = trs_payloadd(request.left)?;
            let right = trs_payloadd(request.right)?;
            let matrix = left
                .to_affine()
                .and_then(|affine| affine.compose(right.to_affine()?))
                .map_err(|error| error.to_string())?
                .matrix;
            Ok(matrix4d_value(matrix))
        }
    }
}

fn transform_inverse_value(request: TransformInverseRequest) -> Result<serde_json::Value, String> {
    match request.precision {
        Precision::F32 => {
            let affine = AffineTransform3::from_matrix(
                Matrix4::new(request.matrix).map_err(|error| error.to_string())?,
            )
            .map_err(|error| error.to_string())?;
            let inverse = affine.inverse().map_err(|error| error.to_string())?.matrix;
            Ok(matrix4_value(inverse))
        }
        Precision::F64 => {
            let affine = crate::AffineTransform3d::from_matrix(
                Matrix4d::new(request.matrix.map(|row| row.map(f64::from)))
                    .map_err(|error| error.to_string())?,
            )
            .map_err(|error| error.to_string())?;
            let inverse = affine.inverse().map_err(|error| error.to_string())?.matrix;
            Ok(matrix4d_value(inverse))
        }
    }
}

fn transform_apply_value(request: TransformApplyRequest) -> Result<serde_json::Value, String> {
    match request.precision {
        Precision::F32 => {
            let transform = trs_payload(request.transform)?
                .to_affine()
                .map_err(|error| error.to_string())?;
            let point = request
                .point
                .map(|value| {
                    transform
                        .apply_point(point(value))
                        .map(|point| point.to_array())
                })
                .transpose()
                .map_err(|error| error.to_string())?;
            let vector = request
                .vector
                .map(|value| {
                    transform
                        .apply_vector(Vector3::new(value[0], value[1], value[2]))
                        .map(Vector3::to_array)
                })
                .transpose()
                .map_err(|error| error.to_string())?;
            Ok(serde_json::json!({
                "point": point,
                "vector": vector
            }))
        }
        Precision::F64 => {
            let transform = trs_payloadd(request.transform)?
                .to_affine()
                .map_err(|error| error.to_string())?;
            let point = request
                .point
                .map(|value| {
                    transform
                        .apply_point(pointd(value))
                        .map(|point| point.to_array())
                })
                .transpose()
                .map_err(|error| error.to_string())?;
            let vector = request
                .vector
                .map(|value| {
                    transform
                        .apply_vector(Vector3d::new(
                            value[0] as f64,
                            value[1] as f64,
                            value[2] as f64,
                        ))
                        .map(Vector3d::to_array)
                })
                .transpose()
                .map_err(|error| error.to_string())?;
            Ok(serde_json::json!({
                "precision": "f64",
                "point": point,
                "vector": vector
            }))
        }
    }
}

fn camera_project_value(request: CameraProjectRequest) -> Result<serde_json::Value, String> {
    match request.precision {
        Precision::F32 => {
            let pose = camera_pose_payload(request.pose)?;
            let intrinsics = intrinsics_payload(request.intrinsics)?;
            let projection = pose
                .project_point(intrinsics, point(request.point))
                .map_err(|error| error.to_string())?;
            Ok(serde_json::json!({
                "pixel": projection,
                "visible": projection.is_some()
            }))
        }
        Precision::F64 => {
            let pose = camera_pose_payloadd(request.pose)?;
            let intrinsics = intrinsics_payloadd(request.intrinsics)?;
            let projection = pose
                .project_point(intrinsics, pointd(request.point))
                .map_err(|error| error.to_string())?;
            Ok(serde_json::json!({
                "precision": "f64",
                "pixel": projection,
                "visible": projection.is_some()
            }))
        }
    }
}

fn camera_pixel_ray_value(request: CameraPixelRayRequest) -> Result<serde_json::Value, String> {
    match request.precision {
        Precision::F32 => {
            let pose = camera_pose_payload(request.pose)?;
            let intrinsics = intrinsics_payload(request.intrinsics)?;
            let ray = pose
                .pixel_ray(intrinsics, request.pixel, request.near, request.far)
                .map_err(|error| error.to_string())?;
            Ok(serde_json::json!({
                "origin": ray.origin.to_array(),
                "direction": ray.direction.to_array(),
                "tMin": ray.t_min,
                "tMax": ray.t_max
            }))
        }
        Precision::F64 => {
            let pose = camera_pose_payloadd(request.pose)?;
            let intrinsics = intrinsics_payloadd(request.intrinsics)?;
            let ray = pose
                .pixel_ray(
                    intrinsics,
                    [request.pixel[0] as f64, request.pixel[1] as f64],
                    request.near as f64,
                    request.far as f64,
                )
                .map_err(|error| error.to_string())?;
            Ok(serde_json::json!({
                "precision": "f64",
                "origin": ray.origin.to_array(),
                "direction": ray.direction.to_array(),
                "tMin": ray.t_min,
                "tMax": ray.t_max
            }))
        }
    }
}

fn camera_view_matrix_value(request: CameraViewMatrixRequest) -> Result<serde_json::Value, String> {
    match request.precision {
        Precision::F32 => {
            let pose = camera_pose_payload(request.pose)?;
            let matrix = match request.target {
                CameraViewTarget::Workspace => pose.view_matrix(),
                CameraViewTarget::Webgl => pose.gltf_webgl_view_matrix(),
            }
            .map_err(|error| error.to_string())?;
            Ok(matrix4_value(matrix))
        }
        Precision::F64 => {
            let pose = camera_pose_payloadd(request.pose)?;
            let matrix = match request.target {
                CameraViewTarget::Workspace => pose.view_matrix(),
                CameraViewTarget::Webgl => pose.gltf_webgl_view_matrix(),
            }
            .map_err(|error| error.to_string())?;
            Ok(matrix4d_value(matrix))
        }
    }
}

fn camera_projection_matrix_value(
    request: CameraProjectionMatrixRequest,
) -> Result<serde_json::Value, String> {
    match request.precision {
        Precision::F32 => {
            let intrinsics = intrinsics_payload(request.intrinsics)?;
            let matrix = match request.target {
                CameraProjectionTarget::OpencvDepth => {
                    intrinsics.projection_matrix_opencv_depth(request.near, request.far)
                }
                CameraProjectionTarget::Webgl => {
                    intrinsics.projection_matrix_webgl(request.near, request.far)
                }
            }
            .map_err(|error| error.to_string())?;
            Ok(matrix4_value(matrix))
        }
        Precision::F64 => {
            let intrinsics = intrinsics_payloadd(request.intrinsics)?;
            let matrix = match request.target {
                CameraProjectionTarget::OpencvDepth => intrinsics
                    .projection_matrix_opencv_depth(request.near as f64, request.far as f64),
                CameraProjectionTarget::Webgl => {
                    intrinsics.projection_matrix_webgl(request.near as f64, request.far as f64)
                }
            }
            .map_err(|error| error.to_string())?;
            Ok(matrix4d_value(matrix))
        }
    }
}

fn colmap_pose_value(request: ColmapPoseRequest) -> Result<serde_json::Value, String> {
    let pose = CameraPose3::from_colmap_world_to_camera(
        request.qw, request.qx, request.qy, request.qz, request.tx, request.ty, request.tz,
    )
    .map_err(|error| error.to_string())?;
    Ok(serde_json::json!({
        "position": pose.position.to_array(),
        "right": pose.right.to_array(),
        "up": pose.up.to_array(),
        "forward": pose.forward.to_array(),
        "viewMatrix": pose.view_matrix().map_err(|error| error.to_string())?.rows,
        "webglViewMatrix": pose.gltf_webgl_view_matrix().map_err(|error| error.to_string())?.rows
    }))
}

fn gltf_matrix_value(request: MatrixRequest) -> Result<serde_json::Value, String> {
    match request.precision {
        Precision::F32 => {
            let matrix = Matrix4::new(request.matrix).map_err(|error| error.to_string())?;
            Ok(matrix4_value(matrix))
        }
        Precision::F64 => {
            let matrix = Matrix4d::new(request.matrix.map(|row| row.map(f64::from)))
                .map_err(|error| error.to_string())?;
            Ok(matrix4d_value(matrix))
        }
    }
}

fn matrix_inspect_value(request: MatrixRequest) -> Result<serde_json::Value, String> {
    match request.precision {
        Precision::F32 => {
            let matrix = Matrix4::new(request.matrix).map_err(|error| error.to_string())?;
            let inverse = matrix.inverse();
            Ok(serde_json::json!({
                "rows": matrix.rows,
                "rowMajor": matrix.to_row_major_array(),
                "columnMajor": matrix.to_column_major_array(),
                "invertible": inverse.is_ok(),
                "inverse": inverse.ok().map(|matrix| matrix.rows)
            }))
        }
        Precision::F64 => {
            let matrix = Matrix4d::new(request.matrix.map(|row| row.map(f64::from)))
                .map_err(|error| error.to_string())?;
            let inverse = matrix.inverse();
            Ok(serde_json::json!({
                "precision": "f64",
                "rows": matrix.rows,
                "rowMajor": matrix.to_row_major_array(),
                "columnMajor": matrix.to_column_major_array(),
                "invertible": inverse.is_ok(),
                "inverse": inverse.ok().map(|matrix| matrix.rows)
            }))
        }
    }
}

fn rotation_inspect_value(request: RotationInspectRequest) -> Result<serde_json::Value, String> {
    let quaternion = Quaternion::new(
        request.quaternion[0],
        request.quaternion[1],
        request.quaternion[2],
        request.quaternion[3],
    )
    .normalize()
    .map_err(|error| error.to_string())?;
    Ok(serde_json::json!({
        "quaternion": [quaternion.x, quaternion.y, quaternion.z, quaternion.w],
        "norm": quaternion.norm(),
        "rotationMatrix": quaternion.to_rotation_matrix().map_err(|error| error.to_string())?.rows
    }))
}

fn transform_diagnostics_value(
    request: TransformDiagnosticsRequest,
) -> Result<serde_json::Value, String> {
    let transform = trs_payload(request.transform)?;
    let affine = transform.to_affine().map_err(|error| error.to_string())?;
    Ok(serde_json::json!({
        "matrix": affine.matrix.rows,
        "inverseAvailable": affine.inverse().is_ok(),
        "translation": transform.translation.to_array(),
        "scale": transform.scale.to_array()
    }))
}

fn parse_points(points: Vec<[f32; 3]>) -> Vec<Point3> {
    points.into_iter().map(point).collect()
}

fn point(value: [f32; 3]) -> Point3 {
    Point3::new(value[0], value[1], value[2])
}

fn pointd(value: [f32; 3]) -> Point3d {
    Point3d::new(value[0] as f64, value[1] as f64, value[2] as f64)
}

fn matrix4_value(matrix: Matrix4) -> serde_json::Value {
    serde_json::json!({
        "matrix": matrix.rows,
        "rowMajor": matrix.to_row_major_array(),
        "columnMajor": matrix.to_column_major_array()
    })
}

fn matrix4d_value(matrix: Matrix4d) -> serde_json::Value {
    serde_json::json!({
        "precision": "f64",
        "matrix": matrix.rows,
        "rowMajor": matrix.to_row_major_array(),
        "columnMajor": matrix.to_column_major_array()
    })
}

fn ray_payload(payload: RayPayload) -> Result<Ray3, String> {
    Ray3::new(
        point(payload.origin),
        Vector3::new(
            payload.direction[0],
            payload.direction[1],
            payload.direction[2],
        ),
    )
    .map_err(|error| error.to_string())
}

fn sphere_payload(payload: SpherePayload) -> Result<Sphere3, String> {
    Sphere3::new(point(payload.center), payload.radius).map_err(|error| error.to_string())
}

fn bounds_payload(payload: BoundsPayload) -> Result<Bounds3, String> {
    Bounds3::new(point(payload.min), point(payload.max)).map_err(|error| error.to_string())
}

fn trs_payload(payload: TrsPayload) -> Result<TrsTransform3, String> {
    TrsTransform3::new(
        Vector3::new(
            payload.translation[0],
            payload.translation[1],
            payload.translation[2],
        ),
        Quaternion::new(
            payload.rotation[0],
            payload.rotation[1],
            payload.rotation[2],
            payload.rotation[3],
        ),
        Vector3::new(payload.scale[0], payload.scale[1], payload.scale[2]),
    )
    .map_err(|error| error.to_string())
}

fn trs_payloadd(payload: TrsPayload) -> Result<TrsTransform3d, String> {
    TrsTransform3d::new(
        Vector3d::new(
            payload.translation[0] as f64,
            payload.translation[1] as f64,
            payload.translation[2] as f64,
        ),
        crate::Quaterniond::new(
            payload.rotation[0] as f64,
            payload.rotation[1] as f64,
            payload.rotation[2] as f64,
            payload.rotation[3] as f64,
        ),
        Vector3d::new(
            payload.scale[0] as f64,
            payload.scale[1] as f64,
            payload.scale[2] as f64,
        ),
    )
    .map_err(|error| error.to_string())
}

fn camera_pose_payload(payload: CameraPosePayload) -> Result<CameraPose3, String> {
    CameraPose3::new(
        point(payload.position),
        Vector3::new(payload.right[0], payload.right[1], payload.right[2]),
        Vector3::new(payload.up[0], payload.up[1], payload.up[2]),
        Vector3::new(payload.forward[0], payload.forward[1], payload.forward[2]),
    )
    .map_err(|error| error.to_string())
}

fn camera_pose_payloadd(payload: CameraPosePayload) -> Result<CameraPose3d, String> {
    CameraPose3d::new(
        pointd(payload.position),
        Vector3d::new(
            payload.right[0] as f64,
            payload.right[1] as f64,
            payload.right[2] as f64,
        ),
        Vector3d::new(
            payload.up[0] as f64,
            payload.up[1] as f64,
            payload.up[2] as f64,
        ),
        Vector3d::new(
            payload.forward[0] as f64,
            payload.forward[1] as f64,
            payload.forward[2] as f64,
        ),
    )
    .map_err(|error| error.to_string())
}

fn intrinsics_payload(payload: PinholeIntrinsicsPayload) -> Result<PinholeIntrinsics, String> {
    PinholeIntrinsics::new(
        payload.width,
        payload.height,
        payload.fx,
        payload.fy,
        payload.cx,
        payload.cy,
    )
    .map_err(|error| error.to_string())
}

fn intrinsics_payloadd(payload: PinholeIntrinsicsPayload) -> Result<PinholeIntrinsicsd, String> {
    PinholeIntrinsicsd::new(
        payload.width,
        payload.height,
        payload.fx as f64,
        payload.fy as f64,
        payload.cx as f64,
        payload.cy as f64,
    )
    .map_err(|error| error.to_string())
}

fn default_far() -> f32 {
    f32::MAX
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_point_cloud_summary_is_valid() {
        let response = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("threeD.pointCloud.summary"),
            input: serde_json::json!({"points": []}),
        })
        .expect("summary");
        assert_eq!(response.value["count"], 0);
        assert!(response.value["bounds"].is_null());
    }

    #[test]
    fn downsample_rejects_non_positive_voxel_size() {
        let error = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("threeD.pointCloud.downsample"),
            input: serde_json::json!({"points": [[0, 0, 0]], "voxelSize": 0.0}),
        })
        .expect_err("bad voxel");
        assert!(error.contains("voxel"));
    }

    #[test]
    fn ray_sphere_intersections_are_stable() {
        let response = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("threeD.geometry.intersections"),
            input: serde_json::json!({"mode": "raySphere", "ray": {"origin": [0, 0, -2], "direction": [0, 0, 1]}, "sphere": {"center": [0, 0, 0], "radius": 1.0}}),
        })
        .expect("intersections");
        assert_eq!(response.value["points"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn surface_reports_ray_bounds_and_sphere_collisions() {
        let ray_bounds = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("threeD.geometry.intersections"),
            input: serde_json::json!({"mode": "rayBounds", "ray": {"origin": [0, 0, -2], "direction": [0, 0, 1]}, "bounds": {"min": [-1, -1, -1], "max": [1, 1, 1]}}),
        })
        .expect("ray bounds");
        assert_eq!(ray_bounds.value["intersection"]["entryDistance"], 1.0);

        let sphere_sphere = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("threeD.geometry.intersections"),
            input: serde_json::json!({"mode": "sphereSphere", "leftSphere": {"center": [0, 0, 0], "radius": 1.0}, "rightSphere": {"center": [1.5, 0, 0], "radius": 1.0}}),
        })
        .expect("sphere sphere");
        assert_eq!(sphere_sphere.value["intersects"], true);
        assert_eq!(sphere_sphere.value["collision"]["penetrationDepth"], 0.5);
    }

    #[test]
    fn surface_reports_sphere_bounds_and_misses() {
        let sphere_bounds = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("threeD.geometry.intersections"),
            input: serde_json::json!({"mode": "sphereBounds", "sphere": {"center": [2, 0, 0], "radius": 1.0}, "bounds": {"min": [-1, -1, -1], "max": [1, 1, 1]}}),
        })
        .expect("sphere bounds");
        assert_eq!(sphere_bounds.value["intersects"], true);

        let miss = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("threeD.geometry.intersections"),
            input: serde_json::json!({"mode": "rayBounds", "ray": {"origin": [2, 0, -2], "direction": [0, 0, 1]}, "bounds": {"min": [-1, -1, -1], "max": [1, 1, 1]}}),
        })
        .expect("ray bounds miss");
        assert!(miss.value["intersection"].is_null());
    }

    #[test]
    fn surface_rejects_missing_geometry_payloads() {
        let error = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("threeD.geometry.intersections"),
            input: serde_json::json!({"mode": "rayBounds", "ray": {"origin": [0, 0, -2], "direction": [0, 0, 1]}}),
        })
        .expect_err("missing bounds");
        assert!(error.contains("bounds"));
    }
}
