#![doc = include_str!("../README.md")]

pub mod surface;
use std::collections::BTreeMap;

use image_analysis_core::ImageView;
use model_runtime::{HuggingFaceModelSpec, ModelPreset, ModelRuntimeBackend, ModelTask};
use serde::{Deserialize, Serialize};
use video_analysis_core::{DetectError, Result};

pub use image_analysis_detection::{
    ColorBlobDetectionOptions, ColorBlobDetector, FaceBox, FaceDetection, FaceDetectionPreset,
    FaceDetectorBackend, FaceLandmarks, FrameDetections, ImageDetection, ImageDetectionRequest,
    MaskProposalDetector, TargetColor,
};
pub use image_analysis_ocr::{
    default_ocr_model_spec, ExternalOcrCommandSpec, FrameOcrDocument, ModelBackedOcrBackend,
    OcrBackend, OcrBlockKind, OcrConfidence, OcrDocument, OcrPipeline, OcrPreset, OcrRequest,
    OcrTechnique, OcrTextBlock, OcrTextLine, OcrToken, VideoOcrDocument,
};
pub use image_analysis_segmentation::{
    default_sam_model_spec, BinaryMask, ImageSegment, ImageSegmentationBackend,
    ImageSegmentationPrompt, ImageSegmentationRequest, ModelBackedImageSegmentationBackend,
    PointLabel, SamImagePreset, SegmentationPoint,
};
pub use image_analysis_synthesis::{
    image_from_luma_histogram, paint_regions, solid_image, vertical_gradient, ImageSynthesisConfig,
    RegionPaint, RgbColor,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
/// Image task families exposed by the aggregate task layer.
pub enum ImageTask {
    /// Whole-image classification.
    ImageClassification,
    /// Dense image embedding.
    ImageEmbedding,
    /// Image caption generation.
    ImageCaptioning,
    /// Prompted or automatic image segmentation.
    ImageSegmentation,
    /// Object or region detection.
    ObjectDetection,
    /// Face detection.
    FaceDetection,
    /// Face embedding.
    FaceEmbedding,
    /// Optical character recognition.
    Ocr,
    /// Deterministic or model-backed image generation.
    ImageGeneration,
}

impl ImageTask {
    /// Returns all task variants.
    pub const ALL: &'static [Self] = &[
        Self::ImageClassification,
        Self::ImageEmbedding,
        Self::ImageCaptioning,
        Self::ImageSegmentation,
        Self::ObjectDetection,
        Self::FaceDetection,
        Self::FaceEmbedding,
        Self::Ocr,
        Self::ImageGeneration,
    ];

    /// Returns the stable API path segment for this task.
    pub fn path_segment(self) -> &'static str {
        match self {
            Self::ImageClassification => "classify",
            Self::ImageEmbedding => "embed",
            Self::ImageCaptioning => "caption",
            Self::ImageSegmentation => "segment",
            Self::ObjectDetection => "detect",
            Self::FaceDetection => "faces",
            Self::FaceEmbedding => "face-embed",
            Self::Ocr => "ocr",
            Self::ImageGeneration => "generate",
        }
    }
}

/// Runtime families for image execution and postprocessing.
pub type ImageRuntime = ModelRuntimeBackend;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
/// Shared task runtime metadata for UI and CLI discovery.
pub struct ImageTaskCatalogEntry {
    /// Stable local entry id.
    pub id: String,
    /// Upstream model id or strategy id.
    pub model_id: String,
    /// Task family.
    pub task: ImageTask,
    /// Preferred runtime.
    pub runtime: ImageRuntime,
    /// Whether the runtime is available in the default contributor build.
    pub supported: bool,
    /// Optional fallback entry id.
    pub fallback: Option<String>,
    /// Human-readable note for unsupported or fallback-only paths.
    pub note: Option<String>,
}

/// Compatibility alias for callers migrating from model-shaped metadata names.
pub type ImageModelMetadata = ImageTaskCatalogEntry;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
/// Variants describing image classification preset.
pub enum ImageClassificationPreset {
    #[default]
    /// The ViT base patch16 224 variant.
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
    /// The CLIP ViT-B/32 variant.
    ClipVitBasePatch32,
    /// The Xenova CLIP ViT-B/32 ONNX variant.
    XenovaClipVitBasePatch32Onnx,
}

