//! Library-owned runtime surface for `comfyui-data`.

use serde::de::DeserializeOwned;
use serde::Deserialize;
use video_analysis_core::runtime::{
    OperationId, PackageSurface, RuntimeCapabilities, SurfaceOperation, SurfaceRequest,
    SurfaceResponse,
};

use crate::{parse_prompt_link, ComfyWorkflow, PromptGraph, WorkflowTypeInventory};

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
                "Serde contracts for ComfyUI workflow and prompt data.",
                serde_json::json!({"includeOperations": true}),
            ),
            operation(
                "comfy.workflow.validate",
                "Validate workflow",
                "Validates ComfyUI workflow node and link references.",
                serde_json::json!({"workflow": {"nodes": [], "links": []}}),
            ),
            operation(
                "comfy.workflow.inventory",
                "Workflow inventory",
                "Returns normalized workflow socket inventory by input, output, and link position.",
                serde_json::json!({"workflow": {"nodes": [], "links": []}}),
            ),
            operation(
                "comfy.prompt.links",
                "Prompt links",
                "Lists parsed ComfyUI prompt graph input links and invalid link-shaped values.",
                serde_json::json!({"prompt": {}}),
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
        "comfy.workflow.validate" => validate_workflow_value(parse_input(request.input)?)?,
        "comfy.workflow.inventory" => inventory_value(parse_input(request.input)?)?,
        "comfy.prompt.links" => prompt_links_value(parse_input(request.input)?)?,
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
struct WorkflowRequest {
    workflow: ComfyWorkflow,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PromptRequest {
    prompt: PromptGraph,
}

fn validate_workflow_value(request: WorkflowRequest) -> Result<serde_json::Value, String> {
    request
        .workflow
        .validate()
        .map_err(|error| error.to_string())?;
    Ok(serde_json::json!({
        "valid": true,
        "nodeCount": request.workflow.nodes.len(),
        "linkCount": request.workflow.links.len()
    }))
}

fn inventory_value(request: WorkflowRequest) -> Result<serde_json::Value, String> {
    request
        .workflow
        .validate()
        .map_err(|error| error.to_string())?;
    Ok(inventory_json(&request.workflow.observed_socket_types()))
}

fn inventory_json(inventory: &WorkflowTypeInventory) -> serde_json::Value {
    serde_json::json!({
        "inputs": inventory.inputs,
        "outputs": inventory.outputs,
        "links": inventory.links,
        "all": inventory.all()
    })
}

fn prompt_links_value(request: PromptRequest) -> Result<serde_json::Value, String> {
    let mut links = Vec::new();
    let mut invalid_link_count = 0_u64;
    for (node_id, node) in request.prompt {
        for (input_name, value) in node.inputs {
            if let Some(link) = parse_prompt_link(&value) {
                links.push(serde_json::json!({
                    "nodeId": node_id,
                    "input": input_name,
                    "sourceNodeId": link.node_id,
                    "outputIndex": link.output_index
                }));
            } else if value.as_array().is_some() {
                invalid_link_count += 1;
            }
        }
    }
    Ok(serde_json::json!({
        "links": links,
        "linkCount": links.len(),
        "invalidLinkCount": invalid_link_count
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_workflow_passes_validation() {
        let response = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("comfy.workflow.validate"),
            input: serde_json::json!({"workflow": {"nodes": [], "links": []}}),
        })
        .expect("validate");
        assert_eq!(response.value["valid"], true);
    }

    #[test]
    fn missing_link_node_is_reported() {
        let error = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("comfy.workflow.validate"),
            input: serde_json::json!({
                "workflow": {
                    "nodes": [{"id": 1, "type": "KSampler"}],
                    "links": [[1, 1, 0, 2, 0, "IMAGE"]]
                }
            }),
        })
        .expect_err("missing node");
        assert!(error.contains("missing") || error.contains("node"));
    }

    #[test]
    fn inventory_includes_socket_rows() {
        let response = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("comfy.workflow.inventory"),
            input: serde_json::json!({
                "workflow": {
                    "nodes": [{
                        "id": 1,
                        "type": "PreviewImage",
                        "inputs": [{"name": "image", "type": "IMAGE"}],
                        "outputs": [{"name": "mask", "type": "MASK"}]
                    }],
                    "links": []
                }
            }),
        })
        .expect("inventory");
        assert!(response.value["inputs"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!("image")));
        assert!(response.value["outputs"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!("mask")));
    }

    #[test]
    fn prompt_links_are_parsed() {
        let response = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("comfy.prompt.links"),
            input: serde_json::json!({
                "prompt": {
                    "2": {"class_type": "PreviewImage", "inputs": {"image": ["1", 0], "bad": [true]}}
                }
            }),
        })
        .expect("links");
        assert_eq!(response.value["linkCount"], 1);
        assert_eq!(response.value["invalidLinkCount"], 1);
    }
}
