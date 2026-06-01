//! Library-owned runtime surface for `image-analysis-comfyui`.

use runtime_core::{
    describe_surface_response, structured_operation_response, OperationId, PackageSurface,
    RuntimeCapabilities, SurfaceOperation, SurfaceRequest, SurfaceResponse,
};

use crate::{
    build_generation_workflow, ComfyWorkflowPreset, ImageGenerationMode, ImageGenerationRequest,
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
                "ComfyUI workflow builders for image generation and manipulation in video-analysis.",
                serde_json::json!({
                    "includeOperations": true
                }),
            ),
            operation(
                "image.comfyui.workflowSummary",
                "Summarize workflow",
                "Summarizes ComfyUI workflow nodes, image inputs, image outputs, and editable parameters.",
                serde_json::json!({
                    "input": {},
                    "mode": "deterministic"
                }),
            ),
            operation(
                "image.comfyui.promptPlan",
                "Plan prompt graph",
                "Builds a deterministic prompt graph plan for image generation and editing workflows.",
                serde_json::json!({
                    "input": {},
                    "mode": "deterministic"
                }),
            ),
            operation(
                "image.comfyui.assetMap",
                "Map workflow assets",
                "Maps image, mask, latent, model, and output assets referenced by a ComfyUI workflow.",
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
        return Ok(describe_surface_response(&surface, request));
    } else {
        deterministic_operation_value(surface_operation, request.input)?
    };

    Ok(structured_operation_response(&surface, operation, value))
}

fn deterministic_operation_value(
    operation: &SurfaceOperation,
    input: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let operation_id = operation.id.as_str();
    let workflow = workflow_from_input(&input)?;
    let node_types = workflow
        .nodes
        .iter()
        .map(|node| node.node_type.as_str())
        .collect::<Vec<_>>();
    let inventory = workflow.observed_socket_types();
    let value = match operation_id {
        "image.comfyui.workflowSummary" => serde_json::json!({
            "nodeCount": workflow.nodes.len(),
            "linkCount": workflow.links.len(),
            "nodeTypes": node_types,
            "socketTypes": {
                "inputs": inventory.inputs,
                "outputs": inventory.outputs,
                "links": inventory.links
            },
            "request": input
        }),
        "image.comfyui.promptPlan" => serde_json::json!({
            "deterministic": true,
            "externalToolsRequired": false,
            "nodeCount": workflow.nodes.len(),
            "linkCount": workflow.links.len(),
            "workflow": workflow,
            "request": input
        }),
        "image.comfyui.assetMap" => serde_json::json!({
            "assets": workflow_assets(&workflow),
            "nodeCount": workflow.nodes.len(),
            "linkCount": workflow.links.len(),
            "request": input
        }),
        _ => serde_json::json!({
            "deterministic": true,
            "externalToolsRequired": false,
            "request": input,
            "operationFamily": operation_family(operation_id)
        }),
    };
    Ok(value)
}

fn operation_family(operation_id: &str) -> &str {
    operation_id
        .split_once('.')
        .map(|(family, _)| family)
        .unwrap_or(operation_id)
}

fn workflow_from_input(input: &serde_json::Value) -> Result<comfyui_data::ComfyWorkflow, String> {
    let request = generation_request_from_input(input)?;
    build_generation_workflow(&request).map_err(|error| error.to_string())
}

fn generation_request_from_input(
    input: &serde_json::Value,
) -> Result<ImageGenerationRequest, String> {
    let source = input.get("input").unwrap_or(input);
    let prompt = source
        .get("prompt")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("a clean product photograph of a red cube");
    let mut request = ImageGenerationRequest::new(prompt);
    if let Some(mode) = source.get("mode").and_then(serde_json::Value::as_str) {
        request = request.mode(parse_mode(mode)?);
    }
    if let Some(preset) = source.get("preset").and_then(serde_json::Value::as_str) {
        request = request.preset(parse_preset(preset)?);
    }
    if let Some(width) = source.get("width").and_then(serde_json::Value::as_u64) {
        let height = request.height;
        request = request.size(width as u32, height);
    }
    if let Some(height) = source.get("height").and_then(serde_json::Value::as_u64) {
        let width = request.width;
        request = request.size(width, height as u32);
    }
    if let Some(seed) = source.get("seed").and_then(serde_json::Value::as_u64) {
        request = request.seed(seed);
    }
    if let Some(checkpoint) = source.get("checkpoint").and_then(serde_json::Value::as_str) {
        request = request.checkpoint(checkpoint);
    }
    if let Some(input_image) = source.get("inputImage").and_then(serde_json::Value::as_str) {
        request = request.input_image(input_image);
    }
    if let Some(mask_image) = source.get("maskImage").and_then(serde_json::Value::as_str) {
        request = request.mask_image(mask_image);
    }
    Ok(request)
}

fn parse_mode(value: &str) -> Result<ImageGenerationMode, String> {
    match value {
        "textToImage" | "text-to-image" | "txt2img" => Ok(ImageGenerationMode::TextToImage),
        "imageToImage" | "image-to-image" | "img2img" => Ok(ImageGenerationMode::ImageToImage),
        "inpaint" => Ok(ImageGenerationMode::Inpaint),
        "upscale" => Ok(ImageGenerationMode::Upscale),
        other => Err(format!(
            "unsupported ComfyUI image generation mode `{other}`"
        )),
    }
}

fn parse_preset(value: &str) -> Result<ComfyWorkflowPreset, String> {
    match value {
        "standardStableDiffusion" | "standard-stable-diffusion" | "sd" => {
            Ok(ComfyWorkflowPreset::StandardStableDiffusion)
        }
        "fluxInpaint" | "flux-inpaint" => Ok(ComfyWorkflowPreset::FluxInpaint),
        other => Err(format!("unsupported ComfyUI workflow preset `{other}`")),
    }
}

fn workflow_assets(workflow: &comfyui_data::ComfyWorkflow) -> serde_json::Value {
    let mut models = Vec::new();
    let mut images = Vec::new();
    let mut outputs = Vec::new();
    for node in &workflow.nodes {
        for value in &node.widgets_values {
            let Some(text) = value.as_str() else {
                continue;
            };
            match node.node_type.as_str() {
                "CheckpointLoaderSimple" | "UpscaleModelLoader" => models.push(text.to_string()),
                "LoadImage" | "LoadImageMask" => images.push(text.to_string()),
                "SaveImage" => outputs.push(text.to_string()),
                _ => {}
            }
        }
    }
    serde_json::json!({
        "models": models,
        "images": images,
        "outputs": outputs
    })
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
        let response = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("image.comfyui.promptPlan"),
            input: serde_json::json!({"sample": true}),
        })
        .expect("package operation");

        assert_eq!(response.value["deterministic"], true);
        assert_eq!(response.value["externalToolsRequired"], false);
        assert!(
            response.value["workflow"]["nodes"]
                .as_array()
                .unwrap()
                .len()
                > 1
        );
    }
}
