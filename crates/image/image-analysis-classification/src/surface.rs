//! Library-owned runtime surface for `image-analysis-classification`.

use std::collections::BTreeMap;

use runtime_core::{
    describe_surface_response, structured_operation_response, OperationId, PackageSurface,
    RuntimeCapabilities, SurfaceOperation, SurfaceRequest, SurfaceResponse,
};
use serde::Deserialize;

use crate::{image_classification_catalog, parse_task, schema_summary, ImageClassification};

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
                "Image classification catalogs, schemas, and imported prediction validation.",
                serde_json::json!({"includeOperations": true}),
            ),
            operation(
                "image.classification.models",
                "Classification models",
                "Lists deterministic image classification catalog entries without running inference.",
                serde_json::json!({"task": "classify"}),
            ),
            operation(
                "image.classification.schema",
                "Classification schema",
                "Returns image classification task and preset schema metadata.",
                serde_json::json!({}),
            ),
            operation(
                "image.classification.imported",
                "Imported classifications",
                "Validates caller-supplied labels and scores into normalized classification values.",
                serde_json::json!({"labels": [{"label": "landscape", "score": 0.92, "attributes": {"source": "fixture"}}]}),
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
    let value = match request.operation.as_str() {
        "describe" => return Ok(describe_surface_response(&surface, request)),
        "image.classification.models" => models_value(parse_input(request.input)?)?,
        "image.classification.schema" => schema_summary(),
        "image.classification.imported" => imported_value(parse_input(request.input)?)?,
        operation => {
            return Err(format!(
                "unsupported operation `{operation}` for {}",
                env!("CARGO_PKG_NAME")
            ));
        }
    };
    Ok(structured_operation_response(&surface, operation, value))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ModelsRequest {
    task: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ImportedRequest {
    #[serde(default, alias = "items", alias = "classifications")]
    labels: Vec<ImportedClassification>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ImportedClassification {
    label: String,
    score: Option<f32>,
    #[serde(default)]
    attributes: BTreeMap<String, String>,
}

fn models_value(request: ModelsRequest) -> Result<serde_json::Value, String> {
    let task = request
        .task
        .as_deref()
        .map(|task| {
            parse_task(task).ok_or_else(|| format!("unsupported classification task `{task}`"))
        })
        .transpose()?;
    Ok(serde_json::json!({
        "models": image_classification_catalog(task)
    }))
}

fn imported_value(request: ImportedRequest) -> Result<serde_json::Value, String> {
    let classifications = request
        .labels
        .into_iter()
        .map(|item| {
            if item.label.trim().is_empty() {
                return Err("classification label must not be empty".to_string());
            }
            if let Some(score) = item.score {
                if !score.is_finite() {
                    return Err("classification score must be finite".to_string());
                }
            }
            let mut classification = ImageClassification::new(item.label.trim().to_string());
            if let Some(score) = item.score {
                classification = classification.score(score);
            }
            for (key, value) in item.attributes {
                classification = classification.attribute(key, value);
            }
            Ok(serde_json::json!({
                "label": classification.label,
                "score": classification.score,
                "attributes": classification.attributes
            }))
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(serde_json::json!({
        "count": classifications.len(),
        "classifications": classifications
    }))
}

fn parse_input<T: for<'de> Deserialize<'de>>(input: serde_json::Value) -> Result<T, String> {
    serde_json::from_value(input).map_err(|error| format!("invalid request: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn package_surface_lists_classification_operations() {
        let ids = package_surface()
            .operations
            .into_iter()
            .map(|operation| operation.id.0)
            .collect::<Vec<_>>();
        assert!(ids.contains(&"describe".to_string()));
        assert!(ids.contains(&"image.classification.models".to_string()));
        assert!(ids.contains(&"image.classification.imported".to_string()));
    }

    #[test]
    fn imported_operation_normalizes_labels() {
        let response = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("image.classification.imported"),
            input: serde_json::json!({"labels": [{"label": " landscape ", "score": 0.9}]}),
        })
        .expect("imported");
        assert_eq!(response.value["count"], 1);
        assert_eq!(response.value["classifications"][0]["label"], "landscape");
    }

    #[test]
    fn imported_operation_rejects_empty_label() {
        let error = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("image.classification.imported"),
            input: serde_json::json!({"labels": [{"label": ""}]}),
        })
        .expect_err("empty label");
        assert!(error.contains("label"));
    }
}
