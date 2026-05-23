#![doc = include_str!("../README.md")]

pub mod surface;
use math_geometry_2d::RectU32;
use math_linear::Kernel2d;
use video_analysis_core::{
    BoundingBox, DetectError, OwnedVideoFrame, PixelFormat, Result, VideoFrame,
};

#[derive(Debug, Clone, PartialEq)]
/// Variants describing frame edit.
pub enum FrameEdit {
    /// The crop variant.
    Crop(BoundingBox),
    /// The box blur variant.
    BoxBlur {
        /// The radius value for this variant.
        radius: u32,
    },
    /// The grayscale variant.
    Grayscale,
    /// The invert variant.
    Invert,
    /// The brightness contrast variant.
    BrightnessContrast {
        /// The brightness value for this variant.
        brightness: i16,
        /// The contrast value for this variant.
        contrast: f32,
    },
    /// The filter3x3 variant.
    Filter3x3 {
        /// The kernel value for this variant.
        kernel: [f32; 9],
        /// The divisor value for this variant.
        divisor: f32,
        /// The bias value for this variant.
        bias: f32,
    },
}

#[derive(Debug, Clone, Default, PartialEq)]
/// Data type for frame editor.
pub struct FrameEditor {
    edits: Vec<FrameEdit>,
}

impl FrameEditor {
    /// Creates a new value.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns edit.
    pub fn edit(mut self, edit: FrameEdit) -> Self {
        self.edits.push(edit);
        self
    }

    /// Returns crop.
    pub fn crop(self, region: BoundingBox) -> Self {
        self.edit(FrameEdit::Crop(region))
    }

    /// Returns crop rect.
    pub fn crop_rect(self, region: RectU32) -> Result<Self> {
        Ok(self.crop(region.try_into()?))
    }

    /// Returns box blur.
    pub fn box_blur(self, radius: u32) -> Self {
        self.edit(FrameEdit::BoxBlur { radius })
    }

    /// Returns grayscale.
    pub fn grayscale(self) -> Self {
        self.edit(FrameEdit::Grayscale)
    }

    /// Returns invert.
    pub fn invert(self) -> Self {
        self.edit(FrameEdit::Invert)
    }

    /// Returns brightness contrast.
    pub fn brightness_contrast(self, brightness: i16, contrast: f32) -> Self {
        self.edit(FrameEdit::BrightnessContrast {
            brightness,
            contrast,
        })
    }

    /// Returns filter 3x3.
    pub fn filter_3x3(self, kernel: [f32; 9], divisor: f32, bias: f32) -> Self {
        self.edit(FrameEdit::Filter3x3 {
            kernel,
            divisor,
            bias,
        })
    }

    /// Returns filter 3x3 kernel.
    pub fn filter_3x3_kernel(self, kernel: Kernel2d, divisor: f32, bias: f32) -> Result<Self> {
        Ok(self.filter_3x3(kernel.as_array_3x3()?, divisor, bias))
    }

    /// Returns edits.
    pub fn edits(&self) -> &[FrameEdit] {
        &self.edits
    }

    /// Returns apply.
    pub fn apply(&self, frame: &VideoFrame<'_>) -> Result<OwnedVideoFrame> {
        let mut current = compact_frame(frame);
        for edit in &self.edits {
            current = apply_edit(&current.as_frame(), edit)?;
        }
        Ok(current)
    }
}

/// Returns crop frame.
pub fn crop_frame(frame: &VideoFrame<'_>, region: BoundingBox) -> Result<OwnedVideoFrame> {
    crop_frame_rect(frame, region.into())
}

/// Returns crop frame rect.
pub fn crop_frame_rect(frame: &VideoFrame<'_>, region: RectU32) -> Result<OwnedVideoFrame> {
    validate_region(frame, region)?;
    let pixel_format = frame.pixel_format;
    let stride = region.width as usize * 3;
    let mut data = vec![0; stride * region.height as usize];
    for y in 0..region.height {
        let src_start = (region.y + y) as usize * frame.stride + region.x as usize * 3;
        let dst_start = y as usize * stride;
        data[dst_start..dst_start + stride]
            .copy_from_slice(&frame.data[src_start..src_start + stride]);
    }
    Ok(OwnedVideoFrame {
        position: frame.position,
        width: region.width,
        height: region.height,
        pixel_format,
        data,
        stride,
    })
}

