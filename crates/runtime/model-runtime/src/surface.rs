//! Library-owned runtime surface for `model-runtime`.

use runtime_core::{
    OperationId, PackageSurface, RuntimeCapabilities, SurfaceOperation, SurfaceRequest,
    SurfaceResponse,
};
use serde::Deserialize;

use crate::{ModelFileRequest, ModelPreset, ModelSource, ModelSpec};

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
                "Generic model specs, bundle plans, presets, and job helpers for multimodal runtimes.",
                serde_json::json!({"includeOperations": true}),
            ),
            operation(
                "model.presets",
                "Model presets",
                "Lists model preset ids and derived model spec summaries without downloading models.",
                serde_json::json!({}),
            ),
            operation(
                "model.spec",
                "Model spec",
                "Validates a ModelSpec-shaped input and returns safe name, task, source, files, and revision.",
                serde_json::json!({"name": "demo-model", "task": "text_embedding", "source": {"kind": "hugging_face", "repo_id": "demo/model", "revision": "main"}, "files": [{"required": "config.json"}]}),
            ),
            operation(
                "model.bundlePlan",
                "Model bundle plan",
                "Plans manifest layout and artifact references for a model spec and optional local file list without network or materialization.",
                serde_json::json!({"spec": {"name": "demo-model", "task": "text_embedding", "source": {"kind": "hugging_face", "repo_id": "demo/model", "revision": "main"}, "files": [{"required": "config.json"}]}, "localFiles": ["config.json"]}),
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
        "model.presets" => presets_value(),
        "model.spec" => spec_value(parse_input(request.input)?)?,
        "model.bundlePlan" => bundle_plan_value(parse_input(request.input)?)?,
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

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BundlePlanRequest {
    spec: ModelSpec,
    #[serde(default)]
    local_files: Vec<String>,
}

fn presets_value() -> serde_json::Value {
    serde_json::json!({
        "presets": ModelPreset::ALL.iter().map(|preset| {
            let spec = preset.spec();
            serde_json::json!({
                "id": preset.as_str(),
                "name": spec.name,
                "safeName": spec.safe_name(),
                "task": spec.task.as_protocol_str(),
                "source": source_json(&spec.source),
                "revision": spec.revision_value(),
                "requestedFiles": file_requests_json(&spec.files)
            })
        }).collect::<Vec<_>>()
    })
}

fn spec_value(spec: ModelSpec) -> Result<serde_json::Value, String> {
    validate_spec(&spec)?;
    Ok(spec_summary_json(&spec))
}

fn bundle_plan_value(request: BundlePlanRequest) -> Result<serde_json::Value, String> {
    validate_spec(&request.spec)?;
    let safe_name = request.spec.safe_name();
    let revision = request.spec.revision_value().unwrap_or("main");
    let expected_files = resolve_requested_files(&request.spec.files, &request.local_files);
    let bundle_root = format!("{safe_name}/{revision}");
    let manifest_files = expected_files
        .iter()
        .map(|remote_path| {
            serde_json::json!({
                "remotePath": remote_path,
                "localPath": format!("files/{remote_path}"),
                "presentLocally": request.local_files.iter().any(|path| path == remote_path)
            })
        })
        .collect::<Vec<_>>();
    Ok(serde_json::json!({
        "spec": spec_summary_json(&request.spec),
        "manifestPath": format!("{bundle_root}/manifest.json"),
        "filesDirectory": format!("{bundle_root}/files"),
        "manifest": {
            "schemaVersion": 1,
            "name": request.spec.name,
            "repoId": request.spec.repo_id_value(),
            "revision": request.spec.revision_value(),
            "task": request.spec.task.as_protocol_str(),
            "files": manifest_files
        },
        "artifactRefs": expected_files.iter().map(|remote_path| serde_json::json!({
            "id": format!("model:{}", remote_path.replace(['/', '\\'], "_")),
            "kind": model_file_kind(remote_path),
            "mediaType": model_file_media_type(remote_path),
            "uri": format!("{bundle_root}/files/{remote_path}")
        })).collect::<Vec<_>>(),
        "downloadsRequired": false
    }))
}

fn spec_summary_json(spec: &ModelSpec) -> serde_json::Value {
    serde_json::json!({
        "name": spec.name,
        "safeName": spec.safe_name(),
        "task": spec.task.as_protocol_str(),
        "source": source_json(&spec.source),
        "requestedFiles": file_requests_json(&spec.files),
        "revision": spec.revision_value(),
        "repoId": spec.repo_id_value(),
        "metadata": spec.metadata
    })
}

fn validate_spec(spec: &ModelSpec) -> Result<(), String> {
    if spec.name.trim().is_empty() {
        return Err("model name must not be empty".to_string());
    }
    Ok(())
}

fn source_json(source: &ModelSource) -> serde_json::Value {
    match source {
        ModelSource::HuggingFace { repo_id, revision } => serde_json::json!({
            "kind": source.kind(),
            "repoId": repo_id,
            "revision": revision
        }),
        ModelSource::LocalPath { path } => serde_json::json!({
            "kind": source.kind(),
            "path": path
        }),
        ModelSource::ExternalCommand { command } => serde_json::json!({
            "kind": source.kind(),
            "command": command
        }),
        ModelSource::ComfyUiInventory { root } => serde_json::json!({
            "kind": source.kind(),
            "root": root
        }),
        ModelSource::Custom(kind) => serde_json::json!({
            "kind": kind
        }),
    }
}

fn file_requests_json(files: &[ModelFileRequest]) -> Vec<serde_json::Value> {
    files
        .iter()
        .map(|request| match request {
            ModelFileRequest::Required(path) => {
                serde_json::json!({"kind": "required", "path": path})
            }
            ModelFileRequest::Optional(path) => {
                serde_json::json!({"kind": "optional", "path": path})
            }
            ModelFileRequest::FirstAvailable(paths) => {
                serde_json::json!({"kind": "firstAvailable", "paths": paths})
            }
        })
        .collect()
}

fn resolve_requested_files(files: &[ModelFileRequest], local_files: &[String]) -> Vec<String> {
    files
        .iter()
        .filter_map(|request| match request {
            ModelFileRequest::Required(path) | ModelFileRequest::Optional(path) => {
                Some(path.clone())
            }
            ModelFileRequest::FirstAvailable(paths) => paths
                .iter()
                .find(|path| local_files.iter().any(|local| local == *path))
                .cloned()
                .or_else(|| paths.first().cloned()),
        })
        .collect()
}

fn model_file_kind(remote_path: &str) -> &'static str {
    if remote_path.ends_with(".json") {
        "json"
    } else if remote_path.ends_with(".txt") || remote_path.ends_with(".md") {
        "text"
    } else {
        "binary"
    }
}

