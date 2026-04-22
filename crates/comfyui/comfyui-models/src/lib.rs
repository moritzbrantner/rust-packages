use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use thiserror::Error;

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

pub const CONFIG_EXTENSIONS: &[&str] = &["yaml", "yml"];

#[derive(Debug, Error)]
pub enum ComfyModelError {
    #[error("unknown ComfyUI model folder key `{0}`")]
    UnknownModelKind(String),
    #[error("model path `{path}` is outside root `{root}`")]
    PathOutsideRoot { path: PathBuf, root: PathBuf },
    #[error("model inventory error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, ComfyModelError>;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComfyModelKind {
    Checkpoint,
    Config,
    Lora,
    Vae,
    TextEncoder,
    DiffusionModel,
    ClipVision,
    StyleModel,
    Embedding,
    Diffusers,
    VaeApprox,
    ControlNet,
    Gligen,
    UpscaleModel,
    LatentUpscaleModel,
    CustomNodes,
    Hypernetwork,
    Photomaker,
    Classifier,
    ModelPatch,
    AudioEncoder,
    Custom(String),
}

impl ComfyModelKind {
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

    pub fn accepted_extensions(&self) -> BTreeSet<&'static str> {
        match self {
            Self::Config => CONFIG_EXTENSIONS.iter().copied().collect(),
            Self::Diffusers | Self::CustomNodes => BTreeSet::new(),
            Self::Classifier => [""].into_iter().collect(),
            Self::Custom(_) => SUPPORTED_MODEL_EXTENSIONS.iter().copied().collect(),
            _ => SUPPORTED_MODEL_EXTENSIONS.iter().copied().collect(),
        }
    }

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
pub struct ComfyModelAsset {
    pub kind: ComfyModelKind,
    pub name: String,
    pub relative_path: PathBuf,
    pub full_path: PathBuf,
    pub source_root: PathBuf,
    pub bytes: Option<u64>,
    pub is_directory: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComfyModelRoot {
    pub base_path: PathBuf,
    extra_paths: BTreeMap<ComfyModelKind, Vec<PathBuf>>,
}

impl ComfyModelRoot {
    pub fn new(base_path: impl Into<PathBuf>) -> Self {
        Self {
            base_path: base_path.into(),
            extra_paths: BTreeMap::new(),
        }
    }

    pub fn add_extra_path(&mut self, kind: ComfyModelKind, path: impl Into<PathBuf>) -> &mut Self {
        self.extra_paths.entry(kind).or_default().push(path.into());
        self
    }

    pub fn with_extra_path(mut self, kind: ComfyModelKind, path: impl Into<PathBuf>) -> Self {
        self.add_extra_path(kind, path);
        self
    }

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

    pub fn extra_paths(&self) -> &BTreeMap<ComfyModelKind, Vec<PathBuf>> {
        &self.extra_paths
    }

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
pub struct ExtraModelPathsConfig {
    pub sections: BTreeMap<String, ExtraModelPathSection>,
}

impl ExtraModelPathsConfig {
    pub fn new() -> Self {
        Self {
            sections: BTreeMap::new(),
        }
    }

    pub fn insert_section(
        mut self,
        name: impl Into<String>,
        section: ExtraModelPathSection,
    ) -> Self {
        self.sections.insert(name.into(), section);
        self
    }

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
pub struct ExtraModelPathSection {
    pub base_path: PathBuf,
    pub is_default: bool,
    pub paths: BTreeMap<String, Vec<PathBuf>>,
}

impl ExtraModelPathSection {
    pub fn new(base_path: impl Into<PathBuf>) -> Self {
        Self {
            base_path: base_path.into(),
            is_default: false,
            paths: BTreeMap::new(),
        }
    }

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

    pub fn default_first(mut self, value: bool) -> Self {
        self.is_default = value;
        self
    }

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
}
