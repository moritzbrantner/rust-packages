//! Library-owned runtime surface for `numbers-core`.

use serde::Deserialize;
use video_analysis_core::runtime::{
    OperationId, PackageSurface, RuntimeCapabilities, SurfaceOperation, SurfaceRequest,
    SurfaceResponse,
};

use crate::{histogram, quantile, quartiles, summarize_numbers, HistogramConfig, NumberRange};

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
                "Deterministic scalar numeric summaries, quantiles, ranges, and histograms for video-analysis.",
                serde_json::json!({"includeOperations": true}),
            ),
            operation(
                "numbers.summary",
                "Number summary",
                "Returns finite/non-finite counts and weighted scalar summary statistics.",
                serde_json::json!({"values": [1.0, 2.0, 3.0, 4.0]}),
            ),
            operation(
                "numbers.histogram",
                "Number histogram",
                "Builds fixed-width histogram bins over finite numeric values.",
                serde_json::json!({"values": [0.0, 0.5, 1.0], "bins": 2}),
            ),
            operation(
                "numbers.quantiles",
                "Number quantiles",
                "Returns interpolated quantiles and quartile summary values.",
                serde_json::json!({"values": [1.0, 2.0, 3.0, 4.0], "quantiles": [0.25, 0.5, 0.75]}),
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
        "numbers.summary" => summary_value(parse_input(request.input)?)?,
        "numbers.histogram" => histogram_value(parse_input(request.input)?)?,
        "numbers.quantiles" => quantiles_value(parse_input(request.input)?)?,
        operation => {
            return Err(format!(
                "unsupported operation `{operation}` for {}",
                env!("CARGO_PKG_NAME")
            ));
        }
    };
    Ok(response(operation, value))
}

