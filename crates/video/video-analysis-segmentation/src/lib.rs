#![doc = include_str!("../README.md")]

pub mod surface;
use image_analysis_segmentation::{ImageSegment, ImageSegmentationPrompt};
use model_runtime::{HuggingFaceModelSpec, ModelTask};
use video_analysis_core::{FramePosition, Result, VideoFrame};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
/// Variants describing SAM video preset.
pub enum SamVideoPreset {
    #[default]
    /// The sam2 1 hiera large variant.
    Sam2_1HieraLarge,
}

impl SamVideoPreset {
    /// Constant for all.
    pub const ALL: &'static [Self] = &[Self::Sam2_1HieraLarge];

    /// Borrows this value as a str.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Sam2_1HieraLarge => "sam2.1-hiera-large",
        }
    }

    /// Returns repo identifier.
    pub fn repo_id(self) -> &'static str {
        match self {
            Self::Sam2_1HieraLarge => "facebook/sam2.1-hiera-large",
        }
    }

    /// Returns model spec.
    pub fn model_spec(self) -> HuggingFaceModelSpec {
        HuggingFaceModelSpec::new(
            self.repo_id(),
            ModelTask::Custom("video_segmentation".to_string()),
        )
        .name(self.as_str())
        .file("config.json")
        .file("preprocessor_config.json")
        .file("processor_config.json")
        .file("video_preprocessor_config.json")
        .file("sam2.1_hiera_l.yaml")
        .first_available_file(["model.safetensors", "sam2.1_hiera_large.pt"])
    }
}

/// Returns default sam2 model spec.
pub fn default_sam2_model_spec() -> HuggingFaceModelSpec {
    SamVideoPreset::default().model_spec()
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Data type for video segmentation prompt.
pub struct VideoSegmentationPrompt {
    /// The prompt value.
    pub prompt: ImageSegmentationPrompt,
    /// The object identifier value.
    pub object_id: Option<String>,
    /// The propagate value.
    pub propagate: bool,
}

impl VideoSegmentationPrompt {
    /// Creates a new value.
    pub fn new(prompt: ImageSegmentationPrompt) -> Self {
        Self {
            prompt,
            object_id: None,
            propagate: true,
        }
    }

    /// Returns object identifier.
    pub fn object_id(mut self, value: impl Into<String>) -> Self {
        self.object_id = Some(value.into());
        self
    }

    /// Returns propagate.
    pub fn propagate(mut self, value: bool) -> Self {
        self.propagate = value;
        self
    }
}

impl Default for VideoSegmentationPrompt {
    fn default() -> Self {
        Self::new(ImageSegmentationPrompt::automatic_mask_generation())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
/// Data type for video segmentation request.
pub struct VideoSegmentationRequest {
    /// The prompt value.
    pub prompt: VideoSegmentationPrompt,
    /// The min mask pixels value.
    pub min_mask_pixels: usize,
}

impl VideoSegmentationRequest {
    /// Returns min mask pixels.
    pub fn min_mask_pixels(mut self, value: usize) -> Self {
        self.min_mask_pixels = value.max(1);
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
/// Data type for tracked segment.
pub struct TrackedSegment {
    /// The object identifier value.
    pub object_id: Option<String>,
    /// The segment value.
    pub segment: ImageSegment,
}

impl TrackedSegment {
    /// Creates a new value.
    pub fn new(segment: ImageSegment) -> Self {
        Self {
            object_id: None,
            segment,
        }
    }

    /// Returns object identifier.
    pub fn object_id(mut self, value: impl Into<String>) -> Self {
        self.object_id = Some(value.into());
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
/// Data type for frame segmentation.
pub struct FrameSegmentation {
    /// The position value.
    pub position: FramePosition,
    /// The segments value.
    pub segments: Vec<TrackedSegment>,
}

/// Trait for video segmentation backend implementations.
pub trait VideoSegmentationBackend {
    /// Returns model spec.
    fn model_spec(&self) -> HuggingFaceModelSpec {
        default_sam2_model_spec()
    }

    /// Returns segment frame.
    fn segment_frame(
        &mut self,
        frame: &VideoFrame<'_>,
        request: &VideoSegmentationRequest,
    ) -> Result<Vec<TrackedSegment>>;
}

/// Data type for video segmenter.
pub struct VideoSegmenter<B> {
    backend: B,
    request: VideoSegmentationRequest,
}

impl<B> VideoSegmenter<B> {
    /// Creates a new value.
    pub fn new(backend: B) -> Self {
        Self {
            backend,
            request: VideoSegmentationRequest::default(),
        }
    }

    /// Returns request.
    pub fn request(mut self, value: VideoSegmentationRequest) -> Self {
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

impl<B: VideoSegmentationBackend> VideoSegmenter<B> {
    /// Returns model spec.
    pub fn model_spec(&self) -> HuggingFaceModelSpec {
        self.backend.model_spec()
    }

    /// Returns process frame.
    pub fn process_frame(&mut self, frame: &VideoFrame<'_>) -> Result<FrameSegmentation> {
        let segments = self.backend.segment_frame(frame, &self.request)?;
        Ok(FrameSegmentation {
            position: frame.position,
            segments,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image_analysis_segmentation::{BinaryMask, ImageSegment};
    use video_analysis_core::{BoundingBox, PixelFormat, Timebase, Timestamp};

    struct StubVideoBackend;

    impl VideoSegmentationBackend for StubVideoBackend {
        fn segment_frame(
            &mut self,
            frame: &VideoFrame<'_>,
            request: &VideoSegmentationRequest,
        ) -> Result<Vec<TrackedSegment>> {
            let id = request
                .prompt
                .object_id
                .clone()
                .unwrap_or_else(|| format!("frame-{}", frame.position.frame_index));
            Ok(vec![TrackedSegment::new(
                ImageSegment::new(
                    BinaryMask::filled_rect(
                        frame.width,
                        frame.height,
                        BoundingBox::new(1, 1, 2, 2).unwrap(),
                    )
                    .unwrap(),
                )
                .unwrap(),
            )
            .object_id(id)])
        }
    }

    fn frame<'a>(data: &'a [u8]) -> VideoFrame<'a> {
        VideoFrame::packed(
            FramePosition {
                frame_index: 7,
                timestamp: Timestamp::new(7, Timebase::new(1, 1)),
            },
            4,
            4,
            PixelFormat::Rgb24,
            data,
            4 * 3,
        )
        .unwrap()
    }

    #[test]
    fn default_video_spec_uses_sam2_1() {
        assert_eq!(
            default_sam2_model_spec().repo_id_value(),
            Some("facebook/sam2.1-hiera-large")
        );
    }

    #[test]
    fn video_segmenter_preserves_frame_position() {
        let bytes = vec![0_u8; 4 * 4 * 3];
        let mut segmenter =
            VideoSegmenter::new(StubVideoBackend).request(VideoSegmentationRequest {
                prompt: VideoSegmentationPrompt::default().object_id("car"),
                min_mask_pixels: 1,
            });
        let frame = frame(&bytes);
        let output = segmenter.process_frame(&frame).unwrap();
        assert_eq!(output.position.frame_index, 7);
        assert_eq!(output.segments[0].object_id.as_deref(), Some("car"));
    }
}
