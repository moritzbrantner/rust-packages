#![doc = include_str!("../README.md")]

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use text_core::tokenize_words;
use text_lexical::{
    english_stop_words, extractive_summary, sentiment as lexical_sentiment,
    ExtractiveSummaryOptions, SentimentLexicon,
};
use video_analysis_core::{DetectError, Result};
use video_analysis_models::ModelPreset;

/// NLP task families exposed by the shared model layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NlpTask {
    /// Token classification, including named entity recognition.
    TokenClassification,
    /// Single-text classification.
    TextClassification,
    /// Sentiment analysis.
    Sentiment,
    /// Dense text embedding.
    Embedding,
    /// Zero-shot label classification.
    ZeroShotClassification,
    /// Text summarization.
    Summarization,
    /// Query/document reranking.
    Reranking,
    /// Extractive question answering.
    QuestionAnswering,
}

impl NlpTask {
    /// Returns all task variants.
    pub const ALL: &'static [Self] = &[
        Self::TokenClassification,
        Self::TextClassification,
        Self::Sentiment,
        Self::Embedding,
        Self::ZeroShotClassification,
        Self::Summarization,
        Self::Reranking,
        Self::QuestionAnswering,
    ];

    /// Returns the stable API path segment for this task.
    pub fn path_segment(self) -> &'static str {
        match self {
            Self::TokenClassification => "entities",
            Self::TextClassification => "classify",
            Self::Sentiment => "sentiment",
            Self::Embedding => "embed",
            Self::ZeroShotClassification => "zero-shot",
            Self::Summarization => "summarize",
            Self::Reranking => "rerank",
            Self::QuestionAnswering => "question-answer",
        }
    }
}

/// Runtime families for NLP execution and postprocessing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NlpRuntime {
    /// Candle-backed native inference.
    Candle,
    /// ONNX Runtime-backed native inference.
    Onnx,
    /// Browser-safe WASM postprocessing.
    WasmPostprocess,
    /// Deterministic lexical fallback.
    Lexical,
    /// Caller-supplied model predictions.
    ImportedPredictions,
}

/// Fallback behavior when the selected native model cannot run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FallbackPolicy {
    /// Return a typed error.
    Error,
    /// Use a fast deterministic fallback.
    FastFallback,
    /// Use a lexical fallback.
    LexicalFallback,
}

impl Default for FallbackPolicy {
    fn default() -> Self {
        Self::Error
    }
}

/// Model selection supplied by API, CLI, or UI callers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ModelSelection {
    /// Optional preset or Hugging Face model identifier.
    #[serde(default)]
    pub model_id: Option<String>,
    /// Optional preferred runtime.
    #[serde(default)]
    pub runtime: Option<NlpRuntime>,
    /// Fallback policy for unsupported native paths.
    #[serde(default)]
    pub fallback_policy: FallbackPolicy,
}

/// Shared model metadata for UI and CLI discovery.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NlpModelMetadata {
    /// Stable local preset id.
    pub id: String,
    /// Upstream model id or strategy id.
    pub model_id: String,
    /// Task family.
    pub task: NlpTask,
    /// Preferred runtime.
    pub runtime: NlpRuntime,
    /// Whether the runtime is available in the default contributor build.
    pub supported: bool,
    /// Optional fallback preset id.
    pub fallback: Option<String>,
    /// Human-readable note for unsupported or fallback-only paths.
    pub note: Option<String>,
}

/// Text span used by NLP responses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextSpanRef {
    /// Byte start offset.
    pub byte_start: usize,
    /// Byte end offset.
    pub byte_end: usize,
    /// Character start offset.
    pub char_start: usize,
    /// Character end offset.
    pub char_end: usize,
}

/// Imported prediction used by server, CLI, and WASM postprocessing.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportedPrediction {
    /// Optional raw prediction kind.
    #[serde(default)]
    pub kind: Option<String>,
    /// Label emitted by the model.
    pub label: String,
    /// Optional text span or document text.
    #[serde(default)]
    pub text: Option<String>,
    /// Confidence score.
    pub score: f32,
    /// Arbitrary model attributes, including offsets.
    #[serde(default)]
    pub attributes: BTreeMap<String, String>,
}

/// One label prediction.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextClassPrediction {
    /// Label name.
    pub label: String,
    /// Confidence score.
    pub score: f32,
}

/// Request for single-text classification.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextClassificationRequest {
    /// Input text.
    pub text: String,
    /// Optional fixed label set.
    #[serde(default)]
    pub labels: Vec<String>,
    /// Maximum prediction count.
    #[serde(default = "default_top_k")]
    pub top_k: usize,
    /// Whether labels are independent.
    #[serde(default)]
    pub multi_label: bool,
    /// Model selection.
    #[serde(default)]
    pub model: ModelSelection,
    /// Caller-supplied model predictions.
    #[serde(default)]
    pub imported_predictions: Vec<ImportedPrediction>,
}

