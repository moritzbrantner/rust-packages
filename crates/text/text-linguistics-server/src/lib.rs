use std::io::{self, BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use text_linguistics as _;
use text_linguistics::{
    analyze_text, LinguisticAnalysis, LinguisticAnalysisOptions, TextNlpConfig, TextNlpPipeline,
};
use text_nlp_models::{
    analyze_sentiment, answer_question, classify_text, embed_texts, model_catalog, parse_task,
    rerank, schema_summary, summarize, zero_shot_classify, EmbeddingRequest,
    QuestionAnsweringRequest, RerankRequest, SentimentRequest, SummaryRequest,
    TextClassificationRequest, ZeroShotClassificationRequest,
};

/// Wrapped library crate name.
pub const LIBRARY_CRATE: &str = "text-linguistics";
/// Adapter surface kind.
pub const SURFACE_KIND: &str = "api";
/// Rust import path for the wrapped crate.
pub const LIBRARY_IMPORT: &str = "use text_linguistics";
/// Companion CLI package name.
pub const CLI_PACKAGE: &str = "text-linguistics-cli";
/// Companion React app package name.
pub const APP_PACKAGE: &str = "text-linguistics-app";

/// Minimal HTTP response used by the generated API adapter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpResponse {
    pub status_code: u16,
    pub reason: &'static str,
    pub content_type: &'static str,
    pub body: String,
}

/// Serves the package API on the provided socket address.
pub fn serve(addr: &str) -> io::Result<()> {
    let listener = TcpListener::bind(addr)?;
    for stream in listener.incoming() {
        handle_stream(stream?)?;
    }
    Ok(())
}

/// Returns a response for one generated API request.
pub fn response_for(method: &str, path: &str, body: &str) -> HttpResponse {
    match (method, path) {
        ("OPTIONS", _) => HttpResponse {
            status_code: 204,
            reason: "No Content",
            content_type: "application/json",
            body: String::new(),
        },
        ("GET", "/health") => json_response(
            200,
            "OK",
            serde_json::json!({
                "ok": true,
                "package": format!("{}-server", LIBRARY_CRATE),
                "library": LIBRARY_CRATE
            }),
        ),
        ("GET", "/api/package") => json_response(200, "OK", package_metadata_value()),
        ("GET", "/api/schema") => json_response(200, "OK", schema_value()),
        ("GET", "/api/models") => json_response(200, "OK", serde_json::json!(model_catalog(None))),
        ("GET", path) if path.starts_with("/api/models/") => {
            let task = path.trim_start_matches("/api/models/");
            match parse_task(task) {
                Some(task) => {
                    json_response(200, "OK", serde_json::json!(model_catalog(Some(task))))
                }
                None => typed_error_response(
                    400,
                    "Bad Request",
                    "invalid_request",
                    &format!("unknown NLP task `{task}`"),
                ),
            }
        }
        ("POST", "/api/entities") => run_response(body),
        ("POST", "/api/classify") => classify_response(body),
        ("POST", "/api/sentiment") => sentiment_response(body),
        ("POST", "/api/embed") => embed_response(body),
        ("POST", "/api/zero-shot") => zero_shot_response(body),
        ("POST", "/api/summarize") => summarize_response(body),
        ("POST", "/api/rerank") => rerank_response(body),
        ("POST", "/api/question-answer") => question_answer_response(body),
        ("POST", "/api/run") => run_response(body),
        _ => json_response(
            404,
            "Not Found",
            serde_json::json!({
                "error": "not found",
                "path": path
            }),
        ),
    }
}

