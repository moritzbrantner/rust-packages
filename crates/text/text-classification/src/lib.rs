#![doc = include_str!("../README.md")]

pub mod surface;

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use text_core::tokenize_words;
use text_lexical::{sentiment as lexical_sentiment, SentimentLexicon};
use text_model_runtime::{SequenceClassifier, TextRuntimeBackend};
use video_analysis_core::{DetectError, Result};

/// Text classification capability families exposed by this crate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TextClassificationTask {
    /// Single-text classification.
    TextClassification,
    /// Sentiment analysis.
    Sentiment,
    /// Zero-shot label classification.
    ZeroShotClassification,
}

impl TextClassificationTask {
    /// Returns all task variants.
    pub const ALL: &'static [Self] = &[
        Self::TextClassification,
        Self::Sentiment,
        Self::ZeroShotClassification,
    ];

    /// Returns the stable API path segment for this task.
    pub fn path_segment(self) -> &'static str {
        match self {
            Self::TextClassification => "classify",
            Self::Sentiment => "sentiment",
            Self::ZeroShotClassification => "zero-shot",
        }
    }
}

/// Runtime families for text classification execution and postprocessing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TextClassificationRuntime {
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum FallbackPolicy {
    /// Return a typed error.
    #[default]
    Error,
    /// Use a fast deterministic fallback.
    FastFallback,
    /// Use a lexical fallback.
    LexicalFallback,
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
    pub runtime: Option<TextClassificationRuntime>,
    /// Fallback policy for unsupported native paths.
    #[serde(default)]
    pub fallback_policy: FallbackPolicy,
}

/// Caller-supplied native or external runtime backends for classification.
#[derive(Default)]
pub struct TextClassificationExecutionContext<'a> {
    /// Optional text classifier.
    pub classifier: Option<&'a mut dyn SequenceClassifier>,
}

/// Model metadata for UI and CLI discovery.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextClassificationModelMetadata {
    /// Stable local preset id.
    pub id: String,
    /// Upstream model id or strategy id.
    pub model_id: String,
    /// Task family.
    pub task: TextClassificationTask,
    /// Preferred runtime.
    pub runtime: TextClassificationRuntime,
    /// Whether the runtime is available in the default contributor build.
    pub supported: bool,
    /// Optional fallback preset id.
    pub fallback: Option<String>,
    /// Human-readable note for unsupported or fallback-only paths.
    pub note: Option<String>,
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
    /// Arbitrary model attributes.
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
    pub runtime: TextClassificationRuntime,
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
    pub runtime: TextClassificationRuntime,
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
    pub runtime: TextClassificationRuntime,
    /// Ranked labels.
    pub predictions: Vec<TextClassPrediction>,
    /// Constructed hypotheses.
    pub hypotheses: Vec<String>,
}

/// Returns the model catalog, optionally filtered by classification task.
pub fn model_catalog(task: Option<TextClassificationTask>) -> Vec<TextClassificationModelMetadata> {
    let models = vec![
        metadata(
            "twitter-roberta-sentiment-latest",
            "cardiffnlp/twitter-roberta-base-sentiment-latest",
            TextClassificationTask::Sentiment,
            TextClassificationRuntime::Onnx,
            false,
            Some("distilbert-sst2"),
            Some("ONNX preset metadata is registered; native runner support is explicit/fallback-gated."),
        ),
        metadata(
            "distilbert-sst2",
            "distilbert-base-uncased-finetuned-sst-2-english",
            TextClassificationTask::TextClassification,
            TextClassificationRuntime::Candle,
            false,
            None,
            Some("Sequence-classification native execution is not enabled in this shared fallback crate."),
        ),
        metadata(
            "bart-large-mnli",
            "facebook/bart-large-mnli",
            TextClassificationTask::ZeroShotClassification,
            TextClassificationRuntime::Onnx,
            false,
            None,
            Some("Use imported predictions or explicit lexical fallback until ONNX pair classification is wired."),
        ),
    ];

    models
        .into_iter()
        .filter(|model| task.map(|task| task == model.task).unwrap_or(true))
        .collect()
}

/// Runs classification with an optional classifier backend.
pub fn classify_text_with_context(
    request: TextClassificationRequest,
    context: &mut TextClassificationExecutionContext<'_>,
) -> Result<TextClassificationResponse> {
    ensure_non_empty(&request.text, "text")?;
    if !request.imported_predictions.is_empty() {
        return classify_text(request);
    }
    let model_id = request
        .model
        .model_id
        .clone()
        .unwrap_or_else(|| "distilbert-sst2".to_string());
    if let Some(classifier) = context.classifier.as_deref_mut() {
        let runtime = runtime_from_backend(classifier.runtime_backend());
        let raw = classifier.classify_text(&request.text, &request.labels)?;
        return Ok(TextClassificationResponse {
            accepted: true,
            operation: "classify".to_string(),
            text: request.text,
            model_id,
            runtime,
            predictions: class_predictions_from_raw(raw, request.top_k),
        });
    }
    if request.model.fallback_policy != FallbackPolicy::Error {
        return classify_text(request);
    }
    missing_model("classification requires a classifier backend, imported predictions, or an explicit fallback policy")
}