/// Response for single-text classification.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextClassificationResponse {
    /// Accepted flag for generated package surfaces.
    pub accepted: bool,
    /// Operation name.
    pub operation: String,
    /// Input text.
    pub text: String,
    /// Selected model id.
    pub model_id: String,
    /// Runtime used.
    pub runtime: NlpRuntime,
    /// Ranked label predictions.
    pub predictions: Vec<TextClassPrediction>,
}

/// Request for sentiment analysis.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SentimentRequest {
    /// Input text.
    pub text: String,
    /// Model selection.
    #[serde(default)]
    pub model: ModelSelection,
    /// Caller-supplied model predictions.
    #[serde(default)]
    pub imported_predictions: Vec<ImportedPrediction>,
}

/// Response for sentiment analysis.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SentimentResponse {
    /// Accepted flag for generated package surfaces.
    pub accepted: bool,
    /// Operation name.
    pub operation: String,
    /// Input text.
    pub text: String,
    /// Selected model id.
    pub model_id: String,
    /// Runtime used.
    pub runtime: NlpRuntime,
    /// Winning label.
    pub label: String,
    /// Positive score.
    pub positive_score: f32,
    /// Negative score.
    pub negative_score: f32,
    /// Compound score.
    pub compound: f32,
    /// Ranked predictions.
    pub predictions: Vec<TextClassPrediction>,
}

/// Request for embeddings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EmbeddingRequest {
    /// Input texts.
    pub texts: Vec<String>,
    /// Model selection.
    #[serde(default)]
    pub model: ModelSelection,
    /// Vector dimensions for fallback embeddings.
    #[serde(default = "default_embedding_dimensions")]
    pub dimensions: usize,
    /// Whether to L2-normalize vectors.
    #[serde(default = "default_true")]
    pub normalize: bool,
    /// Imported vectors for WASM/client postprocessing.
    #[serde(default)]
    pub imported_embeddings: Vec<Vec<f32>>,
}

/// Response for embeddings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EmbeddingResponse {
    /// Accepted flag for generated package surfaces.
    pub accepted: bool,
    /// Operation name.
    pub operation: String,
    /// Selected model id.
    pub model_id: String,
    /// Runtime used.
    pub runtime: NlpRuntime,
    /// Vector dimensions.
    pub dimensions: usize,
    /// Dense vectors.
    pub embeddings: Vec<Vec<f32>>,
}

/// Request for zero-shot classification.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ZeroShotClassificationRequest {
    /// Input text.
    pub text: String,
    /// Candidate labels.
    pub labels: Vec<String>,
    /// Hypothesis template.
    #[serde(default = "default_hypothesis_template")]
    pub hypothesis_template: String,
    /// Model selection.
    #[serde(default)]
    pub model: ModelSelection,
    /// Caller-supplied model predictions.
    #[serde(default)]
    pub imported_predictions: Vec<ImportedPrediction>,
}

/// Response for zero-shot classification.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ZeroShotClassificationResponse {
    /// Accepted flag for generated package surfaces.
    pub accepted: bool,
    /// Operation name.
    pub operation: String,
    /// Input text.
    pub text: String,
    /// Selected model id.
    pub model_id: String,
    /// Runtime used.
    pub runtime: NlpRuntime,
    /// Ranked labels.
    pub predictions: Vec<TextClassPrediction>,
    /// Constructed hypotheses.
    pub hypotheses: Vec<String>,
}

/// Summary strategy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SummaryStrategy {
    /// Lexical extractive summary.
    LexicalExtractive,
    /// Embedding-assisted extractive summary.
    EmbeddingExtractive,
}

impl Default for SummaryStrategy {
    fn default() -> Self {
        Self::EmbeddingExtractive
    }
}

/// Request for summarization.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SummaryRequest {
    /// Input text.
    pub text: String,
    /// Maximum sentence count.
    #[serde(default = "default_summary_sentences")]
    pub max_sentences: usize,
    /// Summary strategy.
    #[serde(default)]
    pub strategy: SummaryStrategy,
    /// Model selection.
    #[serde(default)]
    pub model: ModelSelection,
    /// Imported sentence embeddings for client postprocessing.
    #[serde(default)]
    pub imported_sentence_embeddings: Vec<Vec<f32>>,
}

/// One summary sentence.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SummarySentencePrediction {
    /// Original sentence index.
    pub index: usize,
    /// Sentence text.
    pub text: String,
    /// Sentence span.
    pub span: TextSpanRef,
    /// Sentence score.
    pub score: f32,
}

