//! Library-owned runtime surface for `image-analysis-captioning`.

use std::collections::BTreeMap;

use runtime_core::{
    describe_surface_response, structured_operation_response, OperationId, PackageSurface,
    RuntimeCapabilities, SurfaceOperation, SurfaceRequest, SurfaceResponse,
};
use serde::Deserialize;

use crate::{image_captioning_catalog, parse_task, schema_summary, ImageCaption};

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
                "Image captioning catalogs, schemas, and imported caption validation.",
                serde_json::json!({"includeOperations": true}),
            ),
            operation(
                "image.captioning.models",
                "Captioning models",
                "Lists deterministic image captioning catalog entries without running captioning.",
                serde_json::json!({"task": "caption"}),
            ),
            operation(
                "image.captioning.schema",
                "Captioning schema",
                "Returns image captioning task and preset schema metadata.",
                serde_json::json!({}),
            ),
            operation(
                "image.captioning.imported",
                "Imported captions",
                "Validates caller-supplied caption strings and scores into normalized captions.",
                serde_json::json!({"captions": [{"text": "a red cube", "score": 0.8}]}),
            ),
            operation(
                "image.captioning.rankCaptions",
                "Rank captions",
                "Ranks normalized imported captions by score and returns the best caption.",
                serde_json::json!({"captions": [{"text": "a red cube", "score": 0.8}, {"text": "a studio object", "score": 0.5}], "limit": 2}),
            ),
            operation(
                "image.captioning.captionReport",
                "Caption report",
                "Summarizes a normalized caption set with score, length, and attribute-key statistics.",
                serde_json::json!({"captions": [{"text": "a red cube", "score": 0.8, "attributes": {"source": "fixture"}}]}),
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
        "image.captioning.models" => models_value(parse_input(request.input)?)?,
        "image.captioning.schema" => schema_summary(),
        "image.captioning.imported" => imported_value(parse_input(request.input)?)?,
        "image.captioning.rankCaptions" => rank_captions_value(parse_input(request.input)?)?,
        "image.captioning.captionReport" => caption_report_value(parse_input(request.input)?)?,
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
    #[serde(default, alias = "items")]
    captions: Vec<ImportedCaption>,
    limit: Option<usize>,
    min_score: Option<f32>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ImportedCaption {
    #[serde(alias = "caption")]
    text: String,
    score: Option<f32>,
    #[serde(default)]
    attributes: BTreeMap<String, String>,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct NormalizedCaption {
    text: String,
    score: Option<f32>,
    attributes: BTreeMap<String, String>,
}

fn models_value(request: ModelsRequest) -> Result<serde_json::Value, String> {
    let task = request
        .task
        .as_deref()
        .map(|task| parse_task(task).ok_or_else(|| format!("unsupported captioning task `{task}`")))
        .transpose()?;
    Ok(serde_json::json!({
        "models": image_captioning_catalog(task)
    }))
}

fn imported_value(request: ImportedRequest) -> Result<serde_json::Value, String> {
    let captions = normalize_captions(request.captions)?;
    Ok(serde_json::json!({
        "count": captions.len(),
        "captions": captions
    }))
}

fn rank_captions_value(request: ImportedRequest) -> Result<serde_json::Value, String> {
    let mut captions = normalize_captions(request.captions)?;
    if let Some(limit) = request.limit {
        if limit == 0 {
            return Err("caption rank limit must be greater than zero".to_string());
        }
    }
    if let Some(min_score) = request.min_score {
        if !min_score.is_finite() {
            return Err("caption minScore must be finite".to_string());
        }
        captions.retain(|caption| caption.score.is_some_and(|score| score >= min_score));
    }
    captions.sort_by(|left, right| compare_scores_desc(left.score, right.score));
    let filtered_count = captions.len();
    if let Some(limit) = request.limit {
        captions.truncate(limit);
    }
    Ok(serde_json::json!({
        "count": captions.len(),
        "filteredCount": filtered_count,
        "bestCaption": captions.first(),
        "captions": captions
    }))
}

fn caption_report_value(request: ImportedRequest) -> Result<serde_json::Value, String> {
    let mut captions = normalize_captions(request.captions)?;
    captions.sort_by(|left, right| compare_scores_desc(left.score, right.score));
    let scores = captions
        .iter()
        .filter_map(|caption| caption.score)
        .collect::<Vec<_>>();
    let lengths = captions
        .iter()
        .map(|caption| caption.text.chars().count())
        .collect::<Vec<_>>();
    let attribute_keys = captions
        .iter()
        .flat_map(|caption| caption.attributes.keys().cloned())
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let average_score = if scores.is_empty() {
        None
    } else {
        Some(scores.iter().sum::<f32>() / scores.len() as f32)
    };
    Ok(serde_json::json!({
        "count": captions.len(),
        "scoredCount": scores.len(),
        "unscoredCount": captions.len().saturating_sub(scores.len()),
        "bestCaption": captions.first(),
        "averageScore": average_score,
        "length": {
            "min": lengths.iter().min().copied().unwrap_or(0),
            "max": lengths.iter().max().copied().unwrap_or(0),
            "average": if lengths.is_empty() { 0.0 } else { lengths.iter().sum::<usize>() as f32 / lengths.len() as f32 }
        },
        "attributeKeys": attribute_keys,
        "captions": captions
    }))
}

fn normalize_captions(captions: Vec<ImportedCaption>) -> Result<Vec<NormalizedCaption>, String> {
    captions
        .into_iter()
        .map(|item| {
            if item.text.trim().is_empty() {
                return Err("caption text must not be empty".to_string());
            }
            if let Some(score) = item.score {
                if !score.is_finite() {
                    return Err("caption score must be finite".to_string());
                }
            }
            let mut caption = ImageCaption::new(item.text.trim().to_string());
            if let Some(score) = item.score {
                caption = caption.score(score);
            }
            for (key, value) in item.attributes {
                caption = caption.attribute(key, value);
            }
            Ok(NormalizedCaption {
                text: caption.text,
                score: caption.score,
                attributes: caption.attributes,
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
    fn package_surface_lists_captioning_operations() {
        let ids = package_surface()
            .operations
            .into_iter()
            .map(|operation| operation.id.0)
            .collect::<Vec<_>>();
        assert!(ids.contains(&"image.captioning.models".to_string()));
        assert!(ids.contains(&"image.captioning.imported".to_string()));
        assert!(ids.contains(&"image.captioning.rankCaptions".to_string()));
        assert!(ids.contains(&"image.captioning.captionReport".to_string()));
    }

    #[test]
    fn imported_operation_normalizes_caption_text() {
        let response = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("image.captioning.imported"),
            input: serde_json::json!({"captions": [{"text": " a red cube ", "score": 0.8}]}),
        })
        .expect("imported");
        assert_eq!(response.value["count"], 1);
        assert_eq!(response.value["captions"][0]["text"], "a red cube");
    }

    #[test]
    fn imported_operation_rejects_empty_caption() {
        let error = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("image.captioning.imported"),
            input: serde_json::json!({"captions": [{"text": ""}]}),
        })
        .expect_err("empty caption");
        assert!(error.contains("caption text"));
    }

    #[test]
    fn rank_operation_returns_best_caption() {
        let response = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("image.captioning.rankCaptions"),
            input: serde_json::json!({"captions": [
                {"text": "low", "score": 0.1},
                {"text": "high", "score": 0.9}
            ]}),
        })
        .expect("rank");
        assert_eq!(response.value["bestCaption"]["text"], "high");
    }
}
