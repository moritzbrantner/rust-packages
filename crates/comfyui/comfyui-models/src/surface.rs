//! Library-owned runtime surface for `comfyui-models`.

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::de::DeserializeOwned;
use serde::Deserialize;
use video_analysis_core::runtime::{
    OperationId, PackageSurface, RuntimeCapabilities, SurfaceOperation, SurfaceRequest,
    SurfaceResponse,
};

use crate::{
    ComfyModelKind, ComfyModelRef, ComfyModelRole, ExtraModelPathSection, ExtraModelPathsConfig,
};

/// Returns the package surface exposed by every transport wrapper.
pub fn package_surface() -> PackageSurface {
    PackageSurface {
        library: env!("CARGO_PKG_NAME").to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        capabilities: RuntimeCapabilities::pure_rust(),
        operations: vec![
            operation("describe", "Describe package", "ComfyUI model folder, inventory, and extra path contracts.", serde_json::json!({"includeOperations": true})),
            operation("comfy.models.defaults", "Default model folders", "Lists ComfyUI core model folder keys and default relative paths.", serde_json::json!({})),
            operation("comfy.models.extraPathsPlan", "Extra paths plan", "Builds an extra_model_paths YAML plan without writing files.", serde_json::json!({"name": "video-analysis", "root": "/models", "folders": {"checkpoints": ["models/checkpoints"]}})),
            operation("comfy.models.reference", "Model reference", "Builds a typed ComfyUI model reference from role and name.", serde_json::json!({"role": "checkpoint", "name": "model.safetensors"})),
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
        "comfy.models.defaults" => defaults_value(),
        "comfy.models.extraPathsPlan" => extra_paths_plan_value(parse_input(request.input)?)?,
        "comfy.models.reference" => reference_value(parse_input(request.input)?)?,
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
struct ExtraPathsPlanRequest {
    name: String,
    root: PathBuf,
    #[serde(default)]
    folders: BTreeMap<String, Vec<PathBuf>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ReferenceRequest {
    role: ComfyModelRole,
    name: String,
    relative_path: Option<PathBuf>,
    full_path: Option<PathBuf>,
}

fn defaults_value() -> serde_json::Value {
    serde_json::json!({
        "folders": ComfyModelKind::CORE
            .iter()
            .map(|kind| serde_json::json!({
                "key": kind.key(),
                "defaultRelativePaths": kind.default_relative_paths()
            }))
            .collect::<Vec<_>>()
    })
}

fn extra_paths_plan_value(request: ExtraPathsPlanRequest) -> Result<serde_json::Value, String> {
    let mut section = ExtraModelPathSection::new(request.root).default_first(true);
    for (key, paths) in request.folders {
        let kind = ComfyModelKind::from_key(&key).map_err(|error| error.to_string())?;
        for path in paths {
            section.add_path(kind.clone(), path);
        }
    }
    let config = ExtraModelPathsConfig::new().insert_section(request.name, section);
    Ok(serde_json::json!({ "yaml": config.to_yaml_string() }))
}

fn reference_value(request: ReferenceRequest) -> Result<serde_json::Value, String> {
    let mut model_ref = ComfyModelRef::new(request.role, request.name);
    model_ref.relative_path = request.relative_path;
    model_ref.full_path = request.full_path;
    serde_json::to_value(model_ref).map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_include_checkpoint_and_vae() {
        let response = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("comfy.models.defaults"),
            input: serde_json::json!({}),
        })
        .expect("defaults");
        let text = response.value.to_string();
        assert!(text.contains("checkpoints"));
        assert!(text.contains("vae"));
    }

    #[test]
    fn extra_paths_plan_is_deterministic() {
        let response = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("comfy.models.extraPathsPlan"),
            input: serde_json::json!({
                "name": "shared",
                "root": "/models",
                "folders": {"checkpoints": ["models/checkpoints"]}
            }),
        })
        .expect("plan");
        assert!(response.value["yaml"].as_str().unwrap().contains("shared:"));
        assert!(response.value["yaml"].as_str().unwrap().contains("checkpoints"));
    }

    #[test]
    fn unknown_role_errors() {
        let error = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("comfy.models.reference"),
            input: serde_json::json!({"role": "missing", "name": "model.safetensors"}),
        })
        .expect_err("unknown role");
        assert!(error.contains("invalid request"));
    }
}