/// Response for summarization.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SummaryResponse {
    /// Accepted flag for generated package surfaces.
    pub accepted: bool,
    /// Operation name.
    pub operation: String,
    /// Selected model id.
    pub model_id: String,
    /// Runtime used.
    pub runtime: NlpRuntime,
    /// Strategy used.
    pub strategy: SummaryStrategy,
    /// Combined summary text.
    pub summary: String,
    /// Selected sentences.
    pub sentences: Vec<SummarySentencePrediction>,
}

/// Request for reranking documents against a query.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RerankRequest {
    /// Query text.
    pub query: String,
    /// Documents to rerank.
    pub documents: Vec<String>,
    /// Maximum output count.
    #[serde(default = "default_top_k")]
    pub top_k: usize,
    /// Model selection.
    #[serde(default)]
    pub model: ModelSelection,
    /// Imported document scores.
    #[serde(default)]
    pub imported_scores: Vec<f32>,
}

/// One reranked document.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RerankResult {
    /// Original document index.
    pub index: usize,
    /// Document text.
    pub document: String,
    /// Relevance score.
    pub score: f32,
}

/// Response for reranking.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RerankResponse {
    /// Accepted flag for generated package surfaces.
    pub accepted: bool,
    /// Operation name.
    pub operation: String,
    /// Query text.
    pub query: String,
    /// Selected model id.
    pub model_id: String,
    /// Runtime used.
    pub runtime: NlpRuntime,
    /// Ranked documents.
    pub results: Vec<RerankResult>,
}

/// Request for question answering.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuestionAnsweringRequest {
    /// Question text.
    pub question: String,
    /// Context text.
    pub context: String,
    /// Maximum answer count.
    #[serde(default = "default_top_k")]
    pub top_k: usize,
    /// Model selection.
    #[serde(default)]
    pub model: ModelSelection,
    /// Imported span predictions.
    #[serde(default)]
    pub imported_predictions: Vec<ImportedPrediction>,
}

/// One QA answer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnswerPrediction {
    /// Answer text.
    pub answer: String,
    /// Confidence score.
    pub score: f32,
    /// Optional context span.
    pub span: Option<TextSpanRef>,
}

/// Response for question answering.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuestionAnsweringResponse {
    /// Accepted flag for generated package surfaces.
    pub accepted: bool,
    /// Operation name.
    pub operation: String,
    /// Question text.
    pub question: String,
    /// Selected model id.
    pub model_id: String,
    /// Runtime used.
    pub runtime: NlpRuntime,
    /// Candidate answers.
    pub answers: Vec<AnswerPrediction>,
}

/// Returns the shared model catalog, optionally filtered by task.
pub fn model_catalog(task: Option<NlpTask>) -> Vec<NlpModelMetadata> {
    let models = vec![
        metadata(
            "bert-base-ner",
            "dslim/bert-base-NER",
            NlpTask::TokenClassification,
            NlpRuntime::Candle,
            true,
            None,
            None,
        ),
        metadata(
            "twitter-roberta-sentiment-latest",
            "cardiffnlp/twitter-roberta-base-sentiment-latest",
            NlpTask::Sentiment,
            NlpRuntime::Onnx,
            false,
            Some("distilbert-sst2"),
            Some("ONNX preset metadata is registered; native runner support is explicit/fallback-gated."),
        ),
        metadata(
            "distilbert-sst2",
            "distilbert-base-uncased-finetuned-sst-2-english",
            NlpTask::TextClassification,
            NlpRuntime::Candle,
            false,
            None,
            Some("Sequence-classification native execution is not enabled in this shared fallback crate."),
        ),
        metadata(
            "all-mpnet-base-v2",
            "sentence-transformers/all-mpnet-base-v2",
            NlpTask::Embedding,
            NlpRuntime::Onnx,
            false,
            Some("minilm-l6-v2"),
            Some("Use imported embeddings or explicit fallback until ONNX embedding execution is wired."),
        ),
        metadata(
            "minilm-l6-v2",
            "sentence-transformers/all-MiniLM-L6-v2",
            NlpTask::Embedding,
            NlpRuntime::Candle,
            false,
            None,
            Some("Existing preset remains available; deterministic embedding fallback is explicit."),
        ),
        metadata(
            "bart-large-mnli",
            "facebook/bart-large-mnli",
            NlpTask::ZeroShotClassification,
            NlpRuntime::Onnx,
            false,
            None,
            Some("Use imported predictions or explicit lexical fallback until ONNX pair classification is wired."),
        ),
        metadata(
            "embedding-extractive-summary",
            "embedding_extractive",
            NlpTask::Summarization,
            NlpRuntime::Lexical,
            true,
            None,
            Some("Uses lexical extraction locally and accepts imported embeddings for browser/server parity."),
        ),
        metadata(
            "bart-large-cnn",
            "facebook/bart-large-cnn",
            NlpTask::Summarization,
            NlpRuntime::Onnx,
            false,
            Some("embedding-extractive-summary"),
            Some("Registered for discovery; abstractive generation is not implemented in the first pass."),
        ),
        metadata(
            "ms-marco-minilm-l6-v2",
            "cross-encoder/ms-marco-MiniLM-L-6-v2",
            NlpTask::Reranking,
            NlpRuntime::Onnx,
            false,
            None,
            Some("Use imported scores or explicit lexical fallback until ONNX pair classification is wired."),
        ),
        metadata(
            "roberta-base-squad2",
            "deepset/roberta-base-squad2",
            NlpTask::QuestionAnswering,
            NlpRuntime::Onnx,
            false,
            None,
            Some("Schema and imported span support are available; native span extraction is not wired."),
        ),
    ];

    models
        .into_iter()
        .filter(|model| task.map(|task| task == model.task).unwrap_or(true))
        .collect()
}

