#![doc = include_str!("../README.md")]

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Constant for supported model extensions.
pub const SUPPORTED_MODEL_EXTENSIONS: &[&str] = &[
    "ckpt",
    "pt",
    "pt2",
    "bin",
    "pth",
    "safetensors",
    "pkl",
    "sft",
];

/// Constant for config extensions.
pub const CONFIG_EXTENSIONS: &[&str] = &["yaml", "yml"];

#[derive(Debug, Error)]
/// Variants describing comfy model error.
pub enum ComfyModelError {
    #[error("unknown ComfyUI model folder key `{0}`")]
    /// The unknown model kind variant.
    UnknownModelKind(String),
    #[error("model path `{path}` is outside root `{root}`")]
    /// The path outside root variant.
    PathOutsideRoot {
        /// Filesystem path for this variant.
        path: PathBuf,
        /// Root filesystem path for this variant.
        root: PathBuf,
    },
    #[error("model inventory error: {0}")]
    /// The I/O variant.
    Io(#[from] std::io::Error),
}

/// Type alias for result.
pub type Result<T> = std::result::Result<T, ComfyModelError>;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
/// Variants describing comfy model kind.
pub enum ComfyModelKind {
    /// The checkpoint variant.
    Checkpoint,
    /// The config variant.
    Config,
    /// The lora variant.
    Lora,
    /// The vae variant.
    Vae,
    /// The text encoder variant.
    TextEncoder,
    /// The diffusion model variant.
    DiffusionModel,
    /// The clip vision variant.
    ClipVision,
    /// The style model variant.
    StyleModel,
    /// The embedding variant.
    Embedding,
    /// The diffusers variant.
    Diffusers,
    /// The vae approx variant.
    VaeApprox,
    /// The control net variant.
    ControlNet,
    /// The gligen variant.
    Gligen,
    /// The upscale model variant.
    UpscaleModel,
    /// The latent upscale model variant.
    LatentUpscaleModel,
    /// The custom nodes variant.
    CustomNodes,
    /// The hypernetwork variant.
    Hypernetwork,
    /// The photomaker variant.
    Photomaker,
    /// The classifier variant.
    Classifier,
    /// The model patch variant.
    ModelPatch,
    /// The audio encoder variant.
    AudioEncoder,
    /// The custom variant.
    Custom(String),
}

impl ComfyModelKind {
    /// Constant for core.
    pub const CORE: &'static [Self] = &[
        Self::Checkpoint,
        Self::Config,
        Self::Lora,
        Self::Vae,
        Self::TextEncoder,
        Self::DiffusionModel,
        Self::ClipVision,
        Self::StyleModel,
        Self::Embedding,
        Self::Diffusers,
        Self::VaeApprox,
        Self::ControlNet,
        Self::Gligen,
        Self::UpscaleModel,
        Self::LatentUpscaleModel,
        Self::CustomNodes,
        Self::Hypernetwork,
        Self::Photomaker,
        Self::Classifier,
        Self::ModelPatch,
        Self::AudioEncoder,
    ];

    /// Returns key.
    pub fn key(&self) -> &str {
        match self {
            Self::Checkpoint => "checkpoints",
            Self::Config => "configs",
            Self::Lora => "loras",
            Self::Vae => "vae",
            Self::TextEncoder => "text_encoders",
            Self::DiffusionModel => "diffusion_models",
            Self::ClipVision => "clip_vision",
            Self::StyleModel => "style_models",
            Self::Embedding => "embeddings",
            Self::Diffusers => "diffusers",
            Self::VaeApprox => "vae_approx",
            Self::ControlNet => "controlnet",
            Self::Gligen => "gligen",
            Self::UpscaleModel => "upscale_models",
            Self::LatentUpscaleModel => "latent_upscale_models",
            Self::CustomNodes => "custom_nodes",
            Self::Hypernetwork => "hypernetworks",
            Self::Photomaker => "photomaker",
            Self::Classifier => "classifiers",
            Self::ModelPatch => "model_patches",
            Self::AudioEncoder => "audio_encoders",
            Self::Custom(key) => key.as_str(),
        }
    }

