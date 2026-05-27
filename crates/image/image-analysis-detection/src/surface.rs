//! Library-owned runtime surface for `image-analysis-detection`.

use image_analysis_core::{ImagePixelFormat, ImageView};
use serde::de::DeserializeOwned;
use serde::Deserialize;
use video_analysis_core::runtime::{
    OperationId, PackageSurface, RuntimeCapabilities, SurfaceOperation, SurfaceRequest,
    SurfaceResponse,
};

use video_analysis_core::BoundingBox;

use crate::{ColorBlobDetectionOptions, ColorBlobDetector, FaceDetectionPreset, TargetColor};

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
                "Canonical image detection types and mask-proposal adapters for video-analysis.",
                serde_json::json!({"includeOperations": true}),
            ),
            operation(
                "image.detection.colorBlob",
                "Color blob detection",
                "Detects connected red-dominant color blobs in an in-memory image.",
                serde_json::json!({"image": sample_image_json(), "targetColor": "red", "minAreaPixels": 1, "mergeAdjacent": true}),
            ),
            operation(
                "image.detection.models",
                "Detection models",
                "Returns deterministic detection model catalog and default specs without running detectors.",
                serde_json::json!({}),
            ),
            operation(
                "image.detection.boxSummary",
                "Detection box summary",
                "Validates imported bounding boxes and detections, then returns aggregate score and union bounds.",
                serde_json::json!({"detections": [{"label": "face", "score": 0.9, "region": {"x": 1, "y": 2, "width": 3, "height": 4}}]}),
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
        "image.detection.colorBlob" => color_blob_value(parse_input(request.input)?)?,
        "image.detection.models" => models_value(),
        "image.detection.boxSummary" => box_summary_value(parse_input(request.input)?)?,
        operation => {
            return Err(format!(
                "unsupported operation `{operation}` for {}",
                env!("CARGO_PKG_NAME")
            ))
        }
    };
    Ok(surface_response(operation, value))
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

fn surface_response(operation: OperationId, value: serde_json::Value) -> SurfaceResponse {
    SurfaceResponse {
        operation,
        value,
        diagnostics: Vec::new(),
        artifacts: Vec::new(),
    }
}

