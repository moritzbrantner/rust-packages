use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use video_analysis_core::ObservationKind;

use crate::ModelPreset;

/// Documentation URL for the cuda-oxide compiler/runtime stack.
pub const CUDA_OXIDE_BOOK_URL: &str = "https://nvlabs.github.io/cuda-oxide/index.html";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
/// Variants describing model task.
pub enum ModelTask {
    /// The object detection variant.
    ObjectDetection,
    /// The pose estimation2d variant.
    PoseEstimation2d,
    /// The pose lifting3d variant.
    PoseLifting3d,
    /// The image classification variant.
    ImageClassification,
    /// The text classification variant.
    TextClassification,
    /// The token classification variant.
    TokenClassification,
    /// The zero shot classification variant.
    ZeroShotClassification,
    /// The text embedding variant.
    TextEmbedding,
    /// The summarization variant.
    Summarization,
    /// The reranking variant.
    Reranking,
    /// The question answering variant.
    QuestionAnswering,
    /// The custom variant.
    Custom(String),
}

impl ModelTask {
    /// Returns default kind.
    pub fn default_kind(&self) -> ObservationKind {
        match self {
            Self::ObjectDetection => ObservationKind::Object,
            Self::PoseEstimation2d | Self::PoseLifting3d => {
                ObservationKind::Custom("posture".to_string())
            }
            Self::ImageClassification => ObservationKind::Scene,
            Self::TextClassification | Self::TokenClassification | Self::ZeroShotClassification => {
                ObservationKind::Text
            }
            Self::TextEmbedding => ObservationKind::Custom("embedding".to_string()),
            Self::Summarization => ObservationKind::Custom("summary".to_string()),
            Self::Reranking => ObservationKind::Custom("reranking".to_string()),
            Self::QuestionAnswering => ObservationKind::Custom("question_answering".to_string()),
            Self::Custom(kind) => ObservationKind::Custom(kind.clone()),
        }
    }

