//! Library-owned runtime surface for `runtime-contracts`.

use crate::{
    OperationId, PackageSurface, RuntimeCapabilities, SurfaceOperation, SurfaceRequest,
    SurfaceResponse,
};

const SURFACE_LIBRARY_NAME: &str = "runtime-contracts";

/// Returns the package surface exposed by every transport wrapper.
pub fn package_surface() -> PackageSurface {
    PackageSurface {
        library: SURFACE_LIBRARY_NAME.to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        capabilities: RuntimeCapabilities::pure_rust(),
        operations: vec![SurfaceOperation {
            id: OperationId::new("describe"),
            name: "Describe package".to_string(),
            description: Some("Shared operation, diagnostic, capability, job, and artifact DTOs for runtime surfaces.".to_string()),
            input_schema: serde_json::json!({
                "type": "object",
                "additionalProperties": true
            }),
            output_schema: serde_json::json!({
                "type": "object",
                "required": ["library", "version", "operationCount"]
            }),
            example_request: serde_json::json!({
                "includeOperations": true
            }),
            wasm_supported: true,
            server_supported: true,
        }],
    }
}

/// Runs one library-owned operation.
pub fn run_surface_operation(request: SurfaceRequest) -> Result<SurfaceResponse, String> {
    match request.operation.as_str() {
        "describe" => {
            let surface = package_surface();
            Ok(SurfaceResponse {
                operation: request.operation,
                value: serde_json::json!({
                    "library": surface.library,
                    "version": surface.version,
                    "operationCount": surface.operations.len(),
                    "operations": surface
                        .operations
                        .iter()
                        .map(|operation| operation.id.as_str())
                        .collect::<Vec<_>>(),
                    "input": request.input
                }),
                diagnostics: Vec::new(),
                artifacts: Vec::new(),
            })
        }
        operation => Err(format!(
            "unsupported operation `{operation}` for {SURFACE_LIBRARY_NAME}"
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn package_surface_has_describe_operation() {
        let surface = package_surface();
        assert_eq!(surface.library, SURFACE_LIBRARY_NAME);
        assert!(!surface.operations.is_empty());
    }

    #[test]
    fn describe_operation_returns_surface_summary() {
        let response = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("describe"),
            input: serde_json::json!({"includeOperations": true}),
        })
        .expect("describe operation");

        assert_eq!(response.operation.as_str(), "describe");
        assert_eq!(response.value["library"], SURFACE_LIBRARY_NAME);
    }
}
