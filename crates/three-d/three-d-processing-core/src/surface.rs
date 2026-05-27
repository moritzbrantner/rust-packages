//! Library-owned runtime surface for `three-d-processing-core`.

use serde::de::DeserializeOwned;
use serde::Deserialize;
use video_analysis_core::runtime::{
    OperationId, PackageSurface, RuntimeCapabilities, SurfaceOperation, SurfaceRequest,
    SurfaceResponse,
};

use crate::{
    centroid, intersect_ray_sphere, voxel_downsample, Bounds3, Point3, Ray3, Sphere3, Vector3,
};

/// Returns the package surface exposed by every transport wrapper.
pub fn package_surface() -> PackageSurface {
    PackageSurface {
        library: env!("CARGO_PKG_NAME").to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        capabilities: RuntimeCapabilities::pure_rust(),
        operations: vec![
            operation("describe", "Describe package", "Shared 3D geometry primitives and transforms for video-analysis.", serde_json::json!({"includeOperations": true})),
            operation("threeD.pointCloud.summary", "Point cloud summary", "Summarizes point count, centroid, and bounds.", serde_json::json!({"points": [[0, 0, 0], [1, 1, 1]]})),
            operation("threeD.pointCloud.downsample", "Point cloud downsample", "Runs deterministic voxel downsampling over points.", serde_json::json!({"points": [[0, 0, 0], [0.01, 0, 0]], "voxelSize": 0.1})),
            operation("threeD.geometry.intersections", "Geometry intersections", "Computes supported 3D geometry intersections.", serde_json::json!({"mode": "raySphere", "ray": {"origin": [0, 0, -2], "direction": [0, 0, 1]}, "sphere": {"center": [0, 0, 0], "radius": 1}})),
        ],
    }
}

fn operation(id: &str, name: &str, description: &str, example_request: serde_json::Value) -> SurfaceOperation {
    SurfaceOperation {
        id: OperationId::new(id),
        name: name.to_string(),
        description: Some(description.to_string()),
        input_schema: serde_json::json!({"type": "object", "additionalProperties": true}),
        output_schema: serde_json::json!({"type": "object"}),
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
        "threeD.pointCloud.downsample" => point_cloud_downsample_value(parse_input(request.input)?)?,
        "threeD.geometry.intersections" => intersections_value(parse_input(request.input)?)?,
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
    SurfaceResponse { operation, value, diagnostics: Vec::new(), artifacts: Vec::new() }
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
#[serde(rename_all = "camelCase")]
struct IntersectionsRequest {
    mode: String,
    ray: RayPayload,
    sphere: SpherePayload,
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
    match request.mode.as_str() {
        "raySphere" => {
            let ray = Ray3::new(
                point(request.ray.origin),
                Vector3::new(
                    request.ray.direction[0],
                    request.ray.direction[1],
                    request.ray.direction[2],
                ),
            )
            .map_err(|error| error.to_string())?;
            let sphere = Sphere3::new(point(request.sphere.center), request.sphere.radius)
                .map_err(|error| error.to_string())?;
            Ok(serde_json::json!({
                "mode": request.mode,
                "points": intersect_ray_sphere(ray, sphere).map_err(|error| error.to_string())?
            }))
        }
        other => Err(format!("unsupported intersection mode `{other}`")),
    }
}

fn parse_points(points: Vec<[f32; 3]>) -> Vec<Point3> {
    points.into_iter().map(point).collect()
}

fn point(value: [f32; 3]) -> Point3 {
    Point3::new(value[0], value[1], value[2])
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
}