fn model_file_media_type(remote_path: &str) -> &'static str {
    if remote_path.ends_with(".json") {
        "application/json"
    } else if remote_path.ends_with(".txt") {
        "text/plain"
    } else {
        "application/octet-stream"
    }
}

fn parse_input<T: for<'de> Deserialize<'de>>(input: serde_json::Value) -> Result<T, String> {
    serde_json::from_value(input).map_err(|error| format!("invalid request: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn package_surface_lists_model_operations() {
        let ids = package_surface()
            .operations
            .into_iter()
            .map(|operation| operation.id.0)
            .collect::<Vec<_>>();
        assert!(ids.contains(&"model.presets".to_string()));
        assert!(ids.contains(&"model.spec".to_string()));
        assert!(ids.contains(&"model.bundlePlan".to_string()));
    }

    #[test]
    fn spec_operation_returns_safe_name() {
        let response = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("model.spec"),
            input: serde_json::json!({
                "name": "demo/model",
                "task": "text_embedding",
                "source": {"kind": "hugging_face", "repo_id": "demo/model", "revision": "main"},
                "files": [{"required": "config.json"}]
            }),
        })
        .expect("spec");
        assert_eq!(response.value["safeName"], "demo_model");
    }

    #[test]
    fn bundle_plan_selects_local_first_available_file() {
        let response = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("model.bundlePlan"),
            input: serde_json::json!({
                "spec": {
                    "name": "demo",
                    "task": "text_embedding",
                    "source": {"kind": "hugging_face", "repo_id": "demo/model", "revision": "main"},
                    "files": [{"first_available": ["model.safetensors", "pytorch_model.bin"]}]
                },
                "localFiles": ["pytorch_model.bin"]
            }),
        })
        .expect("bundle plan");
        assert_eq!(
            response.value["manifest"]["files"][0]["remotePath"],
            "pytorch_model.bin"
        );
        assert_eq!(response.value["downloadsRequired"], false);
    }

    #[test]
    fn spec_operation_rejects_empty_name() {
        let error = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("model.spec"),
            input: serde_json::json!({
                "name": "",
                "task": "text_embedding",
                "source": {"kind": "hugging_face", "repo_id": "demo/model", "revision": "main"},
                "files": []
            }),
        })
        .expect_err("empty name");
        assert!(error.contains("model name"));
    }
}
