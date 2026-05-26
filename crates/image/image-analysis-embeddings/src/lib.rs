#![doc = include_str!("../README.md")]

pub mod surface;

use std::collections::BTreeMap;

use image_analysis_core::ImageView;
use image_analysis_detection::FaceDetection;
use model_runtime::{HuggingFaceModelSpec, ModelRuntimeBackend, ModelTask};
use serde::{Deserialize, Serialize};
use video_analysis_core::{BoundingBox, DetectError, Result};

/// Embedding task family for still images and faces.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImageEmbeddingTask {
    /// Dense image embedding.
    ImageEmbedding,
    /// Dense face embedding.
    FaceEmbedding,
}

impl ImageEmbeddingTask {
    /// Returns all embedding task variants.
    pub const ALL: &'static [Self] = &[Self::ImageEmbedding, Self::FaceEmbedding];

    /// Returns the stable API path segment for this task.
    pub fn path_segment(self) -> &'static str {
        match self {
            Self::ImageEmbedding => "embed",
            Self::FaceEmbedding => "face-embed",
        }
    }
}

/// Runtime families for image embedding execution and postprocessing.
pub type ImageEmbeddingRuntime = ModelRuntimeBackend;

/// Image embedding model metadata for UI and CLI discovery.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageEmbeddingCatalogEntry {
    /// Stable local entry id.
    pub id: String,
    /// Upstream model id or strategy id.
    pub model_id: String,
    /// Task family.
    pub task: ImageEmbeddingTask,
    /// Preferred runtime.
    pub runtime: ImageEmbeddingRuntime,
    /// Whether the runtime is available in the default contributor build.
    pub supported: bool,
    /// Optional fallback entry id.
    pub fallback: Option<String>,
    /// Human-readable note for unsupported or fallback-only paths.
    pub note: Option<String>,
}

/// Variants describing image embedding preset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ImageEmbeddingPreset {
    /// The CLIP ViT-B/32 variant.
    #[default]
    ClipVitBasePatch32,
    /// The Xenova CLIP ViT-B/32 ONNX variant.
    XenovaClipVitBasePatch32Onnx,
}

impl ImageEmbeddingPreset {
    /// Returns the model spec for this preset.
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

/// Variants describing face embedding preset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FaceEmbeddingPreset {
    /// The OpenCV SFace ONNX variant.
    #[default]
    OpenCvSFace,
}

impl FaceEmbeddingPreset {
    /// Returns the model spec for this preset.
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

/// Dense image embedding.
#[derive(Debug, Clone, PartialEq)]
pub struct ImageEmbedding {
    /// Embedding values.
    pub vector: Vec<f32>,
    /// Adapter-specific attributes.
    pub attributes: BTreeMap<String, String>,
}

impl ImageEmbedding {
    /// Creates a new image embedding.
    pub fn new(vector: Vec<f32>) -> Result<Self> {
        validate_vector(&vector)?;
        Ok(Self {
            vector,
            attributes: BTreeMap::new(),
        })
    }

    /// Adds an adapter-specific attribute.
    pub fn attribute(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.attributes.insert(key.into(), value.into());
        self
    }
}

/// Dense face embedding.
#[derive(Debug, Clone, PartialEq)]
pub struct FaceEmbedding {
    /// Optional source face region.
    pub region: Option<BoundingBox>,
    /// Embedding values.
    pub vector: Vec<f32>,
    /// Adapter-specific attributes.
    pub attributes: BTreeMap<String, String>,
}

impl FaceEmbedding {
    /// Creates a new face embedding.
    pub fn new(vector: Vec<f32>) -> Result<Self> {
        validate_vector(&vector)?;
        Ok(Self {
            region: None,
            vector,
            attributes: BTreeMap::new(),
        })
    }

    /// Sets the source face region.
    pub fn region(mut self, value: BoundingBox) -> Self {
        self.region = Some(value);
        self
    }