fn response(operation: OperationId, value: serde_json::Value) -> SurfaceResponse {
    SurfaceResponse {
        operation,
        value,
        diagnostics: Vec::new(),
        artifacts: Vec::new(),
    }
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

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SummaryRequest {
    values: Vec<f64>,
    #[serde(default)]
    weights: Option<Vec<f64>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct HistogramRequest {
    values: Vec<f64>,
    #[serde(default = "default_bins")]
    bins: usize,
    #[serde(default)]
    range: Option<RangeRequest>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct QuantilesRequest {
    values: Vec<f64>,
    #[serde(default = "default_quantiles")]
    quantiles: Vec<f64>,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RangeRequest {
    min: f64,
    max: f64,
}

fn summary_value(request: SummaryRequest) -> Result<serde_json::Value, String> {
    if let Some(weights) = request.weights {
        if weights.len() != request.values.len() {
            return Err("weights length must match values length".to_string());
        }
        let mut stats = crate::RunningStats::new();
        for (value, weight) in request.values.into_iter().zip(weights) {
            stats
                .push_weighted(value, weight)
                .map_err(|error| error.to_string())?;
        }
        Ok(summary_json(stats.summary()))
    } else {
        Ok(summary_json(summarize_numbers(&request.values)))
    }
}

fn summary_json(summary: crate::NumberSummary) -> serde_json::Value {
    serde_json::json!({
        "count": summary.count,
        "finiteCount": summary.finite_count,
        "nonFiniteCount": summary.non_finite_count,
        "min": summary.min,
        "max": summary.max,
        "sum": summary.sum,
        "mean": summary.mean,
        "variance": summary.variance,
        "stdDev": summary.std_dev,
        "weightSum": summary.weight_sum
    })
}

fn histogram_value(request: HistogramRequest) -> Result<serde_json::Value, String> {
    let mut config = HistogramConfig::new(request.bins).map_err(|error| error.to_string())?;
    if let Some(range) = request.range {
        config = config
            .with_range(NumberRange::new(range.min, range.max).map_err(|error| error.to_string())?);
    }
    let histogram = histogram(&request.values, config).map_err(|error| error.to_string())?;
    Ok(serde_json::json!({
        "count": histogram.count,
        "min": histogram.min,
        "max": histogram.max,
        "bins": histogram.bins.into_iter().map(|bin| serde_json::json!({
            "start": bin.start,
            "end": bin.end,
            "count": bin.count
        })).collect::<Vec<_>>()
    }))
}

fn quantiles_value(request: QuantilesRequest) -> Result<serde_json::Value, String> {
    let quantile_values = request
        .quantiles
        .iter()
        .map(|level| {
            quantile(&request.values, *level)
                .map(|value| serde_json::json!({"quantile": level, "value": value}))
                .map_err(|error| error.to_string())
        })
        .collect::<Result<Vec<_>, _>>()?;
    let quartiles = quartiles(&request.values).map_err(|error| error.to_string())?;
    Ok(serde_json::json!({
        "quantiles": quantile_values,
        "quartiles": {
            "q1": quartiles.q1,
            "median": quartiles.median,
            "q3": quartiles.q3,
            "iqr": quartiles.iqr
        }
    }))
}

fn parse_input<T: for<'de> Deserialize<'de>>(input: serde_json::Value) -> Result<T, String> {
    serde_json::from_value(input).map_err(|error| format!("invalid request: {error}"))
}

fn default_bins() -> usize {
    10
}

fn default_quantiles() -> Vec<f64> {
    vec![0.25, 0.5, 0.75]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn package_surface_lists_number_operations() {
        let ids = package_surface()
            .operations
            .into_iter()
            .map(|operation| operation.id.0)
            .collect::<Vec<_>>();

        assert!(ids.contains(&"numbers.summary".to_string()));
        assert!(ids.contains(&"numbers.histogram".to_string()));
        assert!(ids.contains(&"numbers.quantiles".to_string()));
    }

    #[test]
    fn describe_operation_returns_surface_summary() {
        let response = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("describe"),
            input: serde_json::json!({"includeOperations": true}),
        })
        .expect("describe operation");

        assert_eq!(response.operation.as_str(), "describe");
        assert_eq!(response.value["library"], env!("CARGO_PKG_NAME"));
    }

    #[test]
    fn summary_operation_returns_counts_and_mean() {
        let response = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("numbers.summary"),
            input: serde_json::json!({"values": [1.0, 2.0, 3.0]}),
        })
        .expect("summary operation");

        assert_eq!(response.value["count"], 3);
        assert_eq!(response.value["finiteCount"], 3);
        assert_eq!(response.value["mean"], 2.0);
    }

    #[test]
    fn histogram_operation_returns_bins() {
        let response = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("numbers.histogram"),
            input: serde_json::json!({"values": [0.0, 0.5, 1.0], "bins": 2}),
        })
        .expect("histogram operation");

        assert_eq!(response.value["count"], 3);
        assert_eq!(response.value["bins"][0]["count"], 1);
        assert_eq!(response.value["bins"][1]["count"], 2);
    }

    #[test]
    fn quantiles_operation_returns_interpolated_values() {
        let response = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("numbers.quantiles"),
            input: serde_json::json!({"values": [1.0, 2.0, 3.0, 4.0], "quantiles": [0.5]}),
        })
        .expect("quantiles operation");

        assert_eq!(response.value["quantiles"][0]["value"], 2.5);
        assert_eq!(response.value["quartiles"]["q1"], 1.75);
    }

    #[test]
    fn invalid_request_parsing_is_reported() {
        let error = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("numbers.summary"),
            input: serde_json::json!({"values": "not an array"}),
        })
        .expect_err("invalid request");

        assert!(error.contains("invalid request"));
    }

    #[test]
    fn unsupported_operation_is_reported() {
        let error = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("numbers.missing"),
            input: serde_json::json!({}),
        })
        .expect_err("unsupported operation");

        assert!(error.contains("unsupported operation `numbers.missing`"));
    }
}
