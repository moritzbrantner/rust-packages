//! Library-owned runtime surface for `finance-statistics`.

use serde::Deserialize;
use video_analysis_core::runtime::{
    OperationId, PackageSurface, RuntimeCapabilities, SurfaceOperation, SurfaceRequest,
    SurfaceResponse,
};

use crate::{
    annualized_return, annualized_volatility, beta_alpha, cumulative_return,
    historical_value_at_risk, log_returns, max_drawdown, mean_return, sharpe_ratio, simple_returns,
    sortino_ratio, std_dev, VarianceMode,
};

const MAX_VALUES: usize = 100_000;

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
                "Finance-oriented return, risk, and rolling statistics helpers.",
                serde_json::json!({"includeOperations": true}),
            ),
            operation(
                "finance.returns",
                "Finance returns",
                "Computes simple or log returns from strictly positive prices.",
                serde_json::json!({"prices": [100.0, 110.0, 99.0], "method": "simple"}),
            ),
            operation(
                "finance.risk",
                "Finance risk",
                "Computes return, volatility, VaR/CVaR, ratios, and optional beta/alpha.",
                serde_json::json!({"returns": [0.01, -0.02, 0.03], "confidence": 0.95}),
            ),
            operation(
                "finance.drawdown",
                "Finance drawdown",
                "Computes maximum drawdown details from a return series.",
                serde_json::json!({"returns": [0.1, -0.2, 0.05, 0.3]}),
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
        "finance.returns" => returns_value(parse_input(request.input)?)?,
        "finance.risk" => risk_value(parse_input(request.input)?)?,
        "finance.drawdown" => drawdown_value(parse_input(request.input)?)?,
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
struct ReturnsRequest {
    prices: Vec<f64>,
    #[serde(default = "default_returns_method")]
    method: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RiskRequest {
    returns: Vec<f64>,
    #[serde(default = "default_periods_per_year")]
    periods_per_year: f64,
    #[serde(default = "default_confidence")]
    confidence: f64,
    #[serde(default)]
    risk_free_return_per_period: f64,
    #[serde(default)]
    benchmark_returns: Option<Vec<f64>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DrawdownRequest {
    returns: Vec<f64>,
}

fn returns_value(request: ReturnsRequest) -> Result<serde_json::Value, String> {
    validate_value_count(request.prices.len())?;
    let returns = match request.method.as_str() {
        "simple" => simple_returns(&request.prices),
        "log" => log_returns(&request.prices),
        method => return Err(format!("unsupported returns method `{method}`")),
    }
    .map_err(|error| error.to_string())?;
    let cumulative = cumulative_return(&returns).map_err(|error| error.to_string())?;
    Ok(serde_json::json!({
        "method": request.method,
        "returns": returns,
        "cumulativeReturn": cumulative
    }))
}

fn risk_value(request: RiskRequest) -> Result<serde_json::Value, String> {
    validate_value_count(request.returns.len())?;
    if let Some(benchmark) = &request.benchmark_returns {
        validate_value_count(benchmark.len())?;
    }
    let mean = mean_return(&request.returns).map_err(|error| error.to_string())?;
    let std = std_dev(&request.returns, VarianceMode::Sample).map_err(|error| error.to_string())?;
    let historical = historical_value_at_risk(&request.returns, request.confidence)
        .map_err(|error| error.to_string())?;
    let beta_alpha_value = request
        .benchmark_returns
        .as_ref()
        .map(|benchmark| {
            beta_alpha(
                &request.returns,
                benchmark,
                request.risk_free_return_per_period,
            )
        })
        .transpose()
        .map_err(|error| error.to_string())?
        .map(|value| {
            serde_json::json!({
                "beta": value.beta,
                "alpha": value.alpha,
                "correlation": value.correlation,
                "assetMean": value.asset_mean,
                "benchmarkMean": value.benchmark_mean
            })
        });
    Ok(serde_json::json!({
        "meanReturn": mean,
        "stdDev": std,
        "annualizedReturn": annualized_return(&request.returns, request.periods_per_year).map_err(|error| error.to_string())?,
        "annualizedVolatility": annualized_volatility(&request.returns, request.periods_per_year).map_err(|error| error.to_string())?,
        "sharpeRatio": sharpe_ratio(&request.returns, request.risk_free_return_per_period, request.periods_per_year).ok(),
        "sortinoRatio": sortino_ratio(&request.returns, request.risk_free_return_per_period, request.periods_per_year).ok(),
        "valueAtRisk": historical.value_at_risk,
        "conditionalValueAtRisk": historical.conditional_value_at_risk,
        "betaAlpha": beta_alpha_value
    }))
}

fn drawdown_value(request: DrawdownRequest) -> Result<serde_json::Value, String> {
    validate_value_count(request.returns.len())?;
    let drawdown = max_drawdown(&request.returns).map_err(|error| error.to_string())?;
    Ok(serde_json::json!({
        "depth": drawdown.depth,
        "peakIndex": drawdown.peak_index,
        "troughIndex": drawdown.trough_index,
        "recoveryIndex": drawdown.recovery_index
    }))
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

fn default_returns_method() -> String {
    "simple".to_string()
}

fn default_periods_per_year() -> f64 {
    252.0
}

fn default_confidence() -> f64 {
    0.95
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn returns_compute_simple_series() {
        let response = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("finance.returns"),
            input: serde_json::json!({"prices": [100.0, 110.0, 99.0]}),
        })
        .expect("returns");
        assert_eq!(response.value["method"], "simple");
        assert_eq!(response.value["returns"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn risk_returns_core_metrics() {
        let response = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("finance.risk"),
            input: serde_json::json!({"returns": [0.02, -0.01, 0.03, -0.02, 0.01], "confidence": 0.8}),
        }).expect("risk");
        assert!(response.value["annualizedVolatility"].as_f64().unwrap() > 0.0);
        assert!(response.value["valueAtRisk"].as_f64().unwrap() >= 0.0);
    }

    #[test]
    fn drawdown_reports_indices() {
        let response = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("finance.drawdown"),
            input: serde_json::json!({"returns": [0.1, -0.2, 0.05, 0.3]}),
        })
        .expect("drawdown");
        assert_eq!(response.value["peakIndex"], 1);
        assert_eq!(response.value["troughIndex"], 2);
    }
}
