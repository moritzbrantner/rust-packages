#![doc = include_str!("../README.md")]

use std::collections::BTreeMap;

use image_analysis_core::ImageView;
use image_analysis_segmentation::ImageSegmentationBackend;
use video_analysis_core::{DetectError, Result};
use video_analysis_models::{HuggingFaceModelSpec, ModelTask};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
/// Variants describing SAM image preset.
pub enum SamImagePreset {
    #[default]
    /// The vit base variant.
    VitBase,
    /// The vit large variant.
    VitLarge,
    /// The vit huge variant.
    VitHuge,
}

impl SamImagePreset {
    /// Constant for all.
    pub const ALL: &'static [Self] = &[Self::VitBase, Self::VitLarge, Self::VitHuge];

    /// Borrows this value as a str.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::VitBase => "sam-vit-base",
            Self::VitLarge => "sam-vit-large",
            Self::VitHuge => "sam-vit-huge",
        }
    }

    /// Returns repo identifier.
    pub fn repo_id(self) -> &'static str {
        match self {
            Self::VitBase => "facebook/sam-vit-base",
            Self::VitLarge => "facebook/sam-vit-large",
            Self::VitHuge => "facebook/sam-vit-huge",
        }
    }

    /// Returns model spec.
    pub fn model_spec(self) -> HuggingFaceModelSpec {
        HuggingFaceModelSpec::new(
            self.repo_id(),
            ModelTask::Custom("image_segmentation".to_string()),
        )
        .name(self.as_str())
        .file("config.json")
        .file("preprocessor_config.json")
        .first_available_file(["model.safetensors", "pytorch_model.bin"])
    }
}

