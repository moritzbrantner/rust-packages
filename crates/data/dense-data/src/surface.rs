//! Library-owned runtime surface for `dense-data`.

use serde::Deserialize;
use video_analysis_core::runtime::{
    OperationId, PackageSurface, RuntimeCapabilities, SurfaceOperation, SurfaceRequest,
    SurfaceResponse,
};

use crate::{
    BucketGrid, DenseBounds, DensePoint, KMeansConfig, NumericSeriesBinQuery, NumericSeriesIndex,
    NumericSeriesPoint,
};

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
                "Deterministic dense point datasets, bucketing, and clustering for video-analysis.",
                serde_json::json!({
                    "type": "object",
                    "additionalProperties": true
                }),
                serde_json::json!({
                    "type": "object",
                    "required": ["library", "version", "operationCount"]
                }),
                serde_json::json!({
                    "includeOperations": true
                }),
            ),
            operation(
                "summarizeDensePoints",
                "Summarize dense points",
                "Returns weighted per-dimension stats, bounds, and optional scalar value stats.",
                serde_json::json!({
                    "type": "object",
                    "required": ["points"],
                    "properties": {
                        "points": { "type": "array" }
                    }
                }),
                serde_json::json!({
                    "type": "object",
                    "required": ["count", "dimensions", "coordinateStats", "bounds"]
                }),
                serde_json::json!({
                    "points": [
                        { "coordinates": [0, 1], "weight": 1 },
                        { "coordinates": [2, 3], "weight": 2, "value": 4 }
                    ]
                }),
            ),
            operation(
                "bucketDensePoints",
                "Bucket dense points",
                "Groups dense points into deterministic fixed-grid buckets.",
                serde_json::json!({
                    "type": "object",
                    "required": ["points", "grid"],
                    "properties": {
                        "points": { "type": "array" },
                        "grid": { "type": "object" }
                    }
                }),
                serde_json::json!({
                    "type": "object",
                    "required": ["buckets"]
                }),
                serde_json::json!({
                    "points": [{ "coordinates": [0.2, 1.1] }],
                    "grid": { "dimensions": 2, "cellSize": 1 }
                }),
            ),
            operation(
                "clusterDensePoints",
                "Cluster dense points",
                "Runs deterministic weighted k-means over dense points.",
                serde_json::json!({
                    "type": "object",
                    "required": ["points", "clusters"],
                    "properties": {
                        "points": { "type": "array" },
                        "clusters": { "type": "integer", "minimum": 1 }
                    }
                }),
                serde_json::json!({
                    "type": "object",
                    "required": ["iterations", "clusters"]
                }),
                serde_json::json!({
                    "points": [{ "coordinates": [0, 0] }, { "coordinates": [1, 1] }],
                    "clusters": 2
                }),
            ),
            operation(
                "binNumericSeries",
                "Bin numeric series",
                "Bins numeric points into display-sized summaries with metric totals.",
                serde_json::json!({
                    "type": "object",
                    "required": ["points", "xDomain", "targetBinCount"],
                    "properties": {
                        "points": { "type": "array" },
                        "xDomain": { "type": "array", "minItems": 2, "maxItems": 2 },
                        "targetBinCount": { "type": "integer", "minimum": 1 }
                    }
                }),
                serde_json::json!({
                    "type": "object",
                    "required": ["bins"]
                }),
                serde_json::json!({
                    "points": [{ "index": 0, "x": 0, "y": 2, "metrics": { "count": 1 } }],
                    "xDomain": [0, 10],
                    "targetBinCount": 5
                }),
            ),
        ],
    }
}

