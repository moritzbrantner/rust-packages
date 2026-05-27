//! Library-owned runtime surface for `image-analysis-onnx`.

use std::collections::BTreeMap;

use image_analysis_core::{ImagePixelFormat, ImageView};
use serde::de::DeserializeOwned;
use serde::Deserialize;
use video_analysis_core::runtime::{
    OperationId, PackageSurface, RuntimeCapabilities, SurfaceOperation, SurfaceRequest,
    SurfaceResponse,
};

use crate::{
    decode_object_detections, preprocessing_from_config, preprocess_image, BoxFormat, ChannelOrder,
    OnnxImagePreprocessing, OnnxObjectDetectionOutput,
};

const DEFAULT_PREVIEW_LIMIT: usize = 16;
const MAX_PREVIEW_LIMIT: usize = 256;

/// Returns the package surface exposed by every transport wrapper.
pub fn package_surface() -> PackageSurface {
    PackageSurface {
        library: env!("CARGO_PKG_NAME").to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        capabilities: RuntimeCapabilities::pure_rust(),
        operations: vec![
            operation("describe", "Describe package", "ONNX-backed still-image preprocessing and inference adapters for video-analysis.", serde_json::json!({"includeOperations": true})),
            operation("image.onnx.preprocessing", "Parse preprocessing", "Parses deterministic ONNX image preprocessing options from config JSON.", serde_json::json!({"config": {"size": {"width": 2, "height": 2}, "image_mean": [0.0, 0.0, 0.0], "image_std": [1.0, 1.0, 1.0]}})),
            operation("image.onnx.preprocess", "Preprocess image", "Preprocesses an in-memory image into a capped ONNX input tensor preview.", serde_json::json!({"image": sample_image_json(), "preprocessing": preprocessing_value(&OnnxImagePreprocessing::default()), "previewLimit": 16})),
            operation("image.onnx.decodeDetections", "Decode detections", "Decodes deterministic object-detection tensor outputs into image detections.", serde_json::json!({"output": {"boxes": [[0.5, 0.5, 0.5, 0.5]], "classIds": [1], "scores": [0.9], "boxFormat": "cxcywhNormalized"}, "options": {"scoreThreshold": 0.5, "labels": {"1": "object"}}, "imageWidth": 640, "imageHeight": 480})),
        ],
    }
}

fn operation(id: &str, name: &str, description: &str, example_request: serde_json::Value) -> SurfaceOperation {
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
        "image.onnx.preprocessing" => preprocessing_surface_value(parse_input(request.input)?)?,
        "image.onnx.preprocess" => preprocess_value(parse_input(request.input)?)?,
        "image.onnx.decodeDetections" => decode_detections_value(parse_input(request.input)?)?,
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
    SurfaceResponse { operation, value, diagnostics: Vec::new(), artifacts: Vec::new() }
}