/// Runs sentiment with an optional classifier backend.
pub fn analyze_sentiment_with_context(
    request: SentimentRequest,
    context: &mut TextClassificationExecutionContext<'_>,
) -> Result<SentimentResponse> {
    ensure_non_empty(&request.text, "text")?;
    if !request.imported_predictions.is_empty() {
        return analyze_sentiment(request);
    }
    let model_id = request
        .model
        .model_id
        .clone()
        .unwrap_or_else(|| "twitter-roberta-sentiment-latest".to_string());
    if let Some(classifier) = context.classifier.as_deref_mut() {
        let labels = ["negative", "neutral", "positive"]
            .into_iter()
            .map(str::to_string)
            .collect::<Vec<_>>();
        let runtime = runtime_from_backend(classifier.runtime_backend());
        let predictions = class_predictions_from_raw(
            classifier.classify_text(&request.text, &labels)?,
            labels.len(),
        );
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
            runtime,
            label,
            positive_score,
            negative_score,
            compound: positive_score - negative_score,
            predictions,
        });
    }
    if request.model.fallback_policy != FallbackPolicy::Error {
        return analyze_sentiment(request);
    }
    missing_model("sentiment requires a classifier backend, imported predictions, or an explicit fallback policy")
}

/// Runs zero-shot classification with an optional classifier backend.
pub fn zero_shot_classify_with_context(
    request: ZeroShotClassificationRequest,
    context: &mut TextClassificationExecutionContext<'_>,
) -> Result<ZeroShotClassificationResponse> {
    ensure_non_empty(&request.text, "text")?;
    if request.labels.is_empty() {
        return Err(DetectError::InvalidArgument(
            "zero-shot request must include at least one label".to_string(),
        ));
    }
    if !request.imported_predictions.is_empty() {
        return zero_shot_classify(request);
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
    if let Some(classifier) = context.classifier.as_deref_mut() {
        let runtime = runtime_from_backend(classifier.runtime_backend());
        let mut predictions = class_predictions_from_raw(
            classifier.classify_text(&request.text, &request.labels)?,
            request.labels.len(),
        );
        normalize_prediction_scores(&mut predictions);
        return Ok(ZeroShotClassificationResponse {
            accepted: true,
            operation: "zero-shot".to_string(),
            text: request.text,
            model_id,
            runtime,
            predictions,
            hypotheses,
        });
    }
    if request.model.fallback_policy != FallbackPolicy::Error {
        return zero_shot_classify(request);
    }
    missing_model("zero-shot classification requires a classifier backend, imported predictions, or an explicit fallback policy")
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
            runtime: TextClassificationRuntime::ImportedPredictions,
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
                runtime: TextClassificationRuntime::Lexical,
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
            runtime: TextClassificationRuntime::ImportedPredictions,
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
                runtime: TextClassificationRuntime::Lexical,
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
            runtime: TextClassificationRuntime::ImportedPredictions,
            predictions: normalize_predictions(request.imported_predictions, request.labels.len()),
            hypotheses,
        });
    }

    match request.model.fallback_policy {
        FallbackPolicy::FastFallback | FallbackPolicy::LexicalFallback => {
            let mut predictions =
                lexical_label_scores(&request.text, &request.labels, request.labels.len());
            normalize_prediction_scores(&mut predictions);
            Ok(ZeroShotClassificationResponse {
                accepted: true,
                operation: "zero-shot".to_string(),
                text: request.text,
                model_id,
                runtime: TextClassificationRuntime::Lexical,
                predictions,
                hypotheses,
            })
        }
        FallbackPolicy::Error => unsupported_runtime(
            "native zero-shot classification requires imported predictions or an explicit fallback policy",
        ),
    }
}

/// Returns preset ids registered for text classification capabilities.
pub fn registered_text_classification_presets() -> Vec<String> {
    model_catalog(None)
        .into_iter()
        .map(|model| model.id)
        .collect()
}

