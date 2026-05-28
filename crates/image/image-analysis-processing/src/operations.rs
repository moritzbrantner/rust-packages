use image::GrayImage;
use image_analysis_core::{compact_image, ImagePixelFormat, ImageView, OwnedImage};
use serde::de::DeserializeOwned;

use crate::contracts::{
    ApplyRequest, CompositeRequest, HashRequest, ImagePayload, OperationRequest, PipelineRequest,
};
use crate::{
    apply_operation, composite_image, perceptual_hash_luma, sharpen_image, BlendMode,
    CompositeSpec, ImageOperation, ImageRegion,
};

const DEFAULT_PREVIEW_LIMIT: usize = 32;
const MAX_PREVIEW_LIMIT: usize = 512;
const MAX_HASH_SIZE: u32 = 16;

pub fn apply_value(input: serde_json::Value) -> Result<serde_json::Value, String> {
    let request = parse_input::<ApplyRequest>(input)?;
    let image = request.image.view()?;
    let preview_limit = checked_preview_limit(request.preview_limit)?;
    let output = apply_requested_operation(&image, &request.operation)?;
    Ok(image_value(&output, preview_limit))
}

pub fn pipeline_value(input: serde_json::Value) -> Result<serde_json::Value, String> {
    let request = parse_input::<PipelineRequest>(input)?;
    let image = request.image.view()?;
    let preview_limit = checked_preview_limit(request.preview_limit)?;
    let mut current = compact_image(&image).map_err(|error| error.to_string())?;
    for operation in &request.operations {
        current = apply_requested_operation(&current.as_view(), operation)?;
    }
    Ok(image_value(&current, preview_limit))
}

pub fn composite_value(input: serde_json::Value) -> Result<serde_json::Value, String> {
    let request = parse_input::<CompositeRequest>(input)?;
    let base = request.base.view()?;
    let overlay = request.overlay.view()?;
    let mask = request.mask.as_ref().map(ImagePayload::view).transpose()?;
    let preview_limit = checked_preview_limit(request.preview_limit)?;
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

pub fn hash_value(input: serde_json::Value) -> Result<serde_json::Value, String> {
    let request = parse_input::<HashRequest>(input)?;
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

fn parse_input<T: DeserializeOwned>(input: serde_json::Value) -> Result<T, String> {
    serde_json::from_value(input).map_err(|error| format!("invalid request: {error}"))
}

fn checked_preview_limit(preview_limit: Option<usize>) -> Result<usize, String> {
    let preview_limit = preview_limit.unwrap_or(DEFAULT_PREVIEW_LIMIT);
    if preview_limit > MAX_PREVIEW_LIMIT {
        return Err(format!("previewLimit must be <= {MAX_PREVIEW_LIMIT}"));
    }
    Ok(preview_limit)
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

pub fn sample_image_json() -> serde_json::Value {
    serde_json::json!({
        "width": 2,
        "height": 2,
        "pixelFormat": "rgb24",
        "stride": null,
        "data": [255, 0, 0, 0, 255, 0, 0, 0, 255, 255, 255, 255]
    })
}
