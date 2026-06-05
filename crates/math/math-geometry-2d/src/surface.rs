//! Library-owned runtime surface for `math-geometry-2d`.

use runtime_core::{
    OperationId, PackageSurface, RuntimeCapabilities, SurfaceOperation, SurfaceRequest,
    SurfaceResponse,
};
use serde::Deserialize;

use crate::{Affine2, Bounds2f, LineSegment2, Point2f, Polygon2f, RectF32};

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
                "Shared 2D geometry contracts for multimodal image, video, and layout processing.",
                serde_json::json!({"includeOperations": true}),
            ),
            operation(
                "geometry.bounds",
                "Point bounds",
                "Computes 2D bounds, dimensions, and center for finite points.",
                serde_json::json!({"points": [[0.0, 1.0], [2.0, 3.0]]}),
            ),
            operation(
                "geometry.transform",
                "Transform points",
                "Applies an affine transform to finite 2D points and returns transformed bounds.",
                serde_json::json!({
                    "points": [[1.0, 2.0]],
                    "transform": {"m11": 1.0, "m12": 0.0, "m21": 0.0, "m22": 1.0, "tx": 2.0, "ty": 3.0}
                }),
            ),
            operation(
                "geometry.intersections",
                "Rectangle intersections",
                "Checks every rectangle pair and returns intersection rectangles when present.",
                serde_json::json!({"rects": [{"x": 0.0, "y": 0.0, "width": 2.0, "height": 2.0}]}),
            ),
            operation(
                "geometry.overlap",
                "Rectangle overlap",
                "Computes intersection area, IoU, and directional overlap ratios for two rectangles.",
                serde_json::json!({"left": {"x": 0.0, "y": 0.0, "width": 2.0, "height": 2.0}, "right": {"x": 1.0, "y": 1.0, "width": 2.0, "height": 2.0}}),
            ),
            operation(
                "geometry.segmentIntersection",
                "Segment intersection",
                "Computes the point and segment parameters for two finite 2D segment intersections.",
                serde_json::json!({"left": {"start": [0.0, 0.0], "end": [2.0, 2.0]}, "right": {"start": [0.0, 2.0], "end": [2.0, 0.0]}}),
            ),
            operation(
                "geometry.polygonSummary",
                "Polygon summary",
                "Reports area, winding, centroid, and bounds for a finite 2D polygon.",
                serde_json::json!({"points": [[0.0, 0.0], [2.0, 0.0], [2.0, 1.0], [0.0, 1.0]]}),
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
        "geometry.bounds" => bounds_value(parse_input(request.input)?)?,
        "geometry.transform" => transform_value(parse_input(request.input)?)?,
        "geometry.intersections" => intersections_value(parse_input(request.input)?)?,
        "geometry.overlap" => overlap_value(parse_input(request.input)?)?,
        "geometry.segmentIntersection" => segment_intersection_value(parse_input(request.input)?)?,
        "geometry.polygonSummary" => polygon_summary_value(parse_input(request.input)?)?,
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
struct PointsRequest {
    points: Vec<[f32; 2]>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TransformRequest {
    points: Vec<[f32; 2]>,
    transform: Affine2,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct IntersectionsRequest {
    rects: Vec<RectF32>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OverlapRequest {
    left: RectF32,
    right: RectF32,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SegmentEndpointRequest {
    start: [f32; 2],
    end: [f32; 2],
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SegmentIntersectionRequest {
    left: SegmentEndpointRequest,
    right: SegmentEndpointRequest,
}

fn bounds_value(request: PointsRequest) -> Result<serde_json::Value, String> {
    let points = points_from_arrays(request.points)?;
    bounds_json(bounds_for_points(&points)?)
}

fn transform_value(request: TransformRequest) -> Result<serde_json::Value, String> {
    request
        .transform
        .validate()
        .map_err(|error| error.to_string())?;
    let points = points_from_arrays(request.points)?;
    let transformed = points
        .into_iter()
        .map(|point| request.transform.apply_point(point))
        .collect::<Vec<_>>();
    let bounds = bounds_for_points(&transformed)?;
    Ok(serde_json::json!({
        "determinant": request.transform.determinant().map_err(|error| error.to_string())?,
        "points": transformed.iter().map(point_array).collect::<Vec<_>>(),
        "bounds": bounds_json(bounds)?
    }))
}

fn intersections_value(request: IntersectionsRequest) -> Result<serde_json::Value, String> {
    if request.rects.len() > MAX_VALUES {
        return Err(format!("rects must not exceed {MAX_VALUES}"));
    }
    for rect in &request.rects {
        rect.validate().map_err(|error| error.to_string())?;
    }
    let mut pairs = Vec::new();
    for left_index in 0..request.rects.len() {
        for right_index in (left_index + 1)..request.rects.len() {
            let left = request.rects[left_index];
            let right = request.rects[right_index];
            let intersection = left
                .intersection(right)
                .map_err(|error| error.to_string())?;
            pairs.push(serde_json::json!({
                "left": left_index,
                "right": right_index,
                "intersects": intersection.is_some(),
                "intersection": intersection.map(rect_json)
            }));
        }
    }
    Ok(serde_json::json!({
        "rectCount": request.rects.len(),
        "pairs": pairs
    }))
}

fn overlap_value(request: OverlapRequest) -> Result<serde_json::Value, String> {
    request.left.validate().map_err(|error| error.to_string())?;
    request
        .right
        .validate()
        .map_err(|error| error.to_string())?;
    let intersection = request
        .left
        .intersection(request.right)
        .map_err(|error| error.to_string())?;
    let intersection_area = intersection
        .map(|rect| rect.area())
        .transpose()
        .map_err(|error| error.to_string())?
        .unwrap_or(0.0);
    Ok(serde_json::json!({
        "intersects": intersection.is_some(),
        "intersection": intersection.map(rect_json),
        "intersectionArea": intersection_area,
        "iou": request.left.iou(request.right).map_err(|error| error.to_string())?,
        "leftOverlapRatio": request.left.overlap_ratio(request.right).map_err(|error| error.to_string())?,
        "rightOverlapRatio": request.right.overlap_ratio(request.left).map_err(|error| error.to_string())?
    }))
}

fn segment_intersection_value(
    request: SegmentIntersectionRequest,
) -> Result<serde_json::Value, String> {
    let left = segment_from_request(request.left)?;
    let right = segment_from_request(request.right)?;
    let intersection = left
        .intersection(right)
        .map_err(|error| error.to_string())?;
    Ok(serde_json::json!({
        "intersects": intersection.is_some(),
        "intersection": intersection.map(|value| serde_json::json!({
            "point": point_array(&value.point),
            "leftT": value.left_t,
            "rightT": value.right_t
        }))
    }))
}

fn polygon_summary_value(request: PointsRequest) -> Result<serde_json::Value, String> {
    let polygon =
        Polygon2f::new(points_from_arrays(request.points)?).map_err(|error| error.to_string())?;
    Ok(serde_json::json!({
        "pointCount": polygon.points().len(),
        "area": polygon.area().map_err(|error| error.to_string())?,
        "signedArea": polygon.signed_area().map_err(|error| error.to_string())?,
        "clockwise": polygon.is_clockwise().map_err(|error| error.to_string())?,
        "centroid": point_array(&polygon.centroid().map_err(|error| error.to_string())?),
        "bounds": bounds_json(polygon.bounds().map_err(|error| error.to_string())?)?
    }))
}

fn points_from_arrays(points: Vec<[f32; 2]>) -> Result<Vec<Point2f>, String> {
    if points.is_empty() {
        return Err("points must not be empty".to_string());
    }
    if points.len() > MAX_VALUES {
        return Err(format!("points must not exceed {MAX_VALUES}"));
    }
    points
        .into_iter()
        .map(|point| Point2f::new(point[0], point[1]))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())
}

fn bounds_for_points(points: &[Point2f]) -> Result<Bounds2f, String> {
    Bounds2f::from_points(points).map_err(|error| error.to_string())
}

fn bounds_json(bounds: Bounds2f) -> Result<serde_json::Value, String> {
    Ok(serde_json::json!({
        "min": point_array(&bounds.min),
        "max": point_array(&bounds.max),
        "width": bounds.width(),
        "height": bounds.height(),
        "center": point_array(&bounds.center().map_err(|error| error.to_string())?)
    }))
}

fn segment_from_request(request: SegmentEndpointRequest) -> Result<LineSegment2, String> {
    LineSegment2::new(
        Point2f::new(request.start[0], request.start[1]).map_err(|error| error.to_string())?,
        Point2f::new(request.end[0], request.end[1]).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())
}

fn point_array(point: &Point2f) -> [f32; 2] {
    [point.x, point.y]
}

fn rect_json(rect: RectF32) -> serde_json::Value {
    serde_json::json!({
        "x": rect.x,
        "y": rect.y,
        "width": rect.width,
        "height": rect.height
    })
}

fn parse_input<T: for<'de> Deserialize<'de>>(input: serde_json::Value) -> Result<T, String> {
    serde_json::from_value(input).map_err(|error| format!("invalid request: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounds_reports_dimensions_and_center() {
        let response = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("geometry.bounds"),
            input: serde_json::json!({"points": [[0.0, 1.0], [2.0, 5.0]]}),
        })
        .expect("bounds operation");

        assert_eq!(response.value["min"], serde_json::json!([0.0, 1.0]));
        assert_eq!(response.value["max"], serde_json::json!([2.0, 5.0]));
        assert_eq!(response.value["center"], serde_json::json!([1.0, 3.0]));
    }

    #[test]
    fn transform_applies_affine() {
        let response = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("geometry.transform"),
            input: serde_json::json!({
                "points": [[1.0, 2.0]],
                "transform": {"m11": 2.0, "m12": 0.0, "m21": 0.0, "m22": 3.0, "tx": 1.0, "ty": -1.0}
            }),
        })
        .expect("transform operation");

        assert_eq!(response.value["determinant"], 6.0);
        assert_eq!(response.value["points"], serde_json::json!([[3.0, 5.0]]));
    }

    #[test]
    fn intersections_reports_pairs() {
        let response = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("geometry.intersections"),
            input: serde_json::json!({"rects": [
                {"x": 0.0, "y": 0.0, "width": 2.0, "height": 2.0},
                {"x": 1.0, "y": 1.0, "width": 2.0, "height": 2.0}
            ]}),
        })
        .expect("intersections operation");

        assert_eq!(response.value["rectCount"], 2);
        assert_eq!(response.value["pairs"][0]["intersects"], true);
    }

    #[test]
    fn overlap_segment_and_polygon_operations_run() {
        for operation in [
            "geometry.overlap",
            "geometry.segmentIntersection",
            "geometry.polygonSummary",
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
