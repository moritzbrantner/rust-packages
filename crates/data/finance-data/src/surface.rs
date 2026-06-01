//! Library-owned runtime surface for `finance-data`.

use runtime_core::{
    describe_surface_response, structured_surface_response, surface_operation, OperationId,
    PackageSurface, RuntimeCapabilities, SurfaceRequest, SurfaceResponse,
};
use serde::Deserialize;

use crate::{FinanceSeries, FinanceSeriesIndex, RiskSummaryOptions};

/// Returns the package surface exposed by every transport wrapper.
pub fn package_surface() -> PackageSurface {
    PackageSurface {
        library: env!("CARGO_PKG_NAME").to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        capabilities: RuntimeCapabilities::pure_rust(),
        operations: vec![
            surface_operation(
                "describe",
                "Describe package",
                "Provider-neutral financial market data validation, indexing, and derived series operations.",
                serde_json::json!({"includeOperations": true}),
            ),
            surface_operation(
                "financeData.bounds",
                "Finance data bounds",
                "Returns the timestamp and price bounds for an inline OHLCV series.",
                serde_json::json!({"series": example_series()}),
            ),
            surface_operation(
                "financeData.barsInRange",
                "Finance bars in range",
                "Returns validated OHLCV bars whose timestamps fall inside an inclusive range.",
                serde_json::json!({"series": example_series(), "startMs": 2, "endMs": 3}),
            ),
            surface_operation(
                "financeData.downsampleOhlcv",
                "Downsample OHLCV",
                "Buckets validated OHLCV bars while preserving open, high, low, close, volume, and adjusted close semantics.",
                serde_json::json!({"series": example_series(), "startMs": 1, "endMs": 4, "targetCount": 2}),
            ),
            surface_operation(
                "financeData.returns",
                "Finance data returns",
                "Computes simple or log returns from close or adjusted close prices.",
                serde_json::json!({"series": example_series(), "method": "simple", "adjusted": false}),
            ),
            surface_operation(
                "financeData.riskSummary",
                "Finance data risk summary",
                "Computes return, volatility, ratio, VaR/CVaR, and drawdown summary values from an inline OHLCV series.",
                serde_json::json!({"series": example_series(), "adjusted": false, "periodsPerYear": 252.0, "confidence": 0.95}),
            ),
        ],
    }
}

