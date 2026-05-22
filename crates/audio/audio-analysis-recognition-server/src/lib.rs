use audio_analysis_models::{
    classify_audio, detect_audio_events, diarize_speakers, embed_audio, generate_audio,
    model_catalog, parse_task, schema_summary, separate_sources, transcribe_audio,
    AudioClassificationRequest, AudioEmbeddingRequest, AudioEventDetectionRequest,
    AudioGenerationRequest, SourceSeparationRequest, SpeakerDiarizationRequest,
    SpeechRecognitionRequest,
};
use audio_analysis_recognition as _;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};

/// Wrapped library crate name.
pub const LIBRARY_CRATE: &str = "audio-analysis-recognition";
/// Adapter surface kind.
pub const SURFACE_KIND: &str = "api";
/// Rust import path for the wrapped crate.
pub const LIBRARY_IMPORT: &str = "use audio_analysis_recognition";
/// Companion CLI package name.
pub const CLI_PACKAGE: &str = "audio-analysis-recognition-cli";
/// Companion React app package name.
pub const APP_PACKAGE: &str = "audio-analysis-recognition-app";

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
                    &format!("unknown audio task `{task}`"),
                ),
            }
        }
        ("POST", "/api/classify") => classify_response(body),
        ("POST", "/api/events") => events_response(body),
        ("POST", "/api/embed") => embed_response(body),
        ("POST", "/api/transcribe") => transcribe_response(body),
        ("POST", "/api/diarize") => diarize_response(body),
        ("POST", "/api/separate") => separate_response(body),
        ("POST", "/api/generate") => generate_response(body),
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

/// Returns JSON metadata for this server adapter.
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
            );
        }
    };

    match payload
        .get("operation")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("classify")
    {
        "classify" => classify_response(body),
        "events" | "detect-events" | "detect_events" => events_response(body),
        "embed" => embed_response(body),
        "transcribe" | "asr" => transcribe_response(body),
        "diarize" | "speakers" => diarize_response(body),
        "separate" | "separation" => separate_response(body),
        "generate" | "synthesis" => generate_response(body),
        "introspect" => json_response(
            200,
            "OK",
            serde_json::json!({
                "package": format!("{}-server", LIBRARY_CRATE),
                "library": LIBRARY_CRATE,
                "accepted": true,
                "models": model_catalog(None)
            }),
        ),
        operation => typed_error_response(
            400,
            "Bad Request",
            "invalid_request",
            &format!("unsupported operation `{operation}`"),
        ),
    }
}

fn classify_response(body: &str) -> HttpResponse {
    match serde_json::from_str::<AudioClassificationRequest>(body) {
        Ok(request) => result_response(classify_audio(request)),
        Err(error) => typed_error_response(
            400,
            "Bad Request",
            "invalid_request",
            &format!("invalid classify request: {error}"),
        ),
    }
}

fn events_response(body: &str) -> HttpResponse {
    match serde_json::from_str::<AudioEventDetectionRequest>(body) {
        Ok(request) => result_response(detect_audio_events(request)),
        Err(error) => typed_error_response(
            400,
            "Bad Request",
            "invalid_request",
            &format!("invalid events request: {error}"),
        ),
    }
}

fn embed_response(body: &str) -> HttpResponse {
    match serde_json::from_str::<AudioEmbeddingRequest>(body) {
        Ok(request) => result_response(embed_audio(request)),
        Err(error) => typed_error_response(
            400,
            "Bad Request",
            "invalid_request",
            &format!("invalid embed request: {error}"),
        ),
    }
}

fn transcribe_response(body: &str) -> HttpResponse {
    match serde_json::from_str::<SpeechRecognitionRequest>(body) {
        Ok(request) => result_response(transcribe_audio(request)),
        Err(error) => typed_error_response(
            400,
            "Bad Request",
            "invalid_request",
            &format!("invalid transcribe request: {error}"),
        ),
    }
}

fn diarize_response(body: &str) -> HttpResponse {
    match serde_json::from_str::<SpeakerDiarizationRequest>(body) {
        Ok(request) => result_response(diarize_speakers(request)),
        Err(error) => typed_error_response(
            400,
            "Bad Request",
            "invalid_request",
            &format!("invalid diarize request: {error}"),
        ),
    }
}

fn separate_response(body: &str) -> HttpResponse {
    match serde_json::from_str::<SourceSeparationRequest>(body) {
        Ok(request) => result_response(separate_sources(request)),
        Err(error) => typed_error_response(
            400,
            "Bad Request",
            "invalid_request",
            &format!("invalid separate request: {error}"),
        ),
    }
}

fn generate_response(body: &str) -> HttpResponse {
    match serde_json::from_str::<AudioGenerationRequest>(body) {
        Ok(request) => result_response(generate_audio(request)),
        Err(error) => typed_error_response(
            400,
            "Bad Request",
            "invalid_request",
            &format!("invalid generate request: {error}"),
        ),
    }
}

fn result_response<T: serde::Serialize>(result: video_analysis_core::Result<T>) -> HttpResponse {
    match result {
        Ok(value) => json_response(200, "OK", serde_json::json!(value)),
        Err(error) => {
            let message = error.to_string();
            if message.contains("unsupported_runtime") {
                typed_error_response(422, "Unprocessable Entity", "unsupported_runtime", &message)
            } else if message.contains("non-empty") || message.contains("must include") {
                typed_error_response(400, "Bad Request", "empty_input", &message)
            } else {
                typed_error_response(
                    500,
                    "Internal Server Error",
                    "model_output_mismatch",
                    &message,
                )
            }
        }
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
            "GET /api/models/{task}",
            "POST /api/classify",
            "POST /api/events",
            "POST /api/embed",
            "POST /api/transcribe",
            "POST /api/diarize",
            "POST /api/separate",
            "POST /api/generate",
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
            "/api/models": { "get": { "summary": "List audio model presets" } },
            "/api/models/{task}": { "get": { "summary": "List audio model presets for a task" } },
            "/api/classify": { "post": { "summary": "Audio classification" } },
            "/api/events": { "post": { "summary": "Audio event detection" } },
            "/api/embed": { "post": { "summary": "Audio embeddings" } },
            "/api/transcribe": { "post": { "summary": "Speech recognition" } },
            "/api/diarize": { "post": { "summary": "Speaker diarization" } },
            "/api/separate": { "post": { "summary": "Source separation" } },
            "/api/generate": { "post": { "summary": "Audio generation" } },
            "/api/run": { "post": { "summary": "Legacy generic operation entrypoint" } }
        },
        "components": {
            "schemas": {
                "audioModels": schema_summary()
            }
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
    fn model_endpoint_lists_audio_presets() {
        let response = response_for("GET", "/api/models", "");
        assert_eq!(response.status_code, 200);
        assert!(response.body.contains("ast-audioset"));
    }

    #[test]
    fn classify_endpoint_runs_fallback() {
        let response = response_for(
            "POST",
            "/api/classify",
            r#"{"features":{"rms":0.1,"peak":0.2,"spectralCentroidHz":1800},"labels":["speech","music"],"model":{"fallbackPolicy":"heuristic_fallback"}}"#,
        );
        assert_eq!(response.status_code, 200);
        assert!(response.body.contains("\"operation\":\"classify\""));
        assert!(response.body.contains("\"runtime\":\"spectral\""));
    }
}
