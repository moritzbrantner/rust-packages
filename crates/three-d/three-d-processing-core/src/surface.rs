//! Library-owned runtime surface for `three-d-processing-core`.

use serde::de::DeserializeOwned;
use serde::Deserialize;
use video_analysis_core::runtime::{
    OperationId, PackageSurface, RuntimeCapabilities, SurfaceOperation, SurfaceRequest,
    SurfaceResponse,
};

use crate::{
    centroid, collision_sphere_sphere, intersect_ray_bounds, intersect_ray_sphere,
    sphere_intersects_bounds, voxel_downsample, Bounds3, Point3, Ray3, Sphere3, Vector3,
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
        "threeD.pointCloud.downsample" => {
            point_cloud_downsample_value(parse_input(request.input)?)?
        }
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

fn parse_points(points: Vec<[f32; 3]>) -> Vec<Point3> {
    points.into_iter().map(point).collect()
}

fn point(value: [f32; 3]) -> Point3 {
    Point3::new(value[0], value[1], value[2])
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