/// Runs one library-owned operation.
pub fn run_surface_operation(request: SurfaceRequest) -> Result<SurfaceResponse, String> {
    let surface = package_surface();
    match request.operation.as_str() {
        "describe" => Ok(describe_surface_response(&surface, request)),
        "financeData.bounds" => {
            let input: SeriesRequest = parse_input(request.input)?;
            let index = index(input.series)?;
            let bounds = index.bounds();
            Ok(response(
                request.operation,
                "Finance data bounds",
                serde_json::json!({
                    "barCount": index.series().bars.len(),
                    "hasBounds": bounds.is_some()
                }),
                serde_json::json!({
                    "instrument": index.series().instrument,
                    "barCount": index.series().bars.len(),
                    "bounds": bounds
                }),
            ))
        }
        "financeData.barsInRange" => {
            let input: BarsInRangeRequest = parse_input(request.input)?;
            let index = index(input.series)?;
            let bars = index.bars_in_range(input.start_ms, input.end_ms);
            Ok(response(
                request.operation,
                "Finance bars in range",
                serde_json::json!({
                    "startMs": input.start_ms,
                    "endMs": input.end_ms,
                    "barCount": bars.len()
                }),
                serde_json::json!({
                    "startMs": input.start_ms,
                    "endMs": input.end_ms,
                    "bars": bars
                }),
            ))
        }
        "financeData.downsampleOhlcv" => {
            let input: DownsampleRequest = parse_input(request.input)?;
            let index = index(input.series)?;
            let bars = index
                .downsample_ohlcv(input.start_ms, input.end_ms, input.target_count)
                .map_err(|error| error.to_string())?;
            Ok(response(
                request.operation,
                "Downsample OHLCV",
                serde_json::json!({
                    "startMs": input.start_ms,
                    "endMs": input.end_ms,
                    "targetCount": input.target_count,
                    "barCount": bars.len()
                }),
                serde_json::json!({
                    "startMs": input.start_ms,
                    "endMs": input.end_ms,
                    "targetCount": input.target_count,
                    "bars": bars
                }),
            ))
        }
        "financeData.returns" => {
            let input: ReturnsRequest = parse_input(request.input)?;
            let index = index(input.series)?;
            let returns = match input.method.as_str() {
                "simple" => index.simple_returns(input.adjusted),
                "log" => index.log_returns(input.adjusted),
                method => return Err(format!("unsupported returns method `{method}`")),
            }
            .map_err(|error| error.to_string())?;
            Ok(response(
                request.operation,
                "Finance data returns",
                serde_json::json!({
                    "method": input.method,
                    "adjusted": input.adjusted,
                    "returnCount": returns.len()
                }),
                serde_json::json!({
                    "method": input.method,
                    "adjusted": input.adjusted,
                    "returns": returns
                }),
            ))
        }
        "financeData.riskSummary" => {
            let input: RiskSummaryRequest = parse_input(request.input)?;
            let index = index(input.series)?;
            let options = RiskSummaryOptions {
                adjusted: input.adjusted,
                periods_per_year: input.periods_per_year,
                confidence: input.confidence,
                risk_free_return_per_period: input.risk_free_return_per_period,
            };
            let risk = index
                .risk_summary(options)
                .map_err(|error| error.to_string())?;
            Ok(response(
                request.operation,
                "Finance data risk summary",
                serde_json::json!({
                    "adjusted": options.adjusted,
                    "periodsPerYear": options.periods_per_year,
                    "confidence": options.confidence,
                    "valueAtRisk": risk.value_at_risk,
                    "maxDrawdownDepth": risk.max_drawdown.depth
                }),
                serde_json::json!({
                    "options": options,
                    "risk": risk
                }),
            ))
        }
        operation => Err(format!(
            "unsupported operation `{operation}` for {}",
            env!("CARGO_PKG_NAME")
        )),
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SeriesRequest {
    series: FinanceSeries,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BarsInRangeRequest {
    series: FinanceSeries,
    start_ms: i64,
    end_ms: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DownsampleRequest {
    series: FinanceSeries,
    start_ms: i64,
    end_ms: i64,
    target_count: usize,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ReturnsRequest {
    series: FinanceSeries,
    #[serde(default)]
    adjusted: bool,
    #[serde(default = "default_returns_method")]
    method: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RiskSummaryRequest {
    series: FinanceSeries,
    #[serde(default)]
    adjusted: bool,
    #[serde(default = "default_periods_per_year")]
    periods_per_year: f64,
    #[serde(default = "default_confidence")]
    confidence: f64,
    #[serde(default)]
    risk_free_return_per_period: f64,
}

fn response(
    operation: OperationId,
    title: &str,
    summary: serde_json::Value,
    result: serde_json::Value,
) -> SurfaceResponse {
    structured_surface_response(
        operation,
        title,
        format!("Ran package-surface operation `{title}`."),
        summary,
        result,
    )
}

fn index(series: FinanceSeries) -> Result<FinanceSeriesIndex, String> {
    FinanceSeriesIndex::new(series).map_err(|error| error.to_string())
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

fn example_series() -> serde_json::Value {
    serde_json::json!({
        "instrument": {
            "id": "aapl",
            "symbol": "AAPL",
            "name": "Apple Inc.",
            "exchange": "NASDAQ",
            "currency": "USD",
            "assetClass": "equity"
        },
        "bars": [
            {"timestampMs": 1, "open": 100.0, "high": 110.0, "low": 99.0, "close": 108.0, "volume": 10.0, "adjustedClose": 107.0},
            {"timestampMs": 2, "open": 108.0, "high": 112.0, "low": 105.0, "close": 106.0, "volume": 11.0, "adjustedClose": 105.0},
            {"timestampMs": 3, "open": 106.0, "high": 109.0, "low": 101.0, "close": 102.0, "volume": 12.0, "adjustedClose": 101.0},
            {"timestampMs": 4, "open": 102.0, "high": 120.0, "low": 100.0, "close": 118.0, "volume": 13.0, "adjustedClose": 117.0}
        ]
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(operation: &str) -> SurfaceResponse {
        let surface = package_surface();
        let request = surface
            .operations
            .iter()
            .find(|candidate| candidate.id.as_str() == operation)
            .expect("operation")
            .example_request
            .clone();
        run_surface_operation(SurfaceRequest {
            operation: OperationId::new(operation),
            input: request,
        })
        .expect("surface operation")
    }

    #[test]
    fn package_surface_lists_finance_data_operations() {
        let ids = package_surface()
            .operations
            .into_iter()
            .map(|operation| operation.id.0)
            .collect::<Vec<_>>();
        assert!(ids.contains(&"financeData.bounds".to_string()));
        assert!(ids.contains(&"financeData.barsInRange".to_string()));
        assert!(ids.contains(&"financeData.downsampleOhlcv".to_string()));
        assert!(ids.contains(&"financeData.returns".to_string()));
        assert!(ids.contains(&"financeData.riskSummary".to_string()));
    }

    #[test]
    fn examples_return_structured_values() {
        for operation in [
            "describe",
            "financeData.bounds",
            "financeData.barsInRange",
            "financeData.downsampleOhlcv",
            "financeData.returns",
            "financeData.riskSummary",
        ] {
            let response = run(operation);
            assert_eq!(response.operation.as_str(), operation);
            assert_eq!(response.value["operation"], operation);
            assert!(response.value["title"].is_string());
            assert!(response.value["summary"].is_object());
            assert!(response.value["result"].is_object());
        }
    }
}
