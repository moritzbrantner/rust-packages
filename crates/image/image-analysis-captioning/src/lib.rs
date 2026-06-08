#![doc = include_str!("../README.md")]

pub mod surface;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use image_analysis_core::ImageView;
use image_analysis_processing::{
    image_model_preprocessing_from_config, preprocess_image_for_model,
    validate_image_model_preprocessing, ImageModelPreprocessing,
};
use model_runtime::{HuggingFaceModelSpec, ModelRuntimeBackend, ModelTask};
use serde::{Deserialize, Serialize};
use video_analysis_core::{DetectError, Result};

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
    /// The ViT-GPT2 ONNX captioning variant.
    #[default]
    VitGpt2,
    /// The BLIP base variant.
    BlipBase,
}

impl ImageCaptionPreset {
    /// Returns the model spec for this preset.
    pub fn model_spec(self) -> HuggingFaceModelSpec {
        match self {
            Self::VitGpt2 => HuggingFaceModelSpec::new(
                "Xenova/vit-gpt2-image-captioning",
                ModelTask::Custom("image_captioning".to_string()),
            )
            .name("vit-gpt2-image-captioning")
            .file("config.json")
            .file("generation_config.json")
            .file("preprocessor_config.json")
            .file("tokenizer.json")
            .file("tokenizer_config.json")
            .file("vocab.json")
            .file("merges.txt")
            .first_available_file([
                "onnx/encoder_model_quantized.onnx",
                "onnx/encoder_model.onnx",
            ])
            .first_available_file([
                "onnx/decoder_model_quantized.onnx",
                "onnx/decoder_model.onnx",
            ]),
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
    let entries = vec![
        catalog_entry(
            "vit-gpt2-image-captioning",
            ImageCaptioningTask::ImageCaptioning,
            ImageCaptionPreset::VitGpt2.model_spec(),
            ModelRuntimeBackend::Onnx,
            cfg!(feature = "local-onnx"),
            Some("imported-caption"),
            Some("Server-only ONNX execution uses runtime-onnx inside image-analysis-captioning; imported captions remain the WASM fallback."),
        ),
        catalog_entry(
            "blip-image-captioning-base",
            ImageCaptioningTask::ImageCaptioning,
            ImageCaptionPreset::BlipBase.model_spec(),
            ModelRuntimeBackend::Onnx,
            false,
            Some("imported-caption"),
            Some("Reference-only until BLIP generation is implemented."),
        ),
    ];

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

#[derive(Debug, Clone, PartialEq)]
/// Data type for ONNX image captioning options.
pub struct OnnxImageCaptioningOptions {
    /// The preprocessing value.
    pub preprocessing: ImageModelPreprocessing,
    /// Maximum generated token count.
    pub max_new_tokens: usize,
}

impl Default for OnnxImageCaptioningOptions {
    fn default() -> Self {
        Self {
            preprocessing: ImageModelPreprocessing {
                input_width: 224,
                input_height: 224,
                rescale_factor: 1.0 / 255.0,
                mean: [0.5, 0.5, 0.5],
                std: [0.5, 0.5, 0.5],
                channel_order: image_analysis_processing::ChannelOrder::Rgb,
            },
            max_new_tokens: 32,
        }
    }
}

#[derive(Debug)]
/// Data type for ONNX image captioner.
pub struct OnnxImageCaptioner {
    options: OnnxImageCaptioningOptions,
    encoder_path: PathBuf,
    decoder_path: PathBuf,
    tokenizer_path: PathBuf,
}

impl OnnxImageCaptioner {
    /// Builds this value from bundle.
    pub fn from_bundle(bundle: model_runtime::ModelBundle) -> Result<Self> {
        let info = validate_onnx_captioning_bundle(&bundle)?;
        Ok(Self {
            options: captioning_options_from_bundle(&bundle)?,
            encoder_path: info.encoder_path,
            decoder_path: info.decoder_path,
            tokenizer_path: info.tokenizer_path,
        })
    }

    /// Returns options.
    pub fn options(&self) -> &OnnxImageCaptioningOptions {
        &self.options
    }
}

impl ImageCaptionerBackend for OnnxImageCaptioner {
    fn caption_image(&mut self, image: &ImageView<'_>) -> Result<Vec<ImageCaption>> {
        let _input = preprocess_image_for_model(image, &self.options.preprocessing)?;
        let _ = (&self.encoder_path, &self.decoder_path, &self.tokenizer_path);
        Err(DetectError::Source(
            "native ONNX image caption decoding is not available in this build path".to_string(),
        ))
    }
}

#[derive(Debug, Clone, PartialEq)]
struct OnnxCaptioningBundleInfo {
    config_path: PathBuf,
    preprocessor_config_path: Option<PathBuf>,
    tokenizer_path: PathBuf,
    encoder_path: PathBuf,
    decoder_path: PathBuf,
}

fn validate_onnx_captioning_bundle(
    bundle: &model_runtime::ModelBundle,
) -> Result<OnnxCaptioningBundleInfo> {
    match &bundle.manifest.task {
        ModelTask::Custom(task) if task == "image_captioning" => {}
        task => {
            return Err(DetectError::InvalidArgument(format!(
                "ONNX image captioning bundle task must be image_captioning, got {:?}",
                task
            )))
        }
    }
    let config_path = required_bundle_file(bundle, "config.json")?;
    let preprocessor_config_path = bundle.file_path("preprocessor_config.json");
    let tokenizer_path = required_bundle_file(bundle, "tokenizer.json")?;
    let encoder_path = bundle
        .manifest
        .files
        .values()
        .find(|file| file.remote_path.ends_with(".onnx") && file.remote_path.contains("encoder"))
        .map(|file| bundle.root.join(&file.local_path))
        .ok_or_else(|| {
            DetectError::InvalidArgument(
                "ONNX image captioning bundle must contain an encoder `.onnx` model".to_string(),
            )
        })?;
    let decoder_path = bundle
        .manifest
        .files
        .values()
        .find(|file| file.remote_path.ends_with(".onnx") && file.remote_path.contains("decoder"))
        .map(|file| bundle.root.join(&file.local_path))
        .ok_or_else(|| {
            DetectError::InvalidArgument(
                "ONNX image captioning bundle must contain a decoder `.onnx` model".to_string(),
            )
        })?;
    Ok(OnnxCaptioningBundleInfo {
        config_path,
        preprocessor_config_path,
        tokenizer_path,
        encoder_path,
        decoder_path,
    })
}

fn captioning_options_from_bundle(
    bundle: &model_runtime::ModelBundle,
) -> Result<OnnxImageCaptioningOptions> {
    let info = validate_onnx_captioning_bundle(bundle)?;
    let preprocessing = if let Some(path) = &info.preprocessor_config_path {
        image_model_preprocessing_from_config(&read_json(path)?)?
    } else {
        OnnxImageCaptioningOptions::default().preprocessing
    };
    validate_image_model_preprocessing(&preprocessing)?;
    Ok(OnnxImageCaptioningOptions {
        preprocessing,
        max_new_tokens: 32,
    })
}

fn required_bundle_file(bundle: &model_runtime::ModelBundle, remote_path: &str) -> Result<PathBuf> {
    bundle.file_path(remote_path).ok_or_else(|| {
        DetectError::InvalidArgument(format!(
            "model bundle `{}` is missing required file `{remote_path}`",
            bundle.manifest.name
        ))
    })
}

fn read_json(path: &Path) -> Result<serde_json::Value> {
    let data = std::fs::read(path)?;
    serde_json::from_slice(&data).map_err(|err| {
        DetectError::Source(format!("failed to parse JSON `{}`: {err}", path.display()))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn captioning_catalog_reports_defaults() {
        let catalog = image_captioning_catalog(None);
        assert!(catalog
            .iter()
            .any(|entry| entry.id == "vit-gpt2-image-captioning"));
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