/// Runs classification from imported predictions or an explicit fallback.
pub fn classify_text(request: TextClassificationRequest) -> Result<TextClassificationResponse> {
    ensure_non_empty(&request.text, "text")?;
    let model_id = request
        .model
        .model_id
        .clone()
        .unwrap_or_else(|| "distilbert-sst2".to_string());

    if !request.imported_predictions.is_empty() {
        return Ok(TextClassificationResponse {
            accepted: true,
            operation: "classify".to_string(),
            text: request.text,
            model_id,
            runtime: NlpRuntime::ImportedPredictions,
            predictions: normalize_predictions(request.imported_predictions, request.top_k),
        });
    }

    match request.model.fallback_policy {
        FallbackPolicy::FastFallback | FallbackPolicy::LexicalFallback => {
            let predictions = lexical_label_scores(&request.text, &request.labels, request.top_k);
            Ok(TextClassificationResponse {
                accepted: true,
                operation: "classify".to_string(),
                text: request.text,
                model_id,
                runtime: NlpRuntime::Lexical,
                predictions,
            })
        }
        FallbackPolicy::Error => unsupported_runtime(
            "native text classification requires imported predictions or an explicit fallback policy",
        ),
    }
}

/// Runs sentiment from imported predictions or lexical fallback.
pub fn analyze_sentiment(request: SentimentRequest) -> Result<SentimentResponse> {
    ensure_non_empty(&request.text, "text")?;
    let model_id = request
        .model
        .model_id
        .clone()
        .unwrap_or_else(|| "twitter-roberta-sentiment-latest".to_string());

    if !request.imported_predictions.is_empty() {
        let predictions = normalize_predictions(request.imported_predictions, 3);
        let label = predictions
            .first()
            .map(|prediction| prediction.label.clone())
            .unwrap_or_else(|| "neutral".to_string());
        let positive_score = score_for_label(&predictions, &["positive", "label_2"]);
        let negative_score = score_for_label(&predictions, &["negative", "label_0"]);
        return Ok(SentimentResponse {
            accepted: true,
            operation: "sentiment".to_string(),
            text: request.text,
            model_id,
            runtime: NlpRuntime::ImportedPredictions,
            label,
            positive_score,
            negative_score,
            compound: positive_score - negative_score,
            predictions,
        });
    }

    match request.model.fallback_policy {
        FallbackPolicy::FastFallback | FallbackPolicy::LexicalFallback => {
            let summary = lexical_sentiment(&request.text, &SentimentLexicon::default());
            let predictions = sentiment_predictions(&summary.label, summary.compound);
            Ok(SentimentResponse {
                accepted: true,
                operation: "sentiment".to_string(),
                text: request.text,
                model_id,
                runtime: NlpRuntime::Lexical,
                label: summary.label,
                positive_score: summary.positive_score,
                negative_score: summary.negative_score,
                compound: summary.compound,
                predictions,
            })
        }
        FallbackPolicy::Error => unsupported_runtime(
            "native sentiment requires imported predictions or an explicit fallback policy",
        ),
    }
}

/// Creates imported or deterministic fallback embeddings.
pub fn embed_texts(request: EmbeddingRequest) -> Result<EmbeddingResponse> {
    if request.texts.is_empty() || request.texts.iter().all(|text| text.trim().is_empty()) {
        return Err(DetectError::InvalidArgument(
            "request body must include at least one non-empty text".to_string(),
        ));
    }
    let model_id = request
        .model
        .model_id
        .clone()
        .unwrap_or_else(|| "all-mpnet-base-v2".to_string());

    if !request.imported_embeddings.is_empty() {
        let dimensions = request
            .imported_embeddings
            .first()
            .map(Vec::len)
            .unwrap_or_default();
        return Ok(EmbeddingResponse {
            accepted: true,
            operation: "embed".to_string(),
            model_id,
            runtime: NlpRuntime::ImportedPredictions,
            dimensions,
            embeddings: request.imported_embeddings,
        });
    }

    match request.model.fallback_policy {
        FallbackPolicy::FastFallback | FallbackPolicy::LexicalFallback => {
            let dimensions = request.dimensions.max(1);
            let embeddings = request
                .texts
                .iter()
                .map(|text| hashed_embedding(text, dimensions, request.normalize))
                .collect::<Vec<_>>();
            Ok(EmbeddingResponse {
                accepted: true,
                operation: "embed".to_string(),
                model_id,
                runtime: NlpRuntime::Lexical,
                dimensions,
                embeddings,
            })
        }
        FallbackPolicy::Error => unsupported_runtime(
            "native embedding requires imported embeddings or an explicit fallback policy",
        ),
    }
}

