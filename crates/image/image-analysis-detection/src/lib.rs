#![doc = include_str!("../README.md")]

pub mod surface;
use std::collections::BTreeMap;

use image_analysis_core::ImageView;
use image_analysis_segmentation::{
    ImageSegment, ImageSegmentationBackend, ImageSegmentationRequest,
};
use model_runtime::{HuggingFaceModelSpec, ModelTask};
use video_analysis_core::{BoundingBox, DetectError, FramePosition, Result, VideoFrame};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
/// Variants describing face detection preset.
pub enum FaceDetectionPreset {
    #[default]
    /// The OpenCV YuNet ONNX variant.
    OpenCvYuNet,
}

impl FaceDetectionPreset {
    /// Returns model spec.
    pub fn model_spec(self) -> HuggingFaceModelSpec {
        match self {
            Self::OpenCvYuNet => {
                HuggingFaceModelSpec::new("opencv/face_detection_yunet", ModelTask::FaceDetection)
                    .name("opencv-yunet-onnx")
                    .first_available_file([
                        "face_detection_yunet_2023mar.onnx",
                        "face_detection_yunet_2023mar_int8.onnx",
                        "face_detection_yunet_2023mar_int8bq.onnx",
                    ])
            }
        }
    }
}

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

#[derive(Debug, Clone, Copy, Default, PartialEq)]
/// Normalized face bounding box.
pub struct FaceBox {
    /// Normalized x coordinate.
    pub x: f32,
    /// Normalized y coordinate.
    pub y: f32,
    /// Normalized width.
    pub width: f32,
    /// Normalized height.
    pub height: f32,
}

impl FaceBox {
    /// Creates a new value.
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Result<Self> {
        let bbox = Self {
            x,
            y,
            width,
            height,
        };
        bbox.validate()?;
        Ok(bbox)
    }

