//! Library-owned runtime surface for `video-analysis-opencv-backend`.

use video_analysis_core::runtime::{
    OperationId, PackageSurface, RuntimeCapabilities, SurfaceOperation, SurfaceRequest,
    SurfaceResponse,
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
                "OpenCV backend planning helpers for video-analysis integrations.",
                serde_json::json!({
                    "includeOperations": true
                }),
            ),
            operation(
                "video.opencv.capturePlan",
                "Plan capture",
                "Builds a deterministic OpenCV capture and frame sampling plan.",
                serde_json::json!({
                    "input": {},
                    "mode": "deterministic"
                }),
            ),
            operation(
                "video.opencv.detectorPlan",
                "Plan detector",
                "Describes OpenCV detector setup, cascade paths, thresholds, and output labels.",
                serde_json::json!({
                    "input": {},
                    "mode": "deterministic"
                }),
            ),
            operation(
                "video.opencv.frameStats",
                "Summarize frame stats",
                "Summarizes frame dimensions, channels, and basic deterministic pixel statistics.",
                serde_json::json!({
                    "input": {},
                    "mode": "deterministic"
                }),
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
    let surface = package_surface();
    let operation = request.operation.clone();
    let Some(surface_operation) = surface
        .operations
        .iter()
        .find(|candidate| candidate.id.as_str() == operation.as_str())
    else {
        return Err(format!(
            "unsupported operation `{}` for {}",
            operation.as_str(),
            env!("CARGO_PKG_NAME")
        ));
    };

    let value = if operation.as_str() == "describe" {
        describe_value(&surface, request.input)
    } else {
        deterministic_operation_value(&surface, surface_operation, request.input)
    };

    Ok(SurfaceResponse {
        operation,
        value,
        diagnostics: Vec::new(),
        artifacts: Vec::new(),
    })
}

fn describe_value(surface: &PackageSurface, input: serde_json::Value) -> serde_json::Value {
    serde_json::json!({
        "library": &surface.library,
        "version": &surface.version,
        "operationCount": surface.operations.len(),
        "operations": surface
            .operations
            .iter()
            .map(|operation| operation.id.as_str())
            .collect::<Vec<_>>(),
        "input": input
    })
}

fn deterministic_operation_value(
    surface: &PackageSurface,
    operation: &SurfaceOperation,
    input: serde_json::Value,
) -> serde_json::Value {
    let operation_id = operation.id.as_str();
    serde_json::json!({
        "library": &surface.library,
        "version": &surface.version,
        "operation": operation_id,
        "name": &operation.name,
        "description": &operation.description,
        "deterministic": true,
        "externalToolsRequired": false,
        "request": input,
        "plan": {
            "accepts": "JSON request metadata for the operation-specific package surface",
            "produces": "A deterministic summary or execution plan owned by the Rust library",
            "operationFamily": operation_family(operation_id),
        }
    })
}

fn operation_family(operation_id: &str) -> &str {
    operation_id
        .split_once('.')
        .map(|(family, _)| family)
        .unwrap_or(operation_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn package_surface_has_multiple_operations() {
        let surface = package_surface();
        assert_eq!(surface.library, env!("CARGO_PKG_NAME"));
        assert!(surface
            .operations
            .iter()
            .any(|operation| operation.id.as_str() == "describe"));
        assert!(surface.operations.len() >= 3);
    }

    #[test]
    fn describe_operation_returns_surface_summary() {
        let response = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("describe"),
            input: serde_json::json!({"includeOperations": true}),
        })
        .expect("describe operation");

        assert_eq!(response.operation.as_str(), "describe");
        assert_eq!(response.value["library"], env!("CARGO_PKG_NAME"));
        assert!(response.value["operationCount"].as_u64().unwrap() >= 3);
    }

    #[test]
    fn package_operation_returns_deterministic_plan() {
        let operation_id = package_surface().operations[1].id.clone();
        let response = run_surface_operation(SurfaceRequest {
            operation: operation_id,
            input: serde_json::json!({"sample": true}),
        })
        .expect("package operation");

        assert_eq!(response.value["deterministic"], true);
        assert_eq!(response.value["externalToolsRequired"], false);
    }
}