/// Runs zero-shot classification from imported predictions or lexical label overlap.
pub fn zero_shot_classify(
    request: ZeroShotClassificationRequest,
) -> Result<ZeroShotClassificationResponse> {
    ensure_non_empty(&request.text, "text")?;
    if request.labels.is_empty() {
        return Err(DetectError::InvalidArgument(
            "zero-shot request must include at least one label".to_string(),
        ));
    }
    let model_id = request
        .model
        .model_id
        .clone()
        .unwrap_or_else(|| "bart-large-mnli".to_string());
    let hypotheses = request
        .labels
        .iter()
        .map(|label| request.hypothesis_template.replace("{}", label))
        .collect::<Vec<_>>();

    if !request.imported_predictions.is_empty() {
        return Ok(ZeroShotClassificationResponse {
            accepted: true,
            operation: "zero-shot".to_string(),
            text: request.text,
            model_id,
            runtime: NlpRuntime::ImportedPredictions,
            predictions: normalize_predictions(request.imported_predictions, request.labels.len()),
            hypotheses,
        });
    }

    match request.model.fallback_policy {
        FallbackPolicy::FastFallback | FallbackPolicy::LexicalFallback => {
            let mut predictions = lexical_label_scores(&request.text, &request.labels, request.labels.len());
            normalize_prediction_scores(&mut predictions);
            Ok(ZeroShotClassificationResponse {
                accepted: true,
                operation: "zero-shot".to_string(),
                text: request.text,
                model_id,
                runtime: NlpRuntime::Lexical,
                predictions,
                hypotheses,
            })
        }
        FallbackPolicy::Error => unsupported_runtime(
            "native zero-shot classification requires imported predictions or an explicit fallback policy",
        ),
    }
}

/// Runs extractive summarization.
pub fn summarize(request: SummaryRequest) -> Result<SummaryResponse> {
    ensure_non_empty(&request.text, "text")?;
    let model_id = request
        .model
        .model_id
        .clone()
        .unwrap_or_else(|| "embedding-extractive-summary".to_string());
    let options = ExtractiveSummaryOptions {
        max_sentences: request.max_sentences.max(1),
        min_sentence_words: 3,
        stop_words: english_stop_words(),
    };

    let mut sentences = extractive_summary(&request.text, &options)?
        .into_iter()
        .map(|sentence| SummarySentencePrediction {
            index: sentence.index,
            text: sentence.text,
            span: TextSpanRef {
                byte_start: sentence.span.byte_start,
                byte_end: sentence.span.byte_end,
                char_start: sentence.span.char_start,
                char_end: sentence.span.char_end,
            },
            score: sentence.score,
        })
        .collect::<Vec<_>>();

    if request.strategy == SummaryStrategy::EmbeddingExtractive
        && request.imported_sentence_embeddings.len() == sentences.len()
    {
        let centroid = centroid(&request.imported_sentence_embeddings);
        for (sentence, embedding) in sentences
            .iter_mut()
            .zip(request.imported_sentence_embeddings.iter())
        {
            sentence.score += cosine(embedding, &centroid);
        }
        sentences.sort_by(|left, right| {
            right
                .score
                .total_cmp(&left.score)
                .then_with(|| left.index.cmp(&right.index))
        });
        sentences.truncate(request.max_sentences.max(1));
        sentences.sort_by_key(|sentence| sentence.index);
    }

    let summary = sentences
        .iter()
        .map(|sentence| sentence.text.as_str())
        .collect::<Vec<_>>()
        .join(" ");

    Ok(SummaryResponse {
        accepted: true,
        operation: "summarize".to_string(),
        model_id,
        runtime: NlpRuntime::Lexical,
        strategy: request.strategy,
        summary,
        sentences,
    })
}