    /// Returns default label.
    pub fn default_label(&self) -> &'static str {
        match self {
            Self::ObjectDetection => "object",
            Self::PoseEstimation2d => "pose_2d",
            Self::PoseLifting3d => "pose_3d",
            Self::ImageClassification => "scene",
            Self::TextClassification => "semantic",
            Self::TokenClassification => "token",
            Self::ZeroShotClassification => "zero_shot",
            Self::TextEmbedding => "embedding",
            Self::Summarization => "summary",
            Self::Reranking => "reranking",
            Self::QuestionAnswering => "question_answering",
            Self::Custom(_) => "custom",
        }
    }

    pub(crate) fn as_protocol_str(&self) -> &str {
        match self {
            Self::ObjectDetection => "object_detection",
            Self::PoseEstimation2d => "pose_estimation_2d",
            Self::PoseLifting3d => "pose_lifting_3d",
            Self::ImageClassification => "image_classification",
            Self::TextClassification => "text_classification",
            Self::TokenClassification => "token_classification",
            Self::ZeroShotClassification => "zero_shot_classification",
            Self::TextEmbedding => "text_embedding",
            Self::Summarization => "summarization",
            Self::Reranking => "reranking",
            Self::QuestionAnswering => "question_answering",
            Self::Custom(kind) => kind.as_str(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
/// Variants describing model file request.
pub enum ModelFileRequest {
    /// The required variant.
    Required(String),
    /// The optional variant.
    Optional(String),
    /// The first available variant.
    FirstAvailable(Vec<String>),
}

impl ModelFileRequest {
    /// Returns required.
    pub fn required(path: impl Into<String>) -> Self {
        Self::Required(path.into())
    }

    /// Returns optional.
    pub fn optional(path: impl Into<String>) -> Self {
        Self::Optional(path.into())
    }

    /// Returns first available.
    pub fn first_available(paths: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self::FirstAvailable(paths.into_iter().map(Into::into).collect())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
/// Data type for hugging face model spec.
pub struct HuggingFaceModelSpec {
    /// Human-readable name for this value.
    pub name: String,
    /// The repo identifier value.
    pub repo_id: String,
    /// The revision value.
    pub revision: String,
    /// The task value.
    pub task: ModelTask,
    /// The files value.
    pub files: Vec<ModelFileRequest>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
/// Variants describing model runtime backend.
pub enum ModelRuntimeBackend {
    /// The CPU variant.
    Cpu,
    /// The ONNX variant.
    Onnx,
    /// The candle variant.
    Candle,
    /// The cuda oxide variant.
    CudaOxide,
    /// The external variant.
    External,
    /// The heuristic variant.
    Heuristic,
    /// The custom variant.
    Custom(String),
}

impl ModelRuntimeBackend {
    /// Borrows this value as a str.
    pub fn as_str(&self) -> &str {
        match self {
            Self::Cpu => "cpu",
            Self::Onnx => "onnx",
            Self::Candle => "candle",
            Self::CudaOxide => "cuda_oxide",
            Self::External => "external",
            Self::Heuristic => "heuristic",
            Self::Custom(value) => value.as_str(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
/// Data type for cuda-oxide runtime config.
pub struct CudaOxideRuntimeConfig {
    /// The CUDA device index value.
    pub device_index: u32,
    /// Optional target SM architecture, such as `sm_80`.
    pub target_sm: Option<String>,
    /// Cargo subcommand used to build and run cuda-oxide kernels.
    pub cargo_oxide_command: String,
    /// The cuda-oxide documentation URL used for operator setup.
    pub documentation_url: String,
}

impl Default for CudaOxideRuntimeConfig {
    fn default() -> Self {
        Self {
            device_index: 0,
            target_sm: None,
            cargo_oxide_command: "cargo oxide".to_string(),
            documentation_url: CUDA_OXIDE_BOOK_URL.to_string(),
        }
    }
}

impl CudaOxideRuntimeConfig {
    /// Creates a new value.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns device index.
    pub fn device_index(mut self, value: u32) -> Self {
        self.device_index = value;
        self
    }

    /// Returns target SM.
    pub fn target_sm(mut self, value: impl Into<String>) -> Self {
        self.target_sm = Some(value.into());
        self
    }

    /// Returns cargo oxide command.
    pub fn cargo_oxide_command(mut self, value: impl Into<String>) -> Self {
        self.cargo_oxide_command = value.into();
        self
    }

    /// Returns runtime attributes for prediction metadata and traces.
    pub fn attributes(&self) -> BTreeMap<String, String> {
        let mut attributes = BTreeMap::new();
        attributes.insert(
            "runtime.backend".to_string(),
            ModelRuntimeBackend::CudaOxide.as_str().to_string(),
        );
        attributes.insert(
            "runtime.cuda.device_index".to_string(),
            self.device_index.to_string(),
        );
        attributes.insert(
            "runtime.cuda_oxide.command".to_string(),
            self.cargo_oxide_command.clone(),
        );
        attributes.insert(
            "runtime.cuda_oxide.docs".to_string(),
            self.documentation_url.clone(),
        );
        if let Some(target_sm) = &self.target_sm {
            attributes.insert("runtime.cuda.target_sm".to_string(), target_sm.clone());
        }
        attributes
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
/// Data type for cuda-oxide model plan.
pub struct CudaOxideModelPlan {
    /// The model spec value.
    pub spec: HuggingFaceModelSpec,
    /// The runtime value.
    pub runtime: CudaOxideRuntimeConfig,
    /// The cuda-oxide module name value.
    pub module_name: String,
    /// The kernel names value.
    pub kernel_names: Vec<String>,
}

impl CudaOxideModelPlan {
    /// Creates a new value.
    pub fn new(
        spec: HuggingFaceModelSpec,
        module_name: impl Into<String>,
        kernel_names: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        Self {
            spec,
            runtime: CudaOxideRuntimeConfig::default(),
            module_name: module_name.into(),
            kernel_names: kernel_names.into_iter().map(Into::into).collect(),
        }
    }

    /// Returns runtime.
    pub fn runtime(mut self, runtime: CudaOxideRuntimeConfig) -> Self {
        self.runtime = runtime;
        self
    }

    /// Returns attributes for prediction metadata and traces.
    pub fn attributes(&self) -> BTreeMap<String, String> {
        let mut attributes = self.runtime.attributes();
        attributes.insert(
            "runtime.cuda_oxide.module".to_string(),
            self.module_name.clone(),
        );
        if !self.kernel_names.is_empty() {
            attributes.insert(
                "runtime.cuda_oxide.kernels".to_string(),
                self.kernel_names.join(","),
            );
        }
        attributes
    }
}

impl HuggingFaceModelSpec {
    /// Creates a new value.
    pub fn new(repo_id: impl Into<String>, task: ModelTask) -> Self {
        let repo_id = repo_id.into();
        Self {
            name: repo_id.clone(),
            repo_id,
            revision: "main".to_string(),
            task,
            files: Vec::new(),
        }
    }

    /// Builds this value from preset.
    pub fn from_preset(preset: ModelPreset) -> Self {
        preset.spec()
    }

    /// Returns name.
    pub fn name(mut self, value: impl Into<String>) -> Self {
        self.name = value.into();
        self
    }

    /// Returns revision.
    pub fn revision(mut self, value: impl Into<String>) -> Self {
        self.revision = value.into();
        self
    }

    /// Returns file.
    pub fn file(mut self, path: impl Into<String>) -> Self {
        self.files.push(ModelFileRequest::required(path));
        self
    }

    /// Returns optional file.
    pub fn optional_file(mut self, path: impl Into<String>) -> Self {
        self.files.push(ModelFileRequest::optional(path));
        self
    }

    /// Returns first available file.
    pub fn first_available_file(
        mut self,
        paths: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        self.files.push(ModelFileRequest::first_available(paths));
        self
    }

    /// Builds a cuda-oxide runtime plan for this model spec.
    pub fn cuda_oxide_plan(
        self,
        module_name: impl Into<String>,
        kernel_names: impl IntoIterator<Item = impl Into<String>>,
    ) -> CudaOxideModelPlan {
        CudaOxideModelPlan::new(self, module_name, kernel_names)
    }
}
