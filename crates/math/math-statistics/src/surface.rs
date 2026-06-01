//! Library-owned runtime surface for `math-statistics`.

use runtime_core::{
    OperationId, PackageSurface, RuntimeCapabilities, SurfaceOperation, SurfaceRequest,
    SurfaceResponse,
};
use serde::Deserialize;

use math_linear::{F32Matrix, MatrixShape};

use crate::{MinMaxNormalizer, PrincipalComponents, RunningCovariance, ZScoreNormalizer};

const MAX_VALUES: usize = 100_000;

/// Describes the statistics operations exposed by transport wrappers.
pub fn package_surface() -> PackageSurface {
    PackageSurface {
        library: env!("CARGO_PKG_NAME").to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        capabilities: RuntimeCapabilities::pure_rust(),
        operations: vec![
            operation("describe", "Describe package", "Shared multivariate statistics for dense matrix inputs and streaming observations.", serde_json::json!({"includeOperations": true})),
            operation("stats.normalize", "Normalize matrix", "Applies z-score or min-max column normalization to a finite f32 matrix.", serde_json::json!({"matrix": {"rows": 2, "cols": 2, "values": [0.0, 1.0, 2.0, 3.0]}, "method": "minMax"})),
            operation("stats.covariance", "Covariance matrix", "Computes covariance and optional correlation for matrix rows as observations.", serde_json::json!({"matrix": {"rows": 3, "cols": 2, "values": [1.0, 0.0, 0.0, 1.0, 1.0, 1.0]}, "correlation": true})),
            operation("stats.pca", "PCA", "Fits PCA-lite components and optionally transforms rows into component space.", serde_json::json!({"matrix": {"rows": 3, "cols": 2, "values": [1.0, 1.0, 2.0, 2.0, 3.0, 3.0]}, "componentCount": 1, "transform": true})),
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
        "stats.normalize" => normalize_value(parse_input(request.input)?)?,
        "stats.covariance" => covariance_value(parse_input(request.input)?)?,
        "stats.pca" => pca_value(parse_input(request.input)?)?,
        operation => {
            return Err(format!(
                "unsupported operation `{operation}` for {}",
                env!("CARGO_PKG_NAME")
            ))
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
struct MatrixRequest {
    rows: usize,
    cols: usize,
    values: Vec<f32>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct NormalizeRequest {
    matrix: MatrixRequest,
    method: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CovarianceRequest {
    matrix: MatrixRequest,
    #[serde(default)]
    correlation: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PcaRequest {
    matrix: MatrixRequest,
    component_count: usize,
    #[serde(default)]
    transform: bool,
}

fn normalize_value(request: NormalizeRequest) -> Result<serde_json::Value, String> {
    let matrix = matrix_from_request(request.matrix)?;
    match request.method.as_str() {
        "zScore" => {
            let normalizer =
                ZScoreNormalizer::fit(&matrix.as_view()).map_err(|error| error.to_string())?;
            let normalized = normalizer
                .transform_matrix(&matrix.as_view())
                .map_err(|error| error.to_string())?;
            let shape = normalized.shape();
            Ok(serde_json::json!({
                "method": "zScore",
                "rows": shape.rows,
                "cols": shape.cols,
                "values": normalized.values(),
                "means": normalizer.means(),
                "stdDevs": normalizer.std_devs()
            }))
        }
        "minMax" => {
            let normalizer =
                MinMaxNormalizer::fit(&matrix.as_view()).map_err(|error| error.to_string())?;
            let normalized = normalizer
                .transform_matrix(&matrix.as_view())
                .map_err(|error| error.to_string())?;
            let shape = normalized.shape();
            Ok(serde_json::json!({
                "method": "minMax",
                "rows": shape.rows,
                "cols": shape.cols,
                "values": normalized.values(),
                "ranges": normalizer.ranges().iter().map(|range| serde_json::json!({"min": range.min, "max": range.max})).collect::<Vec<_>>()
            }))
        }
        method => Err(format!("unsupported normalization method `{method}`")),
    }
}

fn covariance_value(request: CovarianceRequest) -> Result<serde_json::Value, String> {
    let matrix = matrix_from_request(request.matrix)?;
    let covariance = RunningCovariance::from_matrix(&matrix.as_view())
        .map_err(|error| error.to_string())?
        .covariance_matrix()
        .map_err(|error| error.to_string())?;
    let mut value = serde_json::json!({
        "count": covariance.count,
        "weightSum": covariance.weight_sum,
        "means": covariance.means,
        "covariance": matrix_projection(&covariance.matrix)
    });
    if request.correlation {
        let correlation = covariance
            .correlation_matrix()
            .map_err(|error| error.to_string())?;
        value["correlation"] = matrix_projection(&correlation);
    }
    Ok(value)
}

fn pca_value(request: PcaRequest) -> Result<serde_json::Value, String> {
    let matrix = matrix_from_request(request.matrix)?;
    let pca = PrincipalComponents::fit(&matrix.as_view(), request.component_count)
        .map_err(|error| error.to_string())?;
    let mut value = serde_json::json!({
        "mean": pca.mean(),
        "components": matrix_projection(pca.components()),
        "explainedVariance": pca.explained_variance()
    });
    if request.transform {
        let transformed = pca
            .transform(&matrix.as_view())
            .map_err(|error| error.to_string())?;
        value["transformed"] = matrix_projection(&transformed);
    }
    Ok(value)
}

fn matrix_from_request(request: MatrixRequest) -> Result<F32Matrix, String> {
    validate_value_count(request.values.len())?;
    F32Matrix::new(
        MatrixShape::new(request.rows, request.cols).map_err(|error| error.to_string())?,
        request.values,
    )
    .map_err(|error| error.to_string())
}

fn matrix_projection(matrix: &F32Matrix) -> serde_json::Value {
    let shape = matrix.shape();
    serde_json::json!({
        "rows": shape.rows,
        "cols": shape.cols,
        "values": matrix.values()
    })
}

fn validate_value_count(count: usize) -> Result<(), String> {
    if count > MAX_VALUES {
        return Err(format!("values must not exceed {MAX_VALUES}"));
    }
    Ok(())
}

fn parse_input<T: for<'de> Deserialize<'de>>(input: serde_json::Value) -> Result<T, String> {
    serde_json::from_value(input).map_err(|error| format!("invalid request: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_min_max() {
        let response = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("stats.normalize"),
            input: serde_json::json!({"matrix": {"rows": 2, "cols": 2, "values": [0.0, 1.0, 2.0, 3.0]}, "method": "minMax"}),
        }).expect("normalize");
        assert_eq!(
            response.value["values"],
            serde_json::json!([0.0, 0.0, 1.0, 1.0])
        );
    }

    #[test]
    fn covariance_can_include_correlation() {
        let response = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("stats.covariance"),
            input: serde_json::json!({"matrix": {"rows": 3, "cols": 2, "values": [1.0, 0.0, 0.0, 1.0, 1.0, 1.0]}, "correlation": true}),
        }).expect("covariance");
        assert_eq!(response.value["count"], 3);
        assert!(response.value["correlation"].is_object());
    }

    #[test]
    fn pca_returns_components() {
        let response = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("stats.pca"),
            input: serde_json::json!({"matrix": {"rows": 3, "cols": 2, "values": [1.0, 1.0, 2.0, 2.0, 3.0, 3.0]}, "componentCount": 1, "transform": true}),
        }).expect("pca");
        assert_eq!(response.value["components"]["rows"], 1);
        assert!(response.value["transformed"].is_object());
    }
}
