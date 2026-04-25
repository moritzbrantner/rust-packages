#![doc = include_str!("../README.md")]

use std::collections::BTreeMap;

use image_analysis_core::{ImagePixelFormat, ImageView, OwnedImage};
use video_analysis_core::{BoundingBox, DetectError, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PointLabel {
    Foreground,
    Background,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SegmentationPoint {
    pub x: u32,
    pub y: u32,
    pub label: PointLabel,
}

impl SegmentationPoint {
    pub const fn foreground(x: u32, y: u32) -> Self {
        Self {
            x,
            y,
            label: PointLabel::Foreground,
        }
    }

    pub const fn background(x: u32, y: u32) -> Self {
        Self {
            x,
            y,
            label: PointLabel::Background,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ImageSegmentationPrompt {
    pub points: Vec<SegmentationPoint>,
    pub boxes: Vec<BoundingBox>,
    pub automatic_mask_generation: bool,
    pub multimask_output: bool,
}

impl ImageSegmentationPrompt {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn automatic_mask_generation() -> Self {
        Self {
            automatic_mask_generation: true,
            multimask_output: true,
            ..Self::default()
        }
    }

    pub fn point(mut self, point: SegmentationPoint) -> Self {
        self.points.push(point);
        self
    }

    pub fn bounding_box(mut self, region: BoundingBox) -> Self {
        self.boxes.push(region);
        self
    }

    pub fn multimask_output(mut self, value: bool) -> Self {
        self.multimask_output = value;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageSegmentationRequest {
    pub prompt: ImageSegmentationPrompt,
    pub min_mask_pixels: usize,
}

impl ImageSegmentationRequest {
    pub fn new(prompt: ImageSegmentationPrompt) -> Self {
        Self {
            prompt,
            min_mask_pixels: 1,
        }
    }

    pub fn automatic_mask_generation() -> Self {
        Self::new(ImageSegmentationPrompt::automatic_mask_generation())
    }

    pub fn min_mask_pixels(mut self, value: usize) -> Self {
        self.min_mask_pixels = value.max(1);
        self
    }
}

impl Default for ImageSegmentationRequest {
    fn default() -> Self {
        Self::new(ImageSegmentationPrompt::default())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BinaryMask {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
}

impl BinaryMask {
    pub fn new(width: u32, height: u32, data: Vec<u8>) -> Result<Self> {
        if width == 0 || height == 0 {
            return Err(DetectError::InvalidDimensions { width, height });
        }
        let expected = width as usize * height as usize;
        if data.len() != expected {
            return Err(DetectError::InvalidFrameBuffer {
                expected,
                actual: data.len(),
            });
        }
        Ok(Self {
            width,
            height,
            data,
        })
    }

    pub fn empty(width: u32, height: u32) -> Result<Self> {
        Self::new(width, height, vec![0; width as usize * height as usize])
    }

    pub fn filled_rect(width: u32, height: u32, region: BoundingBox) -> Result<Self> {
        if region.x.saturating_add(region.width) > width
            || region.y.saturating_add(region.height) > height
        {
            return Err(DetectError::InvalidArgument(
                "mask region must fit inside the mask dimensions".to_string(),
            ));
        }
        let mut mask = Self::empty(width, height)?;
        for y in region.y..region.y + region.height {
            for x in region.x..region.x + region.width {
                let index = mask.index(x, y);
                mask.data[index] = u8::MAX;
            }
        }
        Ok(mask)
    }

    pub fn is_active(&self, x: u32, y: u32) -> bool {
        self.data[self.index(x, y)] != 0
    }

    pub fn active_pixels(&self) -> usize {
        self.data.iter().filter(|value| **value != 0).count()
    }

    pub fn bounding_box(&self) -> Option<BoundingBox> {
        let mut min_x = self.width;
        let mut min_y = self.height;
        let mut max_x = 0_u32;
        let mut max_y = 0_u32;
        let mut found = false;

        for y in 0..self.height {
            for x in 0..self.width {
                if !self.is_active(x, y) {
                    continue;
                }
                found = true;
                min_x = min_x.min(x);
                min_y = min_y.min(y);
                max_x = max_x.max(x);
                max_y = max_y.max(y);
            }
        }

        found.then(|| BoundingBox {
            x: min_x,
            y: min_y,
            width: max_x - min_x + 1,
            height: max_y - min_y + 1,
        })
    }

    pub fn to_image(&self) -> Result<OwnedImage> {
        OwnedImage::new(
            self.width,
            self.height,
            ImagePixelFormat::Gray8,
            self.data.clone(),
            self.width as usize,
        )
    }

    fn index(&self, x: u32, y: u32) -> usize {
        y as usize * self.width as usize + x as usize
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ImageSegment {
    pub label: Option<String>,
    pub score: Option<f32>,
    pub region: BoundingBox,
    pub mask: BinaryMask,
    pub attributes: BTreeMap<String, String>,
}

impl ImageSegment {
    pub fn new(mask: BinaryMask) -> Result<Self> {
        let region = mask.bounding_box().ok_or_else(|| {
            DetectError::InvalidArgument(
                "segmentation mask must contain at least one active pixel".to_string(),
            )
        })?;
        Ok(Self {
            label: None,
            score: None,
            region,
            mask,
            attributes: BTreeMap::new(),
        })
    }

    pub fn label(mut self, value: impl Into<String>) -> Self {
        self.label = Some(value.into());
        self
    }

    pub fn score(mut self, value: f32) -> Self {
        self.score = Some(value);
        self
    }

    pub fn attribute(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.attributes.insert(key.into(), value.into());
        self
    }
}

pub trait ImageSegmentationBackend {
    fn segment_image(
        &mut self,
        image: &ImageView<'_>,
        request: &ImageSegmentationRequest,
    ) -> Result<Vec<ImageSegment>>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn binary_mask_bounding_box_tracks_active_region() {
        let mask = BinaryMask::filled_rect(8, 6, BoundingBox::new(2, 1, 3, 4).unwrap()).unwrap();
        assert_eq!(mask.active_pixels(), 12);
        assert_eq!(
            mask.bounding_box(),
            Some(BoundingBox::new(2, 1, 3, 4).unwrap())
        );
    }

    #[test]
    fn image_segment_rejects_empty_masks() {
        let mask = BinaryMask::empty(4, 4).unwrap();
        assert!(ImageSegment::new(mask).is_err());
    }

    #[test]
    fn segmentation_request_default_is_manual() {
        let request = ImageSegmentationRequest::default();
        assert!(!request.prompt.automatic_mask_generation);
        assert!(request.prompt.points.is_empty());
        assert!(request.prompt.boxes.is_empty());
    }

    #[test]
    fn segmentation_request_can_opt_into_automatic_masks() {
        let request = ImageSegmentationRequest::automatic_mask_generation();
        assert!(request.prompt.automatic_mask_generation);
        assert!(request.prompt.multimask_output);
    }
}
