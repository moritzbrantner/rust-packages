//! Library-owned runtime surface for `image-analysis-core`.

use serde::de::DeserializeOwned;
use serde::Deserialize;
use video_analysis_core::runtime::{
    describe_surface_response, structured_operation_response, OperationId, PackageSurface,
    RuntimeCapabilities, SurfaceOperation, SurfaceRequest, SurfaceResponse,
};

use crate::{luma_histogram, mask_tensor_from_luma, mean_rgb, ImagePixelFormat, ImageView};

const DEFAULT_BINS: usize = 16;
const MAX_BINS: usize = 256;
const DEFAULT_PREVIEW_LIMIT: usize = 16;
const MAX_PREVIEW_LIMIT: usize = 128;

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
                "Shared image views, pixel formats, and image statistics for video-analysis.",
                serde_json::json!({"includeOperations": true}),
            ),
            operation(
                "image.core.summary",
                "Image summary",
                "Returns dimensions, format, compact length, and mean RGB for an in-memory image.",
                serde_json::json!({"image": sample_image_json()}),
            ),
            operation(
                "image.core.lumaHistogram",
                "Luma histogram",
                "Computes a capped luma histogram for an in-memory image.",
                serde_json::json!({"image": sample_image_json(), "bins": 16}),
            ),
            operation(
                "image.core.maskTensorSummary",
                "Mask tensor summary",
                "Builds a luma-derived mask tensor summary with capped value preview.",
                serde_json::json!({"image": sample_image_json(), "previewLimit": 16}),
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
    let surface = package_surface();
    let operation = request.operation.clone();
    let value = match request.operation.as_str() {
        "describe" => return Ok(describe_surface_response(&surface, request)),
        "image.core.summary" => summary_value(parse_input(request.input)?)?,
        "image.core.lumaHistogram" => histogram_value(parse_input(request.input)?)?,
        "image.core.maskTensorSummary" => mask_tensor_summary_value(parse_input(request.input)?)?,
        operation => {
            return Err(format!(
                "unsupported operation `{operation}` for {}",
                env!("CARGO_PKG_NAME")
            ))
        }
    };
    Ok(structured_operation_response(&surface, operation, value))
}

fn parse_input<T: DeserializeOwned>(input: serde_json::Value) -> Result<T, String> {
    serde_json::from_value(input).map_err(|error| format!("invalid request: {error}"))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ImageRequest {
    image: ImagePayload,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct HistogramRequest {
    image: ImagePayload,
    bins: Option<usize>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MaskTensorRequest {
    image: ImagePayload,
    preview_limit: Option<usize>,
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

fn summary_value(request: ImageRequest) -> Result<serde_json::Value, String> {
    let image = request.image.view()?;
    let mean = mean_rgb(&image).map_err(|error| error.to_string())?;
    Ok(serde_json::json!({
        "width": image.width,
        "height": image.height,
        "pixelFormat": pixel_format_name(image.pixel_format),
        "stride": image.stride,
        "compactLength": image.compact_len(),
        "dataLength": image.data.len(),
        "meanRgb": {
            "red": mean.red,
            "green": mean.green,
            "blue": mean.blue
        }
    }))
}

fn histogram_value(request: HistogramRequest) -> Result<serde_json::Value, String> {
    let bins = request.bins.unwrap_or(DEFAULT_BINS);
    if bins == 0 || bins > MAX_BINS {
        return Err(format!("bins must be between 1 and {MAX_BINS}"));
    }
    let image = request.image.view()?;
    let histogram = luma_histogram(&image, bins).map_err(|error| error.to_string())?;
    Ok(serde_json::json!({ "bins": bins, "histogram": histogram }))
}

fn mask_tensor_summary_value(request: MaskTensorRequest) -> Result<serde_json::Value, String> {
    let preview_limit = request.preview_limit.unwrap_or(DEFAULT_PREVIEW_LIMIT);
    if preview_limit > MAX_PREVIEW_LIMIT {
        return Err(format!("previewLimit must be <= {MAX_PREVIEW_LIMIT}"));
    }
    let image = request.image.view()?;
    let tensor = mask_tensor_from_luma(&image).map_err(|error| error.to_string())?;
    Ok(serde_json::json!({
        "shape": tensor.shape().dimensions(),
        "valueCount": tensor.values().len(),
        "valuesPreview": tensor.values().iter().take(preview_limit).copied().collect::<Vec<_>>(),
        "truncated": tensor.values().len() > preview_limit
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

fn pixel_format_name(format: ImagePixelFormat) -> &'static str {
    match format {
        ImagePixelFormat::Rgb24 => "rgb24",
        ImagePixelFormat::Bgr24 => "bgr24",
        ImagePixelFormat::Gray8 => "gray8",
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
    fn rgb_summary_computes_mean_color() {
        let response = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("image.core.summary"),
            input: serde_json::json!({"image": sample_image_json()}),
        })
        .expect("summary");
        assert_eq!(response.value["meanRgb"]["red"], 127.5);
        assert_eq!(response.value["compactLength"], 12);
    }

    #[test]
    fn histogram_validates_bins() {
        let error = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("image.core.lumaHistogram"),
            input: serde_json::json!({"image": sample_image_json(), "bins": 0}),
        })
        .expect_err("invalid bins");
        assert!(error.contains("bins must be"));
    }

    #[test]
    fn mask_tensor_summary_caps_preview() {
        let response = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("image.core.maskTensorSummary"),
            input: serde_json::json!({"image": sample_image_json(), "previewLimit": 2}),
        })
        .expect("mask");
        assert_eq!(response.value["valuesPreview"].as_array().unwrap().len(), 2);
        assert_eq!(response.value["truncated"], true);
    }
}
