#![doc = include_str!("../README.md")]

pub mod surface;

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use text_embeddings::{HashedTextEmbedder, TextEmbedderBackend, TextEmbeddingConfig};
use text_model_runtime::{QuestionAnsweringBackend, TextRuntimeBackend};
use text_retrieval::{IngestionOptions, RetrievalIndex, SearchDocument, SearchQuery, SearchResult};
use video_analysis_core::{DetectError, Result};

/// Runtime families for extractive question answering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QuestionAnsweringRuntime {
    /// Candle-backed native inference.
    Candle,
    /// ONNX Runtime-backed native inference.
    Onnx,
    /// Browser-safe WASM postprocessing.
    WasmPostprocess,
    /// Deterministic or heuristic fallback.
    Heuristic,
    /// Caller-supplied model predictions.
    ImportedPredictions,
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
    pub runtime: Option<QuestionAnsweringRuntime>,
}

/// Caller-supplied native or external runtime backend for QA execution.
#[derive(Default)]
pub struct QuestionAnsweringExecutionContext<'a> {
    /// Optional question-answering backend.
    pub qa: Option<&'a mut dyn QuestionAnsweringBackend>,
}

/// Model metadata for UI and CLI discovery.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuestionAnsweringModelMetadata {
    /// Stable local preset id.
    pub id: String,
    /// Upstream model id or strategy id.
    pub model_id: String,
    /// Preferred runtime.
    pub runtime: QuestionAnsweringRuntime,
    /// Whether the runtime is available in the default contributor build.
    pub supported: bool,
    /// Whether this entry has an implemented native load/run path.
    pub loadable: bool,
    /// Optional fallback preset id.
    pub fallback: Option<String>,
    /// Cargo feature required for native loading.
    pub required_feature: Option<String>,
    /// Setup command or note required before loading.
    pub required_setup: Option<String>,
    /// Surface operation that smokes the implemented path.
    pub smoke_operation: Option<String>,
    /// Human-readable note for unsupported or fallback-only paths.
    pub note: Option<String>,
}

/// Text span used by QA responses.
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

/// Imported span prediction used by server, CLI, and WASM postprocessing.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportedAnswerPrediction {
    /// Optional raw prediction kind.
    #[serde(default)]
    pub kind: Option<String>,
    /// Label emitted by the model.
    #[serde(default)]
    pub label: Option<String>,
    /// Answer text or span text.
    #[serde(default)]
    pub text: Option<String>,
    /// Confidence score.
    pub score: f32,
    /// Arbitrary model attributes, including offsets.
    #[serde(default)]
    pub attributes: BTreeMap<String, String>,
}

/// Request for extractive question answering.
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
    pub imported_predictions: Vec<ImportedAnswerPrediction>,
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

/// Response for extractive question answering.
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
    pub runtime: QuestionAnsweringRuntime,
    /// Candidate answers.
    pub answers: Vec<AnswerPrediction>,
}

/// Citation for an answer produced from retrieved context.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnswerCitation {
    /// Source document id.
    pub document_id: String,
    /// Source retrieval chunk id.
    pub chunk_id: String,
    /// Snippet used as answer context.
    pub snippet: String,
    /// Retrieval or rerank score.
    pub score: f32,
    /// Optional span inside the cited snippet.
    #[serde(default)]
    pub span: Option<TextSpanRef>,
}

/// One cited QA answer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CitedAnswer {
    /// Answer text.
    pub answer: String,
    /// Answer confidence score.
    pub score: f32,
    /// Optional span in the concatenated retrieval context.
    #[serde(default)]
    pub span: Option<TextSpanRef>,
    /// Source citations.
    pub citations: Vec<AnswerCitation>,
}