fn run_response(body: &str) -> HttpResponse {
    let payload = match serde_json::from_str::<serde_json::Value>(body) {
        Ok(value) => value,
        Err(error) => {
            return json_response(
                400,
                "Bad Request",
                serde_json::json!({
                    "package": format!("{}-server", LIBRARY_CRATE),
                    "library": LIBRARY_CRATE,
                    "accepted": false,
                    "error": format!("invalid JSON: {error}")
                }),
            );
        }
    };

    match payload
        .get("operation")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("analyze")
    {
        "entities" | "analyze" => {}
        "classify" => return classify_response(body),
        "sentiment" => return sentiment_response(body),
        "embed" => return embed_response(body),
        "zero-shot" | "zeroShot" | "zero_shot" => return zero_shot_response(body),
        "summarize" => return summarize_response(body),
        "rerank" => return rerank_response(body),
        "question-answer" | "questionAnswer" | "question_answer" => {
            return question_answer_response(body)
        }
        operation => {
            return typed_error_response(
                400,
                "Bad Request",
                "invalid_request",
                &format!("unsupported operation `{operation}`"),
            )
        }
    }

    let text = payload
        .get("text")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    if text.trim().is_empty() {
        return json_response(
            400,
            "Bad Request",
            serde_json::json!({
                "package": format!("{}-server", LIBRARY_CRATE),
                "library": LIBRARY_CRATE,
                "accepted": false,
                "error": "request body must include a non-empty `text` string"
            }),
        );
    }

    match analyze_text_for_payload(text, &payload) {
        Ok(analysis) => json_response(
            200,
            "OK",
            analysis_payload(text, &analysis.analysis, analysis.model_metadata),
        ),
        Err(error) => json_response(
            500,
            "Internal Server Error",
            serde_json::json!({
                "package": format!("{}-server", LIBRARY_CRATE),
                "library": LIBRARY_CRATE,
                "accepted": false,
                "error": error.to_string()
            }),
        ),
    }
}

fn classify_response(body: &str) -> HttpResponse {
    match serde_json::from_str::<TextClassificationRequest>(body) {
        Ok(request) => result_response(classify_text(request)),
        Err(error) => typed_error_response(
            400,
            "Bad Request",
            "invalid_request",
            &format!("invalid classify request: {error}"),
        ),
    }
}

fn sentiment_response(body: &str) -> HttpResponse {
    match serde_json::from_str::<SentimentRequest>(body) {
        Ok(request) => result_response(analyze_sentiment(request)),
        Err(error) => typed_error_response(
            400,
            "Bad Request",
            "invalid_request",
            &format!("invalid sentiment request: {error}"),
        ),
    }
}

fn embed_response(body: &str) -> HttpResponse {
    match serde_json::from_str::<EmbeddingRequest>(body) {
        Ok(request) => result_response(embed_texts(request)),
        Err(error) => typed_error_response(
            400,
            "Bad Request",
            "invalid_request",
            &format!("invalid embed request: {error}"),
        ),
    }
}

fn zero_shot_response(body: &str) -> HttpResponse {
    match serde_json::from_str::<ZeroShotClassificationRequest>(body) {
        Ok(request) => result_response(zero_shot_classify(request)),
        Err(error) => typed_error_response(
            400,
            "Bad Request",
            "invalid_request",
            &format!("invalid zero-shot request: {error}"),
        ),
    }
}

fn summarize_response(body: &str) -> HttpResponse {
    match serde_json::from_str::<SummaryRequest>(body) {
        Ok(request) => result_response(summarize(request)),
        Err(error) => typed_error_response(
            400,
            "Bad Request",
            "invalid_request",
            &format!("invalid summarize request: {error}"),
        ),
    }
}

fn rerank_response(body: &str) -> HttpResponse {
    match serde_json::from_str::<RerankRequest>(body) {
        Ok(request) => result_response(rerank(request)),
        Err(error) => typed_error_response(
            400,
            "Bad Request",
            "invalid_request",
            &format!("invalid rerank request: {error}"),
        ),
    }
}

fn question_answer_response(body: &str) -> HttpResponse {
    match serde_json::from_str::<QuestionAnsweringRequest>(body) {
        Ok(request) => result_response(answer_question(request)),
        Err(error) => typed_error_response(
            400,
            "Bad Request",
            "invalid_request",
            &format!("invalid question-answer request: {error}"),
        ),
    }
}

fn result_response<T: serde::Serialize>(
    result: Result<T, video_analysis_core::DetectError>,
) -> HttpResponse {
    match result {
        Ok(value) => json_response(200, "OK", serde_json::json!(value)),
        Err(error) => error_response(error),
    }
}

fn error_response(error: video_analysis_core::DetectError) -> HttpResponse {
    let message = error.to_string();
    if message.contains("unsupported_runtime")
        || message.contains("Unsupported")
        || message.contains("unsupported")
    {
        return typed_error_response(422, "Unprocessable Entity", "unsupported_runtime", &message);
    }
    if message.contains("non-empty") || message.contains("must include") {
        return typed_error_response(400, "Bad Request", "empty_input", &message);
    }
    typed_error_response(
        500,
        "Internal Server Error",
        "model_output_mismatch",
        &message,
    )
}

