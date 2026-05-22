#![doc = include_str!("../README.md")]

use std::collections::BTreeMap;
#[cfg(feature = "tokenizers")]
use std::fs;
use std::path::{Path, PathBuf};

#[cfg(feature = "candle")]
use candle_core::{DType as CandleDType, Device as CandleDevice, Tensor as CandleTensor};
#[cfg(feature = "candle")]
use candle_nn::{Linear as CandleLinear, Module as CandleModule, VarBuilder as CandleVarBuilder};
#[cfg(feature = "candle")]
use candle_transformers::models::{bert as candle_bert, distilbert as candle_distilbert};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use video_analysis_core::{DetectError, Result};
#[cfg(feature = "model-bundles")]
use video_analysis_models::{HuggingFaceDownloader, HuggingFaceModelSpec, ModelBundle, ModelTask};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
/// Text-oriented raw prediction shared by local and imported NLP runtimes.
pub struct RawPrediction {
    /// Raw prediction kind.
    pub kind: Option<String>,
    /// Label assigned by the model.
    pub label: Option<String>,
    /// Optional text span or document text.
    pub text: Option<String>,
    /// Confidence score.
    pub score: Option<f32>,
    #[serde(default)]
    /// Arbitrary runtime attributes, including offsets.
    pub attributes: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Variants describing truncation strategy.
pub enum TruncationStrategy {
    /// Does not truncate tokenized input.
    None,
    /// Keeps the first sequence up to the configured maximum.
    LongestFirst,
    /// Keeps only the first sequence up to the configured maximum.
    OnlyFirst,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
/// Tokenized text prepared for native text model runtimes.
pub struct TokenizedText {
    /// Token ids.
    pub input_ids: Vec<i64>,
    /// Attention mask values.
    pub attention_mask: Vec<i64>,
    /// Optional token type ids.
    pub token_type_ids: Option<Vec<i64>>,
    /// Byte offsets for each token, when available.
    pub offsets: Vec<Option<(usize, usize)>>,
}

impl TokenizedText {
    /// Truncates every token field to the same maximum length.
    pub fn truncate(&mut self, max_length: usize) {
        self.input_ids.truncate(max_length);
        self.attention_mask.truncate(max_length);
        if let Some(token_type_ids) = &mut self.token_type_ids {
            token_type_ids.truncate(max_length);
        }
        self.offsets.truncate(max_length);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
/// Built-in tokenizer presets.
pub enum TokenizerPreset {
    /// BERT base uncased.
    BertBaseUncased,
    /// DistilBERT SST-2.
    DistilbertSst2,
    #[default]
    /// MiniLM L6 v2.
    MiniLmL6V2,
}

impl TokenizerPreset {
    /// All built-in presets.
    pub const ALL: &'static [Self] = &[
        Self::BertBaseUncased,
        Self::DistilbertSst2,
        Self::MiniLmL6V2,
    ];

    /// Stable preset id.
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
/// Source for a tokenizer bundle.
pub enum TokenizerSource {
    /// Local tokenizer file.
    Local(PathBuf),
    /// Built-in preset.
    Preset(TokenizerPreset),
    /// Hugging Face tokenizer file.
    HuggingFace {
        /// Repository id.
        repo_id: String,
        /// Revision.
        revision: String,
        /// Tokenizer file.
        tokenizer_file: String,
    },
}

impl TokenizerSource {
    /// Uses a local tokenizer file.
    pub fn local(path: impl Into<PathBuf>) -> Self {
        Self::Local(path.into())
    }

    /// Uses a built-in preset.
    pub fn preset(preset: TokenizerPreset) -> Self {
        Self::Preset(preset)
    }

    /// Uses a Hugging Face tokenizer.
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
                #[cfg(feature = "model-bundles")]
                {
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
                #[cfg(not(feature = "model-bundles"))]
                {
                    let _ = (repo_id, revision, tokenizer_file, options);
                    Err(invalid_argument(
                        "Hugging Face tokenizer downloads require the `model-bundles` feature",
                    ))
                }
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
/// Tokenizer download options.
pub struct TokenizerDownloadOptions {
    /// Optional cache directory.
    pub cache_dir: Option<PathBuf>,
    /// Optional Hugging Face token.
    pub token: Option<String>,
    /// Whether to emit progress.
    pub progress: bool,
    /// Maximum download retries.
    pub max_retries: usize,
}

impl TokenizerDownloadOptions {
    /// Builds the configured downloader.
    #[cfg(feature = "model-bundles")]
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
/// Tokenizer file plus runtime truncation settings.
pub struct TokenizerBundle {
    tokenizer_path: PathBuf,
    /// Optional maximum token count.
    pub max_length: Option<usize>,
    /// Truncation strategy.
    pub truncation: TruncationStrategy,
}

impl TokenizerBundle {
    /// Creates a new tokenizer bundle.
    pub fn new(tokenizer_path: impl Into<PathBuf>) -> Self {
        Self {
            tokenizer_path: tokenizer_path.into(),
            max_length: None,
            truncation: TruncationStrategy::None,
        }
    }

    /// Builds from a model bundle containing `tokenizer.json` or `vocab.txt`.
    #[cfg(feature = "model-bundles")]
    pub fn from_bundle(bundle: &ModelBundle) -> Result<Self> {
        let tokenizer_path = bundle
            .file_path("tokenizer.json")
            .or_else(|| bundle.file_path("vocab.txt"))
            .ok_or_else(|| {
                invalid_argument(format!(
                    "model bundle `{}` is missing required tokenizer file `tokenizer.json` or `vocab.txt`",
                    bundle.manifest.name
                ))
            })?;
        Ok(Self::new(tokenizer_path))
    }

    /// Builds from the default cached source.
    pub fn from_default_cached() -> Result<Self> {
        Self::from_cached_source(TokenizerSource::default())
    }

    /// Builds from a cached source.
    pub fn from_cached_source(source: TokenizerSource) -> Result<Self> {
        Self::from_cached_source_with_options(source, &TokenizerDownloadOptions::default())
    }

    /// Builds from a cached source with options.
    pub fn from_cached_source_with_options(
        source: TokenizerSource,
        options: &TokenizerDownloadOptions,
    ) -> Result<Self> {
        Ok(Self::new(source.resolve_path(options)?))
    }

    /// Sets the maximum token count.
    pub fn max_length(mut self, max_length: usize) -> Self {
        self.max_length = Some(max_length);
        self
    }

    /// Sets truncation behavior.
    pub fn truncation(mut self, strategy: TruncationStrategy) -> Self {
        self.truncation = strategy;
        self
    }

    /// Returns the tokenizer path.
    pub fn tokenizer_path(&self) -> &Path {
        &self.tokenizer_path
    }

    #[cfg(feature = "tokenizers")]
    /// Tokenizes text with the configured tokenizer.
    pub fn tokenize(&self, text: &str) -> Result<TokenizedText> {
        let tokenizer = load_tokenizer(&self.tokenizer_path)?;
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
    /// Tokenizes text when the tokenizer feature is available.
    pub fn tokenize(&self, _text: &str) -> Result<TokenizedText> {
        Err(invalid_argument(
            "tokenizer execution requires the `tokenizers` feature",
        ))
    }
}

#[cfg(feature = "tokenizers")]
fn load_tokenizer(path: &Path) -> Result<tokenizers::Tokenizer> {
    if path.file_name().and_then(|value| value.to_str()) == Some("vocab.txt") {
        let vocab_path = path.to_str().ok_or_else(|| {
            invalid_argument(format!(
                "tokenizer vocab path is not valid UTF-8: {}",
                path.display()
            ))
        })?;
        let model = tokenizers::models::wordpiece::WordPiece::from_file(vocab_path)
            .build()
            .map_err(|err| {
                DetectError::Source(format!(
                    "failed to load WordPiece vocab `{}`: {err}",
                    path.display()
                ))
            })?;
        let vocab = tokenizers::Model::get_vocab(&model);
        let cls_id = *vocab.get("[CLS]").unwrap_or(&101);
        let sep_id = *vocab.get("[SEP]").unwrap_or(&102);
        let mut tokenizer = tokenizers::Tokenizer::new(model);
        tokenizer.with_normalizer(Some(bert_normalizer_for_vocab(path)));
        tokenizer.with_pre_tokenizer(Some(tokenizers::pre_tokenizers::bert::BertPreTokenizer));
        tokenizer.with_post_processor(Some(tokenizers::processors::bert::BertProcessing::new(
            ("[SEP]".to_string(), sep_id),
            ("[CLS]".to_string(), cls_id),
        )));
        return Ok(tokenizer);
    }

    tokenizers::Tokenizer::from_file(path).map_err(|err| {
        DetectError::Source(format!(
            "failed to load tokenizer `{}`: {err}",
            path.display()
        ))
    })
}

#[cfg(feature = "tokenizers")]
fn bert_normalizer_for_vocab(path: &Path) -> tokenizers::normalizers::bert::BertNormalizer {
    let config = path
        .parent()
        .map(|parent| parent.join("tokenizer_config.json"))
        .and_then(|path| fs::read_to_string(path).ok())
        .and_then(|contents| serde_json::from_str::<serde_json::Value>(&contents).ok());
    let do_lower_case = config
        .as_ref()
        .and_then(|value| value.get("do_lower_case"))
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(true);
    let strip_accents = config
        .as_ref()
        .and_then(|value| value.get("strip_accents"))
        .and_then(serde_json::Value::as_bool);
    tokenizers::normalizers::bert::BertNormalizer::new(true, true, strip_accents, do_lower_case)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
/// Runtime backend family.
pub enum TextRuntimeBackend {
    /// Tokenizer-only runtime.
    Tokenizers,
    /// ONNX Runtime backend.
    Onnx,
    /// Candle backend.
    Candle,
    /// cuda-oxide backend.
    CudaOxide,
    /// Caller-supplied external backend.
    External,
    /// Deterministic heuristic backend.
    Heuristic,
}

/// Sequence labeling backend, used for NER and token classification.
pub trait SequenceLabeler {
    /// Labels text.
    fn label_text(&mut self, text: &str) -> Result<Vec<RawPrediction>>;

    /// Runtime backend.
    fn runtime_backend(&self) -> TextRuntimeBackend;
}

/// Text or pair classification backend.
pub trait SequenceClassifier {
    /// Classifies text against optional labels.
    fn classify_text(&mut self, text: &str, labels: &[String]) -> Result<Vec<RawPrediction>>;

    /// Runtime backend.
    fn runtime_backend(&self) -> TextRuntimeBackend;
}

/// Query-document reranking backend.
pub trait TextReranker {
    /// Reranks documents for a query.
    fn rerank(&mut self, query: &str, documents: &[String]) -> Result<Vec<f32>>;

    /// Runtime backend.
    fn runtime_backend(&self) -> TextRuntimeBackend;
}

/// Extractive question-answering backend.
pub trait QuestionAnsweringBackend {
    /// Answers a question from context text.
    fn answer(&mut self, question: &str, context: &str) -> Result<Vec<RawPrediction>>;

    /// Runtime backend.
    fn runtime_backend(&self) -> TextRuntimeBackend;
}

/// Token classification backend.
pub trait TokenClassifier {
    /// Classifies tokenized text.
    fn classify_tokenized_text(&mut self, tokens: &TokenizedText) -> Result<Vec<RawPrediction>>;

    /// Runtime backend.
    fn runtime_backend(&self) -> TextRuntimeBackend;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
enum CandleTokenClassifierArchitecture {
    Bert,
    DistilBert,
}

#[derive(Debug, Clone)]
/// Candle token-classification facade.
pub struct CandleTokenClassifier {
    tokenizer: TokenizerBundle,
    labels: Vec<String>,
    config: Value,
    model_paths: Vec<PathBuf>,
    architecture: CandleTokenClassifierArchitecture,
}

impl CandleTokenClassifier {
    /// Builds from a model bundle.
    #[cfg(feature = "model-bundles")]
    pub fn from_bundle(bundle: ModelBundle) -> Result<Self> {
        let config_path = required_bundle_file(&bundle, "config.json")?;
        let config = read_json(&config_path)?;
        let tokenizer = tokenizer_with_model_limit(TokenizerBundle::from_bundle(&bundle)?, &config);
        let model_paths = bundle_files_with_extension(&bundle, "safetensors");
        if model_paths.is_empty() {
            return Err(invalid_argument(
                "Candle token classification bundles must contain a `.safetensors` model file",
            ));
        }
        let architecture = token_classifier_architecture_from_config(&config)?;
        Ok(Self {
            tokenizer,
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

    /// Returns tokenizer.
    pub fn tokenizer(&self) -> &TokenizerBundle {
        &self.tokenizer
    }

    /// Classifies text.
    pub fn classify(&mut self, text: &str) -> Result<Vec<RawPrediction>> {
        let tokens = self.tokenizer.tokenize(text)?;
        self.classify_tokenized(text, &tokens)
    }

    /// Classifies tokenized text.
    pub fn classify_tokenized(
        &self,
        text: &str,
        tokens: &TokenizedText,
    ) -> Result<Vec<RawPrediction>> {
        #[cfg(feature = "candle")]
        {
            let (logits, shape) = run_candle_token_classifier(
                &self.config,
                &self.model_paths,
                self.architecture,
                tokens,
            )?;
            token_predictions_from_logits(text, tokens, &logits, &shape, &self.labels)
        }
        #[cfg(not(feature = "candle"))]
        {
            let _ = (
                text,
                tokens,
                &self.config,
                &self.model_paths,
                self.architecture,
            );
            Err(invalid_argument(
                "native Candle token classification requires the `candle` feature",
            ))
        }
    }
}

impl SequenceLabeler for CandleTokenClassifier {
    fn label_text(&mut self, text: &str) -> Result<Vec<RawPrediction>> {
        self.classify(text)
    }

    fn runtime_backend(&self) -> TextRuntimeBackend {
        TextRuntimeBackend::Candle
    }
}

impl TokenClassifier for CandleTokenClassifier {
    fn classify_tokenized_text(&mut self, tokens: &TokenizedText) -> Result<Vec<RawPrediction>> {
        self.classify_tokenized("", tokens)
    }

    fn runtime_backend(&self) -> TextRuntimeBackend {
        TextRuntimeBackend::Candle
    }
}

/// Converts token-classification logits into token predictions.
pub fn token_predictions_from_logits(
    text: &str,
    tokens: &TokenizedText,
    logits: &[f32],
    shape: &[usize],
    labels: &[String],
) -> Result<Vec<RawPrediction>> {
    let (sequence, label_count) = match shape {
        [sequence, labels] => (*sequence, *labels),
        [batch, sequence, labels] if *batch == 1 => (*sequence, *labels),
        _ => {
            return Err(invalid_argument(format!(
                "unsupported token classification output shape `{shape:?}`"
            )));
        }
    };
    if label_count == 0 || logits.len() != sequence * label_count {
        return Err(invalid_argument(
            "token classification output shape does not match logits",
        ));
    }

    let mut predictions = Vec::new();
    for token_index in 0..sequence {
        if tokens.attention_mask.get(token_index).copied().unwrap_or(0) == 0 {
            continue;
        }
        let Some((start, end)) = tokens.offsets.get(token_index).copied().flatten() else {
            continue;
        };
        if start >= end || (!text.is_empty() && end > text.len()) {
            continue;
        }
        let offset = token_index * label_count;
        let scores = softmax(&logits[offset..offset + label_count]);
        let Some((label_index, score)) = scores
            .iter()
            .copied()
            .enumerate()
            .max_by(|left, right| left.1.total_cmp(&right.1))
        else {
            continue;
        };
        let label = labels
            .get(label_index)
            .cloned()
            .unwrap_or_else(|| format!("LABEL_{label_index}"));
        let mut prediction = RawPrediction {
            kind: Some("token".to_string()),
            label: Some(label),
            text: (end <= text.len()).then(|| text[start..end].to_string()),
            score: Some(score),
            ..RawPrediction::default()
        };
        prediction
            .attributes
            .insert("byte_start".to_string(), start.to_string());
        prediction
            .attributes
            .insert("byte_end".to_string(), end.to_string());
        prediction
            .attributes
            .insert("token_index".to_string(), token_index.to_string());
        predictions.push(prediction);
    }
    Ok(predictions)
}

/// Numerically stable softmax.
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

#[allow(dead_code)]
fn tokenizer_with_model_limit(tokenizer: TokenizerBundle, config: &Value) -> TokenizerBundle {
    match model_max_tokens_from_config(config) {
        Some(max_tokens) => tokenizer.max_length(max_tokens),
        None => tokenizer,
    }
}

#[allow(dead_code)]
fn model_max_tokens_from_config(config: &Value) -> Option<usize> {
    config
        .get("max_position_embeddings")
        .or_else(|| config.get("max_seq_len"))
        .and_then(Value::as_u64)
        .and_then(|value| usize::try_from(value).ok())
}

#[allow(dead_code)]
fn token_classifier_architecture_from_config(
    config: &Value,
) -> Result<CandleTokenClassifierArchitecture> {
    let architectures = architectures_from_config(config);
    if architectures.contains(&"DistilBertForTokenClassification") {
        return Ok(CandleTokenClassifierArchitecture::DistilBert);
    }
    if architectures.contains(&"BertForTokenClassification") {
        return Ok(CandleTokenClassifierArchitecture::Bert);
    }
    Err(invalid_argument(format!(
        "unsupported Candle token classification architecture {}; supported: DistilBertForTokenClassification, BertForTokenClassification",
        if architectures.is_empty() {
            "<missing>".to_string()
        } else {
            architectures.join(", ")
        },
    )))
}

#[allow(dead_code)]
fn architectures_from_config(config: &Value) -> Vec<&str> {
    config
        .get("architectures")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect::<Vec<_>>()
}

#[cfg(feature = "model-bundles")]
fn required_bundle_file(bundle: &ModelBundle, remote_path: &str) -> Result<PathBuf> {
    bundle.file_path(remote_path).ok_or_else(|| {
        invalid_argument(format!(
            "model bundle `{}` is missing required file `{remote_path}`",
            bundle.manifest.name
        ))
    })
}

#[cfg(feature = "model-bundles")]
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

#[allow(dead_code)]
fn read_json(path: &Path) -> Result<Value> {
    let data = std::fs::read(path)?;
    serde_json::from_slice(&data).map_err(|err| {
        DetectError::Source(format!("failed to parse JSON `{}`: {err}", path.display()))
    })
}

#[allow(dead_code)]
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

#[cfg(feature = "candle")]
fn run_candle_token_classifier(
    config: &Value,
    model_paths: &[PathBuf],
    architecture: CandleTokenClassifierArchitecture,
    tokens: &TokenizedText,
) -> Result<(Vec<f32>, Vec<usize>)> {
    let device = CandleDevice::Cpu;
    let vb = candle_var_builder(model_paths, &device)?;
    let prefixes = model_prefix_candidates(config);

    let (sequence_output, used_prefix) = match architecture {
        CandleTokenClassifierArchitecture::Bert => {
            let config: candle_bert::Config =
                serde_json::from_value(config.clone()).map_err(|err| {
                    invalid_argument(format!("failed to parse BERT config for Candle: {err}"))
                })?;
            let (model, used_prefix) = load_candle_bert_model(&vb, &config, &prefixes)?;
            let input_ids = candle_input_ids(tokens, &device)?;
            let token_type_ids = candle_token_type_ids(tokens, &device)?;
            let attention_mask = candle_attention_mask_keep(tokens, &device)?;
            (
                model
                    .forward(&input_ids, &token_type_ids, Some(&attention_mask))
                    .map_err(candle_error)?,
                used_prefix,
            )
        }
        CandleTokenClassifierArchitecture::DistilBert => {
            let config: candle_distilbert::Config = serde_json::from_value(config.clone())
                .map_err(|err| {
                    invalid_argument(format!(
                        "failed to parse DistilBERT config for Candle: {err}"
                    ))
                })?;
            let (model, used_prefix) = load_candle_distilbert_model(&vb, &config, &prefixes)?;
            let input_ids = candle_input_ids(tokens, &device)?;
            let attention_mask = candle_attention_mask_distil(tokens, &device)?;
            (
                model
                    .forward(&input_ids, &attention_mask)
                    .map_err(candle_error)?,
                used_prefix,
            )
        }
    };

    let classifier_candidates = prioritized_layer_candidates(&used_prefix, "classifier");
    let classifier = load_required_candle_linear(&vb, &classifier_candidates, "classifier")?;
    let logits = classifier.forward(&sequence_output).map_err(candle_error)?;
    let shape = logits.dims().to_vec();
    let values = logits
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
fn candle_error(error: candle_core::Error) -> DetectError {
    DetectError::Source(format!("Candle runtime error: {error}"))
}

fn invalid_argument(message: impl Into<String>) -> DetectError {
    DetectError::InvalidArgument(message.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokenized_text_truncates_fields_together() {
        let mut tokenized = TokenizedText {
            input_ids: vec![1, 2, 3],
            attention_mask: vec![1, 1, 1],
            token_type_ids: Some(vec![0, 0, 0]),
            offsets: vec![Some((0, 1)), Some((1, 2)), Some((2, 3))],
        };

        tokenized.truncate(2);

        assert_eq!(tokenized.input_ids, vec![1, 2]);
        assert_eq!(tokenized.attention_mask, vec![1, 1]);
        assert_eq!(tokenized.token_type_ids, Some(vec![0, 0]));
        assert_eq!(tokenized.offsets, vec![Some((0, 1)), Some((1, 2))]);
    }

    #[test]
    fn softmax_normalizes_scores() {
        let scores = softmax(&[1.0, 2.0, 3.0]);
        let total = scores.iter().sum::<f32>();
        assert!((total - 1.0).abs() < 0.0001);
        assert!(scores[2] > scores[1]);
    }
}
