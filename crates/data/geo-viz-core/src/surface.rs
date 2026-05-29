//! Library-owned runtime surface for `geo-viz-core`.

use serde::Deserialize;
use video_analysis_core::runtime::{
    OperationId, PackageSurface, RuntimeCapabilities, SurfaceOperation, SurfaceRequest,
    SurfaceResponse,
};

use crate::{GeoPointIndex, GeoVizAggregationOptions, GeoVizPoint, GeoVizViewportQuery};

/// Returns the package surface exposed by every transport wrapper.
pub fn package_surface() -> PackageSurface {
    PackageSurface {
        library: env!("CARGO_PKG_NAME").to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        capabilities: RuntimeCapabilities::pure_rust(),
        operations: vec![
            operation(
                "geoViz.describe",
                "Describe geo visualization package",
                "Describes geo-viz-core operations.",
                serde_json::json!({"includeOperations": true}),
            ),
            operation(
                "geoViz.bounds",
                "Geo visualization bounds",
                "Computes geographic bounds for finite lon/lat points.",
                serde_json::json!({"points": [{"longitude": 8.0, "latitude": 49.0}]}),
            ),
            operation(
                "geoViz.aggregateViewport",
                "Aggregate geo viewport",
                "Clusters points and returns renderer-agnostic viewport features.",
                serde_json::json!({
                    "points": [{"id": "a", "longitude": 8.0, "latitude": 49.0}],
                    "query": {"bounds": [7.0, 48.0, 9.0, 50.0], "zoom": 8}
                }),
            ),
            operation(
                "geoViz.resampleGeometry",
                "Resample map geometry",
                "Resamples flat 2D coordinates as an open line or closed ring.",
                serde_json::json!({
                    "coordinates": [0.0, 0.0, 10.0, 0.0],
                    "coordinateCount": 3,
                    "closed": false
                }),
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
        "geoViz.describe" | "describe" => describe_value(request.input),
        "geoViz.bounds" => bounds_value(parse_input(request.input)?)?,
        "geoViz.aggregateViewport" => aggregate_value(parse_input(request.input)?)?,
        "geoViz.resampleGeometry" => resample_value(parse_input(request.input)?)?,
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
struct BoundsRequest {
    points: Vec<GeoVizPoint>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AggregateRequest {
    points: Vec<GeoVizPoint>,
    query: GeoVizViewportQuery,
    #[serde(default)]
    options: GeoVizAggregationOptions,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ResampleRequest {
    coordinates: Vec<f64>,
    coordinate_count: usize,
    #[serde(default)]
    closed: bool,
}

fn bounds_value(request: BoundsRequest) -> Result<serde_json::Value, String> {
    let index = GeoPointIndex::new(request.points, GeoVizAggregationOptions::default())
        .map_err(|error| error.to_string())?;
    Ok(serde_json::json!({ "bounds": index.get_bounds() }))
}

fn aggregate_value(request: AggregateRequest) -> Result<serde_json::Value, String> {
    let index =
        GeoPointIndex::new(request.points, request.options).map_err(|error| error.to_string())?;
    serde_json::to_value(
        index
            .get_viewport_aggregation(request.query)
            .map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())
}

fn resample_value(request: ResampleRequest) -> Result<serde_json::Value, String> {
    let coordinates = if request.closed {
        maps_kernels_core::resample_ring_flat(&request.coordinates, request.coordinate_count)
    } else {
        maps_kernels_core::resample_line_flat(&request.coordinates, request.coordinate_count)
    }
    .map_err(|error| error.to_string())?;

    Ok(serde_json::json!({
        "closed": request.closed,
        "coordinateCount": coordinates.len() / 2,
        "coordinates": coordinates
    }))
}

fn parse_input<T: for<'de> Deserialize<'de>>(input: serde_json::Value) -> Result<T, String> {
    serde_json::from_value(input).map_err(|error| format!("invalid request: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn surface_exposes_geo_viz_operations() {
        let surface = package_surface();
        assert_eq!(surface.library, "geo-viz-core");
        assert!(surface
            .operations
            .iter()
            .any(|op| op.id.as_str() == "geoViz.aggregateViewport"));
    }
}
