#![doc = include_str!("../README.md")]

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
#[cfg(feature = "onnx")]
use std::sync::atomic::{AtomicBool, Ordering};
#[cfg(feature = "onnx")]
use std::sync::{Mutex, OnceLock};
#[cfg(feature = "onnx")]
use std::time::Instant;

#[cfg(feature = "candle")]
use candle_core::{DType as CandleDType, Device as CandleDevice, Tensor as CandleTensor};
#[cfg(feature = "candle")]
use candle_nn::{Linear as CandleLinear, Module as CandleModule, VarBuilder as CandleVarBuilder};
#[cfg(feature = "candle")]
use candle_transformers::models::{bert as candle_bert, distilbert as candle_distilbert};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use text_analysis_core::AnnotationProvenance;
use text_analysis_semantics::{
    DenseVector, TextEmbeddingBackend as SemanticTextEmbeddingBackend, TextEmbeddingBackendKind,
    TextEmbeddingMetadata,
};
use video_analysis_core::{DetectError, Result, TextSegment};
use video_analysis_models::{
    CudaOxideRuntimeConfig, HuggingFaceDownloader, HuggingFaceModelSpec, ModelBundle, ModelTask,
    RawPrediction, TextModelBackend,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Variants describing truncation strategy.
pub enum TruncationStrategy {
    /// The none variant.
    None,
    /// The longest first variant.
    LongestFirst,
    /// The only first variant.
    OnlyFirst,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Variants describing pooling strategy.
pub enum PoolingStrategy {
    /// The cls variant.
    Cls,
    /// The mean variant.
    Mean,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Variants describing text runtime backend.
pub enum TextRuntimeBackend {
    /// The tokenizers variant.
    Tokenizers,
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
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
/// Data type for model cache config.
pub struct ModelCacheConfig {
    /// The cache dir value.
    pub cache_dir: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Data type for text runtime config.
pub struct TextRuntimeConfig {
    /// The backend priority value.
    pub backend_priority: Vec<TextRuntimeBackend>,
    /// The tokenizer source value.
    pub tokenizer_source: TokenizerSource,
    /// The cache value.
    pub cache: ModelCacheConfig,
}

impl Default for TextRuntimeConfig {
    fn default() -> Self {
        Self {
            backend_priority: default_backend_priority(),
            tokenizer_source: TokenizerSource::default(),
            cache: ModelCacheConfig::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Data type for text runtime catalog.
pub struct TextRuntimeCatalog {
    /// The default tokenizer value.
    pub default_tokenizer: TokenizerSource,
    /// The classifier presets value.
    pub classifier_presets: Vec<TokenizerPreset>,
    /// The embedder presets value.
    pub embedder_presets: Vec<TokenizerPreset>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Data type for cuda-oxide text runtime target.
pub struct CudaOxideTextRuntimeTarget {
    /// The runtime config value.
    pub runtime: CudaOxideRuntimeConfig,
    /// The module name value.
    pub module_name: String,
    /// The kernel names value.
    pub kernel_names: Vec<String>,
}

impl CudaOxideTextRuntimeTarget {
    /// Creates a new value.
    pub fn new(
        module_name: impl Into<String>,
        kernel_names: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        Self {
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
}

impl Default for TextRuntimeCatalog {
    fn default() -> Self {
        Self {
            default_tokenizer: TokenizerSource::default(),
            classifier_presets: vec![TokenizerPreset::DistilbertSst2],
            embedder_presets: vec![TokenizerPreset::MiniLmL6V2],
        }
    }
}

/// Trait for tokenizer backend implementations.
pub trait TokenizerBackend {
    /// Returns tokenize text.
    fn tokenize_text(&self, text: &str) -> Result<TokenizedText>;

    /// Returns runtime backend.
    fn runtime_backend(&self) -> TextRuntimeBackend {
        TextRuntimeBackend::Tokenizers
    }
}

/// Trait for sequence labeler implementations.
pub trait SequenceLabeler {
    /// Returns label text.
    fn label_text(&mut self, text: &str) -> Result<Vec<RawPrediction>>;

    /// Returns runtime backend.
    fn runtime_backend(&self) -> TextRuntimeBackend;
}

/// Trait for token classifier implementations.
pub trait TokenClassifier {
    /// Returns classify tokenized text.
    fn classify_tokenized_text(&mut self, tokens: &TokenizedText) -> Result<Vec<RawPrediction>>;

    /// Returns runtime backend.
    fn runtime_backend(&self) -> TextRuntimeBackend;
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
/// Data type for embedding model info.
pub struct EmbeddingModelInfo {
    /// The model name value.
    pub model_name: String,
    /// The dimensions value.
    pub dimensions: usize,
    /// The normalized value.
    pub normalized: bool,
    /// The max tokens value.
    pub max_tokens: Option<usize>,
}

/// Trait for embedding cache hooks implementations.
pub trait EmbeddingCacheHooks {
    /// Returns load.
    fn load(&self, text: &str) -> Option<DenseVector>;

    /// Returns store.
    fn store(&self, text: &str, vector: &DenseVector);
}

/// Trait for text embedder backend implementations.
pub trait TextEmbedderBackend: SemanticTextEmbeddingBackend {
    /// Returns embed batch.
    fn embed_batch(&self, texts: &[&str]) -> Result<Vec<DenseVector>> {
        texts.iter().map(|text| self.embed_text(text)).collect()
    }

    /// Returns model info.
    fn model_info(&self) -> EmbeddingModelInfo;

    /// Returns cache hooks.
    fn cache_hooks(&self) -> Option<&dyn EmbeddingCacheHooks> {
        None
    }
}

/// Trait for sentence embedder implementations.
pub trait SentenceEmbedder: TextEmbedderBackend {
    /// Returns runtime backend.
    fn runtime_backend(&self) -> TextRuntimeBackend;
}

/// Trait for text classifier implementations.
pub trait TextClassifier {
    /// Returns classify text.
    fn classify_text(&mut self, text: &str) -> Result<Vec<RawPrediction>>;

    /// Returns runtime backend.
    fn runtime_backend(&self) -> TextRuntimeBackend;
}

/// Returns default backend priority.
pub fn default_backend_priority() -> Vec<TextRuntimeBackend> {
    vec![
        TextRuntimeBackend::Onnx,
        TextRuntimeBackend::Candle,
        TextRuntimeBackend::Tokenizers,
        TextRuntimeBackend::Heuristic,
    ]
}

/// Returns backend priority with cuda-oxide ahead of CPU runtimes.
pub fn cuda_oxide_backend_priority() -> Vec<TextRuntimeBackend> {
    vec![
        TextRuntimeBackend::CudaOxide,
        TextRuntimeBackend::Onnx,
        TextRuntimeBackend::Candle,
        TextRuntimeBackend::Tokenizers,
        TextRuntimeBackend::Heuristic,
    ]
}

/// Returns select text runtime backend.
pub fn select_text_runtime_backend(priority: &[TextRuntimeBackend]) -> TextRuntimeBackend {
    priority
        .iter()
        .copied()
        .next()
        .unwrap_or(TextRuntimeBackend::Heuristic)
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Data type for tokenized text.
pub struct TokenizedText {
    /// The input identifiers value.
    pub input_ids: Vec<i64>,
    /// The attention mask value.
    pub attention_mask: Vec<i64>,
    /// The token type identifiers value.
    pub token_type_ids: Option<Vec<i64>>,
    /// The offsets value.
    pub offsets: Vec<Option<(usize, usize)>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
/// Variants describing tokenizer preset.
pub enum TokenizerPreset {
    /// The bert base uncased variant.
    BertBaseUncased,
    /// The distilbert sst2 variant.
    DistilbertSst2,
    #[default]
    /// The mini lm l6 v2 variant.
    MiniLmL6V2,
}

impl TokenizerPreset {
    /// Constant for all.
    pub const ALL: &'static [Self] = &[
        Self::BertBaseUncased,
        Self::DistilbertSst2,
        Self::MiniLmL6V2,
    ];

    /// Borrows this value as a str.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::BertBaseUncased => "bert-base-uncased",
            Self::DistilbertSst2 => "distilbert-sst2",
            Self::MiniLmL6V2 => "minilm-l6-v2",
        }
    }

    fn source(self) -> TokenizerSource {
        match self {
            Self::BertBaseUncased => TokenizerSource::huggingface("bert-base-uncased"),
            Self::DistilbertSst2 => {
                TokenizerSource::huggingface("distilbert-base-uncased-finetuned-sst-2-english")
            }
            Self::MiniLmL6V2 => {
                TokenizerSource::huggingface("sentence-transformers/all-MiniLM-L6-v2")
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Variants describing tokenizer source.
pub enum TokenizerSource {
    /// The local variant.
    Local(PathBuf),
    /// The preset variant.
    Preset(TokenizerPreset),
    /// The hugging face variant.
    HuggingFace {
        /// The repository identifier value for this variant.
        repo_id: String,
        /// The revision value for this variant.
        revision: String,
        /// The tokenizer file value for this variant.
        tokenizer_file: String,
    },
}

impl TokenizerSource {
    /// Returns local.
    pub fn local(path: impl Into<PathBuf>) -> Self {
        Self::Local(path.into())
    }

    /// Returns preset.
    pub fn preset(preset: TokenizerPreset) -> Self {
        Self::Preset(preset)
    }

    /// Returns huggingface.
    pub fn huggingface(repo_id: impl Into<String>) -> Self {
        Self::HuggingFace {
            repo_id: repo_id.into(),
            revision: "main".to_string(),
            tokenizer_file: "tokenizer.json".to_string(),
        }
    }

    fn resolve_path(&self, options: &TokenizerDownloadOptions) -> Result<PathBuf> {
        match self {
            Self::Local(path) => Ok(path.clone()),
            Self::Preset(preset) => preset.source().resolve_path(options),
            Self::HuggingFace {
                repo_id,
                revision,
                tokenizer_file,
            } => {
                let downloaded = options.downloader().download(
                    &HuggingFaceModelSpec::new(
                        repo_id.clone(),
                        ModelTask::Custom("tokenizer".to_string()),
                    )
                    .name(format!("{repo_id}-tokenizer"))
                    .revision(revision.clone())
                    .file(tokenizer_file.clone()),
                )?;
                downloaded
                    .files
                    .get(tokenizer_file)
                    .cloned()
                    .ok_or_else(|| {
                        DetectError::Source(format!(
                            "downloaded tokenizer `{repo_id}` did not contain `{tokenizer_file}`"
                        ))
                    })
            }
        }
    }
}

impl Default for TokenizerSource {
    fn default() -> Self {
        Self::Preset(TokenizerPreset::default())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Data type for tokenizer download options.
pub struct TokenizerDownloadOptions {
    /// The cache dir value.
    pub cache_dir: Option<PathBuf>,
    /// The token value.
    pub token: Option<String>,
    /// The progress value.
    pub progress: bool,
    /// The max retries value.
    pub max_retries: usize,
}

impl TokenizerDownloadOptions {
    /// Returns downloader.
    pub fn downloader(&self) -> HuggingFaceDownloader {
        let mut downloader = HuggingFaceDownloader::new()
            .progress(self.progress)
            .max_retries(self.max_retries);
        if let Some(cache_dir) = &self.cache_dir {
            downloader = downloader.cache_dir(cache_dir.clone());
        }
        if let Some(token) = &self.token {
            downloader = downloader.token(token.clone());
        }
        downloader
    }
}

impl Default for TokenizerDownloadOptions {
    fn default() -> Self {
        Self {
            cache_dir: None,
            token: None,
            progress: true,
            max_retries: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Data type for tokenizer bundle.
pub struct TokenizerBundle {
    tokenizer_path: PathBuf,
    /// The max length value.
    pub max_length: Option<usize>,
    /// The truncation value.
    pub truncation: TruncationStrategy,
}

impl TokenizerBundle {
    /// Creates a new value.
    pub fn new(tokenizer_path: impl Into<PathBuf>) -> Self {
        Self {
            tokenizer_path: tokenizer_path.into(),
            max_length: None,
            truncation: TruncationStrategy::None,
        }
    }

    /// Builds this value from bundle.
    pub fn from_bundle(bundle: &ModelBundle) -> Result<Self> {
        let tokenizer_path = required_bundle_file(bundle, "tokenizer.json")?;
        Ok(Self::new(tokenizer_path))
    }

    /// Builds this value from default cached.
    pub fn from_default_cached() -> Result<Self> {
        Self::from_cached_source(TokenizerSource::default())
    }

    /// Builds this value from cached source.
    pub fn from_cached_source(source: TokenizerSource) -> Result<Self> {
        Self::from_cached_source_with_options(source, &TokenizerDownloadOptions::default())
    }

    /// Builds this value from cached source with options.
    pub fn from_cached_source_with_options(
        source: TokenizerSource,
        options: &TokenizerDownloadOptions,
    ) -> Result<Self> {
        Ok(Self::new(source.resolve_path(options)?))
    }

    /// Returns max length.
    pub fn max_length(mut self, max_length: usize) -> Self {
        self.max_length = Some(max_length);
        self
    }

    /// Returns truncation.
    pub fn truncation(mut self, strategy: TruncationStrategy) -> Self {
        self.truncation = strategy;
        self
    }

    /// Returns tokenizer path.
    pub fn tokenizer_path(&self) -> &Path {
        &self.tokenizer_path
    }

    #[cfg(feature = "tokenizers")]
    /// Returns tokenize.
    pub fn tokenize(&self, text: &str) -> Result<TokenizedText> {
        let tokenizer = tokenizers::Tokenizer::from_file(&self.tokenizer_path).map_err(|err| {
            DetectError::Source(format!(
                "failed to load tokenizer `{}`: {err}",
                self.tokenizer_path.display()
            ))
        })?;
        let encoding = tokenizer
            .encode(text, true)
            .map_err(|err| DetectError::Source(format!("failed to tokenize text: {err}")))?;
        let mut tokenized = TokenizedText {
            input_ids: encoding
                .get_ids()
                .iter()
                .map(|value| i64::from(*value))
                .collect(),
            attention_mask: encoding
                .get_attention_mask()
                .iter()
                .map(|value| i64::from(*value))
                .collect(),
            token_type_ids: Some(
                encoding
                    .get_type_ids()
                    .iter()
                    .map(|value| i64::from(*value))
                    .collect(),
            ),
            offsets: encoding
                .get_offsets()
                .iter()
                .map(|(start, end)| Some((*start, *end)))
                .collect(),
        };
        if let Some(max_length) = self.max_length {
            tokenized.truncate(max_length);
        }
        Ok(tokenized)
    }

    #[cfg(not(feature = "tokenizers"))]
    /// Returns tokenize.
    pub fn tokenize(&self, _text: &str) -> Result<TokenizedText> {
        Err(invalid_argument(
            "tokenizer execution requires the `tokenizers` feature",
        ))
    }
}

impl TokenizerBackend for TokenizerBundle {
    fn tokenize_text(&self, text: &str) -> Result<TokenizedText> {
        self.tokenize(text)
    }
}

impl TokenizedText {
    /// Returns truncate.
    pub fn truncate(&mut self, max_length: usize) {
        self.input_ids.truncate(max_length);
        self.attention_mask.truncate(max_length);
        if let Some(token_type_ids) = &mut self.token_type_ids {
            token_type_ids.truncate(max_length);
        }
        self.offsets.truncate(max_length);
    }
}

#[derive(Debug, Clone, PartialEq)]
/// Data type for ONNX bundle info.
pub struct OnnxBundleInfo {
    /// The config path value.
    pub config_path: PathBuf,
    /// The tokenizer path value.
    pub tokenizer_path: PathBuf,
    /// The model path value.
    pub model_path: PathBuf,
    /// The labels value.
    pub labels: Vec<String>,
}

/// Trait for ONNX text classifier runner implementations.
pub trait OnnxTextClassifierRunner {
    /// Runs logits.
    fn run_logits(&mut self, tokens: &TokenizedText) -> Result<Vec<f32>>;
}

/// Trait for ONNX text embedding runner implementations.
pub trait OnnxTextEmbeddingRunner {
    /// Runs embeddings.
    fn run_embeddings(&self, tokens: &TokenizedText) -> Result<(Vec<f32>, Vec<usize>)>;
}

#[derive(Debug, Clone, Default)]
/// Data type for unavailable ONNX runner.
pub struct UnavailableOnnxRunner;

impl OnnxTextClassifierRunner for UnavailableOnnxRunner {
    fn run_logits(&mut self, _tokens: &TokenizedText) -> Result<Vec<f32>> {
        Err(DetectError::Source(
            "native ONNX execution is unavailable; construct with a runner or enable an executor"
                .to_string(),
        ))
    }
}

impl OnnxTextEmbeddingRunner for UnavailableOnnxRunner {
    fn run_embeddings(&self, _tokens: &TokenizedText) -> Result<(Vec<f32>, Vec<usize>)> {
        Err(DetectError::Source(
            "native ONNX execution is unavailable; construct with a runner or enable an executor"
                .to_string(),
        ))
    }
}

#[cfg(feature = "onnx")]
#[derive(Debug)]
/// Data type for native ONNX runner.
pub struct NativeOnnxRunner {
    session: Mutex<ort::session::Session>,
    model_path: PathBuf,
    first_run_observed: AtomicBool,
}

#[cfg(feature = "onnx")]
impl NativeOnnxRunner {
    /// Creates a new value.
    pub fn new(model_path: impl AsRef<Path>) -> Result<Self> {
        let model_path = model_path.as_ref().to_path_buf();
        let timing_enabled = onnx_timing_enabled();
        if timing_enabled {
            log_onnx_stage_event("NativeOnnxRunner::new", &model_path, "start");
        }
        let started = timing_enabled.then(Instant::now);

        if timing_enabled {
            log_onnx_stage_event(
                "NativeOnnxRunner::new.Session::builder",
                &model_path,
                "start",
            );
        }
        let builder_started = timing_enabled.then(Instant::now);
        let builder = ort::session::Session::builder();
        if let Some(builder_started) = builder_started {
            log_onnx_stage_timing(
                "NativeOnnxRunner::new.Session::builder",
                &model_path,
                builder_started.elapsed(),
                builder.is_ok(),
            );
        }
        let builder = builder.map_err(ort_error)?;

        if timing_enabled {
            log_onnx_stage_event(
                "NativeOnnxRunner::new.configure_builder",
                &model_path,
                "start",
            );
        }
        let configure_started = timing_enabled.then(Instant::now);
        let builder = configure_native_onnx_session_builder(builder);
        if let Some(configure_started) = configure_started {
            log_onnx_stage_timing(
                "NativeOnnxRunner::new.configure_builder",
                &model_path,
                configure_started.elapsed(),
                builder.is_ok(),
            );
        }
        let mut builder = builder.map_err(ort_error)?;

        if timing_enabled {
            log_onnx_stage_event(
                "NativeOnnxRunner::new.commit_from_file",
                &model_path,
                "start",
            );
        }
        let commit_started = timing_enabled.then(Instant::now);
        let session = builder.commit_from_file(&model_path);
        if let Some(commit_started) = commit_started {
            log_onnx_stage_timing(
                "NativeOnnxRunner::new.commit_from_file",
                &model_path,
                commit_started.elapsed(),
                session.is_ok(),
            );
        }

        if let Some(started) = started {
            log_onnx_stage_timing(
                "NativeOnnxRunner::new",
                &model_path,
                started.elapsed(),
                session.is_ok(),
            );
        }
        let session = session.map_err(ort_error)?;
        if timing_enabled {
            log_onnx_stage_event("NativeOnnxRunner::new", &model_path, "done");
        }
        Ok(Self {
            session: Mutex::new(session),
            model_path,
            first_run_observed: AtomicBool::new(false),
        })
    }

    fn run_first_f32_output(&self, tokens: &TokenizedText) -> Result<(Vec<f32>, Vec<usize>)> {
        use std::borrow::Cow;

        use ort::session::SessionInputValue;
        use ort::value::Tensor;

        let mut session = self
            .session
            .lock()
            .map_err(|_| DetectError::Source("ONNX session mutex was poisoned".to_string()))?;
        let input_names = session
            .inputs()
            .iter()
            .map(|input| input.name().to_string())
            .collect::<Vec<_>>();
        let shape = vec![1_i64, tokens.input_ids.len() as i64];
        let mut inputs = Vec::<(Cow<'_, str>, SessionInputValue<'_>)>::new();
        if input_names.iter().any(|name| name == "input_ids") {
            inputs.push((
                Cow::from("input_ids"),
                Tensor::<i64>::from_array((shape.clone(), tokens.input_ids.clone()))
                    .map_err(ort_error)?
                    .into(),
            ));
        }
        if input_names.iter().any(|name| name == "attention_mask") {
            inputs.push((
                Cow::from("attention_mask"),
                Tensor::<i64>::from_array((shape.clone(), tokens.attention_mask.clone()))
                    .map_err(ort_error)?
                    .into(),
            ));
        }
        if input_names.iter().any(|name| name == "token_type_ids") {
            if let Some(token_type_ids) = &tokens.token_type_ids {
                inputs.push((
                    Cow::from("token_type_ids"),
                    Tensor::<i64>::from_array((shape, token_type_ids.clone()))
                        .map_err(ort_error)?
                        .into(),
                ));
            }
        }
        if inputs.is_empty() {
            return Err(invalid_argument(
                "ONNX text model does not expose a supported text input",
            ));
        }
        let log_first_run =
            onnx_timing_enabled() && !self.first_run_observed.swap(true, Ordering::Relaxed);
        if log_first_run {
            log_onnx_stage_event("NativeOnnxRunner::session.run", &self.model_path, "start");
        }
        let started = log_first_run.then(Instant::now);
        let outputs = session.run(inputs);
        if let Some(started) = started {
            log_onnx_stage_timing(
                "NativeOnnxRunner::session.run",
                &self.model_path,
                started.elapsed(),
                outputs.is_ok(),
            );
        }
        let outputs = outputs.map_err(ort_error)?;
        let (shape, values) = outputs[0].try_extract_tensor::<f32>().map_err(ort_error)?;
        let shape = shape
            .iter()
            .map(|dim| {
                usize::try_from(*dim).map_err(|_| {
                    invalid_argument("ONNX output shape contains a negative dimension")
                })
            })
            .collect::<Result<Vec<_>>>()?;
        Ok((values.to_vec(), shape))
    }
}

#[cfg(feature = "onnx")]
fn configure_native_onnx_session_builder(
    builder: ort::session::builder::SessionBuilder,
) -> ort::session::builder::BuilderResult {
    builder
        .with_no_environment_execution_providers()?
        .with_execution_providers([ort::ep::CPUExecutionProvider::default().build()])?
        .with_optimization_level(ort::session::builder::GraphOptimizationLevel::Disable)?
        .with_parallel_execution(false)?
        .with_intra_threads(1)?
        .with_inter_threads(1)
}

#[cfg(feature = "onnx")]
impl OnnxTextClassifierRunner for NativeOnnxRunner {
    fn run_logits(&mut self, tokens: &TokenizedText) -> Result<Vec<f32>> {
        let (values, shape) = self.run_first_f32_output(tokens)?;
        match shape.as_slice() {
            [_, labels] if values.len() >= *labels => {
                Ok(values[values.len().saturating_sub(*labels)..].to_vec())
            }
            [_labels] => Ok(values),
            _ => Ok(values),
        }
    }
}

#[cfg(feature = "onnx")]
impl OnnxTextEmbeddingRunner for NativeOnnxRunner {
    fn run_embeddings(&self, tokens: &TokenizedText) -> Result<(Vec<f32>, Vec<usize>)> {
        self.run_first_f32_output(tokens)
    }
}

#[derive(Debug, Clone)]
/// Data type for ONNX text classifier.
pub struct OnnxTextClassifier<R = UnavailableOnnxRunner> {
    tokenizer: TokenizerBundle,
    labels: Vec<String>,
    runner: R,
}

#[cfg(not(feature = "onnx"))]
impl OnnxTextClassifier<UnavailableOnnxRunner> {
    /// Builds this value from bundle.
    pub fn from_bundle(bundle: ModelBundle) -> Result<Self> {
        let info = validate_onnx_bundle(&bundle)?;
        Ok(Self {
            tokenizer: TokenizerBundle::new(info.tokenizer_path),
            labels: info.labels,
            runner: UnavailableOnnxRunner,
        })
    }
}

#[cfg(feature = "onnx")]
impl OnnxTextClassifier<NativeOnnxRunner> {
    /// Builds this value from bundle.
    pub fn from_bundle(bundle: ModelBundle) -> Result<Self> {
        let info = validate_onnx_bundle(&bundle)?;
        Ok(Self {
            tokenizer: TokenizerBundle::new(info.tokenizer_path),
            labels: info.labels,
            runner: NativeOnnxRunner::new(info.model_path)?,
        })
    }
}

impl<R: OnnxTextClassifierRunner> OnnxTextClassifier<R> {
    /// Builds this value from runner.
    pub fn from_runner(bundle: ModelBundle, runner: R) -> Result<Self> {
        let info = validate_onnx_bundle(&bundle)?;
        Ok(Self {
            tokenizer: TokenizerBundle::new(info.tokenizer_path),
            labels: info.labels,
            runner,
        })
    }

    /// Returns tokenizer.
    pub fn tokenizer(&self) -> &TokenizerBundle {
        &self.tokenizer
    }

    /// Returns labels.
    pub fn labels(&self) -> &[String] {
        &self.labels
    }

    /// Returns classify.
    pub fn classify(&mut self, text: &str) -> Result<Vec<RawPrediction>> {
        let tokens = self.tokenizer.tokenize(text)?;
        self.classify_tokenized(&tokens)
    }

    /// Returns classify tokenized.
    pub fn classify_tokenized(&mut self, tokens: &TokenizedText) -> Result<Vec<RawPrediction>> {
        let logits = self.runner.run_logits(tokens)?;
        let probabilities = softmax(&logits);
        Ok(probabilities
            .into_iter()
            .enumerate()
            .map(|(index, score)| {
                RawPrediction::label(
                    self.labels
                        .get(index)
                        .cloned()
                        .unwrap_or_else(|| format!("LABEL_{index}")),
                    score,
                )
            })
            .collect())
    }
}

impl<R: OnnxTextClassifierRunner> TextModelBackend for OnnxTextClassifier<R> {
    fn task(&self) -> ModelTask {
        ModelTask::TextClassification
    }

    fn predict_text(&mut self, segment: &TextSegment<'_>) -> Result<Vec<RawPrediction>> {
        self.classify(segment.text)
    }
}

impl<R: OnnxTextClassifierRunner> SequenceLabeler for OnnxTextClassifier<R> {
    fn label_text(&mut self, text: &str) -> Result<Vec<RawPrediction>> {
        self.classify(text)
    }

    fn runtime_backend(&self) -> TextRuntimeBackend {
        TextRuntimeBackend::Onnx
    }
}

impl<R: OnnxTextClassifierRunner> TokenClassifier for OnnxTextClassifier<R> {
    fn classify_tokenized_text(&mut self, tokens: &TokenizedText) -> Result<Vec<RawPrediction>> {
        self.classify_tokenized(tokens)
    }

    fn runtime_backend(&self) -> TextRuntimeBackend {
        TextRuntimeBackend::Onnx
    }
}

impl<R: OnnxTextClassifierRunner> TextClassifier for OnnxTextClassifier<R> {
    fn classify_text(&mut self, text: &str) -> Result<Vec<RawPrediction>> {
        self.classify(text)
    }

    fn runtime_backend(&self) -> TextRuntimeBackend {
        TextRuntimeBackend::Onnx
    }
}

#[derive(Debug, Clone)]
/// Data type for ONNX text embedder.
pub struct OnnxTextEmbedder<R = UnavailableOnnxRunner> {
    tokenizer: TokenizerBundle,
    runner: R,
    pooling: PoolingStrategy,
    normalize: bool,
    model_name: String,
    dimensions: Option<usize>,
    max_tokens: Option<usize>,
}

#[cfg(not(feature = "onnx"))]
impl OnnxTextEmbedder<UnavailableOnnxRunner> {
    /// Builds this value from bundle.
    pub fn from_bundle(bundle: ModelBundle) -> Result<Self> {
        let info = validate_onnx_bundle(&bundle)?;
        Ok(Self {
            tokenizer: TokenizerBundle::new(info.tokenizer_path),
            runner: UnavailableOnnxRunner,
            pooling: PoolingStrategy::Mean,
            normalize: true,
            model_name: bundle.manifest.name,
            dimensions: embedding_dimensions_from_config_path(&info.config_path)?,
            max_tokens: model_max_tokens_from_config_path(&info.config_path)?,
        })
    }
}

#[cfg(feature = "onnx")]
impl OnnxTextEmbedder<NativeOnnxRunner> {
    /// Builds this value from bundle.
    pub fn from_bundle(bundle: ModelBundle) -> Result<Self> {
        let info = validate_onnx_bundle(&bundle)?;
        Ok(Self {
            tokenizer: TokenizerBundle::new(info.tokenizer_path),
            runner: NativeOnnxRunner::new(info.model_path)?,
            pooling: PoolingStrategy::Mean,
            normalize: true,
            model_name: bundle.manifest.name,
            dimensions: embedding_dimensions_from_config_path(&info.config_path)?,
            max_tokens: model_max_tokens_from_config_path(&info.config_path)?,
        })
    }
}

impl<R: OnnxTextEmbeddingRunner> OnnxTextEmbedder<R> {
    /// Builds this value from runner.
    pub fn from_runner(bundle: ModelBundle, runner: R) -> Result<Self> {
        let info = validate_onnx_bundle(&bundle)?;
        Ok(Self {
            tokenizer: TokenizerBundle::new(info.tokenizer_path),
            runner,
            pooling: PoolingStrategy::Mean,
            normalize: true,
            model_name: bundle.manifest.name,
            dimensions: embedding_dimensions_from_config_path(&info.config_path)?,
            max_tokens: model_max_tokens_from_config_path(&info.config_path)?,
        })
    }

    /// Returns pooling.
    pub fn pooling(mut self, pooling: PoolingStrategy) -> Self {
        self.pooling = pooling;
        self
    }

    /// Normalizes this value.
    pub fn normalize(mut self, normalize: bool) -> Self {
        self.normalize = normalize;
        self
    }

    /// Returns embed tokenized.
    pub fn embed_tokenized(&self, tokens: &TokenizedText) -> Result<DenseVector> {
        let (values, shape) = self.runner.run_embeddings(tokens)?;
        pool_embedding_output(
            &values,
            &shape,
            &tokens.attention_mask,
            self.pooling,
            self.normalize,
        )
    }
}

impl<R: OnnxTextEmbeddingRunner> SemanticTextEmbeddingBackend for OnnxTextEmbedder<R> {
    fn embed_text(&self, text: &str) -> Result<DenseVector> {
        let tokens = self.tokenizer.tokenize(text)?;
        self.embed_tokenized(&tokens)
    }

    fn metadata(&self) -> TextEmbeddingMetadata {
        TextEmbeddingMetadata {
            backend: TextEmbeddingBackendKind::Onnx,
            provenance: AnnotationProvenance::Onnx,
            model_name: Some(self.model_name.clone()),
            dimensions: self.dimensions,
        }
    }
}

impl<R: OnnxTextEmbeddingRunner> TextEmbedderBackend for OnnxTextEmbedder<R> {
    fn model_info(&self) -> EmbeddingModelInfo {
        EmbeddingModelInfo {
            model_name: self.model_name.clone(),
            dimensions: self.dimensions.unwrap_or(0),
            normalized: self.normalize,
            max_tokens: self.max_tokens.or(self.tokenizer.max_length),
        }
    }
}

impl<R: OnnxTextEmbeddingRunner> SentenceEmbedder for OnnxTextEmbedder<R> {
    fn runtime_backend(&self) -> TextRuntimeBackend {
        TextRuntimeBackend::Onnx
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CandleClassifierArchitecture {
    Bert,
    DistilBert,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CandleEmbeddingArchitecture {
    Bert,
    DistilBert,
}

#[derive(Debug, Clone)]
/// Data type for candle text classifier.
pub struct CandleTextClassifier {
    tokenizer: TokenizerBundle,
    labels: Vec<String>,
    config: Value,
    model_paths: Vec<PathBuf>,
    architecture: CandleClassifierArchitecture,
}

impl CandleTextClassifier {
    /// Builds this value from bundle.
    pub fn from_bundle(bundle: ModelBundle) -> Result<Self> {
        let config_path = required_bundle_file(&bundle, "config.json")?;
        let tokenizer_path = required_bundle_file(&bundle, "tokenizer.json")?;
        let config = read_json(&config_path)?;
        let model_paths = bundle_files_with_extension(&bundle, "safetensors");
        if model_paths.is_empty() {
            return Err(invalid_argument(
                "Candle text bundles must contain a `.safetensors` model file",
            ));
        }
        let architecture = classifier_architecture_from_config(&config)?;
        Ok(Self {
            tokenizer: TokenizerBundle::new(tokenizer_path),
            labels: labels_from_config(&config),
            config,
            model_paths,
            architecture,
        })
    }

    /// Returns labels.
    pub fn labels(&self) -> &[String] {
        &self.labels
    }

    /// Returns classify.
    pub fn classify(&mut self, text: &str) -> Result<Vec<RawPrediction>> {
        let tokens = self.tokenizer.tokenize(text)?;
        self.classify_tokenized(&tokens)
    }

    /// Returns classify tokenized.
    pub fn classify_tokenized(&self, tokens: &TokenizedText) -> Result<Vec<RawPrediction>> {
        #[cfg(feature = "candle")]
        {
            let logits =
                run_candle_classifier(&self.config, &self.model_paths, self.architecture, tokens)?;
            let probabilities = softmax(&logits);
            Ok(probabilities
                .into_iter()
                .enumerate()
                .map(|(index, score)| {
                    RawPrediction::label(
                        self.labels
                            .get(index)
                            .cloned()
                            .unwrap_or_else(|| format!("LABEL_{index}")),
                        score,
                    )
                })
                .collect())
        }
        #[cfg(not(feature = "candle"))]
        {
            let _ = (&self.config, &self.model_paths, self.architecture, tokens);
            Err(invalid_argument(
                "native Candle execution requires the `candle` feature",
            ))
        }
    }
}

impl TextModelBackend for CandleTextClassifier {
    fn task(&self) -> ModelTask {
        ModelTask::TextClassification
    }

    fn predict_text(&mut self, segment: &TextSegment<'_>) -> Result<Vec<RawPrediction>> {
        self.classify(segment.text)
    }
}

impl SequenceLabeler for CandleTextClassifier {
    fn label_text(&mut self, text: &str) -> Result<Vec<RawPrediction>> {
        self.classify(text)
    }

    fn runtime_backend(&self) -> TextRuntimeBackend {
        TextRuntimeBackend::Candle
    }
}

impl TokenClassifier for CandleTextClassifier {
    fn classify_tokenized_text(&mut self, tokens: &TokenizedText) -> Result<Vec<RawPrediction>> {
        self.classify_tokenized(tokens)
    }

    fn runtime_backend(&self) -> TextRuntimeBackend {
        TextRuntimeBackend::Candle
    }
}

impl TextClassifier for CandleTextClassifier {
    fn classify_text(&mut self, text: &str) -> Result<Vec<RawPrediction>> {
        self.classify(text)
    }

    fn runtime_backend(&self) -> TextRuntimeBackend {
        TextRuntimeBackend::Candle
    }
}

#[derive(Debug, Clone)]
/// Data type for candle text embedder.
pub struct CandleTextEmbedder {
    tokenizer: TokenizerBundle,
    config: Value,
    model_paths: Vec<PathBuf>,
    architecture: CandleEmbeddingArchitecture,
    pooling: PoolingStrategy,
    normalize: bool,
    model_name: String,
    dimensions: Option<usize>,
    max_tokens: Option<usize>,
}

impl CandleTextEmbedder {
    /// Builds this value from bundle.
    pub fn from_bundle(bundle: ModelBundle) -> Result<Self> {
        let config_path = required_bundle_file(&bundle, "config.json")?;
        let tokenizer_path = required_bundle_file(&bundle, "tokenizer.json")?;
        let config = read_json(&config_path)?;
        let model_paths = bundle_files_with_extension(&bundle, "safetensors");
        if model_paths.is_empty() {
            return Err(invalid_argument(
                "Candle text bundles must contain a `.safetensors` model file",
            ));
        }
        let architecture = embedding_architecture_from_config(&config)?;
        let dimensions = embedding_dimensions_from_config(&config);
        let max_tokens = model_max_tokens_from_config(&config);
        Ok(Self {
            tokenizer: TokenizerBundle::new(tokenizer_path),
            config,
            model_paths,
            architecture,
            pooling: PoolingStrategy::Mean,
            normalize: true,
            model_name: bundle.manifest.name,
            dimensions,
            max_tokens,
        })
    }

    /// Returns pooling.
    pub fn pooling(mut self, pooling: PoolingStrategy) -> Self {
        self.pooling = pooling;
        self
    }

    /// Normalizes this value.
    pub fn normalize(mut self, normalize: bool) -> Self {
        self.normalize = normalize;
        self
    }
}

impl SemanticTextEmbeddingBackend for CandleTextEmbedder {
    fn embed_text(&self, text: &str) -> Result<DenseVector> {
        let tokens = self.tokenizer.tokenize(text)?;
        self.embed_tokenized(&tokens)
    }

    fn metadata(&self) -> TextEmbeddingMetadata {
        TextEmbeddingMetadata {
            backend: TextEmbeddingBackendKind::Candle,
            provenance: AnnotationProvenance::Candle,
            model_name: Some(self.model_name.clone()),
            dimensions: self.dimensions,
        }
    }
}

impl TextEmbedderBackend for CandleTextEmbedder {
    fn model_info(&self) -> EmbeddingModelInfo {
        EmbeddingModelInfo {
            model_name: self.model_name.clone(),
            dimensions: self.dimensions.unwrap_or(0),
            normalized: self.normalize,
            max_tokens: self.max_tokens.or(self.tokenizer.max_length),
        }
    }
}

impl CandleTextEmbedder {
    /// Returns embed tokenized.
    pub fn embed_tokenized(&self, tokens: &TokenizedText) -> Result<DenseVector> {
        #[cfg(feature = "candle")]
        {
            let (values, shape) =
                run_candle_embedder(&self.config, &self.model_paths, self.architecture, tokens)?;
            pool_embedding_output(
                &values,
                &shape,
                &tokens.attention_mask,
                self.pooling,
                self.normalize,
            )
        }
        #[cfg(not(feature = "candle"))]
        {
            let _ = (&self.config, &self.model_paths, self.architecture, tokens);
            Err(invalid_argument(
                "native Candle execution requires the `candle` feature",
            ))
        }
    }
}

impl SentenceEmbedder for CandleTextEmbedder {
    fn runtime_backend(&self) -> TextRuntimeBackend {
        TextRuntimeBackend::Candle
    }
}

impl TextEmbedderBackend for text_analysis_semantics::HashedTextEmbedder {
    fn model_info(&self) -> EmbeddingModelInfo {
        EmbeddingModelInfo {
            model_name: "hashed-text-embedder".to_string(),
            dimensions: self.config.dimensions,
            normalized: true,
            max_tokens: None,
        }
    }
}

/// Validates ONNX bundle.
pub fn validate_onnx_bundle(bundle: &ModelBundle) -> Result<OnnxBundleInfo> {
    let config_path = required_bundle_file(bundle, "config.json")?;
    let tokenizer_path = required_bundle_file(bundle, "tokenizer.json")?;
    let model_path = first_bundle_file_with_extension(bundle, "onnx").ok_or_else(|| {
        invalid_argument("ONNX text bundle must contain at least one `.onnx` model file")
    })?;
    let config = read_json(&config_path)?;
    Ok(OnnxBundleInfo {
        config_path,
        tokenizer_path,
        model_path,
        labels: labels_from_config(&config),
    })
}

/// Returns pool embedding output.
pub fn pool_embedding_output(
    values: &[f32],
    shape: &[usize],
    attention_mask: &[i64],
    pooling: PoolingStrategy,
    normalize: bool,
) -> Result<DenseVector> {
    let (sequence, hidden) = match shape {
        [_hidden] => {
            let vector = DenseVector::new(values.to_vec())?;
            return if normalize {
                vector.l2_normalized()
            } else {
                Ok(vector)
            };
        }
        [sequence, hidden] => (*sequence, *hidden),
        [batch, sequence, hidden] if *batch == 1 => (*sequence, *hidden),
        _ => {
            return Err(invalid_argument(format!(
                "unsupported embedding output shape `{shape:?}`"
            )));
        }
    };
    if sequence == 0 || hidden == 0 || values.len() != sequence * hidden {
        return Err(invalid_argument(
            "embedding output shape does not match values",
        ));
    }

    let pooled = match pooling {
        PoolingStrategy::Cls => values[..hidden].to_vec(),
        PoolingStrategy::Mean => {
            let mut pooled = vec![0.0_f32; hidden];
            let mut count = 0.0_f32;
            for token_index in 0..sequence {
                if attention_mask.get(token_index).copied().unwrap_or(1) == 0 {
                    continue;
                }
                count += 1.0;
                let offset = token_index * hidden;
                for dimension in 0..hidden {
                    pooled[dimension] += values[offset + dimension];
                }
            }
            if count <= f32::EPSILON {
                return Err(invalid_argument(
                    "mean pooling requires at least one unmasked token",
                ));
            }
            for value in &mut pooled {
                *value /= count;
            }
            pooled
        }
    };
    let vector = DenseVector::new(pooled)?;
    if normalize {
        vector.l2_normalized()
    } else {
        Ok(vector)
    }
}

/// Returns softmax.
pub fn softmax(logits: &[f32]) -> Vec<f32> {
    if logits.is_empty() {
        return Vec::new();
    }
    let max = logits.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    let mut values = logits
        .iter()
        .map(|value| (value - max).exp())
        .collect::<Vec<_>>();
    let total = values.iter().sum::<f32>();
    if total <= f32::EPSILON || !total.is_finite() {
        return vec![0.0; logits.len()];
    }
    for value in &mut values {
        *value /= total;
    }
    values
}

fn classifier_architecture_from_config(config: &Value) -> Result<CandleClassifierArchitecture> {
    let architectures = architectures_from_config(config);
    if architectures.contains(&"DistilBertForSequenceClassification") {
        return Ok(CandleClassifierArchitecture::DistilBert);
    }
    if architectures.contains(&"BertForSequenceClassification") {
        return Ok(CandleClassifierArchitecture::Bert);
    }
    Err(invalid_argument(format!(
        "unsupported Candle text architecture {}; supported: DistilBertForSequenceClassification, BertForSequenceClassification",
        if architectures.is_empty() {
            "<missing>".to_string()
        } else {
            architectures.join(", ")
        },
    )))
}

fn embedding_architecture_from_config(config: &Value) -> Result<CandleEmbeddingArchitecture> {
    let architectures = architectures_from_config(config);
    if architectures.contains(&"DistilBertModel") {
        return Ok(CandleEmbeddingArchitecture::DistilBert);
    }
    if architectures
        .iter()
        .any(|architecture| matches!(*architecture, "BertModel" | "SentenceTransformer"))
    {
        return Ok(CandleEmbeddingArchitecture::Bert);
    }
    Err(invalid_argument(format!(
        "unsupported Candle text architecture {}; supported: BertModel, DistilBertModel, SentenceTransformer",
        if architectures.is_empty() {
            "<missing>".to_string()
        } else {
            architectures.join(", ")
        },
    )))
}

fn architectures_from_config(config: &Value) -> Vec<&str> {
    config
        .get("architectures")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect::<Vec<_>>()
}

fn embedding_dimensions_from_config_path(config_path: &Path) -> Result<Option<usize>> {
    Ok(embedding_dimensions_from_config(&read_json(config_path)?))
}

fn embedding_dimensions_from_config(config: &Value) -> Option<usize> {
    config
        .get("hidden_size")
        .or_else(|| config.get("dim"))
        .and_then(Value::as_u64)
        .and_then(|value| usize::try_from(value).ok())
}

fn model_max_tokens_from_config_path(config_path: &Path) -> Result<Option<usize>> {
    Ok(model_max_tokens_from_config(&read_json(config_path)?))
}

fn model_max_tokens_from_config(config: &Value) -> Option<usize> {
    config
        .get("max_position_embeddings")
        .or_else(|| config.get("max_seq_len"))
        .and_then(Value::as_u64)
        .and_then(|value| usize::try_from(value).ok())
}

fn required_bundle_file(bundle: &ModelBundle, remote_path: &str) -> Result<PathBuf> {
    bundle.file_path(remote_path).ok_or_else(|| {
        invalid_argument(format!(
            "model bundle `{}` is missing required file `{remote_path}`",
            bundle.manifest.name
        ))
    })
}

fn first_bundle_file_with_extension(bundle: &ModelBundle, extension: &str) -> Option<PathBuf> {
    bundle
        .manifest
        .files
        .keys()
        .find(|path| {
            Path::new(path).extension().and_then(|value| value.to_str()) == Some(extension)
        })
        .and_then(|path| bundle.file_path(path))
}

fn bundle_files_with_extension(bundle: &ModelBundle, extension: &str) -> Vec<PathBuf> {
    bundle
        .manifest
        .files
        .keys()
        .filter(|path| {
            Path::new(path).extension().and_then(|value| value.to_str()) == Some(extension)
        })
        .filter_map(|path| bundle.file_path(path))
        .collect::<Vec<_>>()
}

#[cfg(feature = "candle")]
fn run_candle_classifier(
    config: &Value,
    model_paths: &[PathBuf],
    architecture: CandleClassifierArchitecture,
    tokens: &TokenizedText,
) -> Result<Vec<f32>> {
    let device = CandleDevice::Cpu;
    let vb = candle_var_builder(model_paths, &device)?;
    let prefixes = model_prefix_candidates(config);

    match architecture {
        CandleClassifierArchitecture::Bert => {
            let config: candle_bert::Config =
                serde_json::from_value(config.clone()).map_err(|err| {
                    invalid_argument(format!("failed to parse BERT config for Candle: {err}"))
                })?;
            let (model, used_prefix) = load_candle_bert_model(&vb, &config, &prefixes)?;
            let input_ids = candle_input_ids(tokens, &device)?;
            let token_type_ids = candle_token_type_ids(tokens, &device)?;
            let attention_mask = candle_attention_mask_keep(tokens, &device)?;
            let sequence_output = model
                .forward(&input_ids, &token_type_ids, Some(&attention_mask))
                .map_err(candle_error)?;
            let mut pooled = sequence_output
                .narrow(1, 0, 1)
                .map_err(candle_error)?
                .squeeze(1)
                .map_err(candle_error)?;
            let pooler_candidates = prioritized_layer_candidates(&used_prefix, "pooler.dense");
            if let Some(pooler) = load_first_candle_linear(&vb, &pooler_candidates)? {
                pooled = pooler
                    .forward(&pooled)
                    .map_err(candle_error)?
                    .tanh()
                    .map_err(candle_error)?;
            }
            let classifier_candidates = prioritized_layer_candidates(&used_prefix, "classifier");
            let classifier =
                load_required_candle_linear(&vb, &classifier_candidates, "classifier")?;
            let logits = classifier.forward(&pooled).map_err(candle_error)?;
            candle_logits_from_tensor(logits)
        }
        CandleClassifierArchitecture::DistilBert => {
            let config: candle_distilbert::Config = serde_json::from_value(config.clone())
                .map_err(|err| {
                    invalid_argument(format!(
                        "failed to parse DistilBERT config for Candle: {err}"
                    ))
                })?;
            let (model, used_prefix) = load_candle_distilbert_model(&vb, &config, &prefixes)?;
            let input_ids = candle_input_ids(tokens, &device)?;
            let attention_mask = candle_attention_mask_distil(tokens, &device)?;
            let sequence_output = model
                .forward(&input_ids, &attention_mask)
                .map_err(candle_error)?;
            let mut pooled = sequence_output
                .narrow(1, 0, 1)
                .map_err(candle_error)?
                .squeeze(1)
                .map_err(candle_error)?;
            let pre_classifier_candidates =
                prioritized_layer_candidates(&used_prefix, "pre_classifier");
            if let Some(pre_classifier) = load_first_candle_linear(&vb, &pre_classifier_candidates)?
            {
                pooled = pre_classifier
                    .forward(&pooled)
                    .map_err(candle_error)?
                    .relu()
                    .map_err(candle_error)?;
            }
            let classifier_candidates = prioritized_layer_candidates(&used_prefix, "classifier");
            let classifier =
                load_required_candle_linear(&vb, &classifier_candidates, "classifier")?;
            let logits = classifier.forward(&pooled).map_err(candle_error)?;
            candle_logits_from_tensor(logits)
        }
    }
}

#[cfg(feature = "candle")]
fn run_candle_embedder(
    config: &Value,
    model_paths: &[PathBuf],
    architecture: CandleEmbeddingArchitecture,
    tokens: &TokenizedText,
) -> Result<(Vec<f32>, Vec<usize>)> {
    let device = CandleDevice::Cpu;
    let vb = candle_var_builder(model_paths, &device)?;
    let prefixes = model_prefix_candidates(config);

    let sequence_output = match architecture {
        CandleEmbeddingArchitecture::Bert => {
            let config: candle_bert::Config =
                serde_json::from_value(config.clone()).map_err(|err| {
                    invalid_argument(format!("failed to parse BERT config for Candle: {err}"))
                })?;
            let (model, _) = load_candle_bert_model(&vb, &config, &prefixes)?;
            let input_ids = candle_input_ids(tokens, &device)?;
            let token_type_ids = candle_token_type_ids(tokens, &device)?;
            let attention_mask = candle_attention_mask_keep(tokens, &device)?;
            model
                .forward(&input_ids, &token_type_ids, Some(&attention_mask))
                .map_err(candle_error)?
        }
        CandleEmbeddingArchitecture::DistilBert => {
            let config: candle_distilbert::Config = serde_json::from_value(config.clone())
                .map_err(|err| {
                    invalid_argument(format!(
                        "failed to parse DistilBERT config for Candle: {err}"
                    ))
                })?;
            let (model, _) = load_candle_distilbert_model(&vb, &config, &prefixes)?;
            let input_ids = candle_input_ids(tokens, &device)?;
            let attention_mask = candle_attention_mask_distil(tokens, &device)?;
            model
                .forward(&input_ids, &attention_mask)
                .map_err(candle_error)?
        }
    };

    let shape = sequence_output.dims().to_vec();
    let values = sequence_output
        .flatten_all()
        .map_err(candle_error)?
        .to_vec1::<f32>()
        .map_err(candle_error)?;
    Ok((values, shape))
}

#[cfg(feature = "candle")]
fn candle_var_builder<'a>(
    model_paths: &'a [PathBuf],
    device: &CandleDevice,
) -> Result<CandleVarBuilder<'a>> {
    let paths = model_paths
        .iter()
        .map(|path| path.as_path())
        .collect::<Vec<_>>();
    unsafe { CandleVarBuilder::from_mmaped_safetensors(&paths, CandleDType::F32, device) }
        .map_err(candle_error)
}

#[cfg(feature = "candle")]
fn load_candle_bert_model(
    vb: &CandleVarBuilder<'_>,
    config: &candle_bert::Config,
    prefixes: &[String],
) -> Result<(candle_bert::BertModel, String)> {
    let mut last_error = None;
    for prefix in prefixes {
        let model_vb = if prefix.is_empty() {
            vb.clone()
        } else {
            vb.pp(prefix)
        };
        match candle_bert::BertModel::load(model_vb, config) {
            Ok(model) => return Ok((model, prefix.clone())),
            Err(err) => last_error = Some(err.to_string()),
        }
    }
    Err(DetectError::Source(format!(
        "failed to load Candle BERT model for prefixes [{}]{}",
        prefixes.join(", "),
        last_error.map(|err| format!(": {err}")).unwrap_or_default()
    )))
}

#[cfg(feature = "candle")]
fn load_candle_distilbert_model(
    vb: &CandleVarBuilder<'_>,
    config: &candle_distilbert::Config,
    prefixes: &[String],
) -> Result<(candle_distilbert::DistilBertModel, String)> {
    let mut last_error = None;
    for prefix in prefixes {
        let model_vb = if prefix.is_empty() {
            vb.clone()
        } else {
            vb.pp(prefix)
        };
        match candle_distilbert::DistilBertModel::load(model_vb, config) {
            Ok(model) => return Ok((model, prefix.clone())),
            Err(err) => last_error = Some(err.to_string()),
        }
    }
    Err(DetectError::Source(format!(
        "failed to load Candle DistilBERT model for prefixes [{}]{}",
        prefixes.join(", "),
        last_error.map(|err| format!(": {err}")).unwrap_or_default()
    )))
}

#[cfg(feature = "candle")]
fn load_first_candle_linear(
    vb: &CandleVarBuilder<'_>,
    layer_paths: &[String],
) -> Result<Option<CandleLinear>> {
    for layer_path in layer_paths {
        if let Some(linear) = load_candle_linear(vb, layer_path)? {
            return Ok(Some(linear));
        }
    }
    Ok(None)
}

#[cfg(feature = "candle")]
fn load_required_candle_linear(
    vb: &CandleVarBuilder<'_>,
    layer_paths: &[String],
    layer_name: &str,
) -> Result<CandleLinear> {
    load_first_candle_linear(vb, layer_paths)?.ok_or_else(|| {
        DetectError::Source(format!(
            "failed to load Candle `{layer_name}` layer from [{}]",
            layer_paths.join(", ")
        ))
    })
}

#[cfg(feature = "candle")]
fn load_candle_linear(vb: &CandleVarBuilder<'_>, layer_path: &str) -> Result<Option<CandleLinear>> {
    let layer_vb = vb.pp(layer_path);
    if !layer_vb.contains_tensor("weight") {
        return Ok(None);
    }
    let weight = layer_vb.get_unchecked("weight").map_err(candle_error)?;
    let bias = if layer_vb.contains_tensor("bias") {
        Some(layer_vb.get_unchecked("bias").map_err(candle_error)?)
    } else {
        None
    };
    Ok(Some(CandleLinear::new(weight, bias)))
}

#[cfg(feature = "candle")]
fn prioritized_layer_candidates(primary_prefix: &str, suffix: &str) -> Vec<String> {
    let mut candidates = Vec::new();
    if !primary_prefix.is_empty() {
        push_unique_string(&mut candidates, format!("{primary_prefix}.{suffix}"));
    }
    push_unique_string(&mut candidates, suffix.to_string());
    candidates
}

#[cfg(feature = "candle")]
fn model_prefix_candidates(config: &Value) -> Vec<String> {
    let mut prefixes = Vec::new();
    push_unique_string(&mut prefixes, String::new());
    if let Some(model_type) = config.get("model_type").and_then(Value::as_str) {
        push_unique_string(&mut prefixes, model_type.to_string());
    }
    push_unique_string(&mut prefixes, "bert".to_string());
    push_unique_string(&mut prefixes, "distilbert".to_string());
    push_unique_string(&mut prefixes, "0.auto_model".to_string());
    push_unique_string(&mut prefixes, "auto_model".to_string());
    push_unique_string(&mut prefixes, "model".to_string());
    prefixes
}

#[cfg(feature = "candle")]
fn push_unique_string(values: &mut Vec<String>, value: String) {
    if !values.iter().any(|existing| existing == &value) {
        values.push(value);
    }
}

#[cfg(feature = "candle")]
fn candle_input_ids(tokens: &TokenizedText, device: &CandleDevice) -> Result<CandleTensor> {
    let values = tokens
        .input_ids
        .iter()
        .map(|value| {
            u32::try_from(*value).map_err(|_| {
                invalid_argument(format!(
                    "tokenizer produced an out-of-range input id for Candle: {value}"
                ))
            })
        })
        .collect::<Result<Vec<_>>>()?;
    CandleTensor::from_vec(values, (1, tokens.input_ids.len()), device).map_err(candle_error)
}

#[cfg(feature = "candle")]
fn candle_token_type_ids(tokens: &TokenizedText, device: &CandleDevice) -> Result<CandleTensor> {
    let values = match &tokens.token_type_ids {
        Some(values) => values
            .iter()
            .map(|value| {
                u32::try_from(*value).map_err(|_| {
                    invalid_argument(format!(
                        "tokenizer produced an out-of-range token type id for Candle: {value}"
                    ))
                })
            })
            .collect::<Result<Vec<_>>>()?,
        None => vec![0_u32; tokens.input_ids.len()],
    };
    CandleTensor::from_vec(values, (1, tokens.input_ids.len()), device).map_err(candle_error)
}

#[cfg(feature = "candle")]
fn candle_attention_mask_keep(
    tokens: &TokenizedText,
    device: &CandleDevice,
) -> Result<CandleTensor> {
    let values = tokens
        .attention_mask
        .iter()
        .map(|value| if *value == 0 { 0_u32 } else { 1_u32 })
        .collect::<Vec<_>>();
    CandleTensor::from_vec(values, (1, tokens.attention_mask.len()), device).map_err(candle_error)
}

#[cfg(feature = "candle")]
fn candle_attention_mask_distil(
    tokens: &TokenizedText,
    device: &CandleDevice,
) -> Result<CandleTensor> {
    let values = tokens
        .attention_mask
        .iter()
        .map(|value| if *value == 0 { 1_u8 } else { 0_u8 })
        .collect::<Vec<_>>();
    CandleTensor::from_vec(values, (1, tokens.attention_mask.len()), device).map_err(candle_error)
}

#[cfg(feature = "candle")]
fn candle_logits_from_tensor(logits: CandleTensor) -> Result<Vec<f32>> {
    let logits = if logits.rank() == 2 {
        logits.squeeze(0).map_err(candle_error)?
    } else {
        logits
    };
    logits.to_vec1::<f32>().map_err(candle_error)
}

fn read_json(path: &Path) -> Result<Value> {
    let data = fs::read(path)?;
    serde_json::from_slice(&data).map_err(|err| {
        DetectError::Source(format!("failed to parse JSON `{}`: {err}", path.display()))
    })
}

fn labels_from_config(config: &Value) -> Vec<String> {
    let Some(id2label) = config.get("id2label") else {
        return Vec::new();
    };
    if let Some(map) = id2label.as_object() {
        let mut labels = map
            .iter()
            .filter_map(|(key, value)| {
                Some((key.parse::<usize>().ok()?, value.as_str()?.to_string()))
            })
            .collect::<BTreeMap<_, _>>();
        if labels.is_empty() {
            return Vec::new();
        }
        let max = labels.keys().copied().max().unwrap_or(0);
        return (0..=max)
            .map(|index| {
                labels
                    .remove(&index)
                    .unwrap_or_else(|| format!("LABEL_{index}"))
            })
            .collect();
    }
    if let Some(values) = id2label.as_array() {
        return values
            .iter()
            .enumerate()
            .map(|(index, value)| {
                value
                    .as_str()
                    .map(ToString::to_string)
                    .unwrap_or_else(|| format!("LABEL_{index}"))
            })
            .collect();
    }
    Vec::new()
}

fn invalid_argument(message: impl Into<String>) -> DetectError {
    DetectError::InvalidArgument(message.into())
}

#[cfg(feature = "candle")]
fn candle_error(error: candle_core::Error) -> DetectError {
    DetectError::Source(format!("Candle runtime error: {error}"))
}

#[cfg(feature = "onnx")]
fn ort_error<T>(error: ort::Error<T>) -> DetectError {
    DetectError::Source(format!("ONNX runtime error: {error}"))
}

#[cfg(feature = "onnx")]
fn onnx_timing_enabled() -> bool {
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(|| {
        std::env::var_os("VIDEO_ANALYSIS_ONNX_TIMING")
            .map(|value| {
                let value = value.to_string_lossy();
                !value.is_empty() && value != "0" && !value.eq_ignore_ascii_case("false")
            })
            .unwrap_or(false)
    })
}

#[cfg(feature = "onnx")]
fn log_onnx_stage_timing(stage: &str, model_path: &Path, elapsed: std::time::Duration, ok: bool) {
    eprintln!(
        "text-analysis-models onnx timing: stage={stage} model={} elapsed_ms={} status={}",
        model_path.display(),
        elapsed.as_millis(),
        if ok { "ok" } else { "err" }
    );
}

#[cfg(feature = "onnx")]
fn log_onnx_stage_event(stage: &str, model_path: &Path, event: &str) {
    eprintln!(
        "text-analysis-models onnx timing: stage={stage} model={} event={event}",
        model_path.display()
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use video_analysis_models::{
        HuggingFaceModelSpec, ModelBundle, ModelBundleFile, ModelBundleManifest, ModelTask,
    };

    #[derive(Debug)]
    struct FakeClassifierRunner {
        logits: Vec<f32>,
    }

    impl OnnxTextClassifierRunner for FakeClassifierRunner {
        fn run_logits(&mut self, _tokens: &TokenizedText) -> Result<Vec<f32>> {
            Ok(self.logits.clone())
        }
    }

    #[derive(Debug)]
    struct FakeEmbeddingRunner {
        values: Vec<f32>,
        shape: Vec<usize>,
    }

    impl OnnxTextEmbeddingRunner for FakeEmbeddingRunner {
        fn run_embeddings(&self, _tokens: &TokenizedText) -> Result<(Vec<f32>, Vec<usize>)> {
            Ok((self.values.clone(), self.shape.clone()))
        }
    }

    #[test]
    fn validates_missing_onnx_bundle_files() {
        let dir = tempfile::tempdir().unwrap();
        let bundle = fake_bundle(dir.path(), [("config.json", r#"{"id2label":{"0":"NEG"}}"#)]);
        assert!(validate_onnx_bundle(&bundle).is_err());

        let bundle = fake_bundle(dir.path(), [("tokenizer.json", "{}")]);
        assert!(validate_onnx_bundle(&bundle).is_err());
    }

    #[test]
    fn reads_onnx_label_mapping() {
        let dir = tempfile::tempdir().unwrap();
        let bundle = fake_bundle(
            dir.path(),
            [
                (
                    "config.json",
                    r#"{"id2label":{"0":"NEGATIVE","1":"POSITIVE"}}"#,
                ),
                ("tokenizer.json", "{}"),
                ("onnx/model.onnx", "fake"),
            ],
        );
        let info = validate_onnx_bundle(&bundle).unwrap();
        assert_eq!(info.labels, vec!["NEGATIVE", "POSITIVE"]);

        let mut classifier = OnnxTextClassifier::from_runner(
            bundle,
            FakeClassifierRunner {
                logits: vec![0.0, 2.0, 1.0],
            },
        )
        .unwrap();
        let predictions = classifier
            .classify_tokenized(&TokenizedText {
                input_ids: vec![1, 2],
                attention_mask: vec![1, 1],
                token_type_ids: None,
                offsets: vec![None, None],
            })
            .unwrap();
        assert_eq!(predictions[0].label.as_deref(), Some("NEGATIVE"));
        assert_eq!(predictions[2].label.as_deref(), Some("LABEL_2"));
        assert!(predictions[1].score.unwrap() > predictions[0].score.unwrap());
    }

    #[test]
    fn pools_and_normalizes_onnx_embeddings_with_fake_runner() {
        let dir = tempfile::tempdir().unwrap();
        let bundle = fake_bundle(
            dir.path(),
            [
                ("config.json", "{}"),
                ("tokenizer.json", "{}"),
                ("onnx/model.onnx", "fake"),
            ],
        );
        let embedder = OnnxTextEmbedder::from_runner(
            bundle,
            FakeEmbeddingRunner {
                values: vec![1.0, 0.0, 3.0, 4.0, 10.0, 10.0],
                shape: vec![1, 3, 2],
            },
        )
        .unwrap()
        .pooling(PoolingStrategy::Mean)
        .normalize(false);
        let vector = embedder
            .embed_tokenized(&TokenizedText {
                input_ids: vec![1, 2, 3],
                attention_mask: vec![1, 1, 0],
                token_type_ids: None,
                offsets: vec![None, None, None],
            })
            .unwrap();
        assert_eq!(vector.as_slice(), &[2.0, 2.0]);

        let normalized =
            pool_embedding_output(&[3.0, 4.0], &[2], &[1], PoolingStrategy::Cls, true).unwrap();
        assert_eq!(normalized.as_slice(), &[0.6, 0.8]);
    }

    #[test]
    fn candle_reports_unsupported_architectures() {
        let dir = tempfile::tempdir().unwrap();
        let bundle = fake_bundle(
            dir.path(),
            [
                ("config.json", r#"{"architectures":["UnsupportedForText"]}"#),
                ("tokenizer.json", "{}"),
                ("model.safetensors", "fake"),
            ],
        );
        assert!(CandleTextClassifier::from_bundle(bundle.clone()).is_err());
        assert!(CandleTextEmbedder::from_bundle(bundle).is_err());
    }

    fn fake_bundle<'a>(
        root: &Path,
        files: impl IntoIterator<Item = (&'a str, &'a str)>,
    ) -> ModelBundle {
        let bundle_root = root.join("bundle");
        let files_root = bundle_root.join("files");
        fs::create_dir_all(&files_root).unwrap();
        let mut manifest_files = BTreeMap::new();
        for (remote_path, contents) in files {
            let path = files_root.join(remote_path);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            let mut file = fs::File::create(&path).unwrap();
            file.write_all(contents.as_bytes()).unwrap();
            manifest_files.insert(
                remote_path.to_string(),
                ModelBundleFile {
                    remote_path: remote_path.to_string(),
                    local_path: Path::new("files")
                        .join(remote_path)
                        .to_string_lossy()
                        .replace('\\', "/"),
                    size_bytes: contents.len() as u64,
                },
            );
        }
        ModelBundle {
            root: bundle_root,
            manifest: ModelBundleManifest {
                schema_version: 1,
                name: "fake".to_string(),
                repo_id: "fake/repo".to_string(),
                revision: "main".to_string(),
                task: ModelTask::TextClassification,
                files: manifest_files,
            },
        }
    }

    #[test]
    fn preset_specs_can_be_constructed_for_docs() {
        let spec = HuggingFaceModelSpec::new("fake/repo", ModelTask::TextClassification);
        assert_eq!(spec.repo_id, "fake/repo");
    }

    #[test]
    fn tokenizer_source_defaults_to_minilm() {
        assert_eq!(
            TokenizerSource::default(),
            TokenizerSource::Preset(TokenizerPreset::MiniLmL6V2)
        );
        assert_eq!(TokenizerPreset::default(), TokenizerPreset::MiniLmL6V2);
        assert_eq!(
            TokenizerPreset::ALL,
            &[
                TokenizerPreset::BertBaseUncased,
                TokenizerPreset::DistilbertSst2,
                TokenizerPreset::MiniLmL6V2
            ]
        );
    }

    #[test]
    fn tokenizer_bundle_can_be_built_from_local_source() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("custom-tokenizer.json");
        let bundle =
            TokenizerBundle::from_cached_source(TokenizerSource::local(path.clone())).unwrap();
        assert_eq!(bundle.tokenizer_path(), path.as_path());
    }

    #[test]
    fn runtime_defaults_expose_rich_backend_order() {
        let config = TextRuntimeConfig::default();
        assert_eq!(
            config.backend_priority,
            vec![
                TextRuntimeBackend::Onnx,
                TextRuntimeBackend::Candle,
                TextRuntimeBackend::Tokenizers,
                TextRuntimeBackend::Heuristic,
            ]
        );
        assert_eq!(
            select_text_runtime_backend(&config.backend_priority),
            TextRuntimeBackend::Onnx
        );
        assert_eq!(
            select_text_runtime_backend(&[]),
            TextRuntimeBackend::Heuristic
        );
    }

    #[test]
    fn cuda_oxide_priority_is_explicit_opt_in() {
        let priority = cuda_oxide_backend_priority();
        assert_eq!(priority[0], TextRuntimeBackend::CudaOxide);
        assert_eq!(
            select_text_runtime_backend(&priority),
            TextRuntimeBackend::CudaOxide
        );

        let target = CudaOxideTextRuntimeTarget::new("text_embed_cuda", ["mean_pool_embeddings"])
            .runtime(CudaOxideRuntimeConfig::new().target_sm("sm_80"));
        assert_eq!(target.runtime.target_sm.as_deref(), Some("sm_80"));
        assert_eq!(target.module_name, "text_embed_cuda");
    }

    #[test]
    fn runtime_catalog_tracks_default_presets() {
        let catalog = TextRuntimeCatalog::default();
        assert_eq!(catalog.default_tokenizer, TokenizerSource::default());
        assert!(catalog
            .embedder_presets
            .contains(&TokenizerPreset::MiniLmL6V2));
        assert!(catalog
            .classifier_presets
            .contains(&TokenizerPreset::DistilbertSst2));
    }

    #[cfg(feature = "external-tests")]
    fn tiny_pinned_onnx_classifier_spec() -> HuggingFaceModelSpec {
        HuggingFaceModelSpec::new(
            "onnx-internal-testing/tiny-random-BertForSequenceClassification-ONNX",
            ModelTask::TextClassification,
        )
        .name("tiny-random-bert-sequence-classification-onnx")
        .revision("bd9b3e860b0783fce290eaacd78dff74bb2e88a3")
        .file("config.json")
        .file("tokenizer.json")
        .file("tokenizer_config.json")
        .file("onnx/model.onnx")
    }

    #[cfg(feature = "external-tests")]
    fn tiny_pinned_onnx_embedder_spec() -> HuggingFaceModelSpec {
        HuggingFaceModelSpec::new(
            "onnx-internal-testing/tiny-random-BertModel-ONNX",
            ModelTask::TextEmbedding,
        )
        .name("tiny-random-bert-model-onnx")
        .revision("3f74b196fbb9932eddd11953553cb7ce04ae671a")
        .file("config.json")
        .file("tokenizer.json")
        .file("tokenizer_config.json")
        .file("onnx/model.onnx")
    }

    #[cfg(feature = "external-tests")]
    fn assert_nonempty_file(path: impl AsRef<Path>) {
        let path = path.as_ref();
        let metadata = fs::metadata(path)
            .unwrap_or_else(|err| panic!("expected `{}` metadata: {err}", path.display()));
        assert!(
            metadata.is_file() && metadata.len() > 0,
            "expected `{}` to be a non-empty file",
            path.display()
        );
    }

    #[cfg(feature = "external-tests")]
    fn download_bundle(spec: &HuggingFaceModelSpec) -> (tempfile::TempDir, ModelBundle) {
        let dir = tempfile::tempdir().unwrap();
        let bundle = ModelBundleStore::new(dir.path().join("bundles"))
            .downloader(
                HuggingFaceDownloader::new()
                    .cache_dir(dir.path().join("cache"))
                    .progress(false)
                    .max_retries(1),
            )
            .download(spec)
            .unwrap();
        (dir, bundle)
    }

    #[cfg(feature = "slow-external-tests")]
    fn slow_onnx_tests_enabled() -> bool {
        std::env::var_os("RUN_SLOW_ONNX_TESTS")
            .map(|value| {
                let value = value.to_string_lossy();
                !value.is_empty() && value != "0" && !value.eq_ignore_ascii_case("false")
            })
            .unwrap_or(false)
    }

    #[cfg(all(feature = "slow-external-tests", feature = "onnx"))]
    fn log_slow_test_stage_timing(stage: &str, elapsed: std::time::Duration) {
        if onnx_timing_enabled() {
            eprintln!(
                "text-analysis-models slow onnx test timing: stage={stage} elapsed_ms={}",
                elapsed.as_millis()
            );
        }
    }

    #[cfg(feature = "external-tests")]
    #[test]
    #[ignore]
    fn downloads_tiny_pinned_onnx_classifier_bundle_and_validates_metadata() {
        use video_analysis_models::{HuggingFaceDownloader, ModelBundleStore};

        let dir = tempfile::tempdir().unwrap();
        let bundle = ModelBundleStore::new(dir.path().join("bundles"))
            .downloader(
                HuggingFaceDownloader::new()
                    .cache_dir(dir.path().join("cache"))
                    .progress(false)
                    .max_retries(1),
            )
            .download(&tiny_pinned_onnx_classifier_spec())
            .unwrap();
        let info = validate_onnx_bundle(&bundle).unwrap();
        assert_nonempty_file(bundle.manifest_path());
        assert_nonempty_file(info.config_path);
        assert_nonempty_file(info.tokenizer_path);
        assert_nonempty_file(info.model_path);
    }

    #[cfg(feature = "external-tests")]
    #[test]
    #[ignore]
    fn downloads_tiny_pinned_onnx_embedder_bundle_and_validates_metadata() {
        let (_dir, bundle) = download_bundle(&tiny_pinned_onnx_embedder_spec());
        let info = validate_onnx_bundle(&bundle).unwrap();
        assert_nonempty_file(bundle.manifest_path());
        assert_nonempty_file(info.config_path);
        assert_nonempty_file(info.tokenizer_path);
        assert_nonempty_file(info.model_path);
    }

    #[cfg(all(feature = "slow-external-tests", feature = "onnx"))]
    #[test]
    #[ignore = "requires network access and opt-in slow ONNX runtime coverage"]
    fn runs_tiny_pinned_onnx_classifier_runtime_when_requested() {
        if !slow_onnx_tests_enabled() {
            eprintln!("skipping slow ONNX runtime test; set RUN_SLOW_ONNX_TESTS=1 to enable");
            return;
        }

        let started = std::time::Instant::now();
        let (_dir, bundle) = download_bundle(&tiny_pinned_onnx_classifier_spec());
        log_slow_test_stage_timing("download_bundle", started.elapsed());

        let started = std::time::Instant::now();
        let info = validate_onnx_bundle(&bundle).unwrap();
        log_slow_test_stage_timing("validate_onnx_bundle", started.elapsed());

        let started = std::time::Instant::now();
        let runner = NativeOnnxRunner::new(&info.model_path).unwrap();
        log_slow_test_stage_timing("NativeOnnxRunner::new", started.elapsed());

        let started = std::time::Instant::now();
        let mut classifier = OnnxTextClassifier::from_runner(bundle, runner).unwrap();
        log_slow_test_stage_timing("OnnxTextClassifier::from_runner", started.elapsed());

        let started = std::time::Instant::now();
        let predictions = classifier
            .classify("Rust crates and cargo packages")
            .unwrap();
        log_slow_test_stage_timing("OnnxTextClassifier::classify", started.elapsed());
        assert!(!predictions.is_empty());
        assert!(predictions
            .iter()
            .all(|prediction| prediction.score.unwrap_or(0.0).is_finite()));
    }

    #[cfg(all(feature = "slow-external-tests", feature = "onnx"))]
    #[test]
    #[ignore = "requires network access and opt-in slow ONNX runtime coverage"]
    fn runs_tiny_pinned_onnx_embedder_runtime_when_requested() {
        if !slow_onnx_tests_enabled() {
            eprintln!("skipping slow ONNX runtime test; set RUN_SLOW_ONNX_TESTS=1 to enable");
            return;
        }

        let started = std::time::Instant::now();
        let (_dir, bundle) = download_bundle(&tiny_pinned_onnx_embedder_spec());
        log_slow_test_stage_timing("download_bundle", started.elapsed());

        let started = std::time::Instant::now();
        let info = validate_onnx_bundle(&bundle).unwrap();
        log_slow_test_stage_timing("validate_onnx_bundle", started.elapsed());

        let started = std::time::Instant::now();
        let runner = NativeOnnxRunner::new(&info.model_path).unwrap();
        log_slow_test_stage_timing("NativeOnnxRunner::new", started.elapsed());

        let started = std::time::Instant::now();
        let embedder = OnnxTextEmbedder::from_runner(bundle, runner).unwrap();
        log_slow_test_stage_timing("OnnxTextEmbedder::from_runner", started.elapsed());

        let started = std::time::Instant::now();
        let left = embedder
            .embed_text("Rust crates and cargo packages")
            .unwrap();
        log_slow_test_stage_timing("OnnxTextEmbedder::embed_text.left", started.elapsed());

        let started = std::time::Instant::now();
        let right = embedder
            .embed_text("A completely different sentence")
            .unwrap();
        log_slow_test_stage_timing("OnnxTextEmbedder::embed_text.right", started.elapsed());
        assert_eq!(left.dimensions(), right.dimensions());
        assert!(left.dimensions() > 0);
        assert!(left.as_slice().iter().all(|value| value.is_finite()));
        assert!(right.as_slice().iter().all(|value| value.is_finite()));
    }

    #[cfg(feature = "external-tests")]
    #[test]
    #[ignore]
    fn downloads_distilbert_candle_preset_and_classifies_sentence() {
        use video_analysis_models::{HuggingFaceDownloader, ModelBundleStore, ModelPreset};

        let dir = tempfile::tempdir().unwrap();
        let bundle = match ModelBundleStore::new(dir.path().join("bundles"))
            .downloader(HuggingFaceDownloader::new().progress(false).max_retries(1))
            .download(&ModelPreset::DistilbertSst2.spec())
        {
            Ok(bundle) => bundle,
            Err(err) => {
                if err.to_string().contains("tokenizer.json") {
                    return;
                }
                panic!("failed to download distilbert candle preset: {err}");
            }
        };
        let mut classifier = match CandleTextClassifier::from_bundle(bundle) {
            Ok(classifier) => classifier,
            Err(err) => {
                if err.to_string().contains("tokenizer.json") {
                    return;
                }
                panic!("failed to build candle classifier from bundle: {err}");
            }
        };
        let predictions = classifier.classify("I love reliable Rust tools.").unwrap();
        assert!(!predictions.is_empty());
        assert!(predictions
            .iter()
            .all(|prediction| prediction.score.unwrap_or(0.0).is_finite()));
    }

    #[cfg(feature = "external-tests")]
    #[test]
    #[ignore]
    fn downloads_minilm_candle_preset_and_embeds_sentences() {
        use video_analysis_models::{HuggingFaceDownloader, ModelBundleStore, ModelPreset};

        let dir = tempfile::tempdir().unwrap();
        let bundle = ModelBundleStore::new(dir.path().join("bundles"))
            .downloader(HuggingFaceDownloader::new().progress(false).max_retries(1))
            .download(&ModelPreset::MiniLmL6V2.spec())
            .unwrap();
        let embedder = CandleTextEmbedder::from_bundle(bundle).unwrap();
        let left = embedder
            .embed_text("Rust crates and cargo packages")
            .unwrap();
        let right = embedder
            .embed_text("A completely different sentence")
            .unwrap();
        assert_eq!(left.dimensions(), right.dimensions());
        assert!(left.dimensions() > 0);
    }

    #[cfg(all(feature = "external-tests", feature = "tokenizers"))]
    #[test]
    #[ignore]
    fn downloads_default_tokenizer_once_and_reuses_cached_file() {
        let dir = tempfile::tempdir().unwrap();
        let options = TokenizerDownloadOptions {
            cache_dir: Some(dir.path().join("hf-cache")),
            progress: false,
            max_retries: 1,
            ..TokenizerDownloadOptions::default()
        };

        let first =
            TokenizerBundle::from_cached_source_with_options(TokenizerSource::default(), &options)
                .unwrap();
        let first_metadata = fs::metadata(first.tokenizer_path()).unwrap();
        let tokenized = first.tokenize("Rust crates and cargo packages").unwrap();
        assert!(!tokenized.input_ids.is_empty());

        let second =
            TokenizerBundle::from_cached_source_with_options(TokenizerSource::default(), &options)
                .unwrap();
        let second_metadata = fs::metadata(second.tokenizer_path()).unwrap();
        assert_eq!(first.tokenizer_path(), second.tokenizer_path());
        assert_eq!(first_metadata.len(), second_metadata.len());
    }
}
