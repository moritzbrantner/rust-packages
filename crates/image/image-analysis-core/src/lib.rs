#![doc = include_str!("../README.md")]

use video_analysis_core::{DetectError, PixelFormat, Result, VideoFrame};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImagePixelFormat {
    Rgb24,
    Bgr24,
    Gray8,
}

impl ImagePixelFormat {
    pub const fn bytes_per_pixel(self) -> usize {
        match self {
            Self::Rgb24 | Self::Bgr24 => 3,
            Self::Gray8 => 1,
        }
    }
}

impl From<PixelFormat> for ImagePixelFormat {
    fn from(value: PixelFormat) -> Self {
        match value {
            PixelFormat::Rgb24 => Self::Rgb24,
            PixelFormat::Bgr24 => Self::Bgr24,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ImageView<'a> {
    pub width: u32,
    pub height: u32,
    pub pixel_format: ImagePixelFormat,
    pub data: &'a [u8],
    pub stride: usize,
}

impl<'a> ImageView<'a> {
    pub fn packed(
        width: u32,
        height: u32,
        pixel_format: ImagePixelFormat,
        data: &'a [u8],
    ) -> Result<Self> {
        Self::new(
            width,
            height,
            pixel_format,
            data,
            width as usize * pixel_format.bytes_per_pixel(),
        )
    }

    pub fn new(
        width: u32,
        height: u32,
        pixel_format: ImagePixelFormat,
        data: &'a [u8],
        stride: usize,
    ) -> Result<Self> {
        let image = Self {
            width,
            height,
            pixel_format,
            data,
            stride,
        };
        image.validate()?;
        Ok(image)
    }

    pub fn from_video_frame(frame: &VideoFrame<'a>) -> Result<Self> {
        Self::new(
            frame.width,
            frame.height,
            frame.pixel_format.into(),
            frame.data,
            frame.stride,
        )
    }

    pub fn validate(self) -> Result<()> {
        if self.width == 0 || self.height == 0 {
            return Err(DetectError::InvalidDimensions {
                width: self.width,
                height: self.height,
            });
        }
        let packed_width = self.width as usize * self.pixel_format.bytes_per_pixel();
        if self.stride < packed_width {
            return Err(DetectError::InvalidFrameBuffer {
                expected: packed_width,
                actual: self.stride,
            });
        }
        let expected = self.stride * self.height as usize;
        if self.data.len() < expected {
            return Err(DetectError::InvalidFrameBuffer {
                expected,
                actual: self.data.len(),
            });
        }
        Ok(())
    }

    pub fn compact_len(self) -> usize {
        self.width as usize * self.height as usize * self.pixel_format.bytes_per_pixel()
    }

    pub fn pixel_rgb(self, x: u32, y: u32) -> [u8; 3] {
        let bpp = self.pixel_format.bytes_per_pixel();
        let offset = y as usize * self.stride + x as usize * bpp;
        match self.pixel_format {
            ImagePixelFormat::Rgb24 => [
                self.data[offset],
                self.data[offset + 1],
                self.data[offset + 2],
            ],
            ImagePixelFormat::Bgr24 => [
                self.data[offset + 2],
                self.data[offset + 1],
                self.data[offset],
            ],
            ImagePixelFormat::Gray8 => {
                let value = self.data[offset];
                [value, value, value]
            }
        }
    }

    pub fn luma(self, x: u32, y: u32) -> u8 {
        let [red, green, blue] = self.pixel_rgb(x, y);
        (0.299 * red as f32 + 0.587 * green as f32 + 0.114 * blue as f32).round() as u8
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct OwnedImage {
    pub width: u32,
    pub height: u32,
    pub pixel_format: ImagePixelFormat,
    pub data: Vec<u8>,
    pub stride: usize,
}

impl OwnedImage {
    pub fn new_rgb(width: u32, height: u32, data: Vec<u8>) -> Result<Self> {
        Self::new(
            width,
            height,
            ImagePixelFormat::Rgb24,
            data,
            width as usize * ImagePixelFormat::Rgb24.bytes_per_pixel(),
        )
    }

    pub fn new_bgr(width: u32, height: u32, data: Vec<u8>) -> Result<Self> {
        Self::new(
            width,
            height,
            ImagePixelFormat::Bgr24,
            data,
            width as usize * ImagePixelFormat::Bgr24.bytes_per_pixel(),
        )
    }

    pub fn new_gray(width: u32, height: u32, data: Vec<u8>) -> Result<Self> {
        Self::new(
            width,
            height,
            ImagePixelFormat::Gray8,
            data,
            width as usize * ImagePixelFormat::Gray8.bytes_per_pixel(),
        )
    }

    pub fn new(
        width: u32,
        height: u32,
        pixel_format: ImagePixelFormat,
        data: Vec<u8>,
        stride: usize,
    ) -> Result<Self> {
        let image = Self {
            width,
            height,
            pixel_format,
            data,
            stride,
        };
        image.as_view().validate()?;
        Ok(image)
    }

    pub fn as_view(&self) -> ImageView<'_> {
        ImageView {
            width: self.width,
            height: self.height,
            pixel_format: self.pixel_format,
            data: &self.data,
            stride: self.stride,
        }
    }

    pub fn from_video_frame(frame: &VideoFrame<'_>) -> Result<Self> {
        compact_image(&ImageView::new(
            frame.width,
            frame.height,
            frame.pixel_format.into(),
            frame.data,
            frame.stride,
        )?)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RgbMean {
    pub red: f32,
    pub green: f32,
    pub blue: f32,
}

pub fn compact_image(image: &ImageView<'_>) -> Result<OwnedImage> {
    image.validate()?;
    let bpp = image.pixel_format.bytes_per_pixel();
    let row_len = image.width as usize * bpp;
    let mut data = Vec::with_capacity(image.compact_len());
    for y in 0..image.height as usize {
        let start = y * image.stride;
        data.extend_from_slice(&image.data[start..start + row_len]);
    }
    OwnedImage::new(image.width, image.height, image.pixel_format, data, row_len)
}

pub fn mean_rgb(image: &ImageView<'_>) -> Result<RgbMean> {
    image.validate()?;
    let mut sum = [0_u64; 3];
    for y in 0..image.height {
        for x in 0..image.width {
            let pixel = image.pixel_rgb(x, y);
            sum[0] += pixel[0] as u64;
            sum[1] += pixel[1] as u64;
            sum[2] += pixel[2] as u64;
        }
    }
    let pixels = (image.width as f32) * (image.height as f32);
    Ok(RgbMean {
        red: sum[0] as f32 / pixels,
        green: sum[1] as f32 / pixels,
        blue: sum[2] as f32 / pixels,
    })
}

pub fn luma_histogram(image: &ImageView<'_>, bins: usize) -> Result<Vec<u64>> {
    image.validate()?;
    if bins == 0 || bins > 256 {
        return Err(DetectError::InvalidArgument(
            "histogram bins must be in the range 1..=256".to_string(),
        ));
    }
    let mut histogram = vec![0_u64; bins];
    for y in 0..image.height {
        for x in 0..image.width {
            let bin = (image.luma(x, y) as usize * bins).min(255 * bins) / 256;
            histogram[bin.min(bins - 1)] += 1;
        }
    }
    Ok(histogram)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn computes_luma_histogram() {
        let image = OwnedImage::new_rgb(2, 1, vec![0, 0, 0, 255, 255, 255]).unwrap();
        assert_eq!(luma_histogram(&image.as_view(), 2).unwrap(), vec![1, 1]);
    }

    #[test]
    fn compacts_padded_rows() {
        let view =
            ImageView::new(1, 2, ImagePixelFormat::Rgb24, &[1, 2, 3, 0, 4, 5, 6, 0], 4).unwrap();
        let compact = compact_image(&view).unwrap();
        assert_eq!(compact.stride, 3);
        assert_eq!(compact.data, vec![1, 2, 3, 4, 5, 6]);
    }

    #[test]
    fn packed_view_uses_tight_stride() {
        let view = ImageView::packed(2, 1, ImagePixelFormat::Gray8, &[0, 255]).unwrap();
        assert_eq!(view.stride, 2);
    }
}
