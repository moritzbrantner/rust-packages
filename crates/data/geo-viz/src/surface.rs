//! Library-owned runtime surface for `geo-viz`.

use serde::Deserialize;
use video_analysis_core::runtime::{
    OperationId, PackageSurface, RuntimeCapabilities, SurfaceOperation, SurfaceRequest,
    SurfaceResponse,
};

use crate::{
    GeoFlowIndex, GeoJsonIndex, GeoPointIndex, GeoVizAggregationOptions, GeoVizFlow,
    GeoVizFlowOptions, GeoVizGeoJsonOptions, GeoVizHeatOptions, GeoVizPoint, GeoVizViewportQuery,
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
                "Describe geo visualization package",
                "Describes geo-viz operations.",
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
                "geoViz.heatViewport",
                "Geo heat viewport",
                "Returns weighted heat features for visible geographic points.",
                serde_json::json!({
                    "points": [{"id": "a", "longitude": 8.0, "latitude": 49.0, "metrics": {"weight": 2}}],
                    "query": {"bounds": [7.0, 48.0, 9.0, 50.0], "zoom": 8}
                }),
            ),
            operation(
                "geoViz.geoJsonViewport",
                "GeoJSON viewport",
                "Filters GeoJSON features for a viewport and returns a FeatureCollection.",
                serde_json::json!({
                    "geoJson": {"type": "FeatureCollection", "features": []},
                    "query": {"bounds": [7.0, 48.0, 9.0, 50.0], "zoom": 8}
                }),
            ),
            operation(
                "geoViz.flowViewport",
                "Geo flow viewport",
                "Filters and optionally aggregates geographic flows for a viewport.",
                serde_json::json!({
                    "flows": [{"from": [8.0, 49.0], "to": [9.0, 50.0], "metrics": {"weight": 2}}],
                    "query": {"bounds": [7.0, 48.0, 10.0, 51.0], "zoom": 8}
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
        "geoViz.heatViewport" => heat_value(parse_input(request.input)?)?,
        "geoViz.geoJsonViewport" => geojson_value(parse_input(request.input)?)?,
        "geoViz.flowViewport" => flow_value(parse_input(request.input)?)?,
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
struct HeatRequest {
    points: Vec<GeoVizPoint>,
    query: GeoVizViewportQuery,
    #[serde(default)]
    options: Option<GeoVizHeatOptions>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GeoJsonRequest {
    geo_json: serde_json::Value,
    query: GeoVizViewportQuery,
    #[serde(default)]
    options: GeoVizGeoJsonOptions,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FlowRequest {
    flows: Vec<GeoVizFlow>,
    query: GeoVizViewportQuery,
    #[serde(default)]
    options: GeoVizFlowOptions,
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

fn heat_value(request: HeatRequest) -> Result<serde_json::Value, String> {
    let index = GeoPointIndex::new(request.points, GeoVizAggregationOptions::default())
        .map_err(|error| error.to_string())?;
    serde_json::to_value(
        index
            .get_heat_features(
                request.query,
                request.options.unwrap_or(GeoVizHeatOptions {
                    radius_meters: None,
                    weight_metric: None,
                }),
            )
            .map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())
}

fn geojson_value(request: GeoJsonRequest) -> Result<serde_json::Value, String> {
    let index = GeoJsonIndex::new(request.geo_json).map_err(|error| error.to_string())?;
    serde_json::to_value(
        index
            .get_viewport_features(request.query, request.options)
            .map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())
}

fn flow_value(request: FlowRequest) -> Result<serde_json::Value, String> {
    let index = GeoFlowIndex::new(request.flows).map_err(|error| error.to_string())?;
    serde_json::to_value(
        index
            .get_viewport_flows(request.query, request.options)
            .map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())
}

fn resample_value(request: ResampleRequest) -> Result<serde_json::Value, String> {
    let coordinates = if request.closed {
        resample_ring_flat(&request.coordinates, request.coordinate_count)
    } else {
        resample_line_flat(&request.coordinates, request.coordinate_count)
    }?;

    Ok(serde_json::json!({
        "closed": request.closed,
        "coordinateCount": coordinates.len() / 2,
        "coordinates": coordinates
    }))
}

fn resample_line_flat(coordinates: &[f64], coordinate_count: usize) -> Result<Vec<f64>, String> {
    validate_flat_coordinates(coordinates, 2, "line coordinates")?;
    validate_coordinate_count(coordinate_count, 2)?;

    let source_count = coordinates.len() / 2;

    if source_count == coordinate_count {
        return Ok(coordinates.to_vec());
    }

    let distances = cumulative_distances(coordinates, false);
    let total_distance = *distances.last().unwrap_or(&0.0);

    if total_distance == 0.0 {
        return Ok(repeat_position(coordinates, coordinate_count));
    }

    let mut samples = Vec::with_capacity(coordinate_count * 2);
    let mut segment_index = 0;

    for index in 0..coordinate_count {
        if index == coordinate_count - 1 {
            samples.extend_from_slice(&coordinates[(source_count - 1) * 2..source_count * 2]);
            continue;
        }

        let target_distance = total_distance * index as f64 / (coordinate_count - 1) as f64;
        let sample = interpolate_along_path_from_segment(
            coordinates,
            &distances,
            target_distance,
            false,
            segment_index,
        );

        samples.extend_from_slice(&sample.position);
        segment_index = sample.segment_index;
    }

    Ok(samples)
}

fn resample_ring_flat(open_ring: &[f64], coordinate_count: usize) -> Result<Vec<f64>, String> {
    validate_flat_coordinates(open_ring, 3, "ring coordinates")?;
    validate_coordinate_count(coordinate_count, 3)?;

    let distances = cumulative_distances(open_ring, true);
    let total_distance = *distances.last().unwrap_or(&0.0);

    if total_distance == 0.0 {
        return Ok(repeat_position(open_ring, coordinate_count));
    }

    let mut samples = Vec::with_capacity(coordinate_count * 2);
    let mut segment_index = 0;

    for index in 0..coordinate_count {
        let target_distance = total_distance * index as f64 / coordinate_count as f64;
        let sample = interpolate_along_path_from_segment(
            open_ring,
            &distances,
            target_distance,
            true,
            segment_index,
        );

        samples.extend_from_slice(&sample.position);
        segment_index = sample.segment_index;
    }

    Ok(samples)
}

fn validate_flat_coordinates(
    coordinates: &[f64],
    min_points: usize,
    label: &str,
) -> Result<(), String> {
    if coordinates.len() < min_points * 2 {
        return Err(format!(
            "{label} must contain at least {min_points} positions"
        ));
    }

    if !coordinates.len().is_multiple_of(2) {
        return Err(format!("{label} length must be even"));
    }

    if coordinates.iter().any(|value| !value.is_finite()) {
        return Err(format!("{label} must be finite"));
    }

    Ok(())
}

fn validate_coordinate_count(coordinate_count: usize, minimum: usize) -> Result<(), String> {
    if coordinate_count < minimum {
        return Err(format!("coordinate count must be at least {minimum}"));
    }

    Ok(())
}

fn cumulative_distances(coordinates: &[f64], closed: bool) -> Vec<f64> {
    let point_count = coordinates.len() / 2;
    let segment_count = if closed { point_count } else { point_count - 1 };
    let mut distances = Vec::with_capacity(segment_count + 1);

    distances.push(0.0);

    for index in 0..segment_count {
        let start = position_at(coordinates, index);
        let end = position_at(coordinates, (index + 1) % point_count);
        let previous_distance = *distances.last().unwrap_or(&0.0);

        distances.push(previous_distance + distance(start, end));
    }

    distances
}

struct PathSample {
    position: [f64; 2],
    segment_index: usize,
}

fn interpolate_along_path_from_segment(
    coordinates: &[f64],
    distances: &[f64],
    target_distance: f64,
    closed: bool,
    start_segment_index: usize,
) -> PathSample {
    let point_count = coordinates.len() / 2;
    let segment_count = if closed { point_count } else { point_count - 1 };
    let target_distance = target_distance.clamp(0.0, *distances.last().unwrap_or(&0.0));

    for index in start_segment_index..segment_count {
        let segment_start_distance = distances[index];
        let segment_end_distance = distances[index + 1];

        if target_distance > segment_end_distance {
            continue;
        }

        let start = position_at(coordinates, index);
        let end = position_at(coordinates, (index + 1) % point_count);
        let segment_length = segment_end_distance - segment_start_distance;
        let progress = if segment_length == 0.0 {
            0.0
        } else {
            (target_distance - segment_start_distance) / segment_length
        };

        return PathSample {
            position: interpolate_position(start, end, progress),
            segment_index: index,
        };
    }

    PathSample {
        position: position_at(coordinates, point_count - 1),
        segment_index: segment_count.saturating_sub(1),
    }
}

fn repeat_position(coordinates: &[f64], coordinate_count: usize) -> Vec<f64> {
    let position = &coordinates[0..2];
    let mut samples = Vec::with_capacity(coordinate_count * 2);

    for _ in 0..coordinate_count {
        samples.extend_from_slice(position);
    }

    samples
}

fn position_at(coordinates: &[f64], index: usize) -> [f64; 2] {
    let offset = index * 2;

    [coordinates[offset], coordinates[offset + 1]]
}

fn interpolate_position(start: [f64; 2], end: [f64; 2], progress: f64) -> [f64; 2] {
    [
        start[0] + (end[0] - start[0]) * progress,
        start[1] + (end[1] - start[1]) * progress,
    ]
}

fn distance(start: [f64; 2], end: [f64; 2]) -> f64 {
    (end[0] - start[0]).hypot(end[1] - start[1])
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
        assert_eq!(surface.library, env!("CARGO_PKG_NAME"));
        assert!(surface
            .operations
            .iter()
            .any(|op| op.id.as_str() == "geoViz.aggregateViewport"));
    }
}
