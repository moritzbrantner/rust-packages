//! Library-owned runtime surface for `image-analysis-embeddings`.

use std::collections::BTreeMap;

use serde::Deserialize;
use video_analysis_core::runtime::{
    OperationId, PackageSurface, RuntimeCapabilities, SurfaceOperation, SurfaceRequest,
    SurfaceResponse,
};
use video_analysis_core::BoundingBox;

use crate::{image_embedding_catalog, parse_task, schema_summary, FaceEmbedding, ImageEmbedding};

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
                "Image and face embedding catalogs, schemas, and imported vector validation.",
                serde_json::json!({"includeOperations": true}),
            ),
            operation(
                "image.embeddings.models",
                "Embedding models",
                "Lists deterministic image and face embedding catalog entries without running embedding models.",
                serde_json::json!({"task": "embed"}),
            ),
            operation(
                "image.embeddings.schema",
                "Embedding schema",
                "Returns image and face embedding task and preset schema metadata.",
                serde_json::json!({}),
            ),
            operation(
                "image.embeddings.validate",
                "Validate embeddings",
                "Validates imported image or face embedding vectors and optional face regions.",
                serde_json::json!({"kind": "face", "vector": [0.1, 0.2, 0.3], "region": {"x": 1, "y": 2, "width": 3, "height": 4}}),
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
        "image.embeddings.models" => models_value(parse_input(request.input)?)?,
        "image.embeddings.schema" => schema_summary(),
        "image.embeddings.validate" => validate_value(parse_input(request.input)?)?,
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
struct ModelsRequest {
    task: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ValidateRequest {
    #[serde(default = "default_kind")]
    kind: String,
    vector: Vec<f32>,
    #[serde(default)]
    region: Option<BoxRequest>,
    #[serde(default)]
    attributes: BTreeMap<String, String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BoxRequest {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
}

fn models_value(request: ModelsRequest) -> Result<serde_json::Value, String> {
    let task = request
        .task
        .as_deref()
        .map(|task| parse_task(task).ok_or_else(|| format!("unsupported embedding task `{task}`")))
        .transpose()?;
    Ok(serde_json::json!({
        "models": image_embedding_catalog(task)
    }))
}

fn validate_value(request: ValidateRequest) -> Result<serde_json::Value, String> {
    let finite = request.vector.iter().all(|value| value.is_finite());
    match request.kind.as_str() {
        "image" | "image_embedding" | "embed" => {
            let mut embedding =
                ImageEmbedding::new(request.vector).map_err(|error| error.to_string())?;
            for (key, value) in request.attributes {
                embedding = embedding.attribute(key, value);
            }
            Ok(serde_json::json!({
                "kind": "image",
                "dimensions": embedding.vector.len(),
                "finite": finite,
                "region": null,
                "attributes": embedding.attributes
            }))
        }
        "face" | "face_embedding" | "face-embed" => {
            let mut embedding =
                FaceEmbedding::new(request.vector).map_err(|error| error.to_string())?;
            if let Some(region) = request.region {
                embedding = embedding.region(
                    BoundingBox::new(region.x, region.y, region.width, region.height)
                        .map_err(|error| error.to_string())?,
                );
            }
            for (key, value) in request.attributes {
                embedding = embedding.attribute(key, value);
            }
            Ok(serde_json::json!({
                "kind": "face",
                "dimensions": embedding.vector.len(),
                "finite": finite,
                "region": embedding.region.map(|region| serde_json::json!({
                    "x": region.x,
                    "y": region.y,
                    "width": region.width,
                    "height": region.height
                })),
                "attributes": embedding.attributes
            }))
        }
        other => Err(format!("unsupported embedding kind `{other}`")),
    }
}

fn parse_input<T: for<'de> Deserialize<'de>>(input: serde_json::Value) -> Result<T, String> {
    serde_json::from_value(input).map_err(|error| format!("invalid request: {error}"))
}

fn default_kind() -> String {
    "image".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn package_surface_lists_embedding_operations() {
        let ids = package_surface()
            .operations
            .into_iter()
            .map(|operation| operation.id.0)
            .collect::<Vec<_>>();
        assert!(ids.contains(&"image.embeddings.models".to_string()));
        assert!(ids.contains(&"image.embeddings.validate".to_string()));
    }

    #[test]
    fn validates_face_embedding_region() {
        let response = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("image.embeddings.validate"),
            input: serde_json::json!({
                "kind": "face",
                "vector": [0.1, 0.2],
                "region": {"x": 1, "y": 2, "width": 3, "height": 4}
            }),
        })
        .expect("validate");
        assert_eq!(response.value["dimensions"], 2);
        assert_eq!(response.value["region"]["width"], 3);
    }

    #[test]
    fn rejects_empty_embedding_values() {
        let error = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("image.embeddings.validate"),
            input: serde_json::json!({"vector": []}),
        })
        .expect_err("invalid vector");
        assert!(error.contains("must not be empty"));
    }
}