/// Returns default SAM model spec.
pub fn default_sam_model_spec() -> HuggingFaceModelSpec {
    SamImagePreset::default().model_spec()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
/// Variants describing image classification preset.
pub enum ImageClassificationPreset {
    #[default]
    /// The vit base patch16 224 variant.
    VitBasePatch16_224,
}

impl ImageClassificationPreset {
    /// Returns model spec.
    pub fn model_spec(self) -> HuggingFaceModelSpec {
        match self {
            Self::VitBasePatch16_224 => HuggingFaceModelSpec::new(
                "google/vit-base-patch16-224",
                ModelTask::ImageClassification,
            )
            .name("vit-base-patch16-224")
            .file("config.json")
            .file("preprocessor_config.json")
            .first_available_file(["model.safetensors", "pytorch_model.bin"]),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
/// Variants describing image embedding preset.
pub enum ImageEmbeddingPreset {
    #[default]
    /// The clip vit base patch32 variant.
    ClipVitBasePatch32,
    /// The Xenova CLIP ViT-B/32 ONNX variant.
    XenovaClipVitBasePatch32Onnx,
}

impl ImageEmbeddingPreset {
    /// Returns model spec.
    pub fn model_spec(self) -> HuggingFaceModelSpec {
        match self {
            Self::ClipVitBasePatch32 => HuggingFaceModelSpec::new(
                "openai/clip-vit-base-patch32",
                ModelTask::Custom("image_embedding".to_string()),
            )
            .name("clip-vit-base-patch32")
            .file("config.json")
            .file("preprocessor_config.json")
            .first_available_file(["model.safetensors", "pytorch_model.bin"]),
            Self::XenovaClipVitBasePatch32Onnx => HuggingFaceModelSpec::new(
                "Xenova/clip-vit-base-patch32",
                ModelTask::Custom("image_embedding".to_string()),
            )
            .name("xenova-clip-vit-base-patch32-onnx")
            .file("config.json")
            .file("preprocessor_config.json")
            .first_available_file(["onnx/vision_model_quantized.onnx", "onnx/vision_model.onnx"]),
        }
    }
}

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
            Self::OpenCvYuNet => HuggingFaceModelSpec::new(
                "opencv/face_detection_yunet",
                ModelTask::Custom("face_detection".to_string()),
            )
            .name("opencv-yunet-onnx")
            .first_available_file([
                "face_detection_yunet_2023mar.onnx",
                "face_detection_yunet_2023mar_int8.onnx",
                "face_detection_yunet_2023mar_int8bq.onnx",
            ]),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
/// Variants describing face embedding preset.
pub enum FaceEmbeddingPreset {
    #[default]
    /// The OpenCV SFace ONNX variant.
    OpenCvSFace,
}

impl FaceEmbeddingPreset {
    /// Returns model spec.
    pub fn model_spec(self) -> HuggingFaceModelSpec {
        match self {
            Self::OpenCvSFace => HuggingFaceModelSpec::new(
                "opencv/face_recognition_sface",
                ModelTask::Custom("face_embedding".to_string()),
            )
            .name("opencv-sface-onnx")
            .file("face_recognition_sface_2021dec.onnx"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
/// Variants describing image caption preset.
pub enum ImageCaptionPreset {
    #[default]
    /// The blip base variant.
    BlipBase,
}

impl ImageCaptionPreset {
    /// Returns model spec.
    pub fn model_spec(self) -> HuggingFaceModelSpec {
        match self {
            Self::BlipBase => HuggingFaceModelSpec::new(
                "Salesforce/blip-image-captioning-base",
                ModelTask::Custom("image_captioning".to_string()),
            )
            .name("blip-image-captioning-base")
            .file("config.json")
            .file("preprocessor_config.json")
            .first_available_file(["model.safetensors", "pytorch_model.bin"]),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
/// Data type for image classification.
pub struct ImageClassification {
    /// Label assigned to this value.
    pub label: String,
    /// Score assigned to this value.
    pub score: Option<f32>,
    /// The attributes value.
    pub attributes: BTreeMap<String, String>,
}

impl ImageClassification {
    /// Creates a new value.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            score: None,
            attributes: BTreeMap::new(),
        }
    }

    /// Returns score.
    pub fn score(mut self, value: f32) -> Self {
        self.score = Some(value);
        self
    }

    /// Returns attribute.
    pub fn attribute(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.attributes.insert(key.into(), value.into());
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
/// Data type for image embedding.
pub struct ImageEmbedding {
    /// The vector value.
    pub vector: Vec<f32>,
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
    /// The attributes value.
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

#[derive(Debug, Clone, PartialEq)]
/// Data type for face embedding.
pub struct FaceEmbedding {
    /// The vector value.
    pub vector: Vec<f32>,
    /// The attributes value.
    pub attributes: BTreeMap<String, String>,
}

impl FaceEmbedding {
    /// Creates a new value.
    pub fn new(vector: Vec<f32>) -> Result<Self> {
        if vector.is_empty() || vector.iter().any(|value| !value.is_finite()) {
            return Err(DetectError::InvalidArgument(
                "face embedding vectors must be non-empty and finite".to_string(),
            ));
        }
        Ok(Self {
            vector,
            attributes: BTreeMap::new(),
        })
    }

    /// Returns attribute.
    pub fn attribute(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.attributes.insert(key.into(), value.into());
        self
    }
}

impl ImageEmbedding {
    /// Creates a new value.
    pub fn new(vector: Vec<f32>) -> Result<Self> {
        if vector.is_empty() || vector.iter().any(|value| !value.is_finite()) {
            return Err(DetectError::InvalidArgument(
                "image embedding vectors must be non-empty and finite".to_string(),
            ));
        }
        Ok(Self {
            vector,
            attributes: BTreeMap::new(),
        })
    }

    /// Returns attribute.
    pub fn attribute(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.attributes.insert(key.into(), value.into());
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
/// Data type for image caption.
pub struct ImageCaption {
    /// Text content for this value.
    pub text: String,
    /// Score assigned to this value.
    pub score: Option<f32>,
    /// The attributes value.
    pub attributes: BTreeMap<String, String>,
}

impl ImageCaption {
    /// Creates a new value.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            score: None,
            attributes: BTreeMap::new(),
        }
    }

    /// Returns score.
    pub fn score(mut self, value: f32) -> Self {
        self.score = Some(value);
        self
    }

    /// Returns attribute.
    pub fn attribute(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.attributes.insert(key.into(), value.into());
        self
    }
}

/// Trait for model backed image segmentation backend implementations.
pub trait ModelBackedImageSegmentationBackend: ImageSegmentationBackend {
    /// Returns model spec.
    fn model_spec(&self) -> HuggingFaceModelSpec;
}

/// Trait for image classifier backend implementations.
pub trait ImageClassifierBackend {
    /// Returns classify image.
    fn classify_image(&mut self, image: &ImageView<'_>) -> Result<Vec<ImageClassification>>;
}

/// Trait for image embedder backend implementations.
pub trait ImageEmbedderBackend {
    /// Returns embed image.
    fn embed_image(&mut self, image: &ImageView<'_>) -> Result<ImageEmbedding>;
}

/// Trait for image captioner backend implementations.
pub trait ImageCaptionerBackend {
    /// Returns caption image.
    fn caption_image(&mut self, image: &ImageView<'_>) -> Result<Vec<ImageCaption>>;
}

/// Trait for face detector backend implementations.
pub trait FaceDetectorBackend {
    /// Returns detect faces.
    fn detect_faces(&mut self, image: &ImageView<'_>) -> Result<Vec<FaceDetection>>;
}

/// Trait for face embedder backend implementations.
pub trait FaceEmbedderBackend {
    /// Returns embed face.
    fn embed_face(
        &mut self,
        image: &ImageView<'_>,
        detection: Option<&FaceDetection>,
    ) -> Result<FaceEmbedding>;
}

#[cfg(test)]
mod tests {
    use super::*;

    struct StubClassifier;

    impl ImageClassifierBackend for StubClassifier {
        fn classify_image(&mut self, _image: &ImageView<'_>) -> Result<Vec<ImageClassification>> {
            Ok(vec![ImageClassification::new("demo").score(0.9)])
        }
    }

    #[test]
    fn default_sam_model_spec_uses_vit_base() {
        assert_eq!(default_sam_model_spec().repo_id, "facebook/sam-vit-base");
    }

    #[test]
    fn image_classification_preset_uses_image_classification_task() {
        assert_eq!(
            ImageClassificationPreset::default().model_spec().task,
            ModelTask::ImageClassification
        );
    }

    #[test]
    fn embedding_rejects_non_finite_values() {
        assert!(ImageEmbedding::new(vec![f32::NAN]).is_err());
    }

    #[test]
    fn onnx_embedding_preset_uses_xenova_files() {
        let spec = ImageEmbeddingPreset::XenovaClipVitBasePatch32Onnx.model_spec();
        assert_eq!(spec.repo_id, "Xenova/clip-vit-base-patch32");
        assert_eq!(spec.files.len(), 3);
    }

    #[test]
    fn face_presets_use_opencv_models() {
        assert_eq!(
            FaceDetectionPreset::OpenCvYuNet.model_spec().repo_id,
            "opencv/face_detection_yunet"
        );
        assert_eq!(
            FaceEmbeddingPreset::OpenCvSFace.model_spec().repo_id,
            "opencv/face_recognition_sface"
        );
    }

    #[test]
    fn fake_classifier_backend_returns_labels() {
        let mut backend = StubClassifier;
        let image = image_analysis_core::OwnedImage::new_rgb(1, 1, vec![0, 0, 0]).unwrap();
        let labels = backend.classify_image(&image.as_view()).unwrap();
        assert_eq!(labels[0].label, "demo");
    }
}