impl ImageEmbeddingPreset {
    /// Returns model spec.
    pub fn model_spec(self) -> HuggingFaceModelSpec {
        match self {
            Self::ClipVitBasePatch32 => {
                HuggingFaceModelSpec::new("openai/clip-vit-base-patch32", ModelTask::ImageEmbedding)
                    .name("clip-vit-base-patch32")
                    .file("config.json")
                    .file("preprocessor_config.json")
                    .first_available_file(["model.safetensors", "pytorch_model.bin"])
            }
            Self::XenovaClipVitBasePatch32Onnx => {
                HuggingFaceModelSpec::new("Xenova/clip-vit-base-patch32", ModelTask::ImageEmbedding)
                    .name("xenova-clip-vit-base-patch32-onnx")
                    .file("config.json")
                    .file("preprocessor_config.json")
                    .first_available_file([
                        "onnx/vision_model_quantized.onnx",
                        "onnx/vision_model.onnx",
                    ])
            }
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
            Self::OpenCvSFace => {
                HuggingFaceModelSpec::new("opencv/face_recognition_sface", ModelTask::FaceEmbedding)
                    .name("opencv-sface-onnx")
                    .file("face_recognition_sface_2021dec.onnx")
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
/// Variants describing image caption preset.
pub enum ImageCaptionPreset {
    #[default]
    /// The BLIP base variant.
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
    /// Adapter-specific attributes.
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
    /// Adapter-specific attributes.
    pub attributes: BTreeMap<String, String>,
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
/// Data type for face embedding.
pub struct FaceEmbedding {
    /// The vector value.
    pub vector: Vec<f32>,
    /// Adapter-specific attributes.
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

#[derive(Debug, Clone, PartialEq)]
/// Data type for image caption.
pub struct ImageCaption {
    /// Text content for this value.
    pub text: String,
    /// Score assigned to this value.
    pub score: Option<f32>,
    /// Adapter-specific attributes.
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

/// Trait for face embedder backend implementations.
pub trait FaceEmbedderBackend {
    /// Returns embed face.
    fn embed_face(
        &mut self,
        image: &ImageView<'_>,
        detection: Option<&FaceDetection>,
    ) -> Result<FaceEmbedding>;
}

/// Returns the image task catalog, optionally filtered by task.
pub fn image_task_catalog(task: Option<ImageTask>) -> Vec<ImageTaskCatalogEntry> {
    let entries = vec![
        entry(
            "vit-base-patch16-224",
            "google/vit-base-patch16-224",
            ImageTask::ImageClassification,
            ImageRuntime::Candle,
            false,
            None,
            Some("Preset metadata is registered; native execution is explicit/runtime-gated."),
        ),
        entry(
            "clip-vit-base-patch32",
            "openai/clip-vit-base-patch32",
            ImageTask::ImageEmbedding,
            ImageRuntime::Candle,
            false,
            Some("xenova-clip-vit-base-patch32-onnx"),
            Some("Embedding contract is available; native CLIP execution is adapter-provided."),
        ),
        entry(
            "xenova-clip-vit-base-patch32-onnx",
            "Xenova/clip-vit-base-patch32",
            ImageTask::ImageEmbedding,
            ImageRuntime::Onnx,
            false,
            None,
            Some("Use image-analysis-onnx with an explicit runtime feature for execution."),
        ),
        entry(
            "sam-vit-base",
            "facebook/sam-vit-base",
            ImageTask::ImageSegmentation,
            ImageRuntime::Candle,
            false,
            None,
            Some("Segmentation prompts and mask contracts live in image-analysis-segmentation."),
        ),
        entry(
            "color-blob-detector",
            "color_blob",
            ImageTask::ObjectDetection,
            ImageRuntime::Heuristic,
            true,
            None,
            Some("Deterministic local connected-component detector from image-analysis-detection."),
        ),
        entry(
            "opencv-yunet-onnx",
            "opencv/face_detection_yunet",
            ImageTask::FaceDetection,
            ImageRuntime::Onnx,
            false,
            None,
            Some("Face detection contracts live in image-analysis-detection."),
        ),
        entry(
            "opencv-sface-onnx",
            "opencv/face_recognition_sface",
            ImageTask::FaceEmbedding,
            ImageRuntime::Onnx,
            false,
            None,
            Some("Face embedding contracts live in the aggregate image task layer."),
        ),
        entry(
            "trocr-base-printed",
            "microsoft/trocr-base-printed",
            ImageTask::Ocr,
            ImageRuntime::Candle,
            false,
            Some("heuristic-ocr"),
            Some("OCR request and rich text contracts live in image-analysis-ocr."),
        ),
        entry(
            "heuristic-ocr",
            "heuristic_ocr",
            ImageTask::Ocr,
            ImageRuntime::Heuristic,
            true,
            None,
            Some("Deterministic OCR-compatible placeholder path for tests and adapters."),
        ),
        entry(
            "solid-image",
            "solid_image",
            ImageTask::ImageGeneration,
            ImageRuntime::Heuristic,
            true,
            None,
            Some("Deterministic synthesis helper from image-analysis-synthesis."),
        ),
        entry(
            "blip-image-captioning-base",
            "Salesforce/blip-image-captioning-base",
            ImageTask::ImageCaptioning,
            ImageRuntime::Candle,
            false,
            None,
            Some("Captioning contract is available; model execution is adapter-provided."),
        ),
    ];

    entries
        .into_iter()
        .filter(|entry| task.map(|task| task == entry.task).unwrap_or(true))
        .collect()
}

/// Compatibility function name for routes that still expose `/api/models`.
#[deprecated(note = "use image_task_catalog")]
pub fn model_catalog(task: Option<ImageTask>) -> Vec<ImageTaskCatalogEntry> {
    image_task_catalog(task)
}

/// Parses an image task path segment.
pub fn parse_task(input: &str) -> Option<ImageTask> {
    ImageTask::ALL
        .iter()
        .copied()
        .find(|task| {
            task.path_segment() == input || format!("{task:?}").eq_ignore_ascii_case(input)
        })
        .or(match input {
            "image_classification" | "classification" => Some(ImageTask::ImageClassification),
            "image_embedding" | "embedding" => Some(ImageTask::ImageEmbedding),
            "image_captioning" | "captioning" => Some(ImageTask::ImageCaptioning),
            "image_segmentation" | "segmentation" => Some(ImageTask::ImageSegmentation),
            "object_detection" | "detection" => Some(ImageTask::ObjectDetection),
            "face_detection" | "face" => Some(ImageTask::FaceDetection),
            "face_embedding" => Some(ImageTask::FaceEmbedding),
            "optical_character_recognition" => Some(ImageTask::Ocr),
            "image_generation" | "synthesis" => Some(ImageTask::ImageGeneration),
            _ => None,
        })
}

/// Returns preset ids registered in model-runtime for image tasks.
pub fn registered_image_presets() -> Vec<String> {
    ModelPreset::ALL
        .iter()
        .copied()
        .filter(|preset| {
            matches!(
                preset.spec().task,
                ModelTask::ObjectDetection
                    | ModelTask::ImageClassification
                    | ModelTask::ImageSegmentation
                    | ModelTask::ImageEmbedding
                    | ModelTask::FaceDetection
                    | ModelTask::FaceEmbedding
                    | ModelTask::Ocr
            )
        })
        .map(|preset| preset.as_str().to_string())
        .collect()
}

/// Returns a compact schema summary for OpenAPI metadata.
pub fn schema_summary() -> serde_json::Value {
    serde_json::json!({
        "tasks": ImageTask::ALL.iter().map(|task| task.path_segment()).collect::<Vec<_>>(),
        "runtimes": image_task_catalog(None),
        "models": image_task_catalog(None),
        "registeredPresets": registered_image_presets()
    })
}

fn entry(
    id: &str,
    model_id: &str,
    task: ImageTask,
    runtime: ImageRuntime,
    supported: bool,
    fallback: Option<&str>,
    note: Option<&str>,
) -> ImageTaskCatalogEntry {
    ImageTaskCatalogEntry {
        id: id.to_string(),
        model_id: model_id.to_string(),
        task,
        runtime,
        supported,
        fallback: fallback.map(str::to_string),
        note: note.map(str::to_string),
    }
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
    fn catalog_lists_image_tasks() {
        let entries = image_task_catalog(None);
        assert!(entries.iter().any(|entry| entry.id == "sam-vit-base"));
        assert!(entries
            .iter()
            .any(|entry| entry.task == ImageTask::ObjectDetection));
        assert_eq!(parse_task("classify"), Some(ImageTask::ImageClassification));
    }

    #[test]
    fn deprecated_model_catalog_delegates_to_task_catalog() {
        #[allow(deprecated)]
        let models = model_catalog(Some(ImageTask::ImageEmbedding));
        assert_eq!(models, image_task_catalog(Some(ImageTask::ImageEmbedding)));
    }

    #[test]
    fn presets_use_image_model_tasks() {
        assert_eq!(
            ImageClassificationPreset::default().model_spec().task,
            ModelTask::ImageClassification
        );
        assert_eq!(
            ImageEmbeddingPreset::XenovaClipVitBasePatch32Onnx
                .model_spec()
                .repo_id_value(),
            Some("Xenova/clip-vit-base-patch32")
        );
        assert_eq!(
            FaceEmbeddingPreset::OpenCvSFace
                .model_spec()
                .repo_id_value(),
            Some("opencv/face_recognition_sface")
        );
    }

    #[test]
    fn embeddings_reject_non_finite_values() {
        assert!(ImageEmbedding::new(vec![f32::NAN]).is_err());
        assert!(FaceEmbedding::new(vec![f32::INFINITY]).is_err());
    }

    #[test]
    fn fake_classifier_backend_returns_labels() {
        let mut backend = StubClassifier;
        let image = image_analysis_core::OwnedImage::new_rgb(1, 1, vec![0, 0, 0]).unwrap();
        let labels = backend.classify_image(&image.as_view()).unwrap();
        assert_eq!(labels[0].label, "demo");
    }
}
