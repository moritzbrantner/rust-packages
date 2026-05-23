use std::io::{self, BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};

use runtime_contracts::{Diagnostic, DiagnosticSeverity, OperationId, SurfaceRequest};

/// Wrapped library crate name.
pub const LIBRARY_CRATE: &str = "image-analysis-ocr";
/// Adapter surface kind.
pub const SURFACE_KIND: &str = "api";
/// Rust import path for the wrapped crate.
pub const LIBRARY_IMPORT: &str = "use image_analysis_ocr";
/// Companion CLI package name.
pub const CLI_PACKAGE: &str = "image-analysis-ocr-cli";
/// Companion React app package name.
pub const APP_PACKAGE: &str = "image-analysis-ocr-app";
/// Companion WASM package name.
pub const WASM_PACKAGE: &str = "image-analysis-ocr-wasm";

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
                "package": format!("{}-server", LIBRARY_CRATE),
                "library": LIBRARY_CRATE
            }),
        ),
        ("GET", "/api/package") => json_response(200, "OK", package_metadata_value()),
        ("GET", "/api/schema") => json_response(200, "OK", schema_value()),
        ("GET", "/api/operations") => json_response(
            200,
            "OK",
            serde_json::json!(image_analysis_ocr::surface::package_surface().operations),
        ),
        ("POST", "/api/run") => run_response(body),
        ("POST", path) if path.starts_with("/api/") => {
            let operation = path.trim_start_matches("/api/");
            run_request(SurfaceRequest {
                operation: OperationId::new(operation),
                input: parse_json_or_empty(body),
            })
        }
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

fn package_metadata_value() -> serde_json::Value {
    let surface = image_analysis_ocr::surface::package_surface();
    serde_json::json!({
        "package": format!("{}-server", LIBRARY_CRATE),
        "surface": SURFACE_KIND,
        "library": LIBRARY_CRATE,
        "libraryImport": LIBRARY_IMPORT,
        "cliPackage": CLI_PACKAGE,
        "appPackage": APP_PACKAGE,
        "wasmPackage": WASM_PACKAGE,
        "endpoints": [
            "GET /health",
            "GET /api/package",
            "GET /api/schema",
            "GET /api/operations",
            "POST /api/run",
            "POST /api/<operation-id>"
        ],
        "operations": surface.operations
    })
}

fn schema_value() -> serde_json::Value {
    let operations = image_analysis_ocr::surface::package_surface()
        .operations
        .into_iter()
        .map(|operation| {
            let path = format!("/api/{}", operation.id.as_str());
            (
                path,
                serde_json::json!({
                    "post": {
                        "summary": operation.name,
                        "description": operation.description,
                        "requestBody": operation.input_schema,
                        "responses": {"200": operation.output_schema}
                    }
                }),
            )
        })
        .collect::<serde_json::Map<_, _>>();

    serde_json::json!({
        "openapi": "3.1.0",
        "info": {
            "title": format!("{} API", LIBRARY_CRATE),
            "version": env!("CARGO_PKG_VERSION")
        },
        "paths": operations
    })
}

fn run_response(body: &str) -> HttpResponse {
    let payload = match serde_json::from_str::<serde_json::Value>(body) {
        Ok(value) => value,
        Err(error) => {
            return diagnostic_response(
                400,
                "Bad Request",
                "invalid_request",
                &format!("invalid JSON: {error}"),
            );
        }
    };
    let operation = payload
        .get("operation")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("describe")
        .to_string();
    let input = payload
        .get("input")
        .cloned()
        .unwrap_or_else(|| payload.clone());
    run_request(SurfaceRequest {
        operation: OperationId::new(operation),
        input,
    })
}

fn run_request(request: SurfaceRequest) -> HttpResponse {
    match image_analysis_ocr::surface::run_surface_operation(request) {
        Ok(response) => json_response(200, "OK", serde_json::json!(response)),
        Err(error) => diagnostic_response(400, "Bad Request", "operation_failed", &error),
    }
}

fn diagnostic_response(
    status_code: u16,
    reason: &'static str,
    code: &str,
    message: &str,
) -> HttpResponse {
    json_response(
        status_code,
        reason,
        serde_json::json!({
            "diagnostics": [Diagnostic {
                severity: DiagnosticSeverity::Error,
                code: code.into(),
                message: message.to_string(),
                source: Some(format!("{}-server", LIBRARY_CRATE)),
                help: None,
            }]
        }),
    )
}

fn parse_json_or_empty(body: &str) -> serde_json::Value {
    if body.trim().is_empty() {
        serde_json::json!({})
    } else {
        serde_json::from_str(body).unwrap_or_else(|_| serde_json::json!({"raw": body}))
    }
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