    /// Adds an adapter-specific attribute.
    pub fn attribute(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.attributes.insert(key.into(), value.into());
        self
    }
}

/// Backend trait for dense image embedding.
pub trait ImageEmbedderBackend {
    /// Embeds one image.
    fn embed_image(&mut self, image: &ImageView<'_>) -> Result<ImageEmbedding>;
}

/// Backend trait for dense face embedding.
pub trait FaceEmbedderBackend {
    /// Embeds a face region from one image.
    fn embed_face(
        &mut self,
        image: &ImageView<'_>,
        detection: Option<&FaceDetection>,
    ) -> Result<FaceEmbedding>;
}

/// Returns embedding catalog entries.
pub fn image_embedding_catalog(
    task: Option<ImageEmbeddingTask>,
) -> Vec<ImageEmbeddingCatalogEntry> {
    let entries = vec![
        catalog_entry(
            "clip-vit-base-patch32",
            ImageEmbeddingTask::ImageEmbedding,
            ImageEmbeddingPreset::ClipVitBasePatch32.model_spec(),
            ModelRuntimeBackend::Onnx,
            false,
            Some("hashed-image-fallback"),
            Some("Download the model with model-runtime and run it through an image backend."),
        ),
        catalog_entry(
            "xenova-clip-vit-base-patch32-onnx",
            ImageEmbeddingTask::ImageEmbedding,
            ImageEmbeddingPreset::XenovaClipVitBasePatch32Onnx.model_spec(),
            ModelRuntimeBackend::Onnx,
            false,
            Some("hashed-image-fallback"),
            Some("ONNX model variant for browser/server embedding backends."),
        ),
        catalog_entry(
            "opencv-sface-onnx",
            ImageEmbeddingTask::FaceEmbedding,
            FaceEmbeddingPreset::OpenCvSFace.model_spec(),
            ModelRuntimeBackend::Onnx,
            false,
            Some("imported-face-embedding"),
            Some("Download the model with model-runtime and run it through an image backend."),
        ),
    ];

    entries
        .into_iter()
        .filter(|entry| task.is_none_or(|task| entry.task == task))
        .collect()
}

/// Compatibility alias for callers migrating off aggregate task catalog names.
pub fn model_catalog(task: Option<ImageEmbeddingTask>) -> Vec<ImageEmbeddingCatalogEntry> {
    image_embedding_catalog(task)
}

/// Parses an embedding task string.
pub fn parse_task(input: &str) -> Option<ImageEmbeddingTask> {
    ImageEmbeddingTask::ALL
        .iter()
        .copied()
        .find(|task| task.path_segment() == input)
        .or(match input {
            "image_embedding" | "embedding" => Some(ImageEmbeddingTask::ImageEmbedding),
            "face_embedding" => Some(ImageEmbeddingTask::FaceEmbedding),
            _ => None,
        })
}

/// Returns preset ids registered for image and face embeddings.
pub fn registered_image_embedding_presets() -> Vec<String> {
    image_embedding_catalog(None)
        .into_iter()
        .map(|entry| entry.id)
        .collect()
}

/// Returns a schema-oriented summary.
pub fn schema_summary() -> serde_json::Value {
    serde_json::json!({
        "tasks": ImageEmbeddingTask::ALL
            .iter()
            .map(|task| task.path_segment())
            .collect::<Vec<_>>(),
        "models": image_embedding_catalog(None),
        "registeredPresets": registered_image_embedding_presets()
    })
}

fn catalog_entry(
    id: &str,
    task: ImageEmbeddingTask,
    spec: HuggingFaceModelSpec,
    runtime: ModelRuntimeBackend,
    supported: bool,
    fallback: Option<&str>,
    note: Option<&str>,
) -> ImageEmbeddingCatalogEntry {
    ImageEmbeddingCatalogEntry {
        id: id.to_string(),
        model_id: spec.repo_id_value().unwrap_or(id).to_string(),
        task,
        runtime,
        supported,
        fallback: fallback.map(str::to_string),
        note: note.map(str::to_string),
    }
}

fn validate_vector(vector: &[f32]) -> Result<()> {
    if vector.is_empty() {
        return Err(DetectError::InvalidArgument(
            "embedding vector must not be empty".to_string(),
        ));
    }
    if vector.iter().any(|value| !value.is_finite()) {
        return Err(DetectError::InvalidArgument(
            "embedding vector values must be finite".to_string(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedding_catalog_reports_presets() {
        let catalog = image_embedding_catalog(None);
        assert!(catalog
            .iter()
            .any(|entry| entry.id == "xenova-clip-vit-base-patch32-onnx"));
        assert!(catalog.iter().any(|entry| entry.id == "opencv-sface-onnx"));
    }

    #[test]
    fn embedding_vectors_must_be_finite() {
        assert!(ImageEmbedding::new(vec![0.0, 1.0]).is_ok());
        assert!(ImageEmbedding::new(vec![f32::NAN]).is_err());
        assert!(FaceEmbedding::new(vec![f32::INFINITY]).is_err());
    }
}
