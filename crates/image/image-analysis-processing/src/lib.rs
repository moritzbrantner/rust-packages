#![doc = include_str!("../README.md")]

use image_analysis_core::{ImagePixelFormat, ImageView, OwnedImage};
use video_analysis_core::{DetectError, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImageRegion {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ImageOperation {
    Crop(ImageRegion),
    ResizeNearest {
        width: u32,
        height: u32,
    },
    Grayscale,
    Invert,
    Threshold {
        level: u8,
    },
    Convolve3x3 {
        kernel: [f32; 9],
        divisor: f32,
        bias: f32,
    },
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ImageProcessor {
    operations: Vec<ImageOperation>,
}

impl ImageProcessor {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn operation(mut self, operation: ImageOperation) -> Self {
        self.operations.push(operation);
        self
    }

    pub fn operations(&self) -> &[ImageOperation] {
        &self.operations
    }

    pub fn process(&self, image: &ImageView<'_>) -> Result<OwnedImage> {
        let mut current = image_analysis_core::compact_image(image)?;
        for operation in &self.operations {
            current = apply_operation(&current.as_view(), operation)?;
        }
        Ok(current)
    }
}

pub fn apply_operation(image: &ImageView<'_>, operation: &ImageOperation) -> Result<OwnedImage> {
    match operation {
        ImageOperation::Crop(region) => crop_image(image, *region),
        ImageOperation::ResizeNearest { width, height } => resize_nearest(image, *width, *height),
        ImageOperation::Grayscale => grayscale_image(image),
        ImageOperation::Invert => invert_image(image),
        ImageOperation::Threshold { level } => threshold_image(image, *level),
        ImageOperation::Convolve3x3 {
            kernel,
            divisor,
            bias,
        } => convolve_3x3(image, *kernel, *divisor, *bias),
    }
}

pub fn crop_image(image: &ImageView<'_>, region: ImageRegion) -> Result<OwnedImage> {
    image.validate()?;
    validate_region(image, region)?;
    let bpp = image.pixel_format.bytes_per_pixel();
    let stride = region.width as usize * bpp;
    let mut data = vec![0_u8; stride * region.height as usize];
    for y in 0..region.height {
        let src_start = (region.y + y) as usize * image.stride + region.x as usize * bpp;
        let dst_start = y as usize * stride;
        data[dst_start..dst_start + stride]
            .copy_from_slice(&image.data[src_start..src_start + stride]);
    }
    OwnedImage::new(
        region.width,
        region.height,
        image.pixel_format,
        data,
        stride,
    )
}

pub fn resize_nearest(image: &ImageView<'_>, width: u32, height: u32) -> Result<OwnedImage> {
    image.validate()?;
    if width == 0 || height == 0 {
        return Err(DetectError::InvalidDimensions { width, height });
    }
    let bpp = image.pixel_format.bytes_per_pixel();
    let stride = width as usize * bpp;
    let mut data = vec![0_u8; stride * height as usize];
    for y in 0..height {
        let src_y = (y as u64 * image.height as u64 / height as u64) as u32;
        for x in 0..width {
            let src_x = (x as u64 * image.width as u64 / width as u64) as u32;
            write_pixel(
                &mut data,
                stride,
                image.pixel_format,
                x,
                y,
                image.pixel_rgb(src_x, src_y),
            );
        }
    }
    OwnedImage::new(width, height, image.pixel_format, data, stride)
}

pub fn grayscale_image(image: &ImageView<'_>) -> Result<OwnedImage> {
    map_pixels(image, ImagePixelFormat::Gray8, |source, x, y| {
        [source.luma(x, y); 3]
    })
}

pub fn invert_image(image: &ImageView<'_>) -> Result<OwnedImage> {
    map_pixels(image, image.pixel_format, |source, x, y| {
        let [red, green, blue] = source.pixel_rgb(x, y);
        [255 - red, 255 - green, 255 - blue]
    })
}

pub fn threshold_image(image: &ImageView<'_>, level: u8) -> Result<OwnedImage> {
    map_pixels(image, ImagePixelFormat::Gray8, |source, x, y| {
        let value = if source.luma(x, y) >= level { 255 } else { 0 };
        [value, value, value]
    })
}

pub fn convolve_3x3(
    image: &ImageView<'_>,
    kernel: [f32; 9],
    divisor: f32,
    bias: f32,
) -> Result<OwnedImage> {
    if !divisor.is_finite() || divisor == 0.0 || !bias.is_finite() {
        return Err(DetectError::InvalidArgument(
            "convolution divisor must be finite and non-zero, and bias must be finite".to_string(),
        ));
    }
    if kernel.iter().any(|value| !value.is_finite()) {
        return Err(DetectError::InvalidArgument(
            "convolution kernel values must be finite".to_string(),
        ));
    }
    map_pixels(image, image.pixel_format, |source, x, y| {
        let mut output = [0.0_f32; 3];
        for ky in 0..3 {
            for kx in 0..3 {
                let sx = (x as i64 + kx as i64 - 1).clamp(0, source.width as i64 - 1) as u32;
                let sy = (y as i64 + ky as i64 - 1).clamp(0, source.height as i64 - 1) as u32;
                let pixel = source.pixel_rgb(sx, sy);
                let weight = kernel[ky * 3 + kx];
                output[0] += pixel[0] as f32 * weight;
                output[1] += pixel[1] as f32 * weight;
                output[2] += pixel[2] as f32 * weight;
            }
        }
        [
            clamp_u8(output[0] / divisor + bias),
            clamp_u8(output[1] / divisor + bias),
            clamp_u8(output[2] / divisor + bias),
        ]
    })
}

pub fn sharpen_image(image: &ImageView<'_>) -> Result<OwnedImage> {
    convolve_3x3(
        image,
        [0.0, -1.0, 0.0, -1.0, 5.0, -1.0, 0.0, -1.0, 0.0],
        1.0,
        0.0,
    )
}

fn map_pixels(
    image: &ImageView<'_>,
    output_format: ImagePixelFormat,
    mut map: impl FnMut(ImageView<'_>, u32, u32) -> [u8; 3],
) -> Result<OwnedImage> {
    image.validate()?;
    let bpp = output_format.bytes_per_pixel();
    let stride = image.width as usize * bpp;
    let mut data = vec![0_u8; stride * image.height as usize];
    for y in 0..image.height {
        for x in 0..image.width {
            write_pixel(&mut data, stride, output_format, x, y, map(*image, x, y));
        }
    }
    OwnedImage::new(image.width, image.height, output_format, data, stride)
}

fn write_pixel(
    data: &mut [u8],
    stride: usize,
    format: ImagePixelFormat,
    x: u32,
    y: u32,
    rgb: [u8; 3],
) {
    let offset = y as usize * stride + x as usize * format.bytes_per_pixel();
    match format {
        ImagePixelFormat::Rgb24 => data[offset..offset + 3].copy_from_slice(&rgb),
        ImagePixelFormat::Bgr24 => {
            data[offset..offset + 3].copy_from_slice(&[rgb[2], rgb[1], rgb[0]])
        }
        ImagePixelFormat::Gray8 => data[offset] = rgb[0],
    }
}

fn validate_region(image: &ImageView<'_>, region: ImageRegion) -> Result<()> {
    if region.width == 0 || region.height == 0 {
        return Err(DetectError::InvalidDimensions {
            width: region.width,
            height: region.height,
        });
    }
    let max_x = region
        .x
        .checked_add(region.width)
        .ok_or_else(|| DetectError::InvalidArgument("crop region x range overflows".to_string()))?;
    let max_y = region
        .y
        .checked_add(region.height)
        .ok_or_else(|| DetectError::InvalidArgument("crop region y range overflows".to_string()))?;
    if max_x > image.width || max_y > image.height {
        return Err(DetectError::InvalidArgument(
            "crop region must be contained by the image".to_string(),
        ));
    }
    Ok(())
}

fn clamp_u8(value: f32) -> u8 {
    value.round().clamp(0.0, 255.0) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    fn image() -> OwnedImage {
        OwnedImage::new(
            2,
            2,
            ImagePixelFormat::Rgb24,
            vec![0, 0, 0, 255, 0, 0, 0, 255, 0, 0, 0, 255],
            6,
        )
        .unwrap()
    }

    #[test]
    fn grayscales_to_single_channel() {
        let gray = grayscale_image(&image().as_view()).unwrap();
        assert_eq!(gray.pixel_format, ImagePixelFormat::Gray8);
        assert_eq!(gray.data.len(), 4);
    }

    #[test]
    fn processor_chains_operations() {
        let processed = ImageProcessor::new()
            .operation(ImageOperation::Crop(ImageRegion {
                x: 0,
                y: 0,
                width: 1,
                height: 2,
            }))
            .operation(ImageOperation::ResizeNearest {
                width: 2,
                height: 2,
            })
            .process(&image().as_view())
            .unwrap();
        assert_eq!((processed.width, processed.height), (2, 2));
    }
}