#[derive(Debug, Clone, Copy)]
struct ModelMetadata {
    entity_recognition: &'static str,
    entity_model: Option<&'static str>,
}

#[derive(Debug)]
struct RunAnalysis {
    analysis: LinguisticAnalysis,
    model_metadata: ModelMetadata,
}

fn analyze_text_for_payload(
    text: &str,
    payload: &serde_json::Value,
) -> Result<RunAnalysis, String> {
    match payload
        .get("entityRecognition")
        .or_else(|| payload.get("modelMode"))
        .or_else(|| payload.get("mode"))
        .and_then(serde_json::Value::as_str)
    {
        Some("heuristic") | Some("rules") => {
            let analysis = analyze_text(text, &LinguisticAnalysisOptions::heuristic())
                .map_err(|error| error.to_string())?;
            Ok(RunAnalysis {
                analysis,
                model_metadata: ModelMetadata {
                    entity_recognition: "heuristic",
                    entity_model: None,
                },
            })
        }
        _ => {
            let config = config_from_payload(payload);
            let model_metadata = model_metadata_for_config(&config);
            let analysis = TextNlpPipeline::new(config)
                .analyze_text(text)
                .map_err(|error| error.to_string())?;
            Ok(RunAnalysis {
                analysis,
                model_metadata,
            })
        }
    }
}

fn config_from_payload(payload: &serde_json::Value) -> TextNlpConfig {
    match payload.get("profile").and_then(serde_json::Value::as_str) {
        Some("fast") => TextNlpConfig::fast(),
        Some("balanced") => TextNlpConfig::balanced(),
        _ => TextNlpConfig::rich(),
    }
}

fn model_metadata_for_config(config: &TextNlpConfig) -> ModelMetadata {
    if matches!(config.profile, text_linguistics::AnalysisProfile::Fast) {
        ModelMetadata {
            entity_recognition: "heuristic",
            entity_model: None,
        }
    } else {
        ModelMetadata {
            entity_recognition: "local-model",
            entity_model: Some("bert-base-ner"),
        }
    }
}

