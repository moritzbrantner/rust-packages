//! Library-owned runtime surface for `maps-kernels-core`.

use serde::Deserialize;
use video_analysis_core::runtime::{
    OperationId, PackageSurface, RuntimeCapabilities, SurfaceOperation, SurfaceRequest,
    SurfaceResponse,
};

use crate::{resample_line_flat, resample_ring_flat};

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
                "Numeric kernels for map and temporal GeoJSON processing.",
                serde_json::json!({"includeOperations": true}),
            ),
            operation(
                "maps.kernelSummary",
                "Map kernel summary",
                "Summarizes flat 2D coordinates as an open line or closed ring.",
                serde_json::json!({"coordinates": [0.0, 0.0, 1.0, 0.0], "closed": false}),
            ),
            operation(
                "maps.applyKernel",
                "Apply map kernel",
                "Resamples flat 2D coordinates as an open line or closed ring.",
                serde_json::json!({"coordinates": [0.0, 0.0, 10.0, 0.0], "coordinateCount": 3, "closed": false}),
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
        "maps.kernelSummary" => summary_value(parse_input(request.input)?)?,
        "maps.applyKernel" => apply_value(parse_input(request.input)?)?,
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
struct SummaryRequest {
    coordinates: Vec<f64>,
    #[serde(default)]
    closed: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApplyRequest {
    coordinates: Vec<f64>,
    coordinate_count: usize,
    #[serde(default)]
    closed: bool,
}

fn summary_value(request: SummaryRequest) -> Result<serde_json::Value, String> {
    validate_coordinates(&request.coordinates, request.closed)?;
    let coordinate_count = request.coordinates.len() / 2;
    let segment_count = if request.closed {
        coordinate_count
    } else {
        coordinate_count - 1
    };
    let bbox = bbox(&request.coordinates);
    Ok(serde_json::json!({
        "coordinateCount": coordinate_count,
        "closed": request.closed,
        "segmentCount": segment_count,
        "totalLength": total_length(&request.coordinates, request.closed),
        "bbox": bbox
    }))
}

fn apply_value(request: ApplyRequest) -> Result<serde_json::Value, String> {
    validate_coordinates(&request.coordinates, request.closed)?;
    let coordinates = if request.closed {
        resample_ring_flat(&request.coordinates, request.coordinate_count)
    } else {
        resample_line_flat(&request.coordinates, request.coordinate_count)
    }
    .map_err(|error| error.to_string())?;
    Ok(serde_json::json!({
        "closed": request.closed,
        "coordinateCount": coordinates.len() / 2,
        "coordinates": coordinates
    }))
}

fn validate_coordinates(coordinates: &[f64], closed: bool) -> Result<(), String> {
    if coordinates.len() > MAX_VALUES * 2 {
        return Err(format!("coordinates must not exceed {}", MAX_VALUES * 2));
    }
    if coordinates.len() < if closed { 6 } else { 4 } {
        return Err(if closed {
            "closed coordinates must contain at least three points".to_string()
        } else {
            "open coordinates must contain at least two points".to_string()
        });
    }
    if !coordinates.len().is_multiple_of(2) {
        return Err("coordinates length must be even".to_string());
    }
    if coordinates.iter().any(|value| !value.is_finite()) {
        return Err("coordinates must be finite".to_string());
    }
    Ok(())
}

fn bbox(coordinates: &[f64]) -> [f64; 4] {
    let mut min_x = coordinates[0];
    let mut min_y = coordinates[1];
    let mut max_x = coordinates[0];
    let mut max_y = coordinates[1];
    for point in coordinates.chunks_exact(2).skip(1) {
        min_x = min_x.min(point[0]);
        min_y = min_y.min(point[1]);
        max_x = max_x.max(point[0]);
        max_y = max_y.max(point[1]);
    }
    [min_x, min_y, max_x, max_y]
}

fn total_length(coordinates: &[f64], closed: bool) -> f64 {
    let point_count = coordinates.len() / 2;
    let segment_count = if closed { point_count } else { point_count - 1 };
    (0..segment_count)
        .map(|index| {
            let next = (index + 1) % point_count;
            let x0 = coordinates[index * 2];
            let y0 = coordinates[index * 2 + 1];
            let x1 = coordinates[next * 2];
            let y1 = coordinates[next * 2 + 1];
            (x1 - x0).hypot(y1 - y0)
        })
        .sum()
}

fn parse_input<T: for<'de> Deserialize<'de>>(input: serde_json::Value) -> Result<T, String> {
    serde_json::from_value(input).map_err(|error| format!("invalid request: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn summary_reports_lengths_and_bbox() {
        let response = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("maps.kernelSummary"),
            input: serde_json::json!({"coordinates": [0.0, 0.0, 3.0, 4.0], "closed": false}),
        })
        .expect("summary");
        assert_eq!(response.value["coordinateCount"], 2);
        assert_eq!(response.value["totalLength"], 5.0);
        assert_eq!(
            response.value["bbox"],
            serde_json::json!([0.0, 0.0, 3.0, 4.0])
        );
    }

    #[test]
    fn apply_kernel_resamples_line() {
        let response = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("maps.applyKernel"),
            input: serde_json::json!({"coordinates": [0.0, 0.0, 10.0, 0.0], "coordinateCount": 3}),
        })
        .expect("apply");
        assert_eq!(
            response.value["coordinates"],
            serde_json::json!([0.0, 0.0, 5.0, 0.0, 10.0, 0.0])
        );
    }
}
