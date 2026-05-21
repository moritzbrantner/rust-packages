use std::io::{self, BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use text_linguistics as _;
use text_linguistics::{analyze_text, LinguisticAnalysis, LinguisticAnalysisOptions};

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

    match analyze_text(text, &LinguisticAnalysisOptions::default()) {
        Ok(analysis) => json_response(200, "OK", analysis_payload(text, &analysis)),
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

fn analysis_payload(text: &str, analysis: &LinguisticAnalysis) -> serde_json::Value {
    serde_json::json!({
        "package": format!("{}-server", LIBRARY_CRATE),
        "library": LIBRARY_CRATE,
        "accepted": true,
        "operation": "analyze",
        "text": text,
        "profile": format!("{:?}", analysis.profile),
        "provenance": format!("{:?}", analysis.provenance),
        "confidence": analysis.confidence.get(),
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
            "/api/run": { "post": { "summary": "Generic operation entrypoint" } }
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
}