/// Returns box blur frame.
pub fn box_blur_frame(frame: &VideoFrame<'_>, radius: u32) -> Result<OwnedVideoFrame> {
    if radius == 0 {
        return Ok(compact_frame(frame));
    }
    map_pixels(frame, |x, y| {
        let x0 = x.saturating_sub(radius);
        let y0 = y.saturating_sub(radius);
        let x1 = (x + radius).min(frame.width - 1);
        let y1 = (y + radius).min(frame.height - 1);
        let mut sum = [0u32; 3];
        let mut count = 0u32;
        for sample_y in y0..=y1 {
            for sample_x in x0..=x1 {
                let pixel = frame.pixel_rgb(sample_x, sample_y);
                sum[0] += pixel[0] as u32;
                sum[1] += pixel[1] as u32;
                sum[2] += pixel[2] as u32;
                count += 1;
            }
        }
        [
            (sum[0] / count) as u8,
            (sum[1] / count) as u8,
            (sum[2] / count) as u8,
        ]
    })
}

/// Returns grayscale frame.
pub fn grayscale_frame(frame: &VideoFrame<'_>) -> Result<OwnedVideoFrame> {
    map_pixels(frame, |x, y| {
        let [red, green, blue] = frame.pixel_rgb(x, y);
        let luma = (0.299 * red as f32 + 0.587 * green as f32 + 0.114 * blue as f32).round() as u8;
        [luma, luma, luma]
    })
}

/// Returns invert frame.
pub fn invert_frame(frame: &VideoFrame<'_>) -> Result<OwnedVideoFrame> {
    map_pixels(frame, |x, y| {
        let [red, green, blue] = frame.pixel_rgb(x, y);
        [255 - red, 255 - green, 255 - blue]
    })
}

/// Returns brightness contrast frame.
pub fn brightness_contrast_frame(
    frame: &VideoFrame<'_>,
    brightness: i16,
    contrast: f32,
) -> Result<OwnedVideoFrame> {
    if !contrast.is_finite() {
        return Err(DetectError::InvalidArgument(
            "contrast must be finite".to_string(),
        ));
    }
    map_pixels(frame, |x, y| {
        let [red, green, blue] = frame.pixel_rgb(x, y);
        [
            adjust_channel(red, brightness, contrast),
            adjust_channel(green, brightness, contrast),
            adjust_channel(blue, brightness, contrast),
        ]
    })
}

/// Returns filter 3x3 frame.
pub fn filter_3x3_frame(
    frame: &VideoFrame<'_>,
    kernel: [f32; 9],
    divisor: f32,
    bias: f32,
) -> Result<OwnedVideoFrame> {
    filter_3x3_frame_kernel(frame, &Kernel2d::from(kernel), divisor, bias)
}

