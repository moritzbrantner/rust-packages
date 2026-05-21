use audio_generation_midi as _;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};

/// Wrapped library crate name.
pub const LIBRARY_CRATE: &str = "audio-generation-midi";
/// Adapter surface kind.
pub const SURFACE_KIND: &str = "api";
/// Rust import path for the wrapped crate.
pub const LIBRARY_IMPORT: &str = "use audio_generation_midi";
/// Companion CLI package name.
pub const CLI_PACKAGE: &str = "audio-generation-midi-cli";
/// Companion React app package name.
pub const APP_PACKAGE: &str = "audio-generation-midi-app";

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
        ("POST", "/api/run") => json_response(
            200,
            "OK",
            serde_json::json!({
                "package": format!("{}-server", LIBRARY_CRATE),
                "library": LIBRARY_CRATE,
                "accepted": true,
                "input": body,
                "note": "This generic adapter is ready for crate-specific operations."
            }),
        ),
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
}