/// Reranks documents from imported scores or lexical overlap fallback.
pub fn rerank(request: RerankRequest) -> Result<RerankResponse> {
    ensure_non_empty(&request.query, "query")?;
    if request.documents.is_empty() {
        return Err(DetectError::InvalidArgument(
            "rerank request must include at least one document".to_string(),
        ));
    }
    let model_id = request
        .model
        .model_id
        .clone()
        .unwrap_or_else(|| "ms-marco-minilm-l6-v2".to_string());

    let runtime = if !request.imported_scores.is_empty() {
        NlpRuntime::ImportedPredictions
    } else {
        match request.model.fallback_policy {
            FallbackPolicy::FastFallback | FallbackPolicy::LexicalFallback => NlpRuntime::Lexical,
            FallbackPolicy::Error => {
                return unsupported_runtime(
                    "native reranking requires imported scores or an explicit fallback policy",
                )
            }
        }
    };

    let mut results = request
        .documents
        .iter()
        .enumerate()
        .map(|(index, document)| {
            let score = request
                .imported_scores
                .get(index)
                .copied()
                .unwrap_or_else(|| lexical_overlap_score(&request.query, document));
            RerankResult {
                index,
                document: document.clone(),
                score,
            }
        })
        .collect::<Vec<_>>();
    results.sort_by(|left, right| {
        right
            .score
            .total_cmp(&left.score)
            .then_with(|| left.index.cmp(&right.index))
    });
    results.truncate(request.top_k.max(1));

    Ok(RerankResponse {
        accepted: true,
        operation: "rerank".to_string(),
        query: request.query,
        model_id,
        runtime,
        results,
    })
}

/// Handles QA from imported predictions or returns a typed unsupported-runtime error.
pub fn answer_question(request: QuestionAnsweringRequest) -> Result<QuestionAnsweringResponse> {
    ensure_non_empty(&request.question, "question")?;
    ensure_non_empty(&request.context, "context")?;
    let model_id = request
        .model
        .model_id
        .clone()
        .unwrap_or_else(|| "roberta-base-squad2".to_string());

    if !request.imported_predictions.is_empty() {
        let mut answers = request
            .imported_predictions
            .into_iter()
            .filter_map(|prediction| {
                let answer = prediction.text?;
                Some(AnswerPrediction {
                    answer,
                    score: prediction.score,
                    span: span_from_attributes(&prediction.attributes),
                })
            })
            .collect::<Vec<_>>();
        answers.sort_by(|left, right| right.score.total_cmp(&left.score));
        answers.truncate(request.top_k.max(1));
        return Ok(QuestionAnsweringResponse {
            accepted: true,
            operation: "question-answer".to_string(),
            question: request.question,
            model_id,
            runtime: NlpRuntime::ImportedPredictions,
            answers,
        });
    }

    unsupported_runtime(
        "native question answering requires imported span predictions until ONNX QA span extraction is wired",
    )
}

/// Returns preset ids registered in video-analysis-models for NLP tasks.
pub fn registered_nlp_presets() -> Vec<String> {
    ModelPreset::ALL
        .iter()
        .map(|preset| preset.as_str().to_string())
        .filter(|preset| {
            preset.contains("bert")
                || preset.contains("ner")
                || preset.contains("minilm")
                || preset.contains("mpnet")
                || preset.contains("bart")
                || preset.contains("roberta")
                || preset.contains("marco")
        })
        .collect()
}

fn metadata(
    id: &str,
    model_id: &str,
    task: NlpTask,
    runtime: NlpRuntime,
    supported: bool,
    fallback: Option<&str>,
    note: Option<&str>,
) -> NlpModelMetadata {
    NlpModelMetadata {
        id: id.to_string(),
        model_id: model_id.to_string(),
        task,
        runtime,
        supported,
        fallback: fallback.map(str::to_string),
        note: note.map(str::to_string),
    }
}

fn default_top_k() -> usize {
    3
}

fn default_embedding_dimensions() -> usize {
    384
}

fn default_summary_sentences() -> usize {
    3
}

fn default_true() -> bool {
    true
}

fn default_hypothesis_template() -> String {
    "This example is about {}.".to_string()
}

fn ensure_non_empty(value: &str, name: &str) -> Result<()> {
    if value.trim().is_empty() {
        return Err(DetectError::InvalidArgument(format!(
            "request body must include a non-empty `{name}` string"
        )));
    }
    Ok(())
}

fn unsupported_runtime<T>(message: &str) -> Result<T> {
    Err(DetectError::InvalidArgument(format!(
        "unsupported_runtime: {message}"
    )))
}

fn normalize_predictions(
    predictions: Vec<ImportedPrediction>,
    top_k: usize,
) -> Vec<TextClassPrediction> {
    let mut predictions = predictions
        .into_iter()
        .map(|prediction| TextClassPrediction {
            label: prediction.label,
            score: prediction.score,
        })
        .collect::<Vec<_>>();
    predictions.sort_by(|left, right| right.score.total_cmp(&left.score));
    predictions.truncate(top_k.max(1));
    predictions
}

