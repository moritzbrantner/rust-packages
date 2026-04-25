#![doc = include_str!("../README.md")]

use std::collections::BTreeMap;

use image_analysis_core::ImageView;
use image_analysis_segmentation::ImageSegmentationBackend;
use video_analysis_core::{DetectError, Result};
use video_analysis_models::{HuggingFaceModelSpec, ModelTask};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SamImagePreset {
    #[default]
    VitBase,
    VitLarge,
    VitHuge,
}

impl SamImagePreset {
    pub const ALL: &'static [Self] = &[Self::VitBase, Self::VitLarge, Self::VitHuge];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::VitBase => "sam-vit-base",
            Self::VitLarge => "sam-vit-large",
            Self::VitHuge => "sam-vit-huge",
        }
    }

    pub fn repo_id(self) -> &'static str {
        match self {
            Self::VitBase => "facebook/sam-vit-base",
            Self::VitLarge => "facebook/sam-vit-large",
            Self::VitHuge => "facebook/sam-vit-huge",
        }
    }

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

pub fn default_sam_model_spec() -> HuggingFaceModelSpec {
    SamImagePreset::default().model_spec()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ImageClassificationPreset {
    #[default]
    VitBasePatch16_224,
}

impl ImageClassificationPreset {
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
pub enum ImageEmbeddingPreset {
    #[default]
    ClipVitBasePatch32,
}

impl ImageEmbeddingPreset {
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
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ImageCaptionPreset {
    #[default]
    BlipBase,
}

impl ImageCaptionPreset {
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
pub struct ImageClassification {
    pub label: String,
    pub score: Option<f32>,
    pub attributes: BTreeMap<String, String>,
}

impl ImageClassification {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            score: None,
            attributes: BTreeMap::new(),
        }
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

#[derive(Debug, Clone, PartialEq)]
pub struct ImageEmbedding {
    pub vector: Vec<f32>,
    pub attributes: BTreeMap<String, String>,
}

impl ImageEmbedding {
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

    pub fn attribute(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.attributes.insert(key.into(), value.into());
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ImageCaption {
    pub text: String,
    pub score: Option<f32>,
    pub attributes: BTreeMap<String, String>,
}

impl ImageCaption {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            score: None,
            attributes: BTreeMap::new(),
        }
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

pub trait ModelBackedImageSegmentationBackend: ImageSegmentationBackend {
    fn model_spec(&self) -> HuggingFaceModelSpec;
}

pub trait ImageClassifierBackend {
    fn classify_image(&mut self, image: &ImageView<'_>) -> Result<Vec<ImageClassification>>;
}

pub trait ImageEmbedderBackend {
    fn embed_image(&mut self, image: &ImageView<'_>) -> Result<ImageEmbedding>;
}

pub trait ImageCaptionerBackend {
    fn caption_image(&mut self, image: &ImageView<'_>) -> Result<Vec<ImageCaption>>;
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
    fn fake_classifier_backend_returns_labels() {
        let mut backend = StubClassifier;
        let image = image_analysis_core::OwnedImage::new_rgb(1, 1, vec![0, 0, 0]).unwrap();
        let labels = backend.classify_image(&image.as_view()).unwrap();
        assert_eq!(labels[0].label, "demo");
    }
}
