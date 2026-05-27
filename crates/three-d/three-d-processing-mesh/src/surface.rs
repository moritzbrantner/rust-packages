//! Library-owned runtime surface for `three-d-processing-mesh`.

use serde::de::DeserializeOwned;
use serde::Deserialize;
use three_d_processing_core::Point3;
use video_analysis_core::runtime::{
    OperationId, PackageSurface, RuntimeCapabilities, SurfaceOperation, SurfaceRequest,
    SurfaceResponse,
};

use crate::{
    is_manifold, is_watertight, mesh_diagnostics, mesh_topology, remove_degenerate_triangles,
    sample_points_uniform, surface_area, weld_vertices, Mesh, Triangle,
};

const MAX_SAMPLE_COUNT: usize = 1024;

/// Returns the package surface exposed by every transport wrapper.
pub fn package_surface() -> PackageSurface {
    PackageSurface {
        library: env!("CARGO_PKG_NAME").to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        capabilities: RuntimeCapabilities::pure_rust(),
        operations: vec![
            operation("describe", "Describe package", "Triangle mesh validation and geometry helpers for video-analysis.", serde_json::json!({"includeOperations": true})),
            operation("threeD.mesh.diagnostics", "Mesh diagnostics", "Returns mesh diagnostics, topology counts, area, bounds, and manifold flags.", serde_json::json!({"mesh": sample_mesh_json()})),
            operation("threeD.mesh.repairPreview", "Mesh repair preview", "Runs in-memory degenerate-triangle removal and vertex welding previews.", serde_json::json!({"mesh": sample_mesh_json(), "weldEpsilon": 0.0001, "removeDegenerate": true})),
            operation("threeD.mesh.sample", "Mesh sample", "Samples deterministic points from mesh surface with a capped sample count.", serde_json::json!({"mesh": sample_mesh_json(), "sampleCount": 32})),
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
        "threeD.mesh.diagnostics" => diagnostics_value(parse_input(request.input)?)?,
        "threeD.mesh.repairPreview" => repair_preview_value(parse_input(request.input)?)?,
        "threeD.mesh.sample" => sample_value(parse_input(request.input)?)?,
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
struct MeshRequest {
    mesh: MeshPayload,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RepairPreviewRequest {
    mesh: MeshPayload,
    weld_epsilon: Option<f32>,
    remove_degenerate: Option<bool>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SampleRequest {
    mesh: MeshPayload,
    sample_count: usize,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MeshPayload {
    vertices: Vec<[f32; 3]>,
    triangles: Vec<[usize; 3]>,
}

fn diagnostics_value(request: MeshRequest) -> Result<serde_json::Value, String> {
    let mesh = request.mesh.mesh()?;
    let diagnostics = mesh_diagnostics(&mesh).map_err(|error| error.to_string())?;
    let topology = mesh_topology(&mesh).map_err(|error| error.to_string())?;
    Ok(serde_json::json!({
        "vertexCount": mesh.vertices.len(),
        "triangleCount": mesh.triangles.len(),
        "edgeCount": topology.edges.len(),
        "boundaryEdgeCount": topology.boundary_edges.len(),
        "surfaceArea": surface_area(&mesh).map_err(|error| error.to_string())?,
        "bounds": mesh.bounds().map_err(|error| error.to_string())?,
        "isWatertight": is_watertight(&mesh).map_err(|error| error.to_string())?,
        "isManifold": is_manifold(&mesh).map_err(|error| error.to_string())?,
        "diagnostics": diagnostics
    }))
}

fn repair_preview_value(request: RepairPreviewRequest) -> Result<serde_json::Value, String> {
    let mesh = request.mesh.mesh()?;
    let mut repaired = mesh.clone();
    if request.remove_degenerate.unwrap_or(false) {
        repaired = remove_degenerate_triangles(&repaired).map_err(|error| error.to_string())?;
    }
    if let Some(epsilon) = request.weld_epsilon {
        repaired = weld_vertices(&repaired, epsilon).map_err(|error| error.to_string())?;
    }
    Ok(serde_json::json!({
        "before": {"vertices": mesh.vertices.len(), "triangles": mesh.triangles.len()},
        "after": {"vertices": repaired.vertices.len(), "triangles": repaired.triangles.len()}
    }))
}

fn sample_value(request: SampleRequest) -> Result<serde_json::Value, String> {
    if request.sample_count > MAX_SAMPLE_COUNT {
        return Err(format!("sampleCount must be <= {MAX_SAMPLE_COUNT}"));
    }
    let mesh = request.mesh.mesh()?;
    let points = sample_points_uniform(&mesh, request.sample_count).map_err(|error| error.to_string())?;
    Ok(serde_json::json!({"points": points, "count": points.len()}))
}

impl MeshPayload {
    fn mesh(self) -> Result<Mesh, String> {
        Mesh::new(
            self.vertices
                .into_iter()
                .map(|value| Point3::new(value[0], value[1], value[2]))
                .collect::<Vec<_>>(),
            self.triangles
                .into_iter()
                .map(|value| Triangle::new(value[0], value[1], value[2]))
                .collect::<Vec<_>>(),
        )
        .map_err(|error| error.to_string())
    }
}

fn sample_mesh_json() -> serde_json::Value {
    serde_json::json!({
        "vertices": [[0, 0, 0], [1, 0, 0], [0, 1, 0]],
        "triangles": [[0, 1, 2]]
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diagnostics_detect_degenerate_triangle() {
        let response = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("threeD.mesh.diagnostics"),
            input: serde_json::json!({"mesh": {"vertices": [[0,0,0], [1,0,0], [2,0,0]], "triangles": [[0,1,2]]}}),
        })
        .expect("diagnostics");
        assert_eq!(response.value["diagnostics"]["degenerate_triangles"][0], 0);
    }

    #[test]
    fn repair_preview_reduces_degenerate_triangle_count() {
        let response = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("threeD.mesh.repairPreview"),
            input: serde_json::json!({"mesh": {"vertices": [[0,0,0], [1,0,0], [2,0,0]], "triangles": [[0,1,2]]}, "removeDegenerate": true}),
        })
        .expect("repair");
        assert_eq!(response.value["after"]["triangles"], 0);
    }

    #[test]
    fn sample_rejects_excessive_count() {
        let error = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("threeD.mesh.sample"),
            input: serde_json::json!({"mesh": sample_mesh_json(), "sampleCount": 2048}),
        })
        .expect_err("too many samples");
        assert!(error.contains("sampleCount"));
    }
}