fn normalize_prediction_scores(predictions: &mut [TextClassPrediction]) {
    let total = predictions
        .iter()
        .map(|prediction| prediction.score)
        .sum::<f32>();
    if total > f32::EPSILON {
        for prediction in predictions {
            prediction.score /= total;
        }
    }
}

fn lexical_label_scores(text: &str, labels: &[String], top_k: usize) -> Vec<TextClassPrediction> {
    let labels = if labels.is_empty() {
        vec![
            "positive".to_string(),
            "negative".to_string(),
            "neutral".to_string(),
        ]
    } else {
        labels.to_vec()
    };
    let text_terms = tokenize_words(text).into_iter().collect::<BTreeSet<_>>();
    let mut predictions = labels
        .into_iter()
        .map(|label| {
            let label_terms = tokenize_words(&label);
            let overlap = label_terms
                .iter()
                .filter(|term| text_terms.contains(term.as_str()))
                .count();
            let score = if label_terms.is_empty() {
                0.0
            } else {
                overlap as f32 / label_terms.len() as f32
            };
            TextClassPrediction { label, score }
        })
        .collect::<Vec<_>>();
    if predictions
        .iter()
        .all(|prediction| prediction.score <= f32::EPSILON)
    {
        let sentiment = lexical_sentiment(text, &SentimentLexicon::default());
        let prediction_count = predictions.len().max(1) as f32;
        for prediction in &mut predictions {
            prediction.score = match prediction.label.to_ascii_lowercase().as_str() {
                "positive" => sentiment.positive_score.max(0.0),
                "negative" => sentiment.negative_score.max(0.0),
                "neutral" => 1.0 - sentiment.compound.abs().min(1.0),
                _ => 1.0 / prediction_count,
            };
        }
    }
    predictions.sort_by(|left, right| {
        right
            .score
            .total_cmp(&left.score)
            .then_with(|| left.label.cmp(&right.label))
    });
    predictions.truncate(top_k.max(1));
    predictions
}

fn sentiment_predictions(label: &str, compound: f32) -> Vec<TextClassPrediction> {
    let positive = compound.max(0.0).min(1.0);
    let negative = (-compound).max(0.0).min(1.0);
    let neutral = (1.0 - compound.abs()).max(0.0);
    let mut predictions = vec![
        TextClassPrediction {
            label: "positive".to_string(),
            score: positive,
        },
        TextClassPrediction {
            label: "negative".to_string(),
            score: negative,
        },
        TextClassPrediction {
            label: "neutral".to_string(),
            score: neutral,
        },
    ];
    for prediction in &mut predictions {
        if prediction.label == label {
            prediction.score = prediction.score.max(0.5);
        }
    }
    predictions.sort_by(|left, right| right.score.total_cmp(&left.score));
    predictions
}

fn score_for_label(predictions: &[TextClassPrediction], labels: &[&str]) -> f32 {
    predictions
        .iter()
        .find(|prediction| {
            labels
                .iter()
                .any(|label| prediction.label.eq_ignore_ascii_case(label))
        })
        .map(|prediction| prediction.score)
        .unwrap_or(0.0)
}

fn hashed_embedding(text: &str, dimensions: usize, normalize: bool) -> Vec<f32> {
    let mut vector = vec![0.0; dimensions];
    for term in tokenize_words(text) {
        let mut hash = 1469598103934665603u64;
        for byte in term.as_bytes() {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(1099511628211);
        }
        let index = (hash as usize) % dimensions;
        let sign = if (hash >> 63) == 0 { 1.0 } else { -1.0 };
        vector[index] += sign;
    }
    if normalize {
        normalize_vector(&mut vector);
    }
    vector
}

fn normalize_vector(vector: &mut [f32]) {
    let norm = vector.iter().map(|value| value * value).sum::<f32>().sqrt();
    if norm > f32::EPSILON {
        for value in vector {
            *value /= norm;
        }
    }
}

fn centroid(vectors: &[Vec<f32>]) -> Vec<f32> {
    let dimensions = vectors.first().map(Vec::len).unwrap_or_default();
    let mut centroid = vec![0.0; dimensions];
    if dimensions == 0 {
        return centroid;
    }
    for vector in vectors {
        for (index, value) in vector.iter().take(dimensions).enumerate() {
            centroid[index] += *value;
        }
    }
    for value in &mut centroid {
        *value /= vectors.len().max(1) as f32;
    }
    normalize_vector(&mut centroid);
    centroid
}

fn cosine(left: &[f32], right: &[f32]) -> f32 {
    let mut dot = 0.0;
    let mut left_norm = 0.0;
    let mut right_norm = 0.0;
    for (left, right) in left.iter().zip(right.iter()) {
        dot += left * right;
        left_norm += left * left;
        right_norm += right * right;
    }
    if left_norm <= f32::EPSILON || right_norm <= f32::EPSILON {
        0.0
    } else {
        dot / (left_norm.sqrt() * right_norm.sqrt())
    }
}