/// Request for retrieval-backed question answering.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RetrievalQuestionAnsweringRequest {
    /// Question text.
    pub question: String,
    /// Documents used to build a transient deterministic retrieval index.
    #[serde(default)]
    pub documents: Vec<SearchDocument>,
    /// Retrieved chunk count.
    #[serde(default = "default_top_k_chunks")]
    pub top_k_chunks: usize,
    /// Answer count.
    #[serde(default = "default_top_k")]
    pub top_k_answers: usize,
    /// Imported span predictions mapped back to retrieved chunks.
    #[serde(default)]
    pub imported_predictions: Vec<ImportedAnswerPrediction>,
    /// Model selection for backend execution.
    #[serde(default)]
    pub model: ModelSelection,
}

/// Response for retrieval-backed question answering.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RetrievalQuestionAnsweringResponse {
    /// Accepted flag for generated package surfaces.
    pub accepted: bool,
    /// Operation name.
    pub operation: String,
    /// Question text.
    pub question: String,
    /// Runtime used for answer extraction.
    pub runtime: QuestionAnsweringRuntime,
    /// Retrieved chunks.
    pub retrieved_chunks: Vec<SearchResult>,
    /// Cited answers.
    pub answers: Vec<CitedAnswer>,
}

/// Returns the question-answering model catalog.
pub fn model_catalog() -> Vec<QuestionAnsweringModelMetadata> {
    vec![QuestionAnsweringModelMetadata {
        id: "roberta-base-squad2".to_string(),
        model_id: "deepset/roberta-base-squad2".to_string(),
        runtime: QuestionAnsweringRuntime::Onnx,
        supported: false,
        loadable: false,
        fallback: None,
        required_feature: Some("onnx".to_string()),
        required_setup: Some(
            "Reference-only until a native extractive QA runner is implemented".to_string(),
        ),
        smoke_operation: Some("qa.answer".to_string()),
        note: Some(
            "Schema and imported span support are available; native span extraction is not wired."
                .to_string(),
        ),
    }]
}

/// Answers questions with an optional QA backend.
pub fn answer_question_with_context(
    request: QuestionAnsweringRequest,
    context: &mut QuestionAnsweringExecutionContext<'_>,
) -> Result<QuestionAnsweringResponse> {
    ensure_non_empty(&request.question, "question")?;
    ensure_non_empty(&request.context, "context")?;
    if !request.imported_predictions.is_empty() {
        return answer_question(request);
    }
    let model_id = request
        .model
        .model_id
        .clone()
        .unwrap_or_else(|| "roberta-base-squad2".to_string());
    if let Some(qa) = context.qa.as_deref_mut() {
        let runtime = runtime_from_backend(qa.runtime_backend());
        let mut answers = qa
            .answer(&request.question, &request.context)?
            .into_iter()
            .filter_map(|prediction| {
                let answer = prediction.text?;
                Some(AnswerPrediction {
                    answer,
                    score: prediction.score.unwrap_or(0.0),
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
            runtime,
            answers,
        });
    }
    missing_model("question answering requires a QA backend or imported span predictions")
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
            runtime: QuestionAnsweringRuntime::ImportedPredictions,
            answers,
        });
    }

    unsupported_runtime(
        "native question answering requires imported span predictions until ONNX QA span extraction is wired",
    )
}

/// Answers a question by building a deterministic retrieval index from documents.
pub fn answer_question_with_retrieval(
    request: RetrievalQuestionAnsweringRequest,
) -> Result<RetrievalQuestionAnsweringResponse> {
    let mut context = QuestionAnsweringExecutionContext::default();
    answer_question_with_retrieval_context(request, &mut context)
}

