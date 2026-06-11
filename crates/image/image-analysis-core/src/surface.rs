//! Library-owned runtime surface for `image-analysis-core`.

use runtime_core::{
    describe_surface_response, parse_surface_input, structured_operation_response,
    surface_operation, validate_max_items, OperationId, PackageSurface, RuntimeCapabilities,
    SurfaceError, SurfaceOperation, SurfaceRequest, SurfaceResponse,
};
use serde::Deserialize;

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
    surface_operation(id, name, description, example_request)
}

/// Runs one library-owned operation.
pub fn run_surface_operation(request: SurfaceRequest) -> Result<SurfaceResponse, String> {
    let surface = package_surface();
    let operation = request.operation.clone();
    let value = match request.operation.as_str() {
        "describe" => return Ok(describe_surface_response(&surface, request)),
        "image.core.summary" => summary_value(
            operation.as_str(),
            parse_surface_input(Some(operation.as_str()), request.input)?,
        )?,
        "image.core.lumaHistogram" => histogram_value(
            operation.as_str(),
            parse_surface_input(Some(operation.as_str()), request.input)?,
        )?,
        "image.core.maskTensorSummary" => mask_tensor_summary_value(
            operation.as_str(),
            parse_surface_input(Some(operation.as_str()), request.input)?,
        )?,
        operation => {
            return Err(
                SurfaceError::unsupported_operation(operation, env!("CARGO_PKG_NAME"))
                    .to_error_string(),
            )
        }
    };
    Ok(structured_operation_response(&surface, operation, value))
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

fn summary_value(operation: &str, request: ImageRequest) -> Result<serde_json::Value, String> {
    let image = request.image.view(operation)?;
    let mean = mean_rgb(&image).map_err(|error| invalid_request(operation, error.to_string()))?;
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

fn histogram_value(
    operation: &str,
    request: HistogramRequest,
) -> Result<serde_json::Value, String> {
    let bins = request.bins.unwrap_or(DEFAULT_BINS);
    if bins == 0 || bins > MAX_BINS {
        return Err(SurfaceError::resource_limit(
            Some(OperationId::new(operation)),
            "bins",
            MAX_BINS,
            bins,
        )
        .to_error_string());
    }
    let image = request.image.view(operation)?;
    let histogram = luma_histogram(&image, bins)
        .map_err(|error| invalid_request(operation, error.to_string()))?;
    Ok(serde_json::json!({ "bins": bins, "histogram": histogram }))
}

fn mask_tensor_summary_value(
    operation: &str,
    request: MaskTensorRequest,
) -> Result<serde_json::Value, String> {
    let preview_limit = request.preview_limit.unwrap_or(DEFAULT_PREVIEW_LIMIT);
    validate_max_items(operation, "previewLimit", preview_limit, MAX_PREVIEW_LIMIT)?;
    let image = request.image.view(operation)?;
    let tensor = mask_tensor_from_luma(&image)
        .map_err(|error| invalid_request(operation, error.to_string()))?;
    Ok(serde_json::json!({
        "shape": tensor.shape().dimensions(),
        "valueCount": tensor.values().len(),
        "valuesPreview": tensor.values().iter().take(preview_limit).copied().collect::<Vec<_>>(),
        "truncated": tensor.values().len() > preview_limit
    }))
}

impl ImagePayload {
    fn view(&self, operation: &str) -> Result<ImageView<'_>, String> {
        let pixel_format = parse_pixel_format(operation, &self.pixel_format)?;
        let stride = self
            .stride
            .unwrap_or(self.width as usize * pixel_format.bytes_per_pixel());
        ImageView::new(self.width, self.height, pixel_format, &self.data, stride)
            .map_err(|error| invalid_request(operation, error.to_string()))
    }
}

fn parse_pixel_format(operation: &str, value: &str) -> Result<ImagePixelFormat, String> {
    match value {
        "rgb24" => Ok(ImagePixelFormat::Rgb24),
        "bgr24" => Ok(ImagePixelFormat::Bgr24),
        "gray8" => Ok(ImagePixelFormat::Gray8),
        other => Err(SurfaceError::unsupported_value(
            Some(OperationId::new(operation)),
            "pixelFormat",
            other,
            &["rgb24", "bgr24", "gray8"],
        )
        .to_error_string()),
    }
}

fn invalid_request(operation: &str, message: impl Into<String>) -> String {
    SurfaceError::invalid_request(Some(OperationId::new(operation)), message).to_error_string()
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
        let error: SurfaceError = serde_json::from_str(&error).expect("typed surface error");
        assert_eq!(error.code, "resource_limit");
        assert_eq!(error.details["field"], "bins");
        assert_eq!(error.details["actual"], 0);
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
