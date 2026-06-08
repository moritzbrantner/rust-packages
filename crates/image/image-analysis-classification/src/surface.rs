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
            operation(
                "image.classification.topLabels",
                "Top labels",
                "Ranks normalized imported classification labels by score.",
                serde_json::json!({"labels": [{"label": "cat", "score": 0.91}, {"label": "dog", "score": 0.42}], "limit": 2}),
            ),
            operation(
                "image.classification.thresholdLabels",
                "Threshold labels",
                "Splits normalized imported classification labels by a minimum score threshold.",
                serde_json::json!({"labels": [{"label": "cat", "score": 0.91}, {"label": "dog", "score": 0.42}], "minScore": 0.5}),
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
        "image.classification.topLabels" => top_labels_value(parse_input(request.input)?)?,
        "image.classification.thresholdLabels" => {
            threshold_labels_value(parse_input(request.input)?)?
        }
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
    limit: Option<usize>,
    min_score: Option<f32>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ImportedClassification {
    label: String,
    score: Option<f32>,
    #[serde(default)]
    attributes: BTreeMap<String, String>,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct NormalizedClassification {
    label: String,
    score: Option<f32>,
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
    let classifications = normalize_classifications(request.labels)?;
    Ok(serde_json::json!({
        "count": classifications.len(),
        "classifications": classifications
    }))
}

fn top_labels_value(request: ImportedRequest) -> Result<serde_json::Value, String> {
    let mut classifications = normalize_classifications(request.labels)?;
    if let Some(limit) = request.limit {
        if limit == 0 {
            return Err("classification label limit must be greater than zero".to_string());
        }
    }
    classifications.sort_by(|left, right| compare_scores_desc(left.score, right.score));
    if let Some(limit) = request.limit {
        classifications.truncate(limit);
    }
    Ok(serde_json::json!({
        "count": classifications.len(),
        "topLabel": classifications.first(),
        "classifications": classifications
    }))
}

fn threshold_labels_value(request: ImportedRequest) -> Result<serde_json::Value, String> {
    let threshold = request.min_score.unwrap_or(0.5);
    if !threshold.is_finite() {
        return Err("classification minScore must be finite".to_string());
    }
    if let Some(limit) = request.limit {
        if limit == 0 {
            return Err("classification label limit must be greater than zero".to_string());
        }
    }
    let mut classifications = normalize_classifications(request.labels)?;
    classifications.sort_by(|left, right| compare_scores_desc(left.score, right.score));
    let mut accepted = classifications
        .iter()
        .filter(|classification| classification.score.is_some_and(|score| score >= threshold))
        .cloned()
        .collect::<Vec<_>>();
    if let Some(limit) = request.limit {
        accepted.truncate(limit);
    }
    let rejected = classifications
        .into_iter()
        .filter(|classification| !classification.score.is_some_and(|score| score >= threshold))
        .collect::<Vec<_>>();
    Ok(serde_json::json!({
        "threshold": threshold,
        "count": accepted.len(),
        "rejectedCount": rejected.len(),
        "topLabel": accepted.first(),
        "accepted": accepted,
        "rejected": rejected
    }))
}

fn normalize_classifications(
    labels: Vec<ImportedClassification>,
) -> Result<Vec<NormalizedClassification>, String> {
    labels
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
            Ok(NormalizedClassification {
                label: classification.label,
                score: classification.score,
                attributes: classification.attributes,
            })
        })
        .collect()
}

fn compare_scores_desc(left: Option<f32>, right: Option<f32>) -> std::cmp::Ordering {
    match (left, right) {
        (Some(left), Some(right)) => right.total_cmp(&left),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => std::cmp::Ordering::Equal,
    }
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
        assert!(ids.contains(&"image.classification.topLabels".to_string()));
        assert!(ids.contains(&"image.classification.thresholdLabels".to_string()));
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

    #[test]
    fn top_labels_operation_returns_highest_score() {
        let response = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("image.classification.topLabels"),
            input: serde_json::json!({"labels": [
                {"label": "low", "score": 0.1},
                {"label": "high", "score": 0.9}
            ]}),
        })
        .expect("top labels");
        assert_eq!(response.value["topLabel"]["label"], "high");
    }
}