/// Answers a retrieval-backed question with an optional QA backend.
pub fn answer_question_with_retrieval_context(
    request: RetrievalQuestionAnsweringRequest,
    context: &mut QuestionAnsweringExecutionContext<'_>,
) -> Result<RetrievalQuestionAnsweringResponse> {
    ensure_non_empty(&request.question, "question")?;
    if request.documents.is_empty() {
        return Err(DetectError::InvalidArgument(
            "retrieval QA request must include documents or use an existing retrieval index"
                .to_string(),
        ));
    }
    let embedder = HashedTextEmbedder::new(
        TextEmbeddingConfig {
            dimensions: 64,
            use_idf: false,
        },
        text_lexical::CorpusOptions::default(),
    )?;
    let mut index = RetrievalIndex::new(embedder);
    index.ingest_documents(&request.documents, &IngestionOptions::default())?;
    answer_question_with_retrieval_index(request, &index, context)
}

/// Answers a retrieval-backed question using an existing retrieval index.
pub fn answer_question_with_retrieval_index<B: TextEmbedderBackend>(
    request: RetrievalQuestionAnsweringRequest,
    index: &RetrievalIndex<B>,
    context: &mut QuestionAnsweringExecutionContext<'_>,
) -> Result<RetrievalQuestionAnsweringResponse> {
    ensure_non_empty(&request.question, "question")?;
    let top_k_chunks = request.top_k_chunks.max(1);
    let retrieved_chunks = index.search(&SearchQuery::hybrid(
        request.question.clone(),
        top_k_chunks,
        text_retrieval::HybridConfig {
            rerank_window: top_k_chunks.max(4),
            ..text_retrieval::HybridConfig::default()
        },
    ))?;
    if retrieved_chunks.is_empty() {
        return Ok(RetrievalQuestionAnsweringResponse {
            accepted: true,
            operation: "retrieval-question-answer".to_string(),
            question: request.question,
            runtime: QuestionAnsweringRuntime::Heuristic,
            retrieved_chunks,
            answers: Vec::new(),
        });
    }
    let context_text = retrieved_chunks
        .iter()
        .map(|chunk| chunk.snippet.as_str())
        .collect::<Vec<_>>()
        .join("\n\n");

    let mut runtime = QuestionAnsweringRuntime::Heuristic;
    let answers = if !request.imported_predictions.is_empty() || context.qa.is_some() {
        let qa_response = if context.qa.is_some() && request.imported_predictions.is_empty() {
            answer_question_with_context(
                QuestionAnsweringRequest {
                    question: request.question.clone(),
                    context: context_text.clone(),
                    top_k: request.top_k_answers,
                    model: request.model.clone(),
                    imported_predictions: Vec::new(),
                },
                context,
            )?
        } else {
            answer_question(QuestionAnsweringRequest {
                question: request.question.clone(),
                context: context_text.clone(),
                top_k: request.top_k_answers,
                model: request.model.clone(),
                imported_predictions: request.imported_predictions.clone(),
            })?
        };
        runtime = qa_response.runtime;
        qa_response
            .answers
            .into_iter()
            .map(|answer| cited_answer_from_prediction(answer, &retrieved_chunks))
            .collect::<Vec<_>>()
    } else {
        heuristic_cited_answers(&request.question, &retrieved_chunks, request.top_k_answers)
    };

    Ok(RetrievalQuestionAnsweringResponse {
        accepted: true,
        operation: "retrieval-question-answer".to_string(),
        question: request.question,
        runtime,
        retrieved_chunks,
        answers,
    })
}

/// Returns preset ids registered for question answering.
pub fn registered_question_answering_presets() -> Vec<String> {
    model_catalog().into_iter().map(|model| model.id).collect()
}

fn default_top_k() -> usize {
    3
}

