//! Library-owned runtime surface for `finance-statistics`.

use runtime_core::{
    OperationId, PackageSurface, RuntimeCapabilities, SurfaceOperation, SurfaceRequest,
    SurfaceResponse,
};
use serde::Deserialize;

use crate::{
    annualized_return, annualized_volatility, beta_alpha, calmar_ratio, cumulative_return,
    drawdown_duration, historical_value_at_risk, log_returns, max_drawdown, mean_return,
    omega_ratio, portfolio_returns, portfolio_risk_summary, rolling_correlation, rolling_mean,
    rolling_std_dev, sharpe_ratio, simple_returns, sortino_ratio, std_dev, VarianceMode,
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
            operation(
                "finance.rolling",
                "Finance rolling statistics",
                "Computes rolling mean, sample volatility, and optional benchmark correlation.",
                serde_json::json!({"returns": [0.1, -0.2, 0.05, 0.3], "window": 2}),
            ),
            operation(
                "finance.portfolio",
                "Portfolio returns",
                "Computes weighted portfolio returns and compact portfolio risk metrics.",
                serde_json::json!({"assetReturns": [[0.02, -0.01, 0.03], [0.01, 0.0, 0.02]], "weights": [0.6, 0.4], "confidence": 0.8}),
            ),
            operation(
                "finance.performanceRatios",
                "Performance ratios",
                "Computes Calmar ratio, Omega ratio, and drawdown duration for a return series.",
                serde_json::json!({"returns": [0.1, -0.2, 0.05, 0.3], "thresholdReturnPerPeriod": 0.0}),
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
        "finance.rolling" => rolling_value(parse_input(request.input)?)?,
        "finance.portfolio" => portfolio_value(parse_input(request.input)?)?,
        "finance.performanceRatios" => performance_ratios_value(parse_input(request.input)?)?,
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

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RollingRequest {
    returns: Vec<f64>,
    window: usize,
    #[serde(default)]
    benchmark_returns: Option<Vec<f64>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PortfolioRequest {
    asset_returns: Vec<Vec<f64>>,
    weights: Vec<f64>,
    #[serde(default = "default_periods_per_year")]
    periods_per_year: f64,
    #[serde(default = "default_confidence")]
    confidence: f64,
    #[serde(default)]
    risk_free_return_per_period: f64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PerformanceRatiosRequest {
    returns: Vec<f64>,
    #[serde(default = "default_periods_per_year")]
    periods_per_year: f64,
    #[serde(default)]
    threshold_return_per_period: f64,
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

fn rolling_value(request: RollingRequest) -> Result<serde_json::Value, String> {
    validate_value_count(request.returns.len())?;
    if let Some(benchmark) = &request.benchmark_returns {
        validate_value_count(benchmark.len())?;
    }
    let mut value = serde_json::json!({
        "window": request.window,
        "meanReturn": rolling_mean(&request.returns, request.window).map_err(|error| error.to_string())?,
        "stdDev": rolling_std_dev(&request.returns, request.window).map_err(|error| error.to_string())?
    });
    if let Some(benchmark) = request.benchmark_returns {
        value["correlation"] =
            serde_json::json!(
                rolling_correlation(&request.returns, &benchmark, request.window)
                    .map_err(|error| error.to_string())?
            );
    }
    Ok(value)
}

fn portfolio_value(request: PortfolioRequest) -> Result<serde_json::Value, String> {
    validate_value_count(request.weights.len())?;
    for returns in &request.asset_returns {
        validate_value_count(returns.len())?;
    }
    let returns = portfolio_returns(&request.asset_returns, &request.weights)
        .map_err(|error| error.to_string())?;
    let summary = portfolio_risk_summary(
        &request.asset_returns,
        &request.weights,
        request.periods_per_year,
        request.risk_free_return_per_period,
        request.confidence,
    )
    .map_err(|error| error.to_string())?;
    Ok(serde_json::json!({
        "weights": request.weights,
        "returns": returns,
        "summary": {
            "meanReturn": summary.mean_return,
            "volatility": summary.volatility,
            "sharpe": summary.sharpe,
            "maxDrawdown": summary.max_drawdown,
            "valueAtRisk": summary.value_at_risk
        }
    }))
}

fn performance_ratios_value(
    request: PerformanceRatiosRequest,
) -> Result<serde_json::Value, String> {
    validate_value_count(request.returns.len())?;
    let duration = drawdown_duration(&request.returns).map_err(|error| error.to_string())?;
    Ok(serde_json::json!({
        "calmarRatio": calmar_ratio(&request.returns, request.periods_per_year).ok(),
        "omegaRatio": omega_ratio(&request.returns, request.threshold_return_per_period).ok(),
        "drawdownDuration": {
            "maxDuration": duration.max_duration,
            "currentDuration": duration.current_duration
        }
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

    #[test]
    fn rolling_returns_windowed_values() {
        let response = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("finance.rolling"),
            input: serde_json::json!({"returns": [0.1, -0.2, 0.05, 0.3], "benchmarkReturns": [0.1, -0.2, 0.05, 0.3], "window": 2}),
        })
        .expect("rolling");
        assert_eq!(response.value["meanReturn"].as_array().unwrap().len(), 3);
        assert_eq!(response.value["correlation"].as_array().unwrap().len(), 3);
    }

    #[test]
    fn portfolio_and_performance_operations_run() {
        for operation in ["finance.portfolio", "finance.performanceRatios"] {
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