/// Runs one library-owned operation.
pub fn run_surface_operation(request: SurfaceRequest) -> Result<SurfaceResponse, String> {
    match request.operation.as_str() {
        "describe" => {
            let surface = package_surface();
            Ok(SurfaceResponse {
                operation: request.operation,
                value: serde_json::json!({
                    "library": surface.library,
                    "version": surface.version,
                    "operationCount": surface.operations.len(),
                    "operations": surface
                        .operations
                        .iter()
                        .map(|operation| operation.id.as_str())
                        .collect::<Vec<_>>(),
                    "input": request.input
                }),
                diagnostics: Vec::new(),
                artifacts: Vec::new(),
            })
        }
        "summarizeDensePoints" => {
            let input: DensePointsInput =
                serde_json::from_value(request.input).map_err(|error| error.to_string())?;
            let points = parse_dense_points(input.points)?;
            let summary = crate::dense_summary(&points).map_err(|error| error.to_string())?;
            Ok(surface_response(
                request.operation,
                dense_summary_value(&summary),
            ))
        }
        "bucketDensePoints" => {
            let input: BucketDensePointsInput =
                serde_json::from_value(request.input).map_err(|error| error.to_string())?;
            let points = parse_dense_points(input.points)?;
            let grid = input.grid.into_bucket_grid()?;
            let buckets =
                crate::bucket_points(&points, &grid).map_err(|error| error.to_string())?;
            Ok(surface_response(
                request.operation,
                serde_json::json!({
                    "buckets": buckets
                        .iter()
                        .map(|bucket| {
                            serde_json::json!({
                                "key": { "indices": bucket.key.indices },
                                "count": bucket.count,
                                "weightSum": bucket.weight_sum,
                                "averages": dense_averages_value(&bucket.averages),
                                "bounds": dense_bounds_value(&bucket.bounds),
                                "pointIndices": bucket.point_indices
                            })
                        })
                        .collect::<Vec<_>>()
                }),
            ))
        }
        "clusterDensePoints" => {
            let input: ClusterDensePointsInput =
                serde_json::from_value(request.input).map_err(|error| error.to_string())?;
            let points = parse_dense_points(input.points)?;
            let mut config =
                KMeansConfig::new(input.clusters).map_err(|error| error.to_string())?;
            if let Some(max_iterations) = input.max_iterations {
                config.max_iterations = max_iterations;
            }
            if let Some(tolerance) = input.tolerance {
                config.tolerance = tolerance;
            }
            config.validate().map_err(|error| error.to_string())?;
            let result = crate::k_means(&points, config).map_err(|error| error.to_string())?;
            Ok(surface_response(
                request.operation,
                serde_json::json!({
                    "iterations": result.iterations,
                    "clusters": result
                        .clusters
                        .iter()
                        .map(|cluster| {
                            serde_json::json!({
                                "clusterIndex": cluster.cluster_index,
                                "centroid": cluster.centroid,
                                "count": cluster.count,
                                "weightSum": cluster.weight_sum,
                                "averages": cluster.averages.as_ref().map(dense_averages_value),
                                "bounds": cluster.bounds.as_ref().map(dense_bounds_value),
                                "pointIndices": cluster.point_indices
                            })
                        })
                        .collect::<Vec<_>>()
                }),
            ))
        }
        "binNumericSeries" => {
            let input: BinNumericSeriesInput =
                serde_json::from_value(request.input).map_err(|error| error.to_string())?;
            Ok(surface_response(
                request.operation,
                bin_numeric_series(input)?,
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
struct DensePointInput {
    id: Option<String>,
    coordinates: Vec<f64>,
    weight: Option<f64>,
    value: Option<f64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DensePointsInput {
    points: Vec<DensePointInput>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BucketDensePointsInput {
    points: Vec<DensePointInput>,
    grid: BucketGridInput,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BucketGridInput {
    dimensions: usize,
    cell_size: Option<f64>,
    origin: Option<Vec<f64>>,
    widths: Option<Vec<f64>>,
}

impl BucketGridInput {
    fn into_bucket_grid(self) -> Result<BucketGrid, String> {
        let origin = self.origin.unwrap_or_else(|| vec![0.0; self.dimensions]);
        let widths = match (self.widths, self.cell_size) {
            (Some(widths), _) => widths,
            (None, Some(cell_size)) => vec![cell_size; self.dimensions],
            (None, None) => return Err("bucket grid requires `cellSize` or `widths`".to_string()),
        };
        BucketGrid::new(origin, widths).map_err(|error| error.to_string())
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ClusterDensePointsInput {
    points: Vec<DensePointInput>,
    clusters: usize,
    max_iterations: Option<usize>,
    tolerance: Option<f64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BinNumericSeriesInput {
    points: Vec<NumericSeriesKernelPoint>,
    x_domain: [f64; 2],
    target_bin_count: usize,
    include_empty_bins: Option<bool>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct NumericSeriesKernelPoint {
    index: usize,
    x: f64,
    y: f64,
    metrics: Option<std::collections::BTreeMap<String, f64>>,
}

fn operation(
    id: &str,
    name: &str,
    description: &str,
    input_schema: serde_json::Value,
    output_schema: serde_json::Value,
    example_request: serde_json::Value,
) -> SurfaceOperation {
    SurfaceOperation {
        id: OperationId::new(id),
        name: name.to_string(),
        description: Some(description.to_string()),
        input_schema,
        output_schema,
        example_request,
        wasm_supported: true,
        server_supported: true,
    }
}

fn surface_response(operation: OperationId, value: serde_json::Value) -> SurfaceResponse {
    SurfaceResponse {
        operation,
        value,
        diagnostics: Vec::new(),
        artifacts: Vec::new(),
    }
}

fn parse_dense_points(inputs: Vec<DensePointInput>) -> Result<Vec<DensePoint>, String> {
    inputs
        .into_iter()
        .map(|input| {
            let mut point =
                DensePoint::new(input.coordinates).map_err(|error| error.to_string())?;
            if let Some(id) = input.id {
                point = point.named(id);
            }
            if let Some(weight) = input.weight {
                point = point.weighted(weight).map_err(|error| error.to_string())?;
            }
            if let Some(value) = input.value {
                point = point.valued(value).map_err(|error| error.to_string())?;
            }
            Ok(point)
        })
        .collect()
}

fn bin_numeric_series(input: BinNumericSeriesInput) -> Result<serde_json::Value, String> {
    let index = NumericSeriesIndex::from_points(
        input
            .points
            .into_iter()
            .map(|point| NumericSeriesPoint {
                source_index: point.index,
                x: point.x,
                y: point.y,
                metrics: point.metrics.unwrap_or_default(),
            })
            .collect(),
    )
    .map_err(|error| error.to_string())?;
    let result = index
        .bin(NumericSeriesBinQuery {
            x_domain: input.x_domain,
            target_bin_count: input.target_bin_count,
            include_empty_bins: input.include_empty_bins.unwrap_or(false),
        })
        .map_err(|error| error.to_string())?;

    serde_json::to_value(result).map_err(|error| error.to_string())
}

fn dense_summary_value(summary: &crate::DenseSummary) -> serde_json::Value {
    serde_json::json!({
        "count": summary.count,
        "dimensions": summary.dimensions,
        "weightSum": summary.weight_sum,
        "coordinateStats": summary
            .coordinate_stats
            .iter()
            .map(number_summary_value)
            .collect::<Vec<_>>(),
        "valueStats": summary.value_stats.as_ref().map(number_summary_value),
        "bounds": dense_bounds_value(&summary.bounds)
    })
}

fn dense_averages_value(averages: &crate::DenseAverages) -> serde_json::Value {
    serde_json::json!({
        "count": averages.count,
        "weightSum": averages.weight_sum,
        "coordinates": averages.coordinates,
        "valueCount": averages.value_count,
        "valueWeightSum": averages.value_weight_sum,
        "value": averages.value
    })
}

fn dense_bounds_value(bounds: &DenseBounds) -> serde_json::Value {
    serde_json::json!({
        "min": bounds.min,
        "max": bounds.max
    })
}

fn number_summary_value(summary: &numbers_core::NumberSummary) -> serde_json::Value {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn package_surface_has_describe_operation() {
        let surface = package_surface();
        assert_eq!(surface.library, env!("CARGO_PKG_NAME"));
        assert!(!surface.operations.is_empty());
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
    fn dense_operations_return_computed_values() {
        let summary = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("summarizeDensePoints"),
            input: serde_json::json!({
                "points": [
                    { "id": "a", "coordinates": [0.0, 1.0], "weight": 1.0 },
                    { "id": "b", "coordinates": [2.0, 3.0], "weight": 2.0, "value": 4.0 }
                ]
            }),
        })
        .expect("summary operation");

        assert_eq!(summary.value["count"], 2);
        assert_eq!(summary.value["dimensions"], 2);

        let buckets = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("bucketDensePoints"),
            input: serde_json::json!({
                "points": [
                    { "coordinates": [0.2, 1.9] },
                    { "coordinates": [1.1, 1.2] },
                    { "coordinates": [-0.1, 0.2] }
                ],
                "grid": { "dimensions": 2, "cellSize": 1.0 }
            }),
        })
        .expect("bucket operation");

        assert_eq!(buckets.value["buckets"].as_array().unwrap().len(), 3);

        let clusters = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("clusterDensePoints"),
            input: serde_json::json!({
                "points": [
                    { "coordinates": [0.0, 0.0] },
                    { "coordinates": [0.2, 0.1] },
                    { "coordinates": [9.0, 9.0] },
                    { "coordinates": [10.0, 10.0] }
                ],
                "clusters": 2
            }),
        })
        .expect("cluster operation");

        assert_eq!(clusters.value["clusters"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn numeric_series_bins_with_metrics() {
        let response = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("binNumericSeries"),
            input: serde_json::json!({
                "points": [
                    { "index": 0, "x": 0.0, "y": 2.0, "metrics": { "value": 1.0 } },
                    { "index": 1, "x": 1.0, "y": 4.0, "metrics": { "value": 3.0 } },
                    { "index": 2, "x": 8.0, "y": 6.0, "metrics": { "value": 5.0 } }
                ],
                "xDomain": [0.0, 10.0],
                "targetBinCount": 2,
                "includeEmptyBins": true
            }),
        })
        .expect("bin operation");

        let bins = response.value["bins"].as_array().unwrap();
        assert_eq!(bins.len(), 2);
        assert_eq!(bins[0]["pointCount"], 2);
        assert_eq!(bins[0]["metrics"]["value"], 4.0);
        assert_eq!(bins[1]["firstPointIndex"], 2);
    }
}