/// Returns filter 3x3 frame kernel.
pub fn filter_3x3_frame_kernel(
    frame: &VideoFrame<'_>,
    kernel: &Kernel2d,
    divisor: f32,
    bias: f32,
) -> Result<OwnedVideoFrame> {
    if !divisor.is_finite() || divisor == 0.0 || !bias.is_finite() {
        return Err(DetectError::InvalidArgument(
            "filter divisor must be finite and non-zero, and bias must be finite".to_string(),
        ));
    }
    let kernel = kernel.as_array_3x3()?;
    map_pixels(frame, |x, y| {
        let mut output = [0.0f32; 3];
        for ky in 0..3 {
            for kx in 0..3 {
                let sample_x =
                    clamp_i64(x as i64 + kx as i64 - 1, 0, frame.width as i64 - 1) as u32;
                let sample_y =
                    clamp_i64(y as i64 + ky as i64 - 1, 0, frame.height as i64 - 1) as u32;
                let pixel = frame.pixel_rgb(sample_x, sample_y);
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

/// Returns sharpen frame.
pub fn sharpen_frame(frame: &VideoFrame<'_>) -> Result<OwnedVideoFrame> {
    filter_3x3_frame_kernel(frame, &Kernel2d::sharpen_3x3(), 1.0, 0.0)
}

/// Returns edge detect frame.
pub fn edge_detect_frame(frame: &VideoFrame<'_>) -> Result<OwnedVideoFrame> {
    filter_3x3_frame_kernel(frame, &Kernel2d::edge_3x3(), 1.0, 0.0)
}

fn apply_edit(frame: &VideoFrame<'_>, edit: &FrameEdit) -> Result<OwnedVideoFrame> {
    match edit {
        FrameEdit::Crop(region) => crop_frame(frame, *region),
        FrameEdit::BoxBlur { radius } => box_blur_frame(frame, *radius),
        FrameEdit::Grayscale => grayscale_frame(frame),
        FrameEdit::Invert => invert_frame(frame),
        FrameEdit::BrightnessContrast {
            brightness,
            contrast,
        } => brightness_contrast_frame(frame, *brightness, *contrast),
        FrameEdit::Filter3x3 {
            kernel,
            divisor,
            bias,
        } => filter_3x3_frame(frame, *kernel, *divisor, *bias),
    }
}

fn compact_frame(frame: &VideoFrame<'_>) -> OwnedVideoFrame {
    let stride = frame.width as usize * 3;
    let mut data = vec![0; stride * frame.height as usize];
    for y in 0..frame.height {
        let src_start = y as usize * frame.stride;
        let dst_start = y as usize * stride;
        data[dst_start..dst_start + stride]
            .copy_from_slice(&frame.data[src_start..src_start + stride]);
    }
    OwnedVideoFrame {
        position: frame.position,
        width: frame.width,
        height: frame.height,
        pixel_format: frame.pixel_format,
        data,
        stride,
    }
}

fn map_pixels(
    frame: &VideoFrame<'_>,
    mut f: impl FnMut(u32, u32) -> [u8; 3],
) -> Result<OwnedVideoFrame> {
    let stride = frame.width as usize * 3;
    let mut data = vec![0; stride * frame.height as usize];
    for y in 0..frame.height {
        for x in 0..frame.width {
            let rgb = f(x, y);
            write_native_pixel(&mut data, stride, frame.pixel_format, x, y, rgb);
        }
    }
    Ok(OwnedVideoFrame {
        position: frame.position,
        width: frame.width,
        height: frame.height,
        pixel_format: frame.pixel_format,
        data,
        stride,
    })
}

fn write_native_pixel(
    data: &mut [u8],
    stride: usize,
    pixel_format: PixelFormat,
    x: u32,
    y: u32,
    rgb: [u8; 3],
) {
    let i = y as usize * stride + x as usize * 3;
    match pixel_format {
        PixelFormat::Rgb24 => {
            data[i] = rgb[0];
            data[i + 1] = rgb[1];
            data[i + 2] = rgb[2];
        }
        PixelFormat::Bgr24 => {
            data[i] = rgb[2];
            data[i + 1] = rgb[1];
            data[i + 2] = rgb[0];
        }
    }
}

fn validate_region(frame: &VideoFrame<'_>, region: RectU32) -> Result<()> {
    region.validate()?;
    let x1 = region.x.checked_add(region.width).ok_or_else(|| {
        DetectError::InvalidArgument("crop region exceeds frame boundary".to_string())
    })?;
    let y1 = region.y.checked_add(region.height).ok_or_else(|| {
        DetectError::InvalidArgument("crop region exceeds frame boundary".to_string())
    })?;
    if x1 > frame.width || y1 > frame.height {
        return Err(DetectError::InvalidArgument(
            "crop region exceeds frame boundary".to_string(),
        ));
    }
    Ok(())
}

fn adjust_channel(value: u8, brightness: i16, contrast: f32) -> u8 {
    clamp_u8((value as f32 - 128.0) * contrast + 128.0 + brightness as f32)
}

fn clamp_u8(value: f32) -> u8 {
    value.round().clamp(0.0, 255.0) as u8
}

fn clamp_i64(value: i64, min: i64, max: i64) -> i64 {
    value.max(min).min(max)
}

#[cfg(test)]
mod tests {
    use num_rational::Rational64;
    use video_analysis_core::{FramePosition, OwnedVideoFrame};

    use super::*;

    fn frame() -> OwnedVideoFrame {
        OwnedVideoFrame {
            position: FramePosition::from_frame_index(0, Rational64::new(30, 1)),
            width: 3,
            height: 2,
            pixel_format: PixelFormat::Rgb24,
            data: vec![
                255, 0, 0, 0, 255, 0, 0, 0, 255, 10, 20, 30, 40, 50, 60, 70, 80, 90,
            ],
            stride: 9,
        }
    }

    #[test]
    fn crop_extracts_region() {
        let cropped =
            crop_frame_rect(&frame().as_frame(), RectU32::new(1, 0, 2, 1).unwrap()).unwrap();

        assert_eq!(cropped.width, 2);
        assert_eq!(cropped.height, 1);
        assert_eq!(cropped.data, vec![0, 255, 0, 0, 0, 255]);
    }

    #[test]
    fn grayscale_converts_pixels() {
        let gray = grayscale_frame(&frame().as_frame()).unwrap();

        assert_eq!(&gray.data[0..3], &[76, 76, 76]);
    }

    #[test]
    fn editor_chains_operations() {
        let edited = FrameEditor::new()
            .crop_rect(RectU32::new(0, 0, 2, 1).unwrap())
            .unwrap()
            .invert()
            .apply(&frame().as_frame())
            .unwrap();

        assert_eq!(edited.width, 2);
        assert_eq!(edited.data, vec![0, 255, 255, 255, 0, 255]);
    }

    #[test]
    fn blur_averages_neighbors() {
        let blurred = box_blur_frame(&frame().as_frame(), 1).unwrap();

        assert_eq!(&blurred.data[0..3], &[76, 81, 22]);
    }

    #[test]
    fn shared_kernel_helper_builds_filter_edit() {
        let edited = FrameEditor::new()
            .filter_3x3_kernel(Kernel2d::identity_3x3(), 1.0, 0.0)
            .unwrap()
            .apply(&frame().as_frame())
            .unwrap();
        assert_eq!(edited.width, frame().width);
    }
}
