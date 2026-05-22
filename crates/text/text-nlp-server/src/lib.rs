use std::io::{self, BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};

use text_nlp_tasks::{
    analyze_sentiment, answer_question, classify_text, embed_texts, model_catalog, parse_task,
    rerank, schema_summary, summarize, zero_shot_classify, EmbeddingRequest,
    QuestionAnsweringRequest, RerankRequest, SentimentRequest, SummaryRequest,
    TextClassificationRequest, ZeroShotClassificationRequest,
};

pub const LIBRARY_CRATE: &str = "text-nlp-tasks";
pub const SURFACE_KIND: &str = "api";
pub const LIBRARY_IMPORT: &str = "use text_nlp_tasks";
pub const CLI_PACKAGE: &str = "text-nlp-cli";
pub const APP_PACKAGE: &str = "text-nlp-app";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpResponse {
    pub status_code: u16,
    pub reason: &'static str,
    pub content_type: &'static str,
    pub body: String,
}

pub fn serve(addr: &str) -> io::Result<()> {
    let listener = TcpListener::bind(addr)?;
    for stream in listener.incoming() {
        handle_stream(stream?)?;
    }
    Ok(())
}

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
                "package": "text-nlp-server",
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

pub fn package_metadata_json() -> String {
    package_metadata_value().to_string()
}

fn run_response(body: &str) -> HttpResponse {
    let payload = match serde_json::from_str::<serde_json::Value>(body) {
        Ok(value) => value,
        Err(error) => {
            return typed_error_response(
                400,
                "Bad Request",
                "invalid_request",
                &format!("invalid JSON: {error}"),
            )
        }
    };
    match payload
        .get("operation")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("classify")
    {
        "classify" => classify_response(body),
        "sentiment" => sentiment_response(body),
        "embed" => embed_response(body),
        "zero-shot" | "zeroShot" | "zero_shot" => zero_shot_response(body),
        "summarize" => summarize_response(body),
        "rerank" => rerank_response(body),
        "question-answer" | "questionAnswer" | "question_answer" => question_answer_response(body),
        operation => typed_error_response(
            400,
            "Bad Request",
            "invalid_request",
            &format!("unsupported text NLP operation `{operation}`"),
        ),
    }
}

fn classify_response(body: &str) -> HttpResponse {
    match serde_json::from_str::<TextClassificationRequest>(body) {
        Ok(request) => result_response(classify_text(request)),
        Err(error) => invalid_body("classify", error),
    }
}

fn sentiment_response(body: &str) -> HttpResponse {
    match serde_json::from_str::<SentimentRequest>(body) {
        Ok(request) => result_response(analyze_sentiment(request)),
        Err(error) => invalid_body("sentiment", error),
    }
}

fn embed_response(body: &str) -> HttpResponse {
    match serde_json::from_str::<EmbeddingRequest>(body) {
        Ok(request) => result_response(embed_texts(request)),
        Err(error) => invalid_body("embed", error),
    }
}

fn zero_shot_response(body: &str) -> HttpResponse {
    match serde_json::from_str::<ZeroShotClassificationRequest>(body) {
        Ok(request) => result_response(zero_shot_classify(request)),
        Err(error) => invalid_body("zero-shot", error),
    }
}

fn summarize_response(body: &str) -> HttpResponse {
    match serde_json::from_str::<SummaryRequest>(body) {
        Ok(request) => result_response(summarize(request)),
        Err(error) => invalid_body("summarize", error),
    }
}

fn rerank_response(body: &str) -> HttpResponse {
    match serde_json::from_str::<RerankRequest>(body) {
        Ok(request) => result_response(rerank(request)),
        Err(error) => invalid_body("rerank", error),
    }
}

fn question_answer_response(body: &str) -> HttpResponse {
    match serde_json::from_str::<QuestionAnsweringRequest>(body) {
        Ok(request) => result_response(answer_question(request)),
        Err(error) => invalid_body("question-answer", error),
    }
}

fn invalid_body(operation: &str, error: serde_json::Error) -> HttpResponse {
    typed_error_response(
        400,
        "Bad Request",
        "invalid_request",
        &format!("invalid {operation} request: {error}"),
    )
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

fn package_metadata_value() -> serde_json::Value {
    serde_json::json!({
        "package": "text-nlp-server",
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
            "title": "text-nlp API",
            "version": env!("CARGO_PKG_VERSION")
        },
        "paths": {
            "/health": { "get": { "summary": "Health check" } },
            "/api/package": { "get": { "summary": "Package metadata" } },
            "/api/schema": { "get": { "summary": "API schema" } },
            "/api/models": { "get": { "summary": "List NLP model presets" } },
            "/api/models/{task}": { "get": { "summary": "List NLP model presets for a task" } },
            "/api/classify": { "post": { "summary": "Text classification" } },
            "/api/sentiment": { "post": { "summary": "Sentiment analysis" } },
            "/api/embed": { "post": { "summary": "Text embeddings" } },
            "/api/zero-shot": { "post": { "summary": "Zero-shot classification" } },
            "/api/summarize": { "post": { "summary": "Extractive summarization" } },
            "/api/rerank": { "post": { "summary": "Document reranking" } },
            "/api/question-answer": { "post": { "summary": "Question answering" } },
            "/api/run": { "post": { "summary": "Generic text NLP operation entrypoint" } },
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
            "package": "text-nlp-server",
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
    fn package_metadata_mentions_wrapped_library() {
        let response = response_for("GET", "/api/package", "");
        assert_eq!(response.status_code, 200);
        assert!(response.body.contains("text-nlp-tasks"));
    }
}
