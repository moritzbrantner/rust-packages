use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use text_core::{TextSpan, Token};
use video_analysis_core::{DetectError, Result};
use video_analysis_models::{HuggingFaceDownloader, HuggingFaceModelSpec, ModelBundle, ModelTask};

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
    /// Returns tokenize.
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
        tokenizer.with_normalizer(Some(
            tokenizers::normalizers::bert::BertNormalizer::default(),
        ));
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Variants describing tokenization mode.
pub enum TokenizationMode {
    /// The word variant.
    Word,
    /// The subword variant.
    Subword,
    /// The mixed variant.
    Mixed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Data type for tokenizer policy.
pub struct TokenizerPolicy {
    /// The mode value.
    pub mode: TokenizationMode,
    /// The default source value.
    pub default_source: TokenizerSource,
    /// The language overrides value.
    pub language_overrides: BTreeMap<String, TokenizerSource>,
    /// The task overrides value.
    pub task_overrides: BTreeMap<String, TokenizerSource>,
    /// The model family overrides value.
    pub model_family_overrides: BTreeMap<String, TokenizerSource>,
}

impl Default for TokenizerPolicy {
    fn default() -> Self {
        Self {
            mode: TokenizationMode::Mixed,
            default_source: TokenizerSource::default(),
            language_overrides: BTreeMap::new(),
            task_overrides: BTreeMap::new(),
            model_family_overrides: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Data type for tokenizer selection.
pub struct TokenizerSelection {
    /// The mode value.
    pub mode: TokenizationMode,
    /// The source value.
    pub source: Option<TokenizerSource>,
    /// Language tag for this value.
    pub language: Option<String>,
    /// The task value.
    pub task: Option<String>,
    /// The model family value.
    pub model_family: Option<String>,
    /// The reason value.
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
/// Data type for tokenizer registry.
pub struct TokenizerRegistry {
    /// The policy value.
    pub policy: TokenizerPolicy,
}

impl TokenizerRegistry {
    /// Returns select.
    pub fn select(
        &self,
        language: Option<&str>,
        task: Option<&str>,
        model_family: Option<&str>,
    ) -> TokenizerSelection {
        let source = match self.policy.mode {
            TokenizationMode::Word => None,
            TokenizationMode::Subword | TokenizationMode::Mixed => Some(
                task.and_then(|task| self.policy.task_overrides.get(task))
                    .or_else(|| {
                        model_family
                            .and_then(|family| self.policy.model_family_overrides.get(family))
                    })
                    .or_else(|| {
                        language.and_then(|language| self.policy.language_overrides.get(language))
                    })
                    .cloned()
                    .unwrap_or_else(|| self.policy.default_source.clone()),
            ),
        };
        let reason = if let Some(task) =
            task.filter(|task| self.policy.task_overrides.contains_key(*task))
        {
            format!("task override for `{task}`")
        } else if let Some(family) =
            model_family.filter(|family| self.policy.model_family_overrides.contains_key(*family))
        {
            format!("model family override for `{family}`")
        } else if let Some(language) =
            language.filter(|language| self.policy.language_overrides.contains_key(*language))
        {
            format!("language override for `{language}`")
        } else {
            "default tokenizer policy".to_string()
        };
        TokenizerSelection {
            mode: self.policy.mode,
            source,
            language: language.map(ToString::to_string),
            task: task.map(ToString::to_string),
            model_family: model_family.map(ToString::to_string),
            reason,
        }
    }

    /// Returns align.
    pub fn align(
        &self,
        text: &str,
        tokens: &[Token],
        selection: &TokenizerSelection,
    ) -> Result<Option<TokenAlignmentMap>> {
        let Some(source) = selection.source.clone() else {
            return Ok(None);
        };
        let bundle = TokenizerBundle::from_cached_source(source)?;
        let tokenized = bundle.tokenize(text)?;
        Ok(Some(align_tokenized_text(
            text,
            tokens,
            selection.clone(),
            &tokenized,
        )?))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Data type for subword span.
pub struct SubwordSpan {
    /// The index value.
    pub index: usize,
    /// The input identifier value.
    pub input_id: i64,
    /// The span value.
    pub span: Option<TextSpan>,
    /// Text content for this value.
    pub text: Option<String>,
    /// The token type identifier value.
    pub token_type_id: Option<i64>,
    /// The attention value.
    pub attention: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Data type for aligned token.
pub struct AlignedToken {
    /// The token index value.
    pub token_index: usize,
    /// The token value.
    pub token: Token,
    /// The subword indices value.
    pub subword_indices: Vec<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Data type for token alignment map.
pub struct TokenAlignmentMap {
    /// The selection value.
    pub selection: TokenizerSelection,
    /// The subwords value.
    pub subwords: Vec<SubwordSpan>,
    /// The aligned tokens value.
    pub aligned_tokens: Vec<AlignedToken>,
}

/// Returns align tokenized text.
pub fn align_tokenized_text(
    text: &str,
    tokens: &[Token],
    selection: TokenizerSelection,
    tokenized: &TokenizedText,
) -> Result<TokenAlignmentMap> {
    let subwords = tokenized
        .input_ids
        .iter()
        .enumerate()
        .map(|(index, input_id)| {
            let span = tokenized
                .offsets
                .get(index)
                .copied()
                .flatten()
                .and_then(|(start, end)| {
                    if start >= end {
                        None
                    } else {
                        byte_span_to_text_span(text, start, end)
                    }
                });
            let subword_text = span
                .map(|span| text[span.byte_start..span.byte_end].to_string())
                .filter(|value| !value.is_empty());
            SubwordSpan {
                index,
                input_id: *input_id,
                span,
                text: subword_text,
                token_type_id: tokenized
                    .token_type_ids
                    .as_ref()
                    .and_then(|values| values.get(index).copied()),
                attention: tokenized
                    .attention_mask
                    .get(index)
                    .copied()
                    .unwrap_or_default()
                    != 0,
            }
        })
        .collect::<Vec<_>>();

    let aligned_tokens = tokens
        .iter()
        .cloned()
        .enumerate()
        .map(|(token_index, token)| {
            let subword_indices = subwords
                .iter()
                .filter_map(|subword| {
                    let span = subword.span?;
                    spans_overlap(token.span, span).then_some(subword.index)
                })
                .collect();
            AlignedToken {
                token_index,
                token,
                subword_indices,
            }
        })
        .collect();

    Ok(TokenAlignmentMap {
        selection,
        subwords,
        aligned_tokens,
    })
}

fn spans_overlap(left: TextSpan, right: TextSpan) -> bool {
    left.byte_start < right.byte_end && right.byte_start < left.byte_end
}

fn byte_span_to_text_span(text: &str, byte_start: usize, byte_end: usize) -> Option<TextSpan> {
    if byte_start > byte_end || byte_end > text.len() {
        return None;
    }
    let char_start = text[..byte_start].chars().count();
    let char_end = text[..byte_end].chars().count();
    Some(TextSpan {
        byte_start,
        byte_end,
        char_start,
        char_end,
    })
}

fn invalid_argument(message: impl Into<String>) -> DetectError {
    DetectError::InvalidArgument(message.into())
}
