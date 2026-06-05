//! Library-owned runtime surface for `math-statistics`.

use runtime_core::{
    OperationId, PackageSurface, RuntimeCapabilities, SurfaceOperation, SurfaceRequest,
    SurfaceResponse,
};
use serde::Deserialize;

use math_linear::{F32Matrix, MatrixShape};

use crate::{
    changes, ordinary_least_squares, rank_values, rolling_correlation, rolling_mean,
    rolling_min_max, rolling_std_dev, simple_linear_regression, spearman_correlation,
    summarize_pair, summarize_series, tail_risk, z_scores, ChangeMethod, MinMaxNormalizer,
    PrincipalComponents, RunningCovariance, VarianceMode, ZScoreNormalizer,
};

const MAX_VALUES: usize = 100_000;

/// Describes the statistics operations exposed by transport wrappers.
pub fn package_surface() -> PackageSurface {
    PackageSurface {
        library: env!("CARGO_PKG_NAME").to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        capabilities: RuntimeCapabilities::pure_rust(),
        operations: vec![
            operation("describe", "Describe package", "Shared statistics for scalar series, paired series, dense matrix inputs, and streaming observations.", serde_json::json!({"includeOperations": true})),
            operation("stats.series.describe", "Describe series", "Computes summary statistics for a finite scalar series.", serde_json::json!({"values": [1.0, 2.0, 3.0, 4.0], "varianceMode": "sample"})),
            operation("stats.series.changes", "Series changes", "Converts adjacent values into differences, relative changes, or log-ratios.", serde_json::json!({"values": [100.0, 102.0, 99.0, 105.0], "method": "relative"})),
            operation("stats.series.compare", "Compare series", "Computes pairwise covariance and correlation for two finite scalar series.", serde_json::json!({"left": [0.01, -0.02, 0.03], "right": [0.0, -0.01, 0.02], "varianceMode": "sample"})),
            operation("stats.series.rolling", "Rolling statistics", "Computes rolling mean, standard deviation, ranges, and optional paired correlations.", serde_json::json!({"values": [0.1, -0.2, 0.05, 0.3], "window": 2, "varianceMode": "sample"})),
            operation("stats.series.tailRisk", "Tail risk", "Computes empirical VaR/CVaR style tail-risk statistics.", serde_json::json!({"values": [0.02, -0.01, 0.03, -0.02, 0.01], "confidence": 0.8})),
            operation("stats.series.zScores", "Z-scores", "Standardizes a finite scalar series into z-scores.", serde_json::json!({"values": [1.0, 2.0, 3.0], "varianceMode": "population"})),
            operation("stats.normalize", "Normalize matrix", "Applies z-score or min-max column normalization to a finite f32 matrix.", serde_json::json!({"matrix": {"rows": 2, "cols": 2, "values": [0.0, 1.0, 2.0, 3.0]}, "method": "minMax"})),
            operation("stats.covariance", "Covariance matrix", "Computes covariance and optional correlation for matrix rows as observations.", serde_json::json!({"matrix": {"rows": 3, "cols": 2, "values": [1.0, 0.0, 0.0, 1.0, 1.0, 1.0]}, "correlation": true})),
            operation("stats.pca", "PCA", "Fits PCA-lite components and optionally transforms rows into component space.", serde_json::json!({"matrix": {"rows": 3, "cols": 2, "values": [1.0, 1.0, 2.0, 2.0, 3.0, 3.0]}, "componentCount": 1, "transform": true})),
            operation("stats.series.rankCorrelation", "Rank correlation", "Computes average ranks and Spearman correlation for finite paired scalar series.", serde_json::json!({"left": [1.0, 2.0, 2.0, 4.0], "right": [10.0, 20.0, 20.0, 40.0]})),
            operation("stats.regression.linear", "Linear regression", "Fits a simple y = intercept + slope * x regression for paired finite scalar observations.", serde_json::json!({"x": [1.0, 2.0, 3.0], "y": [3.0, 5.0, 7.0]})),
            operation("stats.regression.ols", "OLS regression", "Fits ordinary least squares from a finite dense design matrix and target vector.", serde_json::json!({"design": {"rows": 3, "cols": 2, "values": [1.0, 1.0, 1.0, 2.0, 1.0, 3.0]}, "target": [3.0, 5.0, 7.0]})),
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
        "stats.series.describe" => series_describe_value(parse_input(request.input)?)?,
        "stats.series.changes" => series_changes_value(parse_input(request.input)?)?,
        "stats.series.compare" => series_compare_value(parse_input(request.input)?)?,
        "stats.series.rolling" => series_rolling_value(parse_input(request.input)?)?,
        "stats.series.tailRisk" => series_tail_risk_value(parse_input(request.input)?)?,
        "stats.series.zScores" => series_z_scores_value(parse_input(request.input)?)?,
        "stats.normalize" => normalize_value(parse_input(request.input)?)?,
        "stats.covariance" => covariance_value(parse_input(request.input)?)?,
        "stats.pca" => pca_value(parse_input(request.input)?)?,
        "stats.series.rankCorrelation" => rank_correlation_value(parse_input(request.input)?)?,
        "stats.regression.linear" => regression_linear_value(parse_input(request.input)?)?,
        "stats.regression.ols" => regression_ols_value(parse_input(request.input)?)?,
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
struct SeriesRequest {
    values: Vec<f64>,
    #[serde(default = "default_variance_mode")]
    variance_mode: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ChangesRequest {
    values: Vec<f64>,
    #[serde(default = "default_change_method")]
    method: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CompareRequest {
    left: Vec<f64>,
    right: Vec<f64>,
    #[serde(default = "default_variance_mode")]
    variance_mode: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RollingRequest {
    values: Vec<f64>,
    window: usize,
    #[serde(default = "default_variance_mode")]
    variance_mode: String,
    #[serde(default)]
    right: Option<Vec<f64>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TailRiskRequest {
    values: Vec<f64>,
    #[serde(default = "default_confidence")]
    confidence: f64,
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

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RankCorrelationRequest {
    left: Vec<f64>,
    right: Vec<f64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LinearRegressionRequest {
    x: Vec<f64>,
    y: Vec<f64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OlsRequest {
    design: MatrixRequest,
    target: Vec<f32>,
}

fn series_describe_value(request: SeriesRequest) -> Result<serde_json::Value, String> {
    validate_value_count(request.values.len())?;
    let mode = parse_variance_mode(&request.variance_mode)?;
    let summary = summarize_series(&request.values, mode).map_err(|error| error.to_string())?;
    Ok(serde_json::json!({
        "count": summary.count,
        "mean": summary.mean,
        "variance": summary.variance,
        "stdDev": summary.std_dev,
        "min": summary.min,
        "max": summary.max,
        "q1": summary.q1,
        "median": summary.median,
        "q3": summary.q3,
        "iqr": summary.iqr,
        "skewness": summary.skewness,
        "excessKurtosis": summary.excess_kurtosis
    }))
}

fn series_changes_value(request: ChangesRequest) -> Result<serde_json::Value, String> {
    validate_value_count(request.values.len())?;
    let method = parse_change_method(&request.method)?;
    let values = changes(&request.values, method).map_err(|error| error.to_string())?;
    Ok(serde_json::json!({
        "method": request.method,
        "values": values
    }))
}

fn series_compare_value(request: CompareRequest) -> Result<serde_json::Value, String> {
    validate_value_count(request.left.len())?;
    validate_value_count(request.right.len())?;
    let mode = parse_variance_mode(&request.variance_mode)?;
    let summary =
        summarize_pair(&request.left, &request.right, mode).map_err(|error| error.to_string())?;
    Ok(serde_json::json!({
        "leftMean": summary.left_mean,
        "rightMean": summary.right_mean,
        "covariance": summary.covariance,
        "correlation": summary.correlation,
        "observations": summary.observations
    }))
}

fn series_rolling_value(request: RollingRequest) -> Result<serde_json::Value, String> {
    validate_value_count(request.values.len())?;
    if let Some(right) = &request.right {
        validate_value_count(right.len())?;
    }
    let mode = parse_variance_mode(&request.variance_mode)?;
    let mut value = serde_json::json!({
        "window": request.window,
        "mean": rolling_mean(&request.values, request.window).map_err(|error| error.to_string())?,
        "stdDev": rolling_std_dev(&request.values, request.window, mode).map_err(|error| error.to_string())?,
        "minMax": rolling_min_max(&request.values, request.window)
            .map_err(|error| error.to_string())?
            .iter()
            .map(|range| serde_json::json!({"min": range.min, "max": range.max}))
            .collect::<Vec<_>>()
    });
    if let Some(right) = request.right {
        value["correlation"] =
            serde_json::json!(rolling_correlation(&request.values, &right, request.window)
                .map_err(|error| error.to_string())?);
    }
    Ok(value)
}

fn series_tail_risk_value(request: TailRiskRequest) -> Result<serde_json::Value, String> {
    validate_value_count(request.values.len())?;
    let risk = tail_risk(&request.values, request.confidence).map_err(|error| error.to_string())?;
    Ok(serde_json::json!({
        "confidence": risk.confidence,
        "threshold": risk.threshold,
        "valueAtRisk": risk.value_at_risk,
        "conditionalValueAtRisk": risk.conditional_value_at_risk,
        "observations": risk.observations
    }))
}

fn series_z_scores_value(request: SeriesRequest) -> Result<serde_json::Value, String> {
    validate_value_count(request.values.len())?;
    let mode = parse_variance_mode(&request.variance_mode)?;
    let values = z_scores(&request.values, mode).map_err(|error| error.to_string())?;
    Ok(serde_json::json!({
        "values": values
    }))
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

fn rank_correlation_value(request: RankCorrelationRequest) -> Result<serde_json::Value, String> {
    validate_value_count(request.left.len())?;
    validate_value_count(request.right.len())?;
    Ok(serde_json::json!({
        "leftRanks": rank_values(&request.left).map_err(|error| error.to_string())?,
        "rightRanks": rank_values(&request.right).map_err(|error| error.to_string())?,
        "spearmanCorrelation": spearman_correlation(&request.left, &request.right).map_err(|error| error.to_string())?,
        "observations": request.left.len()
    }))
}

fn regression_linear_value(request: LinearRegressionRequest) -> Result<serde_json::Value, String> {
    validate_value_count(request.x.len())?;
    validate_value_count(request.y.len())?;
    let summary =
        simple_linear_regression(&request.x, &request.y).map_err(|error| error.to_string())?;
    Ok(serde_json::json!({
        "slope": summary.slope,
        "intercept": summary.intercept,
        "rSquared": summary.r_squared,
        "residualStdError": summary.residual_std_error,
        "observations": summary.observations
    }))
}

fn regression_ols_value(request: OlsRequest) -> Result<serde_json::Value, String> {
    validate_value_count(request.target.len())?;
    let design = matrix_from_request(request.design)?;
    let regression = ordinary_least_squares(&design.as_view(), &request.target)
        .map_err(|error| error.to_string())?;
    Ok(serde_json::json!({
        "coefficients": regression.coefficients,
        "fitted": regression.fitted,
        "residuals": regression.residuals,
        "rSquared": regression.r_squared,
        "observations": regression.observations,
        "predictors": regression.predictors
    }))
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

fn parse_variance_mode(value: &str) -> Result<VarianceMode, String> {
    match value {
        "population" => Ok(VarianceMode::Population),
        "sample" => Ok(VarianceMode::Sample),
        mode => Err(format!("unsupported variance mode `{mode}`")),
    }
}

fn parse_change_method(value: &str) -> Result<ChangeMethod, String> {
    match value {
        "difference" => Ok(ChangeMethod::Difference),
        "relative" => Ok(ChangeMethod::Relative),
        "logRatio" | "log-ratio" | "log" => Ok(ChangeMethod::LogRatio),
        method => Err(format!("unsupported change method `{method}`")),
    }
}

fn default_variance_mode() -> String {
    "sample".to_string()
}

fn default_change_method() -> String {
    "difference".to_string()
}

fn default_confidence() -> f64 {
    0.95
}

fn parse_input<T: for<'de> Deserialize<'de>>(input: serde_json::Value) -> Result<T, String> {
    serde_json::from_value(input).map_err(|error| format!("invalid request: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn surface_lists_series_operations() {
        let ids = package_surface()
            .operations
            .iter()
            .map(|operation| operation.id.as_str().to_string())
            .collect::<Vec<_>>();
        for id in [
            "stats.series.describe",
            "stats.series.changes",
            "stats.series.compare",
            "stats.series.rolling",
            "stats.series.tailRisk",
            "stats.series.zScores",
        ] {
            assert!(ids.contains(&id.to_string()), "missing {id}");
        }
    }

    #[test]
    fn series_operations_run() {
        for operation in package_surface()
            .operations
            .into_iter()
            .filter(|operation| operation.id.as_str().starts_with("stats.series."))
        {
            let response = run_surface_operation(SurfaceRequest {
                operation: operation.id,
                input: operation.example_request,
            })
            .unwrap_or_else(|error| panic!("{} failed: {error}", operation.name));
            assert!(response.value.is_object());
        }
    }

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

    #[test]
    fn regression_operations_run() {
        for operation in [
            "stats.series.rankCorrelation",
            "stats.regression.linear",
            "stats.regression.ols",
        ] {
            let surface_operation = package_surface()
                .operations
                .into_iter()
                .find(|candidate| candidate.id.as_str() == operation)
                .expect("operation metadata");
            let response = run_surface_operation(SurfaceRequest {
                operation: surface_operation.id,
                input: surface_operation.example_request,
            })
            .unwrap_or_else(|error| panic!("{operation} failed: {error}"));
            assert!(response.value.is_object());
        }
    }
}