fn analysis_payload(
    text: &str,
    analysis: &LinguisticAnalysis,
    model_metadata: ModelMetadata,
) -> serde_json::Value {
    serde_json::json!({
        "package": format!("{}-server", LIBRARY_CRATE),
        "library": LIBRARY_CRATE,
        "accepted": true,
        "operation": "analyze",
        "text": text,
        "profile": format!("{:?}", analysis.profile),
        "provenance": format!("{:?}", analysis.provenance),
        "confidence": analysis.confidence.get(),
        "model": {
            "entityRecognition": model_metadata.entity_recognition,
            "entityModel": model_metadata.entity_model,
            "tokenizerMode": format!("{:?}", analysis.tokenizer.mode),
            "tokenizerSource": analysis.tokenizer.source.as_ref().map(|source| format!("{source:?}")),
            "alignmentCount": analysis.alignments.as_ref().map(|alignment| alignment.aligned_tokens.len()).unwrap_or(0)
        },
        "summary": {
            "language": analysis.language.primary.as_ref().map(|prediction| prediction.language.as_str()),
            "tokenCount": analysis.tokens.len(),
            "sentenceCount": analysis.sentences.len(),
            "lemmaCount": analysis.lemmas.len(),
            "entityCount": analysis.entities.len(),
            "eventCount": analysis.events.len(),
            "relationCount": analysis.relations.len(),
            "topicCount": analysis.topics.descriptors.len(),
            "chunkCount": analysis.chunks.len()
        },
        "language": {
            "primary": analysis.language.primary.as_ref().map(|prediction| serde_json::json!({
                "language": prediction.language,
                "confidence": prediction.confidence,
                "script": prediction.script,
                "reason": prediction.reason
            })),
            "dominantScript": analysis.language.dominant_script,
            "isMixed": analysis.language.is_mixed,
            "tokenCount": analysis.language.token_count
        },
        "tokens": analysis.tokens.iter().enumerate().map(|(index, token)| serde_json::json!({
            "index": index,
            "text": token.text,
            "normalized": token.normalized,
            "kind": format!("{:?}", token.kind),
            "start": token.span.char_start,
            "end": token.span.char_end
        })).collect::<Vec<_>>(),
        "sentences": analysis.sentences.iter().enumerate().map(|(index, sentence)| serde_json::json!({
            "index": index,
            "text": sentence.text,
            "start": sentence.span.char_start,
            "end": sentence.span.char_end,
            "tokenCount": sentence.token_count
        })).collect::<Vec<_>>(),
        "lemmas": analysis.lemmas.iter().map(|lemma| serde_json::json!({
            "tokenIndex": lemma.token_index,
            "token": analysis.tokens.get(lemma.token_index).map(|token| token.text.as_str()),
            "lemma": lemma.value,
            "language": lemma.language,
            "confidence": lemma.confidence
        })).collect::<Vec<_>>(),
        "pos": analysis.pos.iter().map(|pos| serde_json::json!({
            "tokenIndex": pos.token_index,
            "token": analysis.tokens.get(pos.token_index).map(|token| token.text.as_str()),
            "tag": format!("{:?}", pos.tag),
            "confidence": pos.confidence,
            "reason": pos.reason
        })).collect::<Vec<_>>(),
        "entities": analysis.entities.iter().map(|entity| serde_json::json!({
            "id": entity.id,
            "text": entity.mention.text,
            "normalized": entity.normalized,
            "kind": format!("{:?}", entity.entity_type),
            "sentenceIndex": entity.sentence_index,
            "tokenStart": entity.token_start,
            "tokenEnd": entity.token_end,
            "confidence": entity.confidence
        })).collect::<Vec<_>>(),
        "events": analysis.events.iter().map(|event| serde_json::json!({
            "sentenceIndex": event.sentence_index,
            "predicate": event.predicate,
            "lemma": event.lemma,
            "relationType": format!("{:?}", event.relation_type),
            "confidence": event.confidence,
            "arguments": event.arguments.iter().map(|argument| serde_json::json!({
                "role": argument.role,
                "text": argument.text,
                "confidence": argument.confidence
            })).collect::<Vec<_>>()
        })).collect::<Vec<_>>(),
        "relations": analysis.relations.iter().map(|relation| serde_json::json!({
            "subject": relation.subject,
            "relation": relation.relation,
            "object": relation.object,
            "relationType": format!("{:?}", relation.relation_type),
            "confidence": relation.confidence
        })).collect::<Vec<_>>(),
        "topics": analysis.topics.descriptors.iter().map(|topic| serde_json::json!({
            "label": topic.label,
            "terms": topic.terms,
            "score": topic.score
        })).collect::<Vec<_>>(),
        "style": {
            "register": format!("{:?}", analysis.style.register),
            "averageSentenceTokens": analysis.style.complexity.average_sentence_tokens,
            "typeTokenRatio": analysis.style.complexity.type_token_ratio,
            "formalityScore": analysis.style.formality_score,
            "questionCount": analysis.style.question_count,
            "exclamationCount": analysis.style.exclamation_count
        }
    })
}

/// Returns JSON metadata for this server adapter.
pub fn package_metadata_json() -> String {
    package_metadata_value().to_string()
}

fn package_metadata_value() -> serde_json::Value {
    serde_json::json!({
        "package": format!("{}-server", LIBRARY_CRATE),
        "surface": SURFACE_KIND,
        "library": LIBRARY_CRATE,
        "libraryImport": LIBRARY_IMPORT,
        "cliPackage": CLI_PACKAGE,
        "appPackage": APP_PACKAGE,
        "endpoints": [
            "GET /health",
            "GET /api/package",
            "GET /api/schema",
            "GET /api/models",
            "GET /api/models/:task",
            "POST /api/entities",
            "POST /api/classify",
            "POST /api/sentiment",
            "POST /api/embed",
            "POST /api/zero-shot",
            "POST /api/summarize",
            "POST /api/rerank",
            "POST /api/question-answer",
            "POST /api/run"
        ]
    })
}