    /// Builds this value from key.
    pub fn from_key(key: &str) -> Result<Self> {
        Ok(match key {
            "checkpoints" => Self::Checkpoint,
            "configs" => Self::Config,
            "loras" => Self::Lora,
            "vae" => Self::Vae,
            "text_encoders" | "clip" => Self::TextEncoder,
            "diffusion_models" | "unet" => Self::DiffusionModel,
            "clip_vision" => Self::ClipVision,
            "style_models" => Self::StyleModel,
            "embeddings" => Self::Embedding,
            "diffusers" => Self::Diffusers,
            "vae_approx" => Self::VaeApprox,
            "controlnet" | "t2i_adapter" => Self::ControlNet,
            "gligen" => Self::Gligen,
            "upscale_models" => Self::UpscaleModel,
            "latent_upscale_models" => Self::LatentUpscaleModel,
            "custom_nodes" => Self::CustomNodes,
            "hypernetworks" => Self::Hypernetwork,
            "photomaker" => Self::Photomaker,
            "classifiers" => Self::Classifier,
            "model_patches" => Self::ModelPatch,
            "audio_encoders" => Self::AudioEncoder,
            "" => return Err(ComfyModelError::UnknownModelKind(key.to_string())),
            value => Self::Custom(value.to_string()),
        })
    }

    /// Returns default relative paths.
    pub fn default_relative_paths(&self) -> &'static [&'static str] {
        match self {
            Self::Checkpoint => &["models/checkpoints"],
            Self::Config => &["models/configs"],
            Self::Lora => &["models/loras"],
            Self::Vae => &["models/vae"],
            Self::TextEncoder => &["models/text_encoders", "models/clip"],
            Self::DiffusionModel => &["models/unet", "models/diffusion_models"],
            Self::ClipVision => &["models/clip_vision"],
            Self::StyleModel => &["models/style_models"],
            Self::Embedding => &["models/embeddings"],
            Self::Diffusers => &["models/diffusers"],
            Self::VaeApprox => &["models/vae_approx"],
            Self::ControlNet => &["models/controlnet", "models/t2i_adapter"],
            Self::Gligen => &["models/gligen"],
            Self::UpscaleModel => &["models/upscale_models"],
            Self::LatentUpscaleModel => &["models/latent_upscale_models"],
            Self::CustomNodes => &["custom_nodes"],
            Self::Hypernetwork => &["models/hypernetworks"],
            Self::Photomaker => &["models/photomaker"],
            Self::Classifier => &["models/classifiers"],
            Self::ModelPatch => &["models/model_patches"],
            Self::AudioEncoder => &["models/audio_encoders"],
            Self::Custom(_) => &[],
        }
    }

    /// Returns accepted extensions.
    pub fn accepted_extensions(&self) -> BTreeSet<&'static str> {
        match self {
            Self::Config => CONFIG_EXTENSIONS.iter().copied().collect(),
            Self::Diffusers | Self::CustomNodes => BTreeSet::new(),
            Self::Classifier => [""].into_iter().collect(),
            Self::Custom(_) => SUPPORTED_MODEL_EXTENSIONS.iter().copied().collect(),
            _ => SUPPORTED_MODEL_EXTENSIONS.iter().copied().collect(),
        }
    }

    /// Returns accepts directories.
    pub fn accepts_directories(&self) -> bool {
        matches!(self, Self::Diffusers | Self::CustomNodes)
    }
}

impl fmt::Display for ComfyModelKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.key())
    }
}

impl FromStr for ComfyModelKind {
    type Err = ComfyModelError;

