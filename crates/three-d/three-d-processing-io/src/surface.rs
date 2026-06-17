//! Library-owned runtime surface for `three-d-processing-io`.

use runtime_core::{
    OperationId, PackageSurface, RuntimeCapabilities, SurfaceOperation, SurfaceRequest,
    SurfaceResponse,
};
use serde::de::DeserializeOwned;
use serde::Deserialize;
use three_d_processing_mesh::Mesh;

use crate::{mesh_to_obj_string, mesh_to_ply_string};

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
                "Mesh and point-cloud interchange formats for video-analysis.",
                serde_json::json!({"type": "object", "additionalProperties": true}),
                serde_json::json!({
                    "type": "object",
                    "required": ["library", "version", "operationCount"]
                }),
                serde_json::json!({"includeOperations": true}),
            ),
            operation(
                "threeD.io.supportedFormats",
                "Supported formats",
                "List mesh formats supported by the pure IO helpers.",
                serde_json::json!({"type": "object", "additionalProperties": false}),
                serde_json::json!({"type": "object", "required": ["formats"]}),
                serde_json::json!({}),
            ),
            operation(
                "threeD.io.objPreview",
                "OBJ preview",
                "Encode a mesh as OBJ and return a capped string preview.",
                serde_json::json!({
                    "type": "object",
                    "required": ["mesh"],
                    "properties": {
                        "mesh": {"type": "object"},
                        "limitBytes": {"type": "integer", "minimum": 1, "default": 4096}
                    }
                }),
                serde_json::json!({
                    "type": "object",
                    "required": ["preview", "byteCount", "truncated"]
                }),
                serde_json::json!({
                    "mesh": {
                        "vertices": [[0, 0, 0], [1, 0, 0], [0, 1, 0]],
                        "triangles": [{"vertices": [0, 1, 2]}]
                    },
                    "limitBytes": 4096
                }),
            ),
            operation(
                "threeD.io.plyPreview",
                "PLY preview",
                "Encode a mesh as ASCII PLY and return a capped string preview.",
                serde_json::json!({
                    "type": "object",
                    "required": ["mesh"],
                    "properties": {
                        "mesh": {"type": "object"},
                        "limitBytes": {"type": "integer", "minimum": 1, "default": 4096}
                    }
                }),
                serde_json::json!({
                    "type": "object",
                    "required": ["preview", "byteCount", "truncated"]
                }),
                serde_json::json!({
                    "mesh": {
                        "vertices": [[0, 0, 0], [1, 0, 0], [0, 1, 0]],
                        "triangles": [{"vertices": [0, 1, 2]}]
                    },
                    "limitBytes": 4096
                }),
            ),
        ],
    }
}

/// Runs one library-owned operation.
pub fn run_surface_operation(request: SurfaceRequest) -> Result<SurfaceResponse, String> {
    let operation_id = request.operation.as_str().to_string();
    match request.operation.as_str() {
        "describe" => Ok(describe_value(request.input)),
        "threeD.io.supportedFormats" => Ok(surface_response(
            request.operation,
            serde_json::json!({"formats": ["obj", "ply", "gltf"]}),
        )),
        "threeD.io.objPreview" => {
            let input = parse_input::<PreviewRequest>(request.input)?;
            let encoded = mesh_to_obj_string(&input.mesh).map_err(|err| err.to_string())?;
            Ok(surface_response(
                request.operation,
                preview_value(&encoded, input.limit_bytes)?,
            ))
        }
        "threeD.io.plyPreview" => {
            let input = parse_input::<PreviewRequest>(request.input)?;
            let encoded = mesh_to_ply_string(&input.mesh).map_err(|err| err.to_string())?;
            Ok(surface_response(
                request.operation,
                preview_value(&encoded, input.limit_bytes)?,
            ))
        }
        operation => Err(format!(
            "unsupported operation `{operation}` for {}",
            env!("CARGO_PKG_NAME")
        )),
    }
    .map_err(|err| {
        if err.starts_with("unsupported operation") || err.starts_with("invalid request") {
            err
        } else {
            format!("{operation_id}: {err}")
        }
    })
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PreviewRequest {
    mesh: Mesh,
    #[serde(default = "default_limit_bytes")]
    limit_bytes: usize,
}

fn default_limit_bytes() -> usize {
    4096
}

fn preview_value(encoded: &str, limit_bytes: usize) -> Result<serde_json::Value, String> {
    if limit_bytes == 0 || limit_bytes > 65_536 {
        return Err("invalid request: limitBytes must be between 1 and 65536".to_string());
    }
    let byte_count = encoded.len();
    let preview = if byte_count > limit_bytes {
        truncate_utf8(encoded, limit_bytes).to_string()
    } else {
        encoded.to_string()
    };
    Ok(serde_json::json!({
        "preview": preview,
        "byteCount": byte_count,
        "returnedBytes": preview.len(),
        "truncated": byte_count > preview.len()
    }))
}

fn truncate_utf8(value: &str, limit: usize) -> &str {
    if limit >= value.len() {
        return value;
    }
    let mut end = limit;
    while !value.is_char_boundary(end) {
        end -= 1;
    }
    &value[..end]
}

fn operation(
    id: &str,
    name: &str,
    description: &str,
    input_schema: serde_json::Value,
    output_schema: serde_json::Value,
    example_request: serde_json::Value,
) -> SurfaceOperation {
    SurfaceOperation {
        id: OperationId::new(id),
        name: name.to_string(),
        description: Some(description.to_string()),
        curation: runtime_core::SurfaceOperationCuration::from_operation_id(id),
        input_schema,
        output_schema,
        example_request,
        wasm_supported: true,
        server_supported: true,
    }
}

fn describe_value(input: serde_json::Value) -> SurfaceResponse {
    let surface = package_surface();
    surface_response(
        OperationId::new("describe"),
        serde_json::json!({
            "library": surface.library,
            "version": surface.version,
            "operationCount": surface.operations.len(),
            "operations": surface
                .operations
                .iter()
                .map(|operation| operation.id.as_str())
                .collect::<Vec<_>>(),
            "input": input
        }),
    )
}

fn parse_input<T: DeserializeOwned>(input: serde_json::Value) -> Result<T, String> {
    serde_json::from_value(input).map_err(|err| format!("invalid request: {err}"))
}

fn surface_response(operation: OperationId, value: serde_json::Value) -> SurfaceResponse {
    SurfaceResponse {
        operation,
        value,
        diagnostics: Vec::new(),
        artifacts: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mesh_value() -> serde_json::Value {
        serde_json::json!({
            "vertices": [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            "triangles": [{"vertices": [0, 1, 2]}]
        })
    }

    #[test]
    fn package_surface_has_multiple_operations() {
        let surface = package_surface();
        assert_eq!(surface.library, env!("CARGO_PKG_NAME"));
        assert!(surface.operations.len() > 1);
    }

    #[test]
    fn preview_truncates_deterministically() {
        let response = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("threeD.io.objPreview"),
            input: serde_json::json!({"mesh": mesh_value(), "limitBytes": 10}),
        })
        .expect("obj preview");

        assert_eq!(response.value["preview"], "v 0 0 0\nv ");
        assert_eq!(response.value["truncated"], true);
    }

    #[test]
    fn unsupported_format_operation_is_not_accepted() {
        let err = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("threeD.io.stlPreview"),
            input: serde_json::json!({}),
        })
        .expect_err("unsupported operation");

        assert!(err.contains("unsupported operation"));
    }
}