fn lexical_overlap_score(query: &str, document: &str) -> f32 {
    let query_terms = tokenize_words(query).into_iter().collect::<BTreeSet<_>>();
    if query_terms.is_empty() {
        return 0.0;
    }
    let document_terms = tokenize_words(document)
        .into_iter()
        .collect::<BTreeSet<_>>();
    let overlap = query_terms
        .iter()
        .filter(|term| document_terms.contains(term.as_str()))
        .count();
    overlap as f32 / query_terms.len() as f32
}

fn span_from_attributes(attributes: &BTreeMap<String, String>) -> Option<TextSpanRef> {
    let byte_start = attributes.get("byte_start")?.parse().ok()?;
    let byte_end = attributes.get("byte_end")?.parse().ok()?;
    let char_start = attributes
        .get("char_start")
        .and_then(|value| value.parse().ok())
        .unwrap_or(byte_start);
    let char_end = attributes
        .get("char_end")
        .and_then(|value| value.parse().ok())
        .unwrap_or(byte_end);
    Some(TextSpanRef {
        byte_start,
        byte_end,
        char_start,
        char_end,
    })
}

/// Parses a task name from API path or CLI input.
pub fn parse_task(value: &str) -> Option<NlpTask> {
    NlpTask::ALL.iter().copied().find(|task| {
        task.path_segment() == value || format!("{task:?}").eq_ignore_ascii_case(value)
    })
}

/// Returns a JSON value for the shared schema catalog.
pub fn schema_summary() -> serde_json::Value {
    serde_json::json!({
        "tasks": NlpTask::ALL.iter().map(|task| serde_json::json!({
            "task": task,
            "path": format!("/api/{}", task.path_segment())
        })).collect::<Vec<_>>(),
        "models": model_catalog(None),
        "registeredPresets": registered_nlp_presets(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_includes_core_tasks() {
        let models = model_catalog(None);
        assert!(models.iter().any(|model| model.id == "bert-base-ner"));
        assert!(models.iter().any(|model| model.task == NlpTask::Embedding));
        assert!(models
            .iter()
            .any(|model| model.task == NlpTask::QuestionAnswering));
    }

    #[test]
    fn sentiment_requires_explicit_fallback_without_imports() {
        let response = analyze_sentiment(SentimentRequest {
            text: "excellent reliable work".to_string(),
            model: ModelSelection {
                fallback_policy: FallbackPolicy::LexicalFallback,
                ..ModelSelection::default()
            },
            imported_predictions: Vec::new(),
        })
        .unwrap();
        assert_eq!(response.runtime, NlpRuntime::Lexical);
        assert_eq!(response.label, "positive");
    }

    #[test]
    fn zero_shot_builds_hypotheses() {
        let response = zero_shot_classify(ZeroShotClassificationRequest {
            text: "The team shipped a Rust API".to_string(),
            labels: vec!["rust".to_string(), "music".to_string()],
            hypothesis_template: default_hypothesis_template(),
            model: ModelSelection {
                fallback_policy: FallbackPolicy::LexicalFallback,
                ..ModelSelection::default()
            },
            imported_predictions: Vec::new(),
        })
        .unwrap();
        assert_eq!(response.hypotheses[0], "This example is about rust.");
        assert_eq!(response.predictions[0].label, "rust");
    }

    #[test]
    fn fallback_embeddings_are_normalized() {
        let response = embed_texts(EmbeddingRequest {
            texts: vec!["rust text models".to_string()],
            model: ModelSelection {
                fallback_policy: FallbackPolicy::FastFallback,
                ..ModelSelection::default()
            },
            dimensions: 8,
            normalize: true,
            imported_embeddings: Vec::new(),
        })
        .unwrap();
        let norm = response.embeddings[0]
            .iter()
            .map(|value| value * value)
            .sum::<f32>()
            .sqrt();
        assert!((norm - 1.0).abs() < 0.001);
    }

    #[test]
    fn qa_uses_imported_spans() {
        let mut attributes = BTreeMap::new();
        attributes.insert("byte_start".to_string(), "0".to_string());
        attributes.insert("byte_end".to_string(), "5".to_string());
        let response = answer_question(QuestionAnsweringRequest {
            question: "Who?".to_string(),
            context: "Alice wrote it.".to_string(),
            top_k: 1,
            model: ModelSelection::default(),
            imported_predictions: vec![ImportedPrediction {
                kind: Some("span".to_string()),
                label: "answer".to_string(),
                text: Some("Alice".to_string()),
                score: 0.9,
                attributes,
            }],
        })
        .unwrap();
        assert_eq!(response.runtime, NlpRuntime::ImportedPredictions);
        assert_eq!(response.answers[0].answer, "Alice");
    }
}