    /// Validates this value.
    pub fn validate(self) -> Result<()> {
        if [self.x, self.y, self.width, self.height]
            .iter()
            .any(|value| !value.is_finite())
            || self.width <= 0.0
            || self.height <= 0.0
        {
            return Err(DetectError::InvalidArgument(
                "face boxes must be finite and have non-zero dimensions".to_string(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
/// Normalized face landmark points.
pub struct FaceLandmarks {
    /// The points value.
    pub points: Vec<[f32; 2]>,
}

impl FaceLandmarks {
    /// Creates a new value.
    pub fn new(points: Vec<[f32; 2]>) -> Result<Self> {
        if points.iter().flatten().any(|value| !value.is_finite()) {
            return Err(DetectError::InvalidArgument(
                "face landmarks must be finite".to_string(),
            ));
        }
        Ok(Self { points })
    }
}

#[derive(Debug, Clone, PartialEq)]
/// Data type for face detection.
pub struct FaceDetection {
    /// The bounding box value.
    pub bbox: FaceBox,
    /// The confidence value.
    pub confidence: f32,
    /// The landmarks value.
    pub landmarks: Option<FaceLandmarks>,
    /// Adapter-specific attributes.
    pub attributes: BTreeMap<String, String>,
}

impl FaceDetection {
    /// Creates a new value.
    pub fn new(bbox: FaceBox, confidence: f32) -> Result<Self> {
        bbox.validate()?;
        if !confidence.is_finite() {
            return Err(DetectError::InvalidArgument(
                "face detection confidence must be finite".to_string(),
            ));
        }
        Ok(Self {
            bbox,
            confidence,
            landmarks: None,
            attributes: BTreeMap::new(),
        })
    }

    /// Returns landmarks.
    pub fn landmarks(mut self, landmarks: FaceLandmarks) -> Self {
        self.landmarks = Some(landmarks);
        self
    }

    /// Returns attribute.
    pub fn attribute(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.attributes.insert(key.into(), value.into());
        self
    }
}

/// Trait for face detector backend implementations.
pub trait FaceDetectorBackend {
    /// Returns detect faces.
    fn detect_faces(&mut self, image: &ImageView<'_>) -> Result<Vec<FaceDetection>>;
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Color target for blob detection.
pub enum TargetColor {
    /// Red-dominant regions.
    Red,
}

#[derive(Debug, Clone, PartialEq)]
/// Options for simple color-blob object detection.
pub struct ColorBlobDetectionOptions {
    /// Target color to detect.
    pub target: TargetColor,
    /// Minimum red channel value for red targets.
    pub min_red: u8,
    /// Minimum dominance ratio versus the other RGB channels.
    pub dominance_ratio: f32,
    /// Minimum connected-component area after morphology.
    pub min_area_pixels: usize,
    /// Output object label.
    pub label: String,
    /// Whether to apply a 3x3 morphological opening before component extraction.
    pub morph_open_3x3: bool,
}

impl ColorBlobDetectionOptions {
    /// Returns red-car defaults matching the external OpenCV adapter.
    pub fn red_car() -> Self {
        Self::default()
    }

    /// Returns min area pixels.
    pub fn min_area_pixels(mut self, value: usize) -> Self {
        self.min_area_pixels = value.max(1);
        self
    }

    /// Returns label.
    pub fn label(mut self, value: impl Into<String>) -> Self {
        self.label = value.into();
        self
    }
}

impl Default for ColorBlobDetectionOptions {
    fn default() -> Self {
        Self {
            target: TargetColor::Red,
            min_red: 96,
            dominance_ratio: 1.35,
            min_area_pixels: 24,
            label: "car".to_string(),
            morph_open_3x3: true,
        }
    }
}

#[derive(Debug, Clone)]
/// Detector for connected color blobs in still images or video frames.
pub struct ColorBlobDetector {
    options: ColorBlobDetectionOptions,
}

impl ColorBlobDetector {
    /// Creates a new detector.
    pub fn new(options: ColorBlobDetectionOptions) -> Result<Self> {
        validate_color_blob_options(&options)?;
        Ok(Self { options })
    }

    /// Creates a red-car detector.
    pub fn red_car() -> Self {
        Self {
            options: ColorBlobDetectionOptions::red_car(),
        }
    }

    /// Returns options.
    pub fn options(&self) -> &ColorBlobDetectionOptions {
        &self.options
    }

    /// Detects color blobs in an image.
    pub fn detect_image(&mut self, image: &ImageView<'_>) -> Result<Vec<ImageDetection>> {
        image.validate()?;
        let width = image.width as usize;
        let height = image.height as usize;
        let mut mask = build_color_mask(*image, &self.options);
        if self.options.morph_open_3x3 {
            mask = dilate_3x3(width, height, &erode_3x3(width, height, &mask));
        }
        Ok(connected_components(
            width,
            height,
            &mask,
            &self.options.label,
            self.options.min_area_pixels,
            self.options.target,
        ))
    }

    /// Detects color blobs in a video frame.
    pub fn detect_frame(&mut self, frame: &VideoFrame<'_>) -> Result<FrameDetections> {
        let image = ImageView::from_video_frame(frame)?;
        let detections = self.detect_image(&image)?;
        Ok(FrameDetections {
            position: frame.position,
            detections,
        })
    }
}

fn validate_color_blob_options(options: &ColorBlobDetectionOptions) -> Result<()> {
    if !options.dominance_ratio.is_finite() || options.dominance_ratio <= 0.0 {
        return Err(DetectError::InvalidArgument(
            "dominance_ratio must be finite and positive".to_string(),
        ));
    }
    if options.min_area_pixels == 0 {
        return Err(DetectError::InvalidArgument(
            "min_area_pixels must be positive".to_string(),
        ));
    }
    if options.label.trim().is_empty() {
        return Err(DetectError::InvalidArgument(
            "color blob label must not be empty".to_string(),
        ));
    }
    Ok(())
}

fn build_color_mask(image: ImageView<'_>, options: &ColorBlobDetectionOptions) -> Vec<bool> {
    let mut mask = vec![false; image.width as usize * image.height as usize];
    for y in 0..image.height {
        for x in 0..image.width {
            let [red, green, blue] = image.pixel_rgb(x, y);
            let active = match options.target {
                TargetColor::Red => {
                    red >= options.min_red
                        && (red as f32) >= options.dominance_ratio * green as f32
                        && (red as f32) >= options.dominance_ratio * blue as f32
                }
            };
            mask[y as usize * image.width as usize + x as usize] = active;
        }
    }
    mask
}

fn erode_3x3(width: usize, height: usize, mask: &[bool]) -> Vec<bool> {
    let mut out = vec![false; mask.len()];
    if width < 3 || height < 3 {
        return out;
    }
    for y in 1..height - 1 {
        for x in 1..width - 1 {
            let mut active = true;
            'neighbors: for yy in y - 1..=y + 1 {
                for xx in x - 1..=x + 1 {
                    if !mask[yy * width + xx] {
                        active = false;
                        break 'neighbors;
                    }
                }
            }
            out[y * width + x] = active;
        }
    }
    out
}

fn dilate_3x3(width: usize, height: usize, mask: &[bool]) -> Vec<bool> {
    let mut out = vec![false; mask.len()];
    if width == 0 || height == 0 {
        return out;
    }
    for y in 0..height {
        for x in 0..width {
            let y0 = y.saturating_sub(1);
            let x0 = x.saturating_sub(1);
            let y1 = (y + 1).min(height - 1);
            let x1 = (x + 1).min(width - 1);
            let mut active = false;
            'neighbors: for yy in y0..=y1 {
                for xx in x0..=x1 {
                    if mask[yy * width + xx] {
                        active = true;
                        break 'neighbors;
                    }
                }
            }
            out[y * width + x] = active;
        }
    }
    out
}

fn connected_components(
    width: usize,
    height: usize,
    mask: &[bool],
    label: &str,
    min_area_pixels: usize,
    target: TargetColor,
) -> Vec<ImageDetection> {
    let mut seen = vec![false; mask.len()];
    let mut detections = Vec::new();
    let mut stack = Vec::new();
    for start in 0..mask.len() {
        if seen[start] || !mask[start] {
            continue;
        }
        let mut min_x = width;
        let mut min_y = height;
        let mut max_x = 0_usize;
        let mut max_y = 0_usize;
        let mut area = 0_usize;
        seen[start] = true;
        stack.push(start);
        while let Some(index) = stack.pop() {
            let x = index % width;
            let y = index / width;
            area += 1;
            min_x = min_x.min(x);
            min_y = min_y.min(y);
            max_x = max_x.max(x);
            max_y = max_y.max(y);

            let x0 = x.saturating_sub(1);
            let y0 = y.saturating_sub(1);
            let x1 = (x + 1).min(width - 1);
            let y1 = (y + 1).min(height - 1);
            for yy in y0..=y1 {
                for xx in x0..=x1 {
                    let next = yy * width + xx;
                    if !seen[next] && mask[next] {
                        seen[next] = true;
                        stack.push(next);
                    }
                }
            }
        }
        if area < min_area_pixels {
            continue;
        }
        let mut attributes = BTreeMap::new();
        attributes.insert(
            "color".to_string(),
            match target {
                TargetColor::Red => "red",
            }
            .to_string(),
        );
        attributes.insert("area".to_string(), area.to_string());
        if let Ok(region) = BoundingBox::new(
            min_x as u32,
            min_y as u32,
            (max_x - min_x + 1) as u32,
            (max_y - min_y + 1) as u32,
        ) {
            detections.push(ImageDetection {
                label: label.to_string(),
                score: Some(0.95),
                region,
                attributes,
            });
        }
    }
    detections
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

    #[test]
    fn red_blob_detector_finds_red_rectangle() {
        let mut data = vec![0_u8; 16 * 16 * 3];
        for y in 4..10 {
            for x in 3..11 {
                let offset = (y * 16 + x) * 3;
                data[offset] = 220;
                data[offset + 1] = 20;
                data[offset + 2] = 20;
            }
        }
        let image = OwnedImage::new_rgb(16, 16, data).unwrap();
        let mut detector = ColorBlobDetector::red_car();
        let detections = detector.detect_image(&image.as_view()).unwrap();
        assert_eq!(detections.len(), 1);
        assert_eq!(detections[0].label, "car");
        assert_eq!(detections[0].attributes["color"], "red");
        assert_eq!(detections[0].region, BoundingBox::new(3, 4, 8, 6).unwrap());
    }

    #[test]
    fn red_blob_detector_removes_small_noise() {
        let mut data = vec![0_u8; 16 * 16 * 3];
        let offset = (7 * 16 + 7) * 3;
        data[offset] = 255;
        let image = OwnedImage::new_rgb(16, 16, data).unwrap();
        let mut detector = ColorBlobDetector::red_car();
        let detections = detector.detect_image(&image.as_view()).unwrap();
        assert!(detections.is_empty());
    }

    #[test]
    fn red_blob_detector_handles_bgr_and_gray() {
        let mut bgr = vec![0_u8; 12 * 12 * 3];
        for y in 2..8 {
            for x in 2..8 {
                let offset = (y * 12 + x) * 3;
                bgr[offset] = 10;
                bgr[offset + 1] = 10;
                bgr[offset + 2] = 220;
            }
        }
        let image = OwnedImage::new_bgr(12, 12, bgr).unwrap();
        let mut detector = ColorBlobDetector::red_car();
        assert_eq!(detector.detect_image(&image.as_view()).unwrap().len(), 1);

        let gray = OwnedImage::new_gray(12, 12, vec![220; 12 * 12]).unwrap();
        assert!(detector.detect_image(&gray.as_view()).unwrap().is_empty());
    }

    #[test]
    fn color_blob_detector_validates_options() {
        let invalid_ratio = ColorBlobDetector::new(ColorBlobDetectionOptions {
            dominance_ratio: f32::NAN,
            ..ColorBlobDetectionOptions::default()
        })
        .unwrap_err();
        assert!(matches!(invalid_ratio, DetectError::InvalidArgument(_)));

        let empty_label = ColorBlobDetector::new(ColorBlobDetectionOptions {
            label: "  ".to_string(),
            ..ColorBlobDetectionOptions::default()
        })
        .unwrap_err();
        assert!(matches!(empty_label, DetectError::InvalidArgument(_)));
    }

    #[test]
    fn color_blob_detector_supports_custom_options_and_video_frames() {
        let mut data = vec![0_u8; 8 * 8 * 3];
        for &(x, y) in &[(1_usize, 1_usize), (6, 2)] {
            let offset = (y * 8 + x) * 3;
            data[offset] = 180;
            data[offset + 1] = 50;
            data[offset + 2] = 50;
        }
        let mut detector = ColorBlobDetector::new(
            ColorBlobDetectionOptions::red_car()
                .min_area_pixels(1)
                .label("marker"),
        )
        .unwrap();
        detector.options.morph_open_3x3 = false;

        let detections = detector.detect_frame(&frame(&data)).unwrap();
        assert_eq!(detections.position.frame_index, 3);
        assert_eq!(detections.detections.len(), 2);
        assert!(detections
            .detections
            .iter()
            .all(|detection| detection.label == "marker"));
        assert_eq!(detections.detections[0].attributes["area"], "1");
    }

    #[test]
    fn face_detection_contracts_validate_inputs() {
        let bbox = FaceBox::new(0.1, 0.2, 0.3, 0.4).unwrap();
        let detection = FaceDetection::new(bbox, 0.95)
            .unwrap()
            .landmarks(FaceLandmarks::new(vec![[0.2, 0.3], [0.4, 0.5]]).unwrap());
        assert_eq!(detection.landmarks.unwrap().points.len(), 2);
        assert!(FaceBox::new(0.0, 0.0, f32::NAN, 1.0).is_err());
        assert_eq!(
            FaceDetectionPreset::OpenCvYuNet
                .model_spec()
                .repo_id_value(),
            Some("opencv/face_detection_yunet")
        );
    }
}
