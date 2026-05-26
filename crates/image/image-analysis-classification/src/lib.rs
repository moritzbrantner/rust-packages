#![doc = include_str!("../README.md")]

pub mod surface;

use std::collections::BTreeMap;

use image_analysis_core::ImageView;
use model_runtime::{HuggingFaceModelSpec, ModelRuntimeBackend, ModelTask};
use serde::{Deserialize, Serialize};
use video_analysis_core::Result;

/// Classification task family for still images.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImageClassificationTask {
    /// Whole-image classification.
    ImageClassification,
}

impl ImageClassificationTask {
    /// Returns all classification task variants.
    pub const ALL: &'static [Self] = &[Self::ImageClassification];

    /// Returns the stable API path segment for this task.
    pub fn path_segment(self) -> &'static str {
        match self {
            Self::ImageClassification => "classify",
        }
    }
}

/// Runtime families for image classification execution and postprocessing.
pub type ImageClassificationRuntime = ModelRuntimeBackend;

/// Classification model metadata for UI and CLI discovery.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageClassificationCatalogEntry {
    /// Stable local entry id.
    pub id: String,
    /// Upstream model id or strategy id.
    pub model_id: String,
    /// Task family.
    pub task: ImageClassificationTask,
    /// Preferred runtime.
    pub runtime: ImageClassificationRuntime,
    /// Whether the runtime is available in the default contributor build.
    pub supported: bool,
    /// Optional fallback entry id.
    pub fallback: Option<String>,
    /// Human-readable note for unsupported or fallback-only paths.
    pub note: Option<String>,
}

/// Variants describing image classification preset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ImageClassificationPreset {
    /// The ViT base patch16 224 variant.
    #[default]
    VitBasePatch16_224,
}

impl ImageClassificationPreset {
    /// Returns the model spec for this preset.
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

/// One image classification prediction.
#[derive(Debug, Clone, PartialEq)]
pub struct ImageClassification {
    /// Label assigned to this value.
    pub label: String,
    /// Score assigned to this value.
    pub score: Option<f32>,
    /// Adapter-specific attributes.
    pub attributes: BTreeMap<String, String>,
}

impl ImageClassification {
    /// Creates a new classification prediction.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            score: None,
            attributes: BTreeMap::new(),
        }
    }

    /// Sets the score.
    pub fn score(mut self, value: f32) -> Self {
        self.score = Some(value);
        self
    }

    /// Adds an adapter-specific attribute.
    pub fn attribute(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.attributes.insert(key.into(), value.into());
        self
    }
}

/// Backend trait for image classification.
pub trait ImageClassifierBackend {
    /// Classifies one image.
    fn classify_image(&mut self, image: &ImageView<'_>) -> Result<Vec<ImageClassification>>;
}

/// Returns classification catalog entries.
pub fn image_classification_catalog(
    task: Option<ImageClassificationTask>,
) -> Vec<ImageClassificationCatalogEntry> {
    let entries = vec![catalog_entry(
        "vit-base-patch16-224",
        ImageClassificationTask::ImageClassification,
        ImageClassificationPreset::VitBasePatch16_224.model_spec(),
        ModelRuntimeBackend::Onnx,
        false,
        Some("imported-classification"),
        Some("Download the model with model-runtime and run it through an image backend."),
    )];

    entries
        .into_iter()
        .filter(|entry| task.is_none_or(|task| entry.task == task))
        .collect()
}

/// Compatibility alias for callers migrating off aggregate task catalog names.
pub fn model_catalog(
    task: Option<ImageClassificationTask>,
) -> Vec<ImageClassificationCatalogEntry> {
    image_classification_catalog(task)
}

/// Parses a classification task string.
pub fn parse_task(input: &str) -> Option<ImageClassificationTask> {
    ImageClassificationTask::ALL
        .iter()
        .copied()
        .find(|task| task.path_segment() == input)
        .or(match input {
            "image_classification" | "classification" => {
                Some(ImageClassificationTask::ImageClassification)
            }
            _ => None,
        })
}

/// Returns preset ids registered for image classification.
pub fn registered_image_classification_presets() -> Vec<String> {
    image_classification_catalog(None)
        .into_iter()
        .map(|entry| entry.id)
        .collect()
}

/// Returns a schema-oriented summary.
pub fn schema_summary() -> serde_json::Value {
    serde_json::json!({
        "tasks": ImageClassificationTask::ALL
            .iter()
            .map(|task| task.path_segment())
            .collect::<Vec<_>>(),
        "models": image_classification_catalog(None),
        "registeredPresets": registered_image_classification_presets()
    })
}

fn catalog_entry(
    id: &str,
    task: ImageClassificationTask,
    spec: HuggingFaceModelSpec,
    runtime: ModelRuntimeBackend,
    supported: bool,
    fallback: Option<&str>,
    note: Option<&str>,
) -> ImageClassificationCatalogEntry {
    ImageClassificationCatalogEntry {
        id: id.to_string(),
        model_id: spec.repo_id_value().unwrap_or(id).to_string(),
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
    fn classification_catalog_reports_preset() {
        let catalog = image_classification_catalog(None);
        assert!(catalog
            .iter()
            .any(|entry| entry.id == "vit-base-patch16-224"));
        assert_eq!(
            parse_task("classify"),
            Some(ImageClassificationTask::ImageClassification)
        );
    }

    #[test]
    fn classifier_trait_returns_predictions() {
        let image = image_analysis_core::OwnedImage::new_rgb(1, 1, vec![0, 0, 0]).unwrap();
        let predictions = StubClassifier
            .classify_image(&image.as_view())
            .expect("classify");
        assert_eq!(predictions[0].label, "demo");
    }
}
