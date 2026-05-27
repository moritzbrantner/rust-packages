//! Library-owned runtime surface for `image-analysis-processing`.

use image::GrayImage;
use image_analysis_core::{compact_image, ImagePixelFormat, ImageView, OwnedImage};
use serde::de::DeserializeOwned;
use serde::Deserialize;
use video_analysis_core::runtime::{
    OperationId, PackageSurface, RuntimeCapabilities, SurfaceOperation, SurfaceRequest,
    SurfaceResponse,
};

use crate::{
    apply_operation, composite_image, perceptual_hash_luma, sharpen_image, BlendMode,
    CompositeSpec, ImageOperation, ImageRegion,
};

const DEFAULT_PREVIEW_LIMIT: usize = 32;
const MAX_PREVIEW_LIMIT: usize = 512;
const MAX_HASH_SIZE: u32 = 16;

/// Returns the package surface exposed by every transport wrapper.
pub fn package_surface() -> PackageSurface {
    PackageSurface {
        library: env!("CARGO_PKG_NAME").to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        capabilities: RuntimeCapabilities::pure_rust(),
        operations: vec![
            operation("describe", "Describe package", "CPU image processing primitives for video-analysis.", serde_json::json!({"includeOperations": true})),
            operation("image.processing.apply", "Apply operation", "Applies one deterministic CPU image operation and returns a capped output preview.", serde_json::json!({"image": sample_image_json(), "operation": {"type": "grayscale"}})),
            operation("image.processing.pipeline", "Apply pipeline", "Applies an ordered deterministic CPU image operation pipeline and returns a capped output preview.", serde_json::json!({"image": sample_image_json(), "operations": [{"type": "flipHorizontal"}, {"type": "brightnessContrast", "brightness": 12, "contrast": 1.1}]})),
            operation("image.processing.composite", "Composite images", "Composites an overlay image onto a base image with opacity, blend mode, and an optional gray mask.", serde_json::json!({"base": sample_image_json(), "overlay": sample_image_json(), "x": 0, "y": 0, "opacity": 0.5, "blendMode": "normal"})),
            operation("image.processing.hash", "Perceptual hash", "Computes a deterministic luma perceptual hash for an in-memory image.", serde_json::json!({"image": sample_image_json(), "hashSize": 8})),
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
        "image.processing.apply" => apply_value(parse_input(request.input)?)?,
        "image.processing.pipeline" => pipeline_value(parse_input(request.input)?)?,
        "image.processing.composite" => composite_value(parse_input(request.input)?)?,
        "image.processing.hash" => hash_value(parse_input(request.input)?)?,
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
struct ApplyRequest {
    image: ImagePayload,
    operation: OperationRequest,
    preview_limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PipelineRequest {
    image: ImagePayload,
    operations: Vec<OperationRequest>,
    preview_limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CompositeRequest {
    base: ImagePayload,
    overlay: ImagePayload,
    mask: Option<ImagePayload>,
    x: Option<i32>,
    y: Option<i32>,
    opacity: Option<f32>,
    blend_mode: Option<String>,
    preview_limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct HashRequest {
    image: ImagePayload,
    hash_size: Option<u32>,
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
struct OperationRequest {
    #[serde(rename = "type")]
    kind: String,
    x: Option<u32>,
    y: Option<u32>,
    width: Option<u32>,
    height: Option<u32>,
    radius: Option<u32>,
    brightness: Option<i16>,
    contrast: Option<f32>,
    saturation: Option<f32>,
    clockwise_turns: Option<u8>,
    level: Option<u8>,
}

fn apply_value(request: ApplyRequest) -> Result<serde_json::Value, String> {
    let image = request.image.view()?;
    let preview_limit = request.preview_limit.unwrap_or(DEFAULT_PREVIEW_LIMIT);
    if preview_limit > MAX_PREVIEW_LIMIT {
        return Err(format!("previewLimit must be <= {MAX_PREVIEW_LIMIT}"));
    }
    let output = apply_requested_operation(&image, &request.operation)?;
    Ok(image_value(&output, preview_limit))
}

fn pipeline_value(request: PipelineRequest) -> Result<serde_json::Value, String> {
    let image = request.image.view()?;
    let preview_limit = request.preview_limit.unwrap_or(DEFAULT_PREVIEW_LIMIT);
    if preview_limit > MAX_PREVIEW_LIMIT {
        return Err(format!("previewLimit must be <= {MAX_PREVIEW_LIMIT}"));
    }
    let mut current = compact_image(&image).map_err(|error| error.to_string())?;
    for operation in &request.operations {
        current = apply_requested_operation(&current.as_view(), operation)?;
    }
    Ok(image_value(&current, preview_limit))
}

fn composite_value(request: CompositeRequest) -> Result<serde_json::Value, String> {
    let base = request.base.view()?;
    let overlay = request.overlay.view()?;
    let mask = request.mask.as_ref().map(ImagePayload::view).transpose()?;
    let preview_limit = request.preview_limit.unwrap_or(DEFAULT_PREVIEW_LIMIT);
    if preview_limit > MAX_PREVIEW_LIMIT {
        return Err(format!("previewLimit must be <= {MAX_PREVIEW_LIMIT}"));
    }
    let spec = CompositeSpec::new(
        request.x.unwrap_or(0),
        request.y.unwrap_or(0),
        request.opacity.unwrap_or(1.0),
        blend_mode(request.blend_mode.as_deref().unwrap_or("normal"))?,
    )
    .map_err(|error| error.to_string())?;
    let output = composite_image(&base, &overlay, mask, spec).map_err(|error| error.to_string())?;
    Ok(image_value(&output, preview_limit))
}

fn hash_value(request: HashRequest) -> Result<serde_json::Value, String> {
    let hash_size = request.hash_size.unwrap_or(crate::PERCEPTUAL_HASH_SIZE);
    if hash_size == 0 || hash_size > MAX_HASH_SIZE {
        return Err(format!("hashSize must be between 1 and {MAX_HASH_SIZE}"));
    }
    let image = request.image.view()?;
    let mut luma = GrayImage::new(image.width, image.height);
    for y in 0..image.height {
        for x in 0..image.width {
            luma.put_pixel(x, y, image::Luma([image.luma(x, y)]));
        }
    }
    Ok(serde_json::json!({
        "hash": perceptual_hash_luma(&luma, hash_size),
        "hashSize": hash_size
    }))
}

fn apply_requested_operation(
    image: &ImageView<'_>,
    operation: &OperationRequest,
) -> Result<OwnedImage, String> {
    match operation.kind.as_str() {
        "sharpen" => sharpen_image(image),
        _ => apply_operation(image, &operation.operation()?),
    }
    .map_err(|error| error.to_string())
}

impl OperationRequest {
    fn operation(&self) -> Result<ImageOperation, String> {
        match self.kind.as_str() {
            "crop" => Ok(ImageOperation::Crop(
                ImageRegion::new(
                    self.x.ok_or("crop requires x")?,
                    self.y.ok_or("crop requires y")?,
                    self.width.ok_or("crop requires width")?,
                    self.height.ok_or("crop requires height")?,
                )
                .map_err(|error| error.to_string())?,
            )),
            "resizeNearest" => Ok(ImageOperation::ResizeNearest {
                width: self.width.ok_or("resizeNearest requires width")?,
                height: self.height.ok_or("resizeNearest requires height")?,
            }),
            "boxBlur" => Ok(ImageOperation::BoxBlur {
                radius: self.radius.unwrap_or(1),
            }),
            "grayscale" => Ok(ImageOperation::Grayscale),
            "invert" => Ok(ImageOperation::Invert),
            "brightnessContrast" => Ok(ImageOperation::BrightnessContrast {
                brightness: self.brightness.unwrap_or(0),
                contrast: self.contrast.unwrap_or(1.0),
            }),
            "saturation" => Ok(ImageOperation::Saturation {
                saturation: self.saturation.unwrap_or(1.0),
            }),
            "threshold" => Ok(ImageOperation::Threshold {
                level: self.level.ok_or("threshold requires level")?,
            }),
            "flipHorizontal" => Ok(ImageOperation::FlipHorizontal),
            "flipVertical" => Ok(ImageOperation::FlipVertical),
            "rotate90" => Ok(ImageOperation::Rotate90 {
                clockwise_turns: self.clockwise_turns.unwrap_or(1),
            }),
            "sharpen" => Ok(ImageOperation::Grayscale),
            other => Err(format!("unsupported image operation `{other}`")),
        }
    }
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

fn image_value(image: &OwnedImage, preview_limit: usize) -> serde_json::Value {
    serde_json::json!({
        "width": image.width,
        "height": image.height,
        "pixelFormat": pixel_format_name(image.pixel_format),
        "stride": image.stride,
        "dataLength": image.data.len(),
        "dataPreview": image.data.iter().take(preview_limit).copied().collect::<Vec<_>>(),
        "truncated": image.data.len() > preview_limit
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

fn pixel_format_name(format: ImagePixelFormat) -> &'static str {
    match format {
        ImagePixelFormat::Rgb24 => "rgb24",
        ImagePixelFormat::Bgr24 => "bgr24",
        ImagePixelFormat::Gray8 => "gray8",
    }
}

fn blend_mode(value: &str) -> Result<BlendMode, String> {
    match value {
        "normal" => Ok(BlendMode::Normal),
        "multiply" => Ok(BlendMode::Multiply),
        "screen" => Ok(BlendMode::Screen),
        "add" => Ok(BlendMode::Add),
        "difference" => Ok(BlendMode::Difference),
        other => Err(format!("unsupported blend mode `{other}`")),
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
    fn grayscale_and_invert_return_expected_preview() {
        let grayscale = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("image.processing.apply"),
            input: serde_json::json!({"image": sample_image_json(), "operation": {"type": "grayscale"}, "previewLimit": 1}),
        })
        .expect("grayscale");
        assert_eq!(grayscale.value["pixelFormat"], "gray8");

        let invert = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("image.processing.apply"),
            input: serde_json::json!({"image": sample_image_json(), "operation": {"type": "invert"}, "previewLimit": 3}),
        })
        .expect("invert");
        assert_eq!(
            invert.value["dataPreview"],
            serde_json::json!([0, 255, 255])
        );
    }

    #[test]
    fn pipeline_and_composite_surfaces_work() {
        let pipeline = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("image.processing.pipeline"),
            input: serde_json::json!({
                "image": sample_image_json(),
                "operations": [
                    {"type": "flipHorizontal"},
                    {"type": "brightnessContrast", "brightness": 10, "contrast": 1.0}
                ],
                "previewLimit": 3
            }),
        })
        .expect("pipeline");
        assert_eq!(pipeline.value["pixelFormat"], "rgb24");

        let composite = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("image.processing.composite"),
            input: serde_json::json!({
                "base": sample_image_json(),
                "overlay": sample_image_json(),
                "x": -1,
                "y": 0,
                "opacity": 0.5,
                "blendMode": "normal",
                "previewLimit": 3
            }),
        })
        .expect("composite");
        assert_eq!(composite.value["width"], 2);
    }

    #[test]
    fn crop_rejects_out_of_bounds_region() {
        let error = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("image.processing.apply"),
            input: serde_json::json!({"image": sample_image_json(), "operation": {"type": "crop", "x": 1, "y": 1, "width": 4, "height": 4}}),
        })
        .expect_err("bad crop");
        assert!(error.contains("crop") || error.contains("region"));
    }

    #[test]
    fn hash_rejects_invalid_size() {
        let error = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("image.processing.hash"),
            input: serde_json::json!({"image": sample_image_json(), "hashSize": 0}),
        })
        .expect_err("bad hash");
        assert!(error.contains("hashSize"));
    }
}