fn parse_input<T: DeserializeOwned>(input: serde_json::Value) -> Result<T, String> {
    serde_json::from_value(input).map_err(|error| format!("invalid request: {error}"))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PreprocessingRequest {
    config: serde_json::Value,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PreprocessRequest {
    image: ImagePayload,
    preprocessing: PreprocessingPayload,
    preview_limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DecodeDetectionsRequest {
    output: DetectionOutputPayload,
    options: DetectionOptionsPayload,
    image_width: u32,
    image_height: u32,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DetectionOutputPayload {
    boxes: Vec<[f32; 4]>,
    class_ids: Vec<i64>,
    scores: Vec<f32>,
    box_format: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DetectionOptionsPayload {
    #[serde(default)]
    labels: BTreeMap<String, String>,
    score_threshold: Option<f32>,
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
struct PreprocessingPayload {
    input_width: u32,
    input_height: u32,
    rescale_factor: f32,
    mean: [f32; 3],
    std: [f32; 3],
    channel_order: String,
}

fn preprocessing_surface_value(request: PreprocessingRequest) -> Result<serde_json::Value, String> {
    let preprocessing =
        preprocessing_from_config(&request.config).map_err(|error| error.to_string())?;
    Ok(preprocessing_value(&preprocessing))
}

fn preprocess_value(request: PreprocessRequest) -> Result<serde_json::Value, String> {
    let preview_limit = request.preview_limit.unwrap_or(DEFAULT_PREVIEW_LIMIT);
    if preview_limit > MAX_PREVIEW_LIMIT {
        return Err(format!("previewLimit must be <= {MAX_PREVIEW_LIMIT}"));
    }
    let image = request.image.view()?;
    let preprocessing = request.preprocessing.into_preprocessing()?;
    let tensor = preprocess_image(&image, &preprocessing).map_err(|error| error.to_string())?;
    Ok(serde_json::json!({
        "shape": [tensor.channels, tensor.height as usize, tensor.width as usize],
        "channels": tensor.channels,
        "width": tensor.width,
        "height": tensor.height,
        "valueCount": tensor.values.len(),
        "valuesPreview": tensor.values.iter().take(preview_limit).copied().collect::<Vec<_>>(),
        "truncated": tensor.values.len() > preview_limit
    }))
}

fn decode_detections_value(request: DecodeDetectionsRequest) -> Result<serde_json::Value, String> {
    let labels = request
        .options
        .labels
        .into_iter()
        .map(|(key, value)| {
            key.parse::<i64>()
                .map(|key| (key, value))
                .map_err(|error| format!("invalid label id `{key}`: {error}"))
        })
        .collect::<Result<BTreeMap<_, _>, _>>()?;
    let output = OnnxObjectDetectionOutput {
        boxes: request.output.boxes,
        class_ids: request.output.class_ids,
        scores: request.output.scores,
        box_format: parse_box_format(&request.output.box_format)?,
    };
    let detections = decode_object_detections(
        &output,
        &labels,
        request.options.score_threshold.unwrap_or(0.0),
        (request.image_width, request.image_height),
    );
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
                }
            }))
            .collect::<Vec<_>>()
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

impl PreprocessingPayload {
    fn into_preprocessing(self) -> Result<OnnxImagePreprocessing, String> {
        Ok(OnnxImagePreprocessing {
            input_width: self.input_width,
            input_height: self.input_height,
            rescale_factor: self.rescale_factor,
            mean: self.mean,
            std: self.std,
            channel_order: parse_channel_order(&self.channel_order)?,
        })
    }
}

fn preprocessing_value(preprocessing: &OnnxImagePreprocessing) -> serde_json::Value {
    serde_json::json!({
        "inputWidth": preprocessing.input_width,
        "inputHeight": preprocessing.input_height,
        "rescaleFactor": preprocessing.rescale_factor,
        "mean": preprocessing.mean,
        "std": preprocessing.std,
        "channelOrder": match preprocessing.channel_order {
            ChannelOrder::Rgb => "rgb",
            ChannelOrder::Bgr => "bgr",
        }
    })
}

fn parse_pixel_format(value: &str) -> Result<ImagePixelFormat, String> {
    match value {
        "rgb24" => Ok(ImagePixelFormat::Rgb24),
        "bgr24" => Ok(ImagePixelFormat::Bgr24),
        "gray8" => Ok(ImagePixelFormat::Gray8),
        other => Err(format!("unsupported image pixel format `{other}`")),
    }
}

fn parse_channel_order(value: &str) -> Result<ChannelOrder, String> {
    match value {
        "rgb" | "RGB" => Ok(ChannelOrder::Rgb),
        "bgr" | "BGR" => Ok(ChannelOrder::Bgr),
        other => Err(format!("unsupported ONNX channel order `{other}`")),
    }
}

fn parse_box_format(value: &str) -> Result<BoxFormat, String> {
    match value {
        "xyxyAbsolute" => Ok(BoxFormat::XyxyAbsolute),
        "xyxyNormalized" => Ok(BoxFormat::XyxyNormalized),
        "cxcywhAbsolute" => Ok(BoxFormat::CxcywhAbsolute),
        "cxcywhNormalized" => Ok(BoxFormat::CxcywhNormalized),
        other => Err(format!("unsupported box format `{other}`")),
    }
}

fn sample_image_json() -> serde_json::Value {
    serde_json::json!({
        "width": 2,
        "height": 2,
        "pixelFormat": "rgb24",
        "stride": null,
        "data": [255, 0, 0, 0, 255, 0, 0, 0, 255, 255, 255, 255]
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preprocessing_parses_config() {
        let response = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("image.onnx.preprocessing"),
            input: serde_json::json!({"config": {"size": {"width": 4, "height": 3}, "channel_order": "BGR"}}),
        })
        .expect("preprocessing");
        assert_eq!(response.value["inputWidth"], 4);
        assert_eq!(response.value["channelOrder"], "bgr");
    }

    #[test]
    fn preprocess_returns_tensor_shape() {
        let response = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("image.onnx.preprocess"),
            input: serde_json::json!({
                "image": sample_image_json(),
                "preprocessing": {"inputWidth": 2, "inputHeight": 2, "rescaleFactor": 1.0, "mean": [0.0, 0.0, 0.0], "std": [1.0, 1.0, 1.0], "channelOrder": "rgb"},
                "previewLimit": 3
            }),
        })
        .expect("preprocess");
        assert_eq!(response.value["shape"], serde_json::json!([3, 2, 2]));
        assert_eq!(response.value["valuesPreview"].as_array().unwrap().len(), 3);
    }

    #[test]
    fn decode_detections_filters_by_score() {
        let response = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("image.onnx.decodeDetections"),
            input: serde_json::json!({
                "output": {"boxes": [[0.5, 0.5, 0.5, 0.5]], "classIds": [7], "scores": [0.4], "boxFormat": "cxcywhNormalized"},
                "options": {"scoreThreshold": 0.5, "labels": {"7": "car"}},
                "imageWidth": 100,
                "imageHeight": 100
            }),
        })
        .expect("decode");
        assert_eq!(response.value["count"], 0);
    }
}