fn schema_value() -> serde_json::Value {
    serde_json::json!({
        "openapi": "3.1.0",
        "info": {
            "title": format!("{} API", LIBRARY_CRATE),
            "version": env!("CARGO_PKG_VERSION")
        },
        "paths": {
            "/health": { "get": { "summary": "Health check" } },
            "/api/package": { "get": { "summary": "Package metadata" } },
            "/api/schema": { "get": { "summary": "API schema" } },
            "/api/models": { "get": { "summary": "List NLP model presets" } },
            "/api/models/{task}": { "get": { "summary": "List NLP model presets for a task" } },
            "/api/entities": { "post": { "summary": "Named entity and linguistic analysis" } },
            "/api/classify": { "post": { "summary": "Text classification" } },
            "/api/sentiment": { "post": { "summary": "Sentiment analysis" } },
            "/api/embed": { "post": { "summary": "Text embeddings" } },
            "/api/zero-shot": { "post": { "summary": "Zero-shot classification" } },
            "/api/summarize": { "post": { "summary": "Extractive summarization" } },
            "/api/rerank": { "post": { "summary": "Document reranking" } },
            "/api/question-answer": { "post": { "summary": "Question answering with imported spans or native runtime" } },
            "/api/run": {
                "post": {
                    "summary": "Legacy generic operation entrypoint",
                    "description": "Runs text-linguistics analysis. Rich and balanced profiles use local bert-base-ner entity recognition unless entityRecognition/modelMode is heuristic."
                }
            },
            "/components/schemas/textNlp": schema_summary()
        }
    })
}

fn handle_stream(mut stream: TcpStream) -> io::Result<()> {
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut request_line = String::new();
    reader.read_line(&mut request_line)?;

    let mut content_length = 0usize;
    loop {
        let mut header = String::new();
        reader.read_line(&mut header)?;
        let trimmed = header.trim_end();
        if trimmed.is_empty() {
            break;
        }
        if let Some((name, value)) = trimmed.split_once(':') {
            if name.eq_ignore_ascii_case("content-length") {
                content_length = value.trim().parse().unwrap_or(0);
            }
        }
    }

    let mut body = vec![0; content_length];
    if content_length > 0 {
        reader.read_exact(&mut body)?;
    }
    let body = String::from_utf8_lossy(&body);

    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or("GET");
    let path = parts.next().unwrap_or("/");
    let response = response_for(method, path, &body);
    write_response(&mut stream, response)
}

fn json_response(status_code: u16, reason: &'static str, value: serde_json::Value) -> HttpResponse {
    HttpResponse {
        status_code,
        reason,
        content_type: "application/json",
        body: value.to_string(),
    }
}

fn typed_error_response(
    status_code: u16,
    reason: &'static str,
    code: &str,
    message: &str,
) -> HttpResponse {
    json_response(
        status_code,
        reason,
        serde_json::json!({
            "package": format!("{}-server", LIBRARY_CRATE),
            "library": LIBRARY_CRATE,
            "accepted": false,
            "error": {
                "code": code,
                "message": message
            }
        }),
    )
}

fn write_response(stream: &mut TcpStream, response: HttpResponse) -> io::Result<()> {
    write!(
        stream,
        "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nAccess-Control-Allow-Origin: *\r\nAccess-Control-Allow-Headers: content-type\r\nAccess-Control-Allow-Methods: GET, POST, OPTIONS\r\nConnection: close\r\n\r\n{}",
        response.status_code,
        response.reason,
        response.content_type,
        response.body.len(),
        response.body
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn health_endpoint_reports_package() {
        let response = response_for("GET", "/health", "");
        assert_eq!(response.status_code, 200);
        assert!(response.body.contains(LIBRARY_CRATE));
    }

    #[test]
    fn run_endpoint_returns_linguistic_analysis() {
        let response = response_for(
            "POST",
            "/api/run",
            r#"{"operation":"analyze","text":"Alice presented the roadmap in Berlin."}"#,
        );
        assert_eq!(response.status_code, 200);
        assert!(response.body.contains("\"operation\":\"analyze\""));
        assert!(response.body.contains("\"entityCount\""));
    }

    #[test]
    fn sentiment_endpoint_supports_explicit_lexical_fallback() {
        let response = response_for(
            "POST",
            "/api/sentiment",
            r#"{"text":"excellent reliable work","model":{"fallbackPolicy":"lexical_fallback"}}"#,
        );
        assert_eq!(response.status_code, 200);
        assert!(response.body.contains("\"operation\":\"sentiment\""));
        assert!(response.body.contains("\"runtime\":\"lexical\""));
    }

    #[test]
    fn unsupported_native_endpoint_returns_typed_error() {
        let response = response_for(
            "POST",
            "/api/rerank",
            r#"{"query":"rust","documents":["rust api"]}"#,
        );
        assert_eq!(response.status_code, 422);
        assert!(response.body.contains("unsupported_runtime"));
    }
}