    fn from_str(value: &str) -> Result<Self> {
        Self::from_key(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
/// Data type for comfy model asset.
pub struct ComfyModelAsset {
    /// The kind value.
    pub kind: ComfyModelKind,
    /// Human-readable name for this value.
    pub name: String,
    /// The relative path value.
    pub relative_path: PathBuf,
    /// The full path value.
    pub full_path: PathBuf,
    /// The source root value.
    pub source_root: PathBuf,
    /// The bytes value.
    pub bytes: Option<u64>,
    /// The is directory value.
    pub is_directory: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
/// Variants describing comfy model role.
pub enum ComfyModelRole {
    /// The checkpoint variant.
    Checkpoint,
    /// The diffusion model variant.
    DiffusionModel,
    /// The text encoder variant.
    TextEncoder,
    /// The vae variant.
    Vae,
    /// The clip vision variant.
    ClipVision,
    /// The control net variant.
    ControlNet,
    /// The upscale model variant.
    UpscaleModel,
    /// The audio encoder variant.
    AudioEncoder,
    /// The model patch variant.
    ModelPatch,
    /// The lora variant.
    Lora,
    /// The embedding variant.
    Embedding,
}

impl ComfyModelRole {
    /// Returns kind.
    pub fn kind(self) -> ComfyModelKind {
        match self {
            Self::Checkpoint => ComfyModelKind::Checkpoint,
            Self::DiffusionModel => ComfyModelKind::DiffusionModel,
            Self::TextEncoder => ComfyModelKind::TextEncoder,
            Self::Vae => ComfyModelKind::Vae,
            Self::ClipVision => ComfyModelKind::ClipVision,
            Self::ControlNet => ComfyModelKind::ControlNet,
            Self::UpscaleModel => ComfyModelKind::UpscaleModel,
            Self::AudioEncoder => ComfyModelKind::AudioEncoder,
            Self::ModelPatch => ComfyModelKind::ModelPatch,
            Self::Lora => ComfyModelKind::Lora,
            Self::Embedding => ComfyModelKind::Embedding,
        }
    }

    /// Builds this value from kind.
    pub fn from_kind(kind: &ComfyModelKind) -> Option<Self> {
        Some(match kind {
            ComfyModelKind::Checkpoint => Self::Checkpoint,
            ComfyModelKind::DiffusionModel => Self::DiffusionModel,
            ComfyModelKind::TextEncoder => Self::TextEncoder,
            ComfyModelKind::Vae => Self::Vae,
            ComfyModelKind::ClipVision => Self::ClipVision,
            ComfyModelKind::ControlNet => Self::ControlNet,
            ComfyModelKind::UpscaleModel => Self::UpscaleModel,
            ComfyModelKind::AudioEncoder => Self::AudioEncoder,
            ComfyModelKind::ModelPatch => Self::ModelPatch,
            ComfyModelKind::Lora => Self::Lora,
            ComfyModelKind::Embedding => Self::Embedding,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
/// Data type for comfy model ref.
pub struct ComfyModelRef {
    /// The role value.
    pub role: ComfyModelRole,
    /// Human-readable name for this value.
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    /// The relative path value.
    pub relative_path: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    /// The full path value.
    pub full_path: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    /// The source root value.
    pub source_root: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    /// The bytes value.
    pub bytes: Option<u64>,
    #[serde(default)]
    /// The is directory value.
    pub is_directory: bool,
}

impl ComfyModelRef {
    /// Creates a new value.
    pub fn new(role: ComfyModelRole, name: impl Into<String>) -> Self {
        Self {
            role,
            name: name.into(),
            relative_path: None,
            full_path: None,
            source_root: None,
            bytes: None,
            is_directory: false,
        }
    }

    /// Builds this value from asset.
    pub fn from_asset(asset: &ComfyModelAsset) -> Option<Self> {
        let role = ComfyModelRole::from_kind(&asset.kind)?;
        Some(Self {
            role,
            name: asset.name.clone(),
            relative_path: Some(asset.relative_path.clone()),
            full_path: Some(asset.full_path.clone()),
            source_root: Some(asset.source_root.clone()),
            bytes: asset.bytes,
            is_directory: asset.is_directory,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Data type for comfy model root.
pub struct ComfyModelRoot {
    /// The base path value.
    pub base_path: PathBuf,
    extra_paths: BTreeMap<ComfyModelKind, Vec<PathBuf>>,
}

impl ComfyModelRoot {
    /// Creates a new value.
    pub fn new(base_path: impl Into<PathBuf>) -> Self {
        Self {
            base_path: base_path.into(),
            extra_paths: BTreeMap::new(),
        }
    }

    /// Adds add extra path to this value.
    pub fn add_extra_path(&mut self, kind: ComfyModelKind, path: impl Into<PathBuf>) -> &mut Self {
        self.extra_paths.entry(kind).or_default().push(path.into());
        self
    }

    /// Returns this value with extra path.
    pub fn with_extra_path(mut self, kind: ComfyModelKind, path: impl Into<PathBuf>) -> Self {
        self.add_extra_path(kind, path);
        self
    }

    /// Returns paths for.
    pub fn paths_for(&self, kind: &ComfyModelKind) -> Vec<PathBuf> {
        let mut paths: Vec<_> = kind
            .default_relative_paths()
            .iter()
            .map(|path| self.base_path.join(path))
            .collect();
        if let Some(extra) = self.extra_paths.get(kind) {
            paths.extend(extra.iter().cloned());
        }
        paths
    }

    /// Returns extra paths.
    pub fn extra_paths(&self) -> &BTreeMap<ComfyModelKind, Vec<PathBuf>> {
        &self.extra_paths
    }

    /// Returns scan.
    pub fn scan(&self) -> Result<Vec<ComfyModelAsset>> {
        let mut assets = Vec::new();
        for kind in ComfyModelKind::CORE {
            self.scan_kind(kind, &mut assets)?;
        }
        for kind in self.extra_paths.keys() {
            if !ComfyModelKind::CORE.contains(kind) {
                self.scan_kind(kind, &mut assets)?;
            }
        }
        assets.sort_by(|left, right| {
            left.kind
                .cmp(&right.kind)
                .then_with(|| left.name.cmp(&right.name))
        });
        Ok(assets)
    }

    /// Returns scan kind.
    pub fn scan_kind(
        &self,
        kind: &ComfyModelKind,
        assets: &mut Vec<ComfyModelAsset>,
    ) -> Result<()> {
        for root in self.paths_for(kind) {
            scan_model_path(kind, &root, &root, assets)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
/// Data type for extra model paths config.
pub struct ExtraModelPathsConfig {
    /// The sections value.
    pub sections: BTreeMap<String, ExtraModelPathSection>,
}

impl ExtraModelPathsConfig {
    /// Creates a new value.
    pub fn new() -> Self {
        Self {
            sections: BTreeMap::new(),
        }
    }

    /// Returns insert section.
    pub fn insert_section(
        mut self,
        name: impl Into<String>,
        section: ExtraModelPathSection,
    ) -> Self {
        self.sections.insert(name.into(), section);
        self
    }

    /// Converts this value to yaml string.
    pub fn to_yaml_string(&self) -> String {
        let mut output = String::new();
        for (name, section) in &self.sections {
            output.push_str(name);
            output.push_str(":\n");
            output.push_str("  base_path: ");
            output.push_str(&quoted_yaml_scalar(&path_to_config_string(
                &section.base_path,
            )));
            output.push('\n');
            if section.is_default {
                output.push_str("  is_default: true\n");
            }
            for (key, paths) in &section.paths {
                if paths.len() == 1 {
                    output.push_str("  ");
                    output.push_str(key);
                    output.push_str(": ");
                    output.push_str(&quoted_yaml_scalar(&path_to_config_string(&paths[0])));
                    output.push('\n');
                } else if !paths.is_empty() {
                    output.push_str("  ");
                    output.push_str(key);
                    output.push_str(": |\n");
                    for path in paths {
                        output.push_str("    ");
                        output.push_str(&path_to_config_string(path));
                        output.push('\n');
                    }
                }
            }
        }
        output
    }
}

impl Default for ExtraModelPathsConfig {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
/// Data type for extra model path section.
pub struct ExtraModelPathSection {
    /// The base path value.
    pub base_path: PathBuf,
    /// The is default value.
    pub is_default: bool,
    /// The paths value.
    pub paths: BTreeMap<String, Vec<PathBuf>>,
}

impl ExtraModelPathSection {
    /// Creates a new value.
    pub fn new(base_path: impl Into<PathBuf>) -> Self {
        Self {
            base_path: base_path.into(),
            is_default: false,
            paths: BTreeMap::new(),
        }
    }

    /// Returns default ComfyUI.
    pub fn default_comfyui(base_path: impl Into<PathBuf>) -> Self {
        let mut section = Self::new(base_path);
        for kind in ComfyModelKind::CORE {
            if kind == &ComfyModelKind::CustomNodes {
                continue;
            }
            let paths = kind.default_relative_paths();
            if !paths.is_empty() {
                section.paths.insert(
                    kind.key().to_string(),
                    paths.iter().map(PathBuf::from).collect(),
                );
            }
        }
        section
    }

    /// Returns default first.
    pub fn default_first(mut self, value: bool) -> Self {
        self.is_default = value;
        self
    }

    /// Adds add path to this value.
    pub fn add_path(&mut self, kind: ComfyModelKind, path: impl Into<PathBuf>) -> &mut Self {
        self.paths
            .entry(kind.key().to_string())
            .or_default()
            .push(path.into());
        self
    }
}

fn scan_model_path(
    kind: &ComfyModelKind,
    root: &Path,
    current: &Path,
    assets: &mut Vec<ComfyModelAsset>,
) -> Result<()> {
    let Ok(entries) = fs::read_dir(current) else {
        return Ok(());
    };

    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        let metadata = entry.metadata()?;
        if metadata.is_dir() {
            if kind.accepts_directories() && current == root {
                assets.push(model_asset(kind.clone(), root, &path, true, None)?);
            } else {
                scan_model_path(kind, root, &path, assets)?;
            }
        } else if metadata.is_file() && accepts_file(kind, &path) {
            assets.push(model_asset(
                kind.clone(),
                root,
                &path,
                false,
                Some(metadata.len()),
            )?);
        }
    }
    Ok(())
}

fn accepts_file(kind: &ComfyModelKind, path: &Path) -> bool {
    let extensions = kind.accepted_extensions();
    if extensions.is_empty() {
        return false;
    }
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    extensions.contains(extension.as_str())
}

fn model_asset(
    kind: ComfyModelKind,
    root: &Path,
    path: &Path,
    is_directory: bool,
    bytes: Option<u64>,
) -> Result<ComfyModelAsset> {
    let relative_path =
        relative_path(root, path).ok_or_else(|| ComfyModelError::PathOutsideRoot {
            path: path.to_path_buf(),
            root: root.to_path_buf(),
        })?;
    Ok(ComfyModelAsset {
        kind,
        name: path_to_config_string(&relative_path),
        relative_path,
        full_path: path.to_path_buf(),
        source_root: root.to_path_buf(),
        bytes,
        is_directory,
    })
}

fn relative_path(root: &Path, path: &Path) -> Option<PathBuf> {
    path.strip_prefix(root).ok().map(|relative| {
        relative
            .components()
            .filter_map(|component| match component {
                Component::Normal(value) => Some(PathBuf::from(value)),
                _ => None,
            })
            .collect()
    })
}

fn path_to_config_string(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn quoted_yaml_scalar(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

#[cfg(test)]
mod tests {
    use std::fs::{self, File};

    use super::*;

    #[test]
    fn maps_legacy_folder_keys() {
        assert_eq!(
            ComfyModelKind::from_key("clip").unwrap(),
            ComfyModelKind::TextEncoder
        );
        assert_eq!(
            ComfyModelKind::from_key("unet").unwrap(),
            ComfyModelKind::DiffusionModel
        );
    }

    #[test]
    fn scans_default_model_folders() {
        let root = tempfile::tempdir().unwrap();
        let checkpoint_dir = root.path().join("models/checkpoints/sdxl");
        let lora_dir = root.path().join("models/loras");
        fs::create_dir_all(&checkpoint_dir).unwrap();
        fs::create_dir_all(&lora_dir).unwrap();
        File::create(checkpoint_dir.join("base.safetensors")).unwrap();
        File::create(lora_dir.join("style.txt")).unwrap();
        File::create(lora_dir.join("style.sft")).unwrap();

        let assets = ComfyModelRoot::new(root.path()).scan().unwrap();
        let names: Vec<_> = assets.iter().map(|asset| asset.name.as_str()).collect();

        assert!(names.contains(&"sdxl/base.safetensors"));
        assert!(names.contains(&"style.sft"));
        assert!(!names.contains(&"style.txt"));
    }

    #[test]
    fn writes_extra_model_paths_yaml() {
        let section = ExtraModelPathSection::default_comfyui("/models").default_first(true);
        let yaml = ExtraModelPathsConfig::new()
            .insert_section("shared", section)
            .to_yaml_string();

        assert!(yaml.contains("shared:"));
        assert!(yaml.contains("base_path: '/models'"));
        assert!(yaml.contains("is_default: true"));
        assert!(yaml.contains("checkpoints: 'models/checkpoints'"));
        assert!(yaml.contains("text_encoders: |"));
    }

    #[test]
    fn model_roles_map_to_expected_folder_keys() {
        assert_eq!(
            ComfyModelRole::Checkpoint.kind(),
            ComfyModelKind::Checkpoint
        );
        assert_eq!(
            ComfyModelRole::TextEncoder.kind(),
            ComfyModelKind::TextEncoder
        );
        assert_eq!(ComfyModelRole::Vae.kind(), ComfyModelKind::Vae);
        assert_eq!(
            ComfyModelRole::ClipVision.kind(),
            ComfyModelKind::ClipVision
        );
        assert_eq!(
            ComfyModelRole::UpscaleModel.kind(),
            ComfyModelKind::UpscaleModel
        );
        assert_eq!(
            ComfyModelRole::ModelPatch.kind(),
            ComfyModelKind::ModelPatch
        );
        assert_eq!(
            ComfyModelRole::AudioEncoder.kind(),
            ComfyModelKind::AudioEncoder
        );
    }

    #[test]
    fn builds_model_refs_from_assets_when_role_is_known() {
        let asset = ComfyModelAsset {
            kind: ComfyModelKind::Vae,
            name: "vae/ae.safetensors".to_string(),
            relative_path: PathBuf::from("vae/ae.safetensors"),
            full_path: PathBuf::from("/models/vae/ae.safetensors"),
            source_root: PathBuf::from("/models"),
            bytes: Some(12),
            is_directory: false,
        };

        let model_ref = ComfyModelRef::from_asset(&asset).unwrap();
        assert_eq!(model_ref.role, ComfyModelRole::Vae);
        assert_eq!(
            model_ref.relative_path.as_deref(),
            Some(Path::new("vae/ae.safetensors"))
        );
    }
}
