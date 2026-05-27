//! Library-owned runtime surface for `video-analysis-test-support`.

use video_analysis_core::runtime::{
    OperationId, PackageSurface, RuntimeCapabilities, SurfaceOperation, SurfaceRequest,
    SurfaceResponse,
};

use crate::{dataset_with_scene_text_and_feature, owned_text_segment, scene};

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
                "Minimal test-support fixture catalog and contract fixture summaries.",
                serde_json::json!({"includeOperations": true}),
            ),
            operation(
                "testSupport.catalog",
                "Fixture catalog",
                "Lists helper fixture categories without creating heavy video, 3D, or external-tool payloads.",
                serde_json::json!({}),
            ),
            operation(
                "testSupport.minimalReport",
                "Minimal report",
                "Returns a compact summary of small contract fixture builders.",
                serde_json::json!({}),
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
        "testSupport.catalog" => catalog_value(),
        "testSupport.minimalReport" => minimal_report_value(),
        operation => {
            return Err(format!(
                "unsupported operation `{operation}` for {}",
                env!("CARGO_PKG_NAME")
            ));
        }
    };
    Ok(response(operation, value))
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

fn response(operation: OperationId, value: serde_json::Value) -> SurfaceResponse {
    SurfaceResponse {
        operation,
        value,
        diagnostics: Vec::new(),
        artifacts: Vec::new(),
    }
}

fn catalog_value() -> serde_json::Value {
    serde_json::json!({
        "categories": [
            {"id": "time", "helpers": ["timestamp", "timestamp_at", "frame_position"]},
            {"id": "scene", "helpers": ["scene"]},
            {"id": "text", "helpers": ["owned_text_segment", "text_segment"]},
            {"id": "dataset", "helpers": ["dataset_with_scene_text_and_feature"]},
            {"id": "environment", "helpers": ["find_command", "command_succeeds"]}
        ],
        "deferredHeavyCategories": ["video_fixture_download", "colmap", "nerfstudio", "comfyui"]
    })
}

fn minimal_report_value() -> serde_json::Value {
    let scene = scene(0, 2);
    let text = owned_text_segment(0, "Rust video analysis test fixture.");
    let dataset = dataset_with_scene_text_and_feature();
    serde_json::json!({
        "scene": {
            "startFrame": scene.start.frame_index,
            "endFrame": scene.end.frame_index
        },
        "textSegment": {
            "segmentIndex": text.segment_index,
            "textLength": text.text.chars().count(),
            "language": text.language
        },
        "dataset": {
            "records": dataset.records.len(),
            "observations": dataset.observations().count(),
            "events": dataset.events().count()
        },
        "producesComplexVideoOr3dPayloads": false
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn package_surface_lists_test_support_operations() {
        let ids = package_surface()
            .operations
            .into_iter()
            .map(|operation| operation.id.0)
            .collect::<Vec<_>>();
        assert!(ids.contains(&"testSupport.catalog".to_string()));
        assert!(ids.contains(&"testSupport.minimalReport".to_string()));
    }

    #[test]
    fn minimal_report_returns_small_fixture_summary() {
        let response = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("testSupport.minimalReport"),
            input: serde_json::json!({}),
        })
        .expect("minimal report");
        assert_eq!(response.value["scene"]["startFrame"], 0);
        assert_eq!(response.value["producesComplexVideoOr3dPayloads"], false);
    }

    #[test]
    fn unsupported_operation_errors() {
        let error = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("testSupport.missing"),
            input: serde_json::json!({}),
        })
        .expect_err("unsupported");
        assert!(error.contains("unsupported operation"));
    }
}