fn parse_input<T: DeserializeOwned>(input: serde_json::Value) -> Result<T, String> {
    serde_json::from_value(input).map_err(|error| format!("invalid request: {error}"))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ColorBlobRequest {
    image: ImagePayload,
    target_color: String,
    min_area_pixels: Option<usize>,
    merge_adjacent: Option<bool>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ImagePayload {
    width: u32,
    height: u32,
    pixel_format: String,
    stride: Option<usize>,
    data: Vec<u8>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BoxSummaryRequest {
    #[serde(default)]
    detections: Vec<DetectionPayload>,
    #[serde(default)]
    boxes: Vec<BoxPayload>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DetectionPayload {
    label: Option<String>,
    score: Option<f32>,
    #[serde(alias = "box")]
    region: BoxPayload,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BoxPayload {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
}

fn color_blob_value(request: ColorBlobRequest) -> Result<serde_json::Value, String> {
    let target = match request.target_color.as_str() {
        "red" => TargetColor::Red,
        other => return Err(format!("unsupported target color `{other}`")),
    };
    let mut options =
        ColorBlobDetectionOptions::default().min_area_pixels(request.min_area_pixels.unwrap_or(24));
    options.target = target;
    options.morph_open_3x3 = request.merge_adjacent.unwrap_or(true);
    let image = request.image.view()?;
    let mut detector = ColorBlobDetector::new(options).map_err(|error| error.to_string())?;
    let detections = detector
        .detect_image(&image)
        .map_err(|error| error.to_string())?;
    Ok(serde_json::json!({
        "count": detections.len(),
        "detections": detections
            .into_iter()
            .map(|detection| serde_json::json!({
                "label": detection.label,
                "score": detection.score,
                "region": {
                    "x": detection.region.x,
                    "y": detection.region.y,
                    "width": detection.region.width,
                    "height": detection.region.height
                },
                "attributes": detection.attributes
            }))
            .collect::<Vec<_>>()
    }))
}

fn models_value() -> serde_json::Value {
    let face_spec = FaceDetectionPreset::OpenCvYuNet.model_spec();
    serde_json::json!({
        "models": [{
            "id": "opencv-yunet-onnx",
            "task": face_spec.task.as_protocol_str(),
            "repoId": face_spec.repo_id_value(),
            "revision": face_spec.revision_value(),
            "name": face_spec.name,
            "files": face_spec.files,
            "supported": false,
            "fallback": "image.detection.colorBlob"
        }],
        "deterministicFallbacks": [{
            "id": "color-blob-red",
            "operation": "image.detection.colorBlob",
            "supported": true
        }]
    })
}

fn box_summary_value(request: BoxSummaryRequest) -> Result<serde_json::Value, String> {
    let mut entries = Vec::new();
    for detection in request.detections {
        if let Some(score) = detection.score {
            if !score.is_finite() {
                return Err("detection score must be finite".to_string());
            }
        }
        let region = detection.region.bounding_box()?;
        entries.push((detection.label, detection.score, region));
    }
    for region in request.boxes {
        entries.push((None, None, region.bounding_box()?));
    }
    let union = union_bounds(entries.iter().map(|(_, _, region)| *region));
    let scored = entries
        .iter()
        .filter_map(|(_, score, _)| *score)
        .collect::<Vec<_>>();
    let average_score = if scored.is_empty() {
        None
    } else {
        Some(scored.iter().sum::<f32>() / scored.len() as f32)
    };
    let labels = entries
        .iter()
        .filter_map(|(label, _, _)| label.as_deref())
        .collect::<Vec<_>>();
    Ok(serde_json::json!({
        "count": entries.len(),
        "scoredCount": scored.len(),
        "averageScore": average_score,
        "labels": labels,
        "unionBounds": union.map(|region| serde_json::json!({
            "x": region.x,
            "y": region.y,
            "width": region.width,
            "height": region.height
        }))
    }))
}

impl ImagePayload {
    fn view(&self) -> Result<ImageView<'_>, String> {
        let pixel_format = parse_pixel_format(&self.pixel_format)?;
        let stride = self
            .stride
            .unwrap_or(self.width as usize * pixel_format.bytes_per_pixel());
        ImageView::new(self.width, self.height, pixel_format, &self.data, stride)
            .map_err(|error| error.to_string())
    }
}

fn parse_pixel_format(value: &str) -> Result<ImagePixelFormat, String> {
    match value {
        "rgb24" => Ok(ImagePixelFormat::Rgb24),
        "bgr24" => Ok(ImagePixelFormat::Bgr24),
        "gray8" => Ok(ImagePixelFormat::Gray8),
        other => Err(format!("unsupported image pixel format `{other}`")),
    }
}

impl BoxPayload {
    fn bounding_box(self) -> Result<BoundingBox, String> {
        BoundingBox::new(self.x, self.y, self.width, self.height).map_err(|error| error.to_string())
    }
}

fn union_bounds(regions: impl IntoIterator<Item = BoundingBox>) -> Option<BoundingBox> {
    let mut regions = regions.into_iter();
    let first = regions.next()?;
    let (mut min_x, mut min_y, mut max_x, mut max_y) = (
        first.x,
        first.y,
        first.x + first.width,
        first.y + first.height,
    );
    for region in regions {
        min_x = min_x.min(region.x);
        min_y = min_y.min(region.y);
        max_x = max_x.max(region.x + region.width);
        max_y = max_y.max(region.y + region.height);
    }
    BoundingBox::new(min_x, min_y, max_x - min_x, max_y - min_y).ok()
}

fn sample_image_json() -> serde_json::Value {
    serde_json::json!({
        "width": 2,
        "height": 2,
        "pixelFormat": "rgb24",
        "stride": null,
        "data": [255, 0, 0, 255, 0, 0, 0, 0, 0, 0, 0, 0]
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_red_block() {
        let response = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("image.detection.colorBlob"),
            input: serde_json::json!({"image": sample_image_json(), "targetColor": "red", "minAreaPixels": 1, "mergeAdjacent": false}),
        })
        .expect("color blob");
        assert_eq!(response.value["count"], 1);
    }

    #[test]
    fn empty_colors_produce_zero_detections() {
        let mut image = sample_image_json();
        image["data"] = serde_json::json!([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        let response = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("image.detection.colorBlob"),
            input: serde_json::json!({"image": image, "targetColor": "red", "minAreaPixels": 1}),
        })
        .expect("color blob");
        assert_eq!(response.value["count"], 0);
    }

    #[test]
    fn unsupported_target_color_errors() {
        let error = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("image.detection.colorBlob"),
            input: serde_json::json!({"image": sample_image_json(), "targetColor": "blue"}),
        })
        .expect_err("unsupported color");
        assert!(error.contains("unsupported target color"));
    }

    #[test]
    fn models_operation_reports_detection_catalog() {
        let response = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("image.detection.models"),
            input: serde_json::json!({}),
        })
        .expect("models");
        assert_eq!(response.value["models"][0]["id"], "opencv-yunet-onnx");
    }

    #[test]
    fn box_summary_returns_union_bounds() {
        let response = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("image.detection.boxSummary"),
            input: serde_json::json!({
                "detections": [
                    {"label": "a", "score": 0.5, "region": {"x": 1, "y": 1, "width": 2, "height": 2}},
                    {"label": "b", "score": 1.0, "region": {"x": 4, "y": 2, "width": 2, "height": 1}}
                ]
            }),
        })
        .expect("box summary");
        assert_eq!(response.value["count"], 2);
        assert_eq!(response.value["unionBounds"]["width"], 5);
    }

    #[test]
    fn box_summary_rejects_empty_boxes() {
        let error = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("image.detection.boxSummary"),
            input: serde_json::json!({"boxes": [{"x": 0, "y": 0, "width": 0, "height": 1}]}),
        })
        .expect_err("invalid box");
        assert!(error.contains("bounding box"));
    }
}
