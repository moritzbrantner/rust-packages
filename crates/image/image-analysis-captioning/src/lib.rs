#![doc = include_str!("../README.md")]

pub mod surface;

use std::collections::BTreeMap;

use image_analysis_core::ImageView;
use model_runtime::{HuggingFaceModelSpec, ModelRuntimeBackend, ModelTask};
use serde::{Deserialize, Serialize};
use video_analysis_core::Result;

/// Captioning task family for still images.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImageCaptioningTask {
    /// Image caption generation.
    ImageCaptioning,
}

impl ImageCaptioningTask {
    /// Returns all captioning task variants.
    pub const ALL: &'static [Self] = &[Self::ImageCaptioning];

    /// Returns the stable API path segment for this task.
    pub fn path_segment(self) -> &'static str {
        match self {
            Self::ImageCaptioning => "caption",
        }
    }
}

/// Runtime families for image captioning execution and postprocessing.
pub type ImageCaptioningRuntime = ModelRuntimeBackend;

/// Captioning model metadata for UI and CLI discovery.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageCaptioningCatalogEntry {
    /// Stable local entry id.
    pub id: String,
    /// Upstream model id or strategy id.
    pub model_id: String,
    /// Task family.
    pub task: ImageCaptioningTask,
    /// Preferred runtime.
    pub runtime: ImageCaptioningRuntime,
    /// Whether the runtime is available in the default contributor build.
    pub supported: bool,
    /// Optional fallback entry id.
    pub fallback: Option<String>,
    /// Human-readable note for unsupported or fallback-only paths.
    pub note: Option<String>,
}

/// Variants describing image caption preset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ImageCaptionPreset {
    /// The BLIP base variant.
    #[default]
    BlipBase,
}

impl ImageCaptionPreset {
    /// Returns the model spec for this preset.
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

/// One generated image caption.
#[derive(Debug, Clone, PartialEq)]
pub struct ImageCaption {
    /// Caption text.
    pub text: String,
    /// Optional score.
    pub score: Option<f32>,
    /// Adapter-specific attributes.
    pub attributes: BTreeMap<String, String>,
}

impl ImageCaption {
    /// Creates a new caption.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
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

/// Backend trait for image captioning.
pub trait ImageCaptionerBackend {
    /// Captions one image.
    fn caption_image(&mut self, image: &ImageView<'_>) -> Result<Vec<ImageCaption>>;
}

/// Returns captioning catalog entries.
pub fn image_captioning_catalog(
    task: Option<ImageCaptioningTask>,
) -> Vec<ImageCaptioningCatalogEntry> {
    let entries = vec![catalog_entry(
        "blip-image-captioning-base",
        ImageCaptioningTask::ImageCaptioning,
        ImageCaptionPreset::BlipBase.model_spec(),
        ModelRuntimeBackend::Onnx,
        false,
        Some("imported-caption"),
        Some("Download the model with model-runtime and run it through an image backend."),
    )];

    entries
        .into_iter()
        .filter(|entry| task.is_none_or(|task| entry.task == task))
        .collect()
}

/// Compatibility alias for callers migrating off aggregate task catalog names.
pub fn model_catalog(task: Option<ImageCaptioningTask>) -> Vec<ImageCaptioningCatalogEntry> {
    image_captioning_catalog(task)
}

/// Parses a captioning task string.
pub fn parse_task(input: &str) -> Option<ImageCaptioningTask> {
    ImageCaptioningTask::ALL
        .iter()
        .copied()
        .find(|task| task.path_segment() == input)
        .or(match input {
            "image_captioning" | "captioning" => Some(ImageCaptioningTask::ImageCaptioning),
            _ => None,
        })
}

/// Returns a schema-oriented summary.
pub fn schema_summary() -> serde_json::Value {
    serde_json::json!({
        "tasks": ImageCaptioningTask::ALL
            .iter()
            .map(|task| task.path_segment())
            .collect::<Vec<_>>(),
        "models": image_captioning_catalog(None)
    })
}

fn catalog_entry(
    id: &str,
    task: ImageCaptioningTask,
    spec: HuggingFaceModelSpec,
    runtime: ModelRuntimeBackend,
    supported: bool,
    fallback: Option<&str>,
    note: Option<&str>,
) -> ImageCaptioningCatalogEntry {
    ImageCaptioningCatalogEntry {
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

    #[test]
    fn captioning_catalog_reports_blip() {
        let catalog = image_captioning_catalog(None);
        assert!(catalog
            .iter()
            .any(|entry| entry.id == "blip-image-captioning-base"));
        assert_eq!(
            parse_task("caption"),
            Some(ImageCaptioningTask::ImageCaptioning)
        );
    }

    #[test]
    fn caption_can_carry_score_and_attributes() {
        let caption = ImageCaption::new("a cube")
            .score(0.8)
            .attribute("source", "fixture");
        assert_eq!(caption.score, Some(0.8));
        assert_eq!(
            caption.attributes.get("source").map(String::as_str),
            Some("fixture")
        );
    }
}
