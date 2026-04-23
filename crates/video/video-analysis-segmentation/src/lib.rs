#![doc = include_str!("../README.md")]

use image_analysis_segmentation::{ImageSegment, ImageSegmentationPrompt};
use video_analysis_core::{FramePosition, Result, VideoFrame};
use video_analysis_models::{HuggingFaceModelSpec, ModelTask};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SamVideoPreset {
    #[default]
    Sam2_1HieraLarge,
}

impl SamVideoPreset {
    pub const ALL: &'static [Self] = &[Self::Sam2_1HieraLarge];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Sam2_1HieraLarge => "sam2.1-hiera-large",
        }
    }

    pub fn repo_id(self) -> &'static str {
        match self {
            Self::Sam2_1HieraLarge => "facebook/sam2.1-hiera-large",
        }
    }

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

pub fn default_sam2_model_spec() -> HuggingFaceModelSpec {
    SamVideoPreset::default().model_spec()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VideoSegmentationPrompt {
    pub prompt: ImageSegmentationPrompt,
    pub object_id: Option<String>,
    pub propagate: bool,
}

impl VideoSegmentationPrompt {
    pub fn new(prompt: ImageSegmentationPrompt) -> Self {
        Self {
            prompt,
            object_id: None,
            propagate: true,
        }
    }

    pub fn object_id(mut self, value: impl Into<String>) -> Self {
        self.object_id = Some(value.into());
        self
    }

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
pub struct VideoSegmentationRequest {
    pub prompt: VideoSegmentationPrompt,
    pub min_mask_pixels: usize,
}

impl VideoSegmentationRequest {
    pub fn min_mask_pixels(mut self, value: usize) -> Self {
        self.min_mask_pixels = value.max(1);
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TrackedSegment {
    pub object_id: Option<String>,
    pub segment: ImageSegment,
}

impl TrackedSegment {
    pub fn new(segment: ImageSegment) -> Self {
        Self {
            object_id: None,
            segment,
        }
    }

    pub fn object_id(mut self, value: impl Into<String>) -> Self {
        self.object_id = Some(value.into());
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct FrameSegmentation {
    pub position: FramePosition,
    pub segments: Vec<TrackedSegment>,
}

pub trait VideoSegmentationBackend {
    fn model_spec(&self) -> HuggingFaceModelSpec {
        default_sam2_model_spec()
    }

    fn segment_frame(
        &mut self,
        frame: &VideoFrame<'_>,
        request: &VideoSegmentationRequest,
    ) -> Result<Vec<TrackedSegment>>;
}

pub struct VideoSegmenter<B> {
    backend: B,
    request: VideoSegmentationRequest,
}

impl<B> VideoSegmenter<B> {
    pub fn new(backend: B) -> Self {
        Self {
            backend,
            request: VideoSegmentationRequest::default(),
        }
    }

    pub fn request(mut self, value: VideoSegmentationRequest) -> Self {
        self.request = value;
        self
    }

    pub fn backend(&self) -> &B {
        &self.backend
    }

    pub fn backend_mut(&mut self) -> &mut B {
        &mut self.backend
    }
}

impl<B: VideoSegmentationBackend> VideoSegmenter<B> {
    pub fn model_spec(&self) -> HuggingFaceModelSpec {
        self.backend.model_spec()
    }

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
            default_sam2_model_spec().repo_id,
            "facebook/sam2.1-hiera-large"
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