fn default_top_k_chunks() -> usize {
    5
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

fn runtime_from_backend(backend: TextRuntimeBackend) -> QuestionAnsweringRuntime {
    match backend {
        TextRuntimeBackend::Candle => QuestionAnsweringRuntime::Candle,
        TextRuntimeBackend::Onnx => QuestionAnsweringRuntime::Onnx,
        TextRuntimeBackend::External => QuestionAnsweringRuntime::ImportedPredictions,
        TextRuntimeBackend::Tokenizers
        | TextRuntimeBackend::CudaOxide
        | TextRuntimeBackend::Heuristic => QuestionAnsweringRuntime::Heuristic,
    }
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

fn cited_answer_from_prediction(answer: AnswerPrediction, chunks: &[SearchResult]) -> CitedAnswer {
    let citations = citations_for_answer(&answer.answer, chunks);
    CitedAnswer {
        answer: answer.answer,
        score: answer.score,
        span: answer.span,
        citations,
    }
}

fn heuristic_cited_answers(
    question: &str,
    chunks: &[SearchResult],
    top_k_answers: usize,
) -> Vec<CitedAnswer> {
    chunks
        .iter()
        .take(top_k_answers.max(1))
        .map(|chunk| {
            let answer = lexical_answer_span(question, &chunk.snippet)
                .map(|span| chunk.snippet[span.byte_start..span.byte_end].to_string())
                .unwrap_or_else(|| chunk.snippet.clone());
            let citations = citations_for_answer(&answer, std::slice::from_ref(chunk));
            CitedAnswer {
                answer,
                score: chunk.score,
                span: None,
                citations,
            }
        })
        .collect()
}

fn citations_for_answer(answer: &str, chunks: &[SearchResult]) -> Vec<AnswerCitation> {
    let mut citations = chunks
        .iter()
        .filter_map(|chunk| {
            let span = find_span(&chunk.snippet, answer);
            if span.is_none() && !answer.trim().is_empty() && !chunk.snippet.contains(answer.trim())
            {
                return None;
            }
            Some(AnswerCitation {
                document_id: chunk.document_id.clone(),
                chunk_id: chunk.chunk_id.clone(),
                snippet: chunk.snippet.clone(),
                score: chunk.score,
                span,
            })
        })
        .collect::<Vec<_>>();
    if citations.is_empty() {
        citations = chunks
            .iter()
            .take(1)
            .map(|chunk| AnswerCitation {
                document_id: chunk.document_id.clone(),
                chunk_id: chunk.chunk_id.clone(),
                snippet: chunk.snippet.clone(),
                score: chunk.score,
                span: None,
            })
            .collect();
    }
    citations
}

fn lexical_answer_span(question: &str, text: &str) -> Option<TextSpanRef> {
    let terms = question
        .split_whitespace()
        .map(|term| {
            term.trim_matches(|ch: char| !ch.is_alphanumeric())
                .to_lowercase()
        })
        .filter(|term| term.len() > 2)
        .collect::<Vec<_>>();
    let lower_text = text.to_lowercase();
    for term in terms {
        if let Some(byte_start) = lower_text.find(&term) {
            let byte_end = (byte_start + term.len()).min(text.len());
            return Some(byte_span_to_ref(text, byte_start, byte_end));
        }
    }
    None
}

fn find_span(text: &str, needle: &str) -> Option<TextSpanRef> {
    let needle = needle.trim();
    if needle.is_empty() {
        return None;
    }
    text.find(needle)
        .map(|byte_start| byte_span_to_ref(text, byte_start, byte_start + needle.len()))
}

fn byte_span_to_ref(text: &str, byte_start: usize, byte_end: usize) -> TextSpanRef {
    let char_start = text[..byte_start].chars().count();
    let char_end = char_start + text[byte_start..byte_end].chars().count();
    TextSpanRef {
        byte_start,
        byte_end,
        char_start,
        char_end,
    }
}

/// Returns a JSON value for the question-answering schema catalog.
pub fn schema_summary() -> serde_json::Value {
    serde_json::json!({
        "task": "question_answering",
        "path": "/api/question-answer",
        "models": model_catalog(),
        "registeredPresets": registered_question_answering_presets(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use text_model_runtime::{RawPrediction, TextRuntimeBackend};

    struct FakeQa;

    impl QuestionAnsweringBackend for FakeQa {
        fn answer(&mut self, _question: &str, _context: &str) -> Result<Vec<RawPrediction>> {
            let mut attributes = BTreeMap::new();
            attributes.insert("byte_start".to_string(), "0".to_string());
            attributes.insert("byte_end".to_string(), "5".to_string());
            Ok(vec![RawPrediction {
                text: Some("Alice".to_string()),
                score: Some(0.8),
                attributes,
                ..RawPrediction::default()
            }])
        }

        fn runtime_backend(&self) -> TextRuntimeBackend {
            TextRuntimeBackend::External
        }
    }

    #[test]
    fn imported_answers_are_sorted() {
        let response = answer_question(QuestionAnsweringRequest {
            question: "Who?".to_string(),
            context: "Alice wrote it.".to_string(),
            top_k: 1,
            model: ModelSelection::default(),
            imported_predictions: vec![
                ImportedAnswerPrediction {
                    text: Some("Bob".to_string()),
                    score: 0.2,
                    kind: None,
                    label: None,
                    attributes: BTreeMap::new(),
                },
                ImportedAnswerPrediction {
                    text: Some("Alice".to_string()),
                    score: 0.9,
                    kind: None,
                    label: None,
                    attributes: BTreeMap::new(),
                },
            ],
        })
        .expect("answer");
        assert_eq!(response.answers[0].answer, "Alice");
    }

    #[test]
    fn context_qa_uses_supplied_backend() {
        let mut qa = FakeQa;
        let mut context = QuestionAnsweringExecutionContext { qa: Some(&mut qa) };
        let response = answer_question_with_context(
            QuestionAnsweringRequest {
                question: "Who?".to_string(),
                context: "Alice wrote it.".to_string(),
                top_k: 1,
                model: ModelSelection::default(),
                imported_predictions: Vec::new(),
            },
            &mut context,
        )
        .expect("context answer");
        assert_eq!(response.answers[0].answer, "Alice");
        assert_eq!(
            response.runtime,
            QuestionAnsweringRuntime::ImportedPredictions
        );
    }

    #[test]
    fn retrieval_qa_returns_cited_heuristic_answer() {
        let response = answer_question_with_retrieval(RetrievalQuestionAnsweringRequest {
            question: "What language has ownership?".to_string(),
            documents: vec![SearchDocument::new(
                "doc-rust",
                "Rust has ownership and deterministic package workflows.",
            )],
            top_k_chunks: 2,
            top_k_answers: 1,
            imported_predictions: Vec::new(),
            model: ModelSelection::default(),
        })
        .expect("retrieval qa");

        assert_eq!(response.runtime, QuestionAnsweringRuntime::Heuristic);
        assert_eq!(response.answers.len(), 1);
        assert_eq!(response.answers[0].citations[0].document_id, "doc-rust");
        assert_eq!(response.answers[0].citations[0].chunk_id, "doc-rust:0");
    }

    #[test]
    fn imported_retrieval_qa_prediction_maps_to_chunk_citation() {
        let response = answer_question_with_retrieval(RetrievalQuestionAnsweringRequest {
            question: "What has ownership?".to_string(),
            documents: vec![SearchDocument::new(
                "doc-rust",
                "Rust has ownership and deterministic package workflows.",
            )],
            top_k_chunks: 1,
            top_k_answers: 1,
            imported_predictions: vec![ImportedAnswerPrediction {
                kind: None,
                label: None,
                text: Some("Rust".to_string()),
                score: 0.9,
                attributes: BTreeMap::new(),
            }],
            model: ModelSelection::default(),
        })
        .expect("retrieval qa");

        assert_eq!(
            response.runtime,
            QuestionAnsweringRuntime::ImportedPredictions
        );
        assert_eq!(response.answers[0].answer, "Rust");
        assert_eq!(response.answers[0].citations[0].document_id, "doc-rust");
        assert!(response.answers[0].citations[0].span.is_some());
    }
}
