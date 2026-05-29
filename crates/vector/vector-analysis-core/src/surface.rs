//! Library-owned runtime surface for `vector-analysis-core`.

use serde::Deserialize;
use video_analysis_core::runtime::{
    OperationId, PackageSurface, RuntimeCapabilities, SurfaceOperation, SurfaceRequest,
    SurfaceResponse,
};

use crate::{
    cosine_similarity, dot, euclidean_distance, l2_norm, manhattan_distance, vector_stats,
    DenseVector, VectorMetric,
};

const DEFAULT_PREVIEW: usize = 16;
const MAX_PREVIEW: usize = 256;
const MAX_VALUES: usize = 100_000;

/// Describes the vector operations exposed by transport wrappers.
pub fn package_surface() -> PackageSurface {
    PackageSurface {
        library: env!("CARGO_PKG_NAME").to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        capabilities: RuntimeCapabilities::pure_rust(),
        operations: vec![
            operation(
                "describe",
                "Describe package",
                "Dense vector validation and metrics for video-analysis.",
                serde_json::json!({"includeOperations": true}),
            ),
            operation(
                "vector.normalize",
                "Normalize vector",
                "Returns the L2 norm and normalized dense vector values.",
                serde_json::json!({"values": [3.0, 4.0]}),
            ),
            operation(
                "vector.distance",
                "Vector distance",
                "Computes cosine, euclidean, manhattan, or dot distance between two dense vectors.",
                serde_json::json!({"left": [1.0, 0.0], "right": [0.5, 0.5], "metric": "cosine"}),
            ),
            operation(
                "vector.summary",
                "Vector summary",
                "Returns per-dimension mean, min, max, and preview values for dense vectors.",
                serde_json::json!({"vectors": [[1.0, 2.0], [3.0, 4.0]], "previewValues": 2}),
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
        "vector.normalize" => normalize_value(parse_input(request.input)?)?,
        "vector.distance" => distance_value(parse_input(request.input)?)?,
        "vector.summary" => summary_value(parse_input(request.input)?)?,
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
        "operations": surface
            .operations
            .iter()
            .map(|operation| operation.id.as_str())
            .collect::<Vec<_>>(),
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
struct NormalizeRequest {
    values: Vec<f32>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DistanceRequest {
    left: Vec<f32>,
    right: Vec<f32>,
    metric: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SummaryRequest {
    vectors: Vec<Vec<f32>>,
    #[serde(default = "default_preview")]
    preview_values: usize,
}

fn normalize_value(request: NormalizeRequest) -> Result<serde_json::Value, String> {
    validate_value_count(request.values.len())?;
    let vector = DenseVector::new(request.values).map_err(|error| error.to_string())?;
    let norm = l2_norm(vector.as_slice()).map_err(|error| error.to_string())?;
    let normalized = vector
        .l2_normalized()
        .map_err(|error| error.to_string())?
        .into_values();
    Ok(serde_json::json!({
        "dimensions": normalized.len(),
        "norm": norm,
        "values": normalized
    }))
}

fn distance_value(request: DistanceRequest) -> Result<serde_json::Value, String> {
    validate_value_count(request.left.len())?;
    validate_value_count(request.right.len())?;
    let metric = parse_metric(&request.metric)?;
    let distance =
        match metric {
            VectorMetric::Cosine => {
                1.0 - cosine_similarity(&request.left, &request.right)
                    .map_err(|error| error.to_string())?
            }
            VectorMetric::Euclidean => euclidean_distance(&request.left, &request.right)
                .map_err(|error| error.to_string())?,
            VectorMetric::Manhattan => manhattan_distance(&request.left, &request.right)
                .map_err(|error| error.to_string())?,
            VectorMetric::Dot => {
                -dot(&request.left, &request.right).map_err(|error| error.to_string())?
            }
        };
    let mut value = serde_json::json!({
        "metric": request.metric,
        "distance": distance
    });
    if metric == VectorMetric::Cosine {
        value["similarity"] =
            serde_json::json!(cosine_similarity(&request.left, &request.right)
                .map_err(|error| error.to_string())?);
    }
    if metric == VectorMetric::Dot {
        value["dot"] = serde_json::json!(
            dot(&request.left, &request.right).map_err(|error| error.to_string())?
        );
    }
    Ok(value)
}

fn summary_value(request: SummaryRequest) -> Result<serde_json::Value, String> {
    let total_values = request.vectors.iter().map(Vec::len).sum::<usize>();
    validate_value_count(total_values)?;
    let vectors = request
        .vectors
        .into_iter()
        .map(DenseVector::new)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    let stats = vector_stats(&vectors).map_err(|error| error.to_string())?;
    let preview_values = request.preview_values.min(MAX_PREVIEW);
    Ok(serde_json::json!({
        "count": vectors.len(),
        "dimensions": stats.dimensions,
        "mean": stats.mean,
        "min": stats.min,
        "max": stats.max,
        "meanPreview": stats.mean.iter().take(preview_values).copied().collect::<Vec<_>>()
    }))
}

fn parse_metric(metric: &str) -> Result<VectorMetric, String> {
    match metric {
        "cosine" => Ok(VectorMetric::Cosine),
        "euclidean" => Ok(VectorMetric::Euclidean),
        "manhattan" => Ok(VectorMetric::Manhattan),
        "dot" => Ok(VectorMetric::Dot),
        _ => Err(format!("unsupported vector metric `{metric}`")),
    }
}

fn validate_value_count(count: usize) -> Result<(), String> {
    if count > MAX_VALUES {
        return Err(format!("vector values must not exceed {MAX_VALUES}"));
    }
    Ok(())
}

fn parse_input<T: for<'de> Deserialize<'de>>(input: serde_json::Value) -> Result<T, String> {
    serde_json::from_value(input).map_err(|error| format!("invalid request: {error}"))
}

fn default_preview() -> usize {
    DEFAULT_PREVIEW
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn package_surface_lists_vector_operations() {
        let ids = package_surface()
            .operations
            .into_iter()
            .map(|operation| operation.id.0)
            .collect::<Vec<_>>();

        assert!(ids.contains(&"vector.normalize".to_string()));
        assert!(ids.contains(&"vector.distance".to_string()));
        assert!(ids.contains(&"vector.summary".to_string()));
    }

    #[test]
    fn normalize_returns_norm_and_values() {
        let response = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("vector.normalize"),
            input: serde_json::json!({"values": [3.0, 4.0]}),
        })
        .expect("normalize operation");

        assert_eq!(response.value["dimensions"], 2);
        assert_eq!(response.value["norm"], 5.0);
        let values = response.value["values"].as_array().unwrap();
        assert!((values[0].as_f64().unwrap() - 0.6).abs() < 1.0e-6);
        assert!((values[1].as_f64().unwrap() - 0.8).abs() < 1.0e-6);
    }

    #[test]
    fn cosine_distance_returns_similarity() {
        let response = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("vector.distance"),
            input: serde_json::json!({"left": [1.0, 0.0], "right": [1.0, 0.0], "metric": "cosine"}),
        })
        .expect("distance operation");

        assert_eq!(response.value["distance"], 0.0);
        assert_eq!(response.value["similarity"], 1.0);
    }

    #[test]
    fn summary_returns_vector_stats() {
        let response = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("vector.summary"),
            input: serde_json::json!({"vectors": [[1.0, 3.0], [3.0, 5.0]], "previewValues": 1}),
        })
        .expect("summary operation");

        assert_eq!(response.value["count"], 2);
        assert_eq!(response.value["dimensions"], 2);
        assert_eq!(response.value["mean"], serde_json::json!([2.0, 4.0]));
        assert_eq!(response.value["min"], serde_json::json!([1.0, 3.0]));
        assert_eq!(response.value["max"], serde_json::json!([3.0, 5.0]));
        assert_eq!(response.value["meanPreview"], serde_json::json!([2.0]));
    }

    #[test]
    fn invalid_request_parsing_is_reported() {
        let error = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("vector.normalize"),
            input: serde_json::json!(["not an object"]),
        })
        .expect_err("invalid request");

        assert!(error.contains("invalid request"));
    }

    #[test]
    fn mismatched_dimensions_return_validation_error() {
        let error = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("vector.distance"),
            input: serde_json::json!({"left": [1.0], "right": [1.0, 2.0], "metric": "euclidean"}),
        })
        .expect_err("dimension mismatch");

        assert!(error.contains("same dimensions"));
    }

    #[test]
    fn unsupported_operation_is_reported() {
        let error = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("vector.missing"),
            input: serde_json::json!({}),
        })
        .expect_err("unsupported operation");

        assert!(error.contains("unsupported operation `vector.missing`"));
    }
}
