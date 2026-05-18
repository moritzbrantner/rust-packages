#![doc = include_str!("../README.md")]

use std::collections::BTreeMap;

use image_analysis_core::ImageView;
use image_analysis_segmentation::{
    ImageSegment, ImageSegmentationBackend, ImageSegmentationRequest,
};
use video_analysis_core::{BoundingBox, FramePosition, Result, VideoFrame};

#[derive(Debug, Clone, PartialEq)]
/// Data type for image detection.
pub struct ImageDetection {
    /// Label assigned to this value.
    pub label: String,
    /// Score assigned to this value.
    pub score: Option<f32>,
    /// The region value.
    pub region: BoundingBox,
    /// The attributes value.
    pub attributes: BTreeMap<String, String>,
}

impl ImageDetection {
    /// Returns attribute.
    pub fn attribute(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.attributes.insert(key.into(), value.into());
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Data type for image detection request.
pub struct ImageDetectionRequest {
    /// The segmentation value.
    pub segmentation: ImageSegmentationRequest,
    /// The min mask pixels value.
    pub min_mask_pixels: usize,
    /// The default label value.
    pub default_label: String,
}

impl ImageDetectionRequest {
    /// Returns automatic mask proposals.
    pub fn automatic_mask_proposals() -> Self {
        Self {
            segmentation: ImageSegmentationRequest::automatic_mask_generation(),
            ..Self::default()
        }
    }

    /// Returns min mask pixels.
    pub fn min_mask_pixels(mut self, value: usize) -> Self {
        self.min_mask_pixels = value.max(1);
        self
    }

    /// Returns default label.
    pub fn default_label(mut self, value: impl Into<String>) -> Self {
        self.default_label = value.into();
        self
    }
}

impl Default for ImageDetectionRequest {
    fn default() -> Self {
        Self {
            segmentation: ImageSegmentationRequest::default(),
            min_mask_pixels: 1,
            default_label: "object".to_string(),
        }
    }
}

/// Returns segment to detection.
pub fn segment_to_detection(segment: &ImageSegment, default_label: &str) -> ImageDetection {
    ImageDetection {
        label: segment
            .label
            .clone()
            .unwrap_or_else(|| default_label.to_string()),
        score: segment.score,
        region: segment.region,
        attributes: segment.attributes.clone(),
    }
}

/// Returns segments to detections.
pub fn segments_to_detections(
    segments: &[ImageSegment],
    min_mask_pixels: usize,
    default_label: &str,
) -> Vec<ImageDetection> {
    segments
        .iter()
        .filter(|segment| segment.mask.active_pixels() >= min_mask_pixels.max(1))
        .map(|segment| segment_to_detection(segment, default_label))
        .collect()
}

/// Data type for mask proposal detector.
pub struct MaskProposalDetector<B> {
    backend: B,
    request: ImageDetectionRequest,
}

impl<B> MaskProposalDetector<B> {
    /// Creates a new value.
    pub fn new(backend: B) -> Self {
        Self {
            backend,
            request: ImageDetectionRequest::default(),
        }
    }

    /// Returns request.
    pub fn request(mut self, value: ImageDetectionRequest) -> Self {
        self.request = value;
        self
    }

    /// Returns backend.
    pub fn backend(&self) -> &B {
        &self.backend
    }

    /// Returns backend mut.
    pub fn backend_mut(&mut self) -> &mut B {
        &mut self.backend
    }
}

impl<B: ImageSegmentationBackend> MaskProposalDetector<B> {
    /// Returns detect image.
    pub fn detect_image(&mut self, image: &ImageView<'_>) -> Result<Vec<ImageDetection>> {
        let segments = self
            .backend
            .segment_image(image, &self.request.segmentation)?;
        Ok(segments_to_detections(
            &segments,
            self.request.min_mask_pixels,
            &self.request.default_label,
        ))
    }

    /// Returns detect frame.
    pub fn detect_frame(&mut self, frame: &VideoFrame<'_>) -> Result<FrameDetections> {
        let image = ImageView::from_video_frame(frame)?;
        let detections = self.detect_image(&image)?;
        Ok(FrameDetections {
            position: frame.position,
            detections,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
/// Data type for frame detections.
pub struct FrameDetections {
    /// The position value.
    pub position: FramePosition,
    /// The detections value.
    pub detections: Vec<ImageDetection>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use image_analysis_core::{ImagePixelFormat, OwnedImage};
    use image_analysis_segmentation::{BinaryMask, ImageSegment};
    use video_analysis_core::{PixelFormat, Timebase, Timestamp};

    struct StubSegmentationBackend {
        segments: Vec<ImageSegment>,
    }

    impl ImageSegmentationBackend for StubSegmentationBackend {
        fn segment_image(
            &mut self,
            _image: &ImageView<'_>,
            _request: &ImageSegmentationRequest,
        ) -> Result<Vec<ImageSegment>> {
            Ok(self.segments.clone())
        }
    }

    fn image() -> OwnedImage {
        OwnedImage::new(8, 8, ImagePixelFormat::Rgb24, vec![32; 8 * 8 * 3], 8 * 3).unwrap()
    }

    fn frame<'a>(bytes: &'a [u8]) -> VideoFrame<'a> {
        VideoFrame::packed(
            FramePosition {
                frame_index: 3,
                timestamp: Timestamp::new(3, Timebase::new(1, 1)),
            },
            8,
            8,
            PixelFormat::Rgb24,
            bytes,
            8 * 3,
        )
        .unwrap()
    }

    #[test]
    fn detector_converts_segments_into_boxes() {
        let segment = ImageSegment::new(
            BinaryMask::filled_rect(8, 8, BoundingBox::new(1, 2, 3, 2).unwrap()).unwrap(),
        )
        .unwrap()
        .label("person")
        .score(0.9);
        let mut detector = MaskProposalDetector::new(StubSegmentationBackend {
            segments: vec![segment],
        });
        let detections = detector.detect_image(&image().as_view()).unwrap();
        assert_eq!(detections[0].label, "person");
        assert_eq!(detections[0].region, BoundingBox::new(1, 2, 3, 2).unwrap());
    }

    #[test]
    fn detector_supports_video_frames() {
        let segment = ImageSegment::new(
            BinaryMask::filled_rect(8, 8, BoundingBox::new(2, 1, 2, 3).unwrap()).unwrap(),
        )
        .unwrap();
        let owned = image();
        let mut detector = MaskProposalDetector::new(StubSegmentationBackend {
            segments: vec![segment],
        });
        let detections = detector.detect_frame(&frame(&owned.data)).unwrap();
        assert_eq!(detections.position.frame_index, 3);
        assert_eq!(detections.detections.len(), 1);
    }

    #[test]
    fn automatic_mask_proposals_are_explicit() {
        assert!(
            !ImageDetectionRequest::default()
                .segmentation
                .prompt
                .automatic_mask_generation
        );
        assert!(
            ImageDetectionRequest::automatic_mask_proposals()
                .segmentation
                .prompt
                .automatic_mask_generation
        );
    }
}