fn metadata(
    id: &str,
    model_id: &str,
    task: TextClassificationTask,
    runtime: TextClassificationRuntime,
    supported: bool,
    fallback: Option<&str>,
    note: Option<&str>,
) -> TextClassificationModelMetadata {
    TextClassificationModelMetadata {
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

fn missing_model<T>(message: &str) -> Result<T> {
    Err(DetectError::InvalidArgument(format!(
        "missing_model: {message}"
    )))
}

fn runtime_from_backend(backend: TextRuntimeBackend) -> TextClassificationRuntime {
    match backend {
        TextRuntimeBackend::Candle => TextClassificationRuntime::Candle,
        TextRuntimeBackend::Onnx => TextClassificationRuntime::Onnx,
        TextRuntimeBackend::External => TextClassificationRuntime::ImportedPredictions,
        TextRuntimeBackend::Tokenizers
        | TextRuntimeBackend::CudaOxide
        | TextRuntimeBackend::Heuristic => TextClassificationRuntime::Lexical,
    }
}

fn class_predictions_from_raw(
    predictions: Vec<text_model_runtime::RawPrediction>,
    top_k: usize,
) -> Vec<TextClassPrediction> {
    let imported = predictions
        .into_iter()
        .filter_map(|prediction| {
            Some(ImportedPrediction {
                kind: prediction.kind,
                label: prediction.label?,
                text: prediction.text,
                score: prediction.score.unwrap_or(0.0),
                attributes: prediction.attributes,
            })
        })
        .collect::<Vec<_>>();
    normalize_predictions(imported, top_k)
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
    let positive = compound.clamp(0.0, 1.0);
    let negative = (-compound).clamp(0.0, 1.0);
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

/// Parses a task name from API path or CLI input.
pub fn parse_task(value: &str) -> Option<TextClassificationTask> {
    TextClassificationTask::ALL.iter().copied().find(|task| {
        task.path_segment() == value || format!("{task:?}").eq_ignore_ascii_case(value)
    })
}

/// Returns a JSON value for the text classification schema catalog.
pub fn schema_summary() -> serde_json::Value {
    serde_json::json!({
        "tasks": TextClassificationTask::ALL.iter().map(|task| serde_json::json!({
            "task": task,
            "path": format!("/api/{}", task.path_segment())
        })).collect::<Vec<_>>(),
        "models": model_catalog(None),
        "registeredPresets": registered_text_classification_presets(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use text_model_runtime::{RawPrediction, TextRuntimeBackend};

    struct FakeClassifier;

    impl SequenceClassifier for FakeClassifier {
        fn classify_text(&mut self, _text: &str, labels: &[String]) -> Result<Vec<RawPrediction>> {
            Ok(labels
                .iter()
                .enumerate()
                .map(|(index, label)| RawPrediction {
                    label: Some(label.clone()),
                    score: Some(1.0 / (index + 1) as f32),
                    ..RawPrediction::default()
                })
                .collect())
        }

        fn runtime_backend(&self) -> TextRuntimeBackend {
            TextRuntimeBackend::External
        }
    }

    #[test]
    fn catalog_is_classification_only() {
        let tasks = model_catalog(None)
            .into_iter()
            .map(|model| model.task)
            .collect::<BTreeSet<_>>();
        assert!(tasks.contains(&TextClassificationTask::TextClassification));
        assert!(tasks.contains(&TextClassificationTask::Sentiment));
        assert!(tasks.contains(&TextClassificationTask::ZeroShotClassification));
    }

    #[test]
    fn fallback_classification_scores_labels() {
        let response = classify_text(TextClassificationRequest {
            text: "rust text classification".to_string(),
            labels: vec!["classification".to_string(), "music".to_string()],
            top_k: 2,
            multi_label: false,
            model: ModelSelection {
                fallback_policy: FallbackPolicy::LexicalFallback,
                ..ModelSelection::default()
            },
            imported_predictions: Vec::new(),
        })
        .expect("classification");
        assert_eq!(response.predictions[0].label, "classification");
    }

    #[test]
    fn zero_shot_builds_hypotheses() {
        let response = zero_shot_classify(ZeroShotClassificationRequest {
            text: "rust text".to_string(),
            labels: vec!["code".to_string(), "music".to_string()],
            hypothesis_template: "This is about {}.".to_string(),
            model: ModelSelection {
                fallback_policy: FallbackPolicy::LexicalFallback,
                ..ModelSelection::default()
            },
            imported_predictions: Vec::new(),
        })
        .expect("zero shot");
        assert_eq!(response.hypotheses[0], "This is about code.");
    }

    #[test]
    fn context_classification_uses_supplied_backend() {
        let mut classifier = FakeClassifier;
        let mut context = TextClassificationExecutionContext {
            classifier: Some(&mut classifier),
        };
        let response = classify_text_with_context(
            TextClassificationRequest {
                text: "hello".to_string(),
                labels: vec!["a".to_string(), "b".to_string()],
                top_k: 2,
                multi_label: false,
                model: ModelSelection::default(),
                imported_predictions: Vec::new(),
            },
            &mut context,
        )
        .expect("context classify");
        assert_eq!(
            response.runtime,
            TextClassificationRuntime::ImportedPredictions
        );
        assert_eq!(response.predictions[0].label, "a");
    }
}
