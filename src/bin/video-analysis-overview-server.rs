use clap::Parser;
use runtime_core::{
    Diagnostic, DiagnosticSeverity, OperationId, PackageSurface, SurfaceRequest, SurfaceResponse,
};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};

#[derive(Debug, Parser)]
#[command(
    name = "video-analysis-overview-server",
    version,
    about = "Aggregate Rust API server for the overview package catalog"
)]
struct Args {
    /// Address to bind, for example 127.0.0.1:3000.
    #[arg(long, default_value = "127.0.0.1:3000")]
    addr: String,
}

#[derive(Debug, Clone, Copy)]
struct ModuleInfo {
    package: &'static str,
    import_path: &'static str,
    domain: &'static str,
    linked: bool,
    required_feature: Option<&'static str>,
}

#[derive(Debug)]
struct Request {
    method: String,
    path: String,
    query: HashMap<String, String>,
    headers: HashMap<String, String>,
    body: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct HttpResponse {
    status_code: u16,
    reason: &'static str,
    content_type: &'static str,
    body: String,
}

fn main() -> io::Result<()> {
    let args = Args::parse();
    eprintln!(
        "video-analysis-overview-server listening on http://{}",
        args.addr
    );
    serve(&args.addr)
}

fn serve(addr: &str) -> io::Result<()> {
    let listener = TcpListener::bind(addr)?;
    for stream in listener.incoming() {
        if let Err(error) = handle_stream(stream?) {
            if is_client_disconnect(&error) {
                eprintln!("ignored client disconnect: {error}");
            } else {
                return Err(error);
            }
        }
    }
    Ok(())
}

fn is_client_disconnect(error: &io::Error) -> bool {
    matches!(
        error.kind(),
        io::ErrorKind::BrokenPipe | io::ErrorKind::ConnectionReset | io::ErrorKind::UnexpectedEof
    )
}

fn response_for(request: &Request) -> HttpResponse {
    if request.method == "OPTIONS" {
        return HttpResponse {
            status_code: 204,
            reason: "No Content",
            content_type: "application/json",
            body: String::new(),
        };
    }

    if let Some((package, nested_path)) = request
        .path
        .strip_prefix("/api/rust/packages/")
        .and_then(split_package_path)
    {
        return package_response(&request.method, package, nested_path, &request.body);
    }

    match (request.method.as_str(), request.path.as_str()) {
        ("GET", "/health") => selected_module(request)
            .map(package_health)
            .unwrap_or_else(overview_health),
        ("GET", "/api/package") => selected_module(request)
            .map(package_metadata_response)
            .unwrap_or_else(|| json_response(200, "OK", overview_metadata_value())),
        ("GET", "/api/operations") => selected_module(request)
            .map(package_operations_response)
            .unwrap_or_else(|| json_response(200, "OK", json!([]))),
        ("GET", "/api/benchmarks") => selected_module(request)
            .map(|module| json_response(200, "OK", json!(benchmark_catalog_for(module))))
            .unwrap_or_else(|| json_response(200, "OK", json!([]))),
        ("GET", "/api/schema") => selected_module(request)
            .map(package_schema_response)
            .unwrap_or_else(|| json_response(200, "OK", overview_schema_value())),
        ("POST", "/api/run") => selected_module(request)
            .map(|module| package_run_response(module, &request.body))
            .unwrap_or_else(|| json_response(200, "OK", overview_run_value(&request.body))),
        ("GET", "/api/modules") | ("GET", "/api/rust/modules") => {
            json_response(200, "OK", json!(modules_value()))
        }
        ("GET", "/api/smoke") | ("GET", "/api/rust/smoke") => {
            json_response(200, "OK", smoke_value())
        }
        ("GET", "/api/rust/packages") | ("GET", "/api/packages") => {
            if let Some(name) = request.query.get("name") {
                match module_by_package_or_library(name) {
                    Some(module) => package_metadata_response(module),
                    None => json_response(
                        404,
                        "Not Found",
                        json!({ "message": format!("unknown package `{name}`") }),
                    ),
                }
            } else {
                json_response(
                    200,
                    "OK",
                    json!(MODULES
                        .iter()
                        .map(package_metadata_value)
                        .collect::<Vec<_>>()),
                )
            }
        }
        _ => json_response(
            404,
            "Not Found",
            json!({
                "error": "not found",
                "path": request.path
            }),
        ),
    }
}

fn package_response(method: &str, package: &str, path: &str, body: &str) -> HttpResponse {
    let Some(module) = module_by_package_or_library(package) else {
        return json_response(
            404,
            "Not Found",
            json!({ "message": format!("unknown package `{package}`") }),
        );
    };

    match (method, path) {
        ("GET", "/health") => package_health(module),
        ("GET", "/api/package") => package_metadata_response(module),
        ("GET", "/api/schema") => package_schema_response(module),
        ("GET", "/api/operations") => package_operations_response(module),
        ("GET", "/api/models") => json_response(200, "OK", json!(model_catalog_for(module, None))),
        ("GET", "/api/benchmarks") => {
            json_response(200, "OK", json!(benchmark_catalog_for(module)))
        }
        ("GET", path) if path.starts_with("/api/models/") => {
            let task = path.trim_start_matches("/api/models/");
            json_response(200, "OK", json!(model_catalog_for(module, Some(task))))
        }
        ("POST", "/api/run") => package_run_response(module, body),
        ("POST", path) if path.starts_with("/api/") => {
            let operation = path.trim_start_matches("/api/");
            package_run_request_response(
                module,
                SurfaceRequest {
                    operation: OperationId::new(operation),
                    input: parse_json_or_empty(body),
                },
            )
        }
        _ => json_response(
            404,
            "Not Found",
            json!({
                "error": "not found",
                "package": module.package,
                "path": path
            }),
        ),
    }
}

fn selected_module(request: &Request) -> Option<ModuleInfo> {
    for key in ["package", "library", "name"] {
        if let Some(value) = request
            .query
            .get(key)
            .and_then(|value| module_by_package_or_library(value))
        {
            return Some(value);
        }
    }

    request
        .headers
        .get("referer")
        .and_then(|referer| module_from_referer(referer))
        .or_else(|| module_from_body(&request.body))
}

fn module_from_referer(referer: &str) -> Option<ModuleInfo> {
    let marker = referer
        .split_once("/wrappers/")
        .or_else(|| referer.split_once("/services/"))?
        .1;
    let slug = marker.split('/').next().unwrap_or_default();
    MODULES
        .iter()
        .copied()
        .find(|module| slugify(module.package) == slug)
}

fn module_from_body(body: &str) -> Option<ModuleInfo> {
    let value = serde_json::from_str::<Value>(body).ok()?;
    ["package", "library", "name"]
        .iter()
        .find_map(|key| value.get(key)?.as_str())
        .and_then(module_by_package_or_library)
}

fn module_by_package_or_library(value: &str) -> Option<ModuleInfo> {
    let normalized = value.strip_suffix("-server").unwrap_or(value);
    MODULES
        .iter()
        .copied()
        .find(|module| module.package == normalized || slugify(module.package) == normalized)
}

fn adapter_package_base(package: &str) -> &str {
    package.strip_prefix("moritzbrantner-").unwrap_or(package)
}

fn split_package_path(value: &str) -> Option<(&str, &str)> {
    let slash = value.find('/')?;
    Some((&value[..slash], &value[slash..]))
}

fn overview_health() -> HttpResponse {
    json_response(
        200,
        "OK",
        json!({
            "ok": true,
            "package": "video-analysis-overview-server",
            "library": "video-analysis",
            "moduleCount": MODULES.len(),
            "linkedModuleCount": MODULES.iter().filter(|module| module.linked).count()
        }),
    )
}

fn package_health(module: ModuleInfo) -> HttpResponse {
    let adapter_base = adapter_package_base(module.package);
    json_response(
        if module.linked { 200 } else { 503 },
        if module.linked {
            "OK"
        } else {
            "Service Unavailable"
        },
        json!({
            "ok": module.linked,
            "package": format!("{adapter_base}-server"),
            "library": module.package,
            "domain": module.domain,
            "linked": module.linked,
            "requiredFeature": module.required_feature
        }),
    )
}

fn overview_metadata_value() -> Value {
    json!({
        "package": "video-analysis-overview-server",
        "surface": "api",
        "library": "video-analysis",
        "libraryImport": "use video_analysis",
        "endpoints": [
            "GET /health",
            "GET /api/modules",
            "GET /api/smoke",
            "GET /api/rust/packages",
            "GET /api/rust/packages/{library}/health",
            "GET /api/rust/packages/{library}/api/package",
            "GET /api/rust/packages/{library}/api/schema",
            "POST /api/rust/packages/{library}/api/run"
        ]
    })
}

fn package_metadata_response(module: ModuleInfo) -> HttpResponse {
    json_response(200, "OK", package_metadata_value(&module))
}

fn package_operations_response(module: ModuleInfo) -> HttpResponse {
    match package_surface_for(module) {
        Some(surface) => json_response(200, "OK", json!(surface.operations)),
        None if module.linked => json_response(200, "OK", json!([])),
        None => json_response(
            503,
            "Service Unavailable",
            json!({
                "package": format!("{}-server", module.package),
                "library": module.package,
                "accepted": false,
                "requiredFeature": module.required_feature,
                "note": "This optional package is not linked into the running overview server."
            }),
        ),
    }
}

fn package_metadata_value(module: &ModuleInfo) -> Value {
    let adapter_base = adapter_package_base(module.package);
    let surface = package_surface_for(*module);
    let operations = surface
        .as_ref()
        .map(|surface| json!(surface.operations))
        .unwrap_or_else(|| json!([]));
    let capabilities = surface
        .as_ref()
        .map(|surface| json!(surface.capabilities))
        .unwrap_or_else(|| json!({}));
    let version = surface
        .as_ref()
        .map(|surface| surface.version.as_str())
        .unwrap_or(env!("CARGO_PKG_VERSION"));
    let endpoints = vec![
        "GET /health",
        "GET /api/package",
        "GET /api/schema",
        "GET /api/operations",
        "GET /api/benchmarks",
        "POST /api/run",
        "POST /api/<operation-id>",
    ];
    json!({
        "package": format!("{adapter_base}-server"),
        "surface": "api",
        "library": module.package,
        "version": version,
        "libraryImport": format!("use {}", module.import_path),
        "cliPackage": format!("{adapter_base}-cli"),
        "appPackage": format!("{adapter_base}-app"),
        "wasmPackage": format!("{adapter_base}-wasm"),
        "domain": module.domain,
        "linked": module.linked,
        "requiredFeature": module.required_feature,
        "endpoints": endpoints,
        "operations": operations,
        "capabilities": capabilities
    })
}

fn overview_schema_value() -> Value {
    json!({
        "openapi": "3.1.0",
        "info": {
            "title": "video-analysis overview API",
            "version": env!("CARGO_PKG_VERSION")
        },
        "paths": {
            "/health": { "get": { "summary": "Aggregate health check" } },
            "/api/modules": { "get": { "summary": "Imported Rust module list" } },
            "/api/smoke": { "get": { "summary": "Run representative facade smoke checks" } },
            "/api/rust/packages/{library}/api/run": { "post": { "summary": "Generic package operation entrypoint" } }
        }
    })
}

fn package_schema_response(module: ModuleInfo) -> HttpResponse {
    json_response(200, "OK", {
        let operation_paths = package_surface_for(module)
            .map(|surface| {
                surface
                    .operations
                    .into_iter()
                    .map(|operation| {
                        let path = format!("/api/{}", operation.id.as_str());
                        (
                            path,
                            json!({
                                "post": {
                                    "summary": operation.name,
                                    "description": operation.description,
                                    "requestBody": operation.input_schema,
                                    "responses": { "200": operation.output_schema }
                                }
                            }),
                        )
                    })
                    .collect::<serde_json::Map<_, _>>()
            })
            .unwrap_or_else(|| {
                serde_json::Map::from_iter([(
                    "/api/run".to_string(),
                    json!({ "post": { "summary": "Generic operation entrypoint" } }),
                )])
            });
        json!({
        "openapi": "3.1.0",
        "info": {
            "title": format!("{} API", module.package),
            "version": env!("CARGO_PKG_VERSION")
        },
        "paths": operation_paths
        })
    })
}

fn overview_run_value(body: &str) -> Value {
    json!({
        "package": "video-analysis-overview-server",
        "library": "video-analysis",
        "accepted": true,
        "input": body,
        "smoke": smoke_value()
    })
}

fn package_run_response(module: ModuleInfo, body: &str) -> HttpResponse {
    if !module.linked {
        return json_response(
            503,
            "Service Unavailable",
            json!({
                "package": format!("{}-server", module.package),
                "library": module.package,
                "accepted": false,
                "requiredFeature": module.required_feature,
                "note": "This optional package is not linked into the running overview server."
            }),
        );
    }

    let payload = match serde_json::from_str::<Value>(body) {
        Ok(value) => value,
        Err(error) => {
            return diagnostic_response(
                400,
                "Bad Request",
                &format!("{}-server", module.package),
                "invalid_request",
                &format!("invalid JSON: {error}"),
            );
        }
    };
    let operation = payload
        .get("operation")
        .and_then(Value::as_str)
        .unwrap_or("describe")
        .to_string();
    let input = payload
        .get("input")
        .cloned()
        .unwrap_or_else(|| payload.clone());

    package_run_request_response(
        module,
        SurfaceRequest {
            operation: OperationId::new(operation),
            input,
        },
    )
}

fn package_run_request_response(module: ModuleInfo, request: SurfaceRequest) -> HttpResponse {
    if !module.linked {
        return json_response(
            503,
            "Service Unavailable",
            json!({
                "package": format!("{}-server", module.package),
                "library": module.package,
                "accepted": false,
                "requiredFeature": module.required_feature,
                "note": "This optional package is not linked into the running overview server."
            }),
        );
    }

    match run_surface_operation_for(module, request) {
        Some(Ok(response)) => json_response(200, "OK", json!(response)),
        Some(Err(error)) => diagnostic_response(
            400,
            "Bad Request",
            &format!("{}-server", module.package),
            "operation_failed",
            &error,
        ),
        None => diagnostic_response(
            404,
            "Not Found",
            &format!("{}-server", module.package),
            "surface_not_found",
            &format!("no runtime surface is registered for `{}`", module.package),
        ),
    }
}

fn benchmark_catalog_for(_module: ModuleInfo) -> Vec<Value> {
    Vec::new()
}

fn model_catalog_for(module: ModuleInfo, task: Option<&str>) -> Vec<Value> {
    let entries = match module.package {
        package if package.starts_with("video-analysis-") => vec![model_catalog_entry(
            "deterministic-video",
            "Deterministic video runtime",
            "analyze",
            "deterministic",
            true,
            None,
            Some("Uses checked-in fixtures and local surface operations."),
        )],
        "comfyui-data" | "comfyui-models" | "comfyui-latents" | "image-analysis-comfyui" => vec![
            model_catalog_entry(
                "comfyui-workflow-fixture",
                "ComfyUI workflow fixture",
                "workflow",
                "comfyui",
                true,
                None,
                Some("Uses workflow JSON contracts without starting ComfyUI."),
            ),
            model_catalog_entry(
                "comfyui-local-server",
                "ComfyUI local server",
                "workflow",
                "external",
                false,
                Some("comfyui-workflow-fixture"),
                Some("Requires a running ComfyUI instance."),
            ),
        ],
        _ => Vec::new(),
    };
    filter_model_catalog(entries, task)
}

fn filter_model_catalog(entries: Vec<Value>, task: Option<&str>) -> Vec<Value> {
    entries
        .into_iter()
        .filter(|entry| {
            task.map(|task| entry.get("task").and_then(Value::as_str) == Some(task))
                .unwrap_or(true)
        })
        .collect()
}

fn model_catalog_entry(
    id: &str,
    label: &str,
    task: &str,
    runtime: &str,
    supported: bool,
    fallback: Option<&str>,
    note: Option<&str>,
) -> Value {
    json!({
        "id": id,
        "label": label,
        "task": task,
        "runtime": runtime,
        "supported": supported,
        "fallback": fallback,
        "note": note,
    })
}

fn smoke_value() -> Value {
    let point = three_d_processing_core::Point3::new(1.0, 2.0, 2.0);

    json!({
        "ok": true,
        "checks": {
            "pointNorm": (point.x * point.x + point.y * point.y + point.z * point.z).sqrt(),
            "moduleCount": MODULES.len()
        }
    })
}

fn modules_value() -> Vec<Value> {
    MODULES.iter().map(package_metadata_value).collect()
}

fn handle_stream(mut stream: TcpStream) -> io::Result<()> {
    let request = read_request(&stream)?;
    let response = response_for(&request);
    write_response(&mut stream, response)
}

fn read_request(stream: &TcpStream) -> io::Result<Request> {
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut request_line = String::new();
    reader.read_line(&mut request_line)?;

    let mut headers = HashMap::new();
    let mut content_length = 0usize;
    loop {
        let mut header = String::new();
        reader.read_line(&mut header)?;
        let trimmed = header.trim_end();
        if trimmed.is_empty() {
            break;
        }
        if let Some((name, value)) = trimmed.split_once(':') {
            let name = name.trim().to_ascii_lowercase();
            let value = value.trim().to_string();
            if name == "content-length" {
                content_length = value.parse().unwrap_or(0);
            }
            headers.insert(name, value);
        }
    }

    let mut body = vec![0; content_length];
    if content_length > 0 {
        reader.read_exact(&mut body)?;
    }
    let body = String::from_utf8_lossy(&body).into_owned();

    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or("GET").to_string();
    let target = parts.next().unwrap_or("/");
    let (path, query) = parse_target(target);

    Ok(Request {
        method,
        path,
        query,
        headers,
        body,
    })
}

fn parse_target(target: &str) -> (String, HashMap<String, String>) {
    let (path, query) = target.split_once('?').unwrap_or((target, ""));
    let query = query
        .split('&')
        .filter(|part| !part.is_empty())
        .filter_map(|part| {
            let (key, value) = part.split_once('=').unwrap_or((part, ""));
            Some((percent_decode(key)?, percent_decode(value)?))
        })
        .collect();
    (path.to_string(), query)
}

fn percent_decode(value: &str) -> Option<String> {
    let mut bytes = Vec::with_capacity(value.len());
    let mut iter = value.as_bytes().iter().copied();
    while let Some(byte) = iter.next() {
        if byte == b'%' {
            let high = iter.next()?;
            let low = iter.next()?;
            let decoded = u8::from_str_radix(std::str::from_utf8(&[high, low]).ok()?, 16).ok()?;
            bytes.push(decoded);
        } else if byte == b'+' {
            bytes.push(b' ');
        } else {
            bytes.push(byte);
        }
    }
    String::from_utf8(bytes).ok()
}

fn json_response(status_code: u16, reason: &'static str, value: Value) -> HttpResponse {
    HttpResponse {
        status_code,
        reason,
        content_type: "application/json",
        body: value.to_string(),
    }
}

fn diagnostic_response(
    status_code: u16,
    reason: &'static str,
    source: &str,
    code: &str,
    message: &str,
) -> HttpResponse {
    json_response(
        status_code,
        reason,
        json!({
            "diagnostics": [Diagnostic {
                severity: DiagnosticSeverity::Error,
                code: code.into(),
                message: message.to_string(),
                source: Some(source.to_string()),
                help: None,
            }]
        }),
    )
}

fn parse_json_or_empty(body: &str) -> Value {
    if body.trim().is_empty() {
        json!({})
    } else {
        serde_json::from_str(body).unwrap_or_else(|_| json!({ "raw": body }))
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

fn slugify(value: &str) -> String {
    let mut slug = String::new();
    let mut previous_dash = false;
    for character in value.chars() {
        if character.is_ascii_alphanumeric() {
            slug.push(character.to_ascii_lowercase());
            previous_dash = false;
        } else if !previous_dash {
            slug.push('-');
            previous_dash = true;
        }
    }
    slug.trim_matches('-').to_string()
}

fn package_surface_for(module: ModuleInfo) -> Option<PackageSurface> {
    if !module.linked {
        return None;
    }

    match module.package {
        "animation-core" => Some(animation_core::surface::package_surface()),
        "comfyui-data" => Some(comfyui_data::surface::package_surface()),
        "comfyui-latents" => Some(comfyui_latents::surface::package_surface()),
        "comfyui-models" => Some(comfyui_models::surface::package_surface()),
        "image-analysis-comfyui" => Some(image_analysis_comfyui::surface::package_surface()),
        "three-d-processing-core" => Some(three_d_processing_core::surface::package_surface()),
        "three-d-processing-io" => Some(three_d_processing_io::surface::package_surface()),
        "three-d-processing-mesh" => Some(three_d_processing_mesh::surface::package_surface()),
        "three-d-scene-svg" => Some(three_d_scene_svg::surface::package_surface()),
        "video-analysis-gaussian-splatting" => {
            Some(video_analysis_gaussian_splatting::surface::package_surface())
        }
        "video-analysis-mvs" => Some(video_analysis_mvs::surface::package_surface()),
        "video-analysis-radiance-fields" => {
            Some(video_analysis_radiance_fields::surface::package_surface())
        }
        "video-analysis-radiance-io" => {
            Some(video_analysis_radiance_io::surface::package_surface())
        }
        "video-analysis-radiance-pipeline" => {
            Some(video_analysis_radiance_pipeline::surface::package_surface())
        }
        "video-analysis-reconstruction" => {
            Some(video_analysis_reconstruction::surface::package_surface())
        }
        "video-analysis-sfm" => Some(video_analysis_sfm::surface::package_surface()),
        _ => None,
    }
}

fn run_surface_operation_for(
    module: ModuleInfo,
    request: SurfaceRequest,
) -> Option<Result<SurfaceResponse, String>> {
    if !module.linked {
        return None;
    }

    match module.package {
        "animation-core" => Some(animation_core::surface::run_surface_operation(request)),
        "comfyui-data" => Some(comfyui_data::surface::run_surface_operation(request)),
        "comfyui-latents" => Some(comfyui_latents::surface::run_surface_operation(request)),
        "comfyui-models" => Some(comfyui_models::surface::run_surface_operation(request)),
        "image-analysis-comfyui" => Some(image_analysis_comfyui::surface::run_surface_operation(
            request,
        )),
        "three-d-processing-core" => Some(three_d_processing_core::surface::run_surface_operation(
            request,
        )),
        "three-d-processing-io" => Some(three_d_processing_io::surface::run_surface_operation(
            request,
        )),
        "three-d-processing-mesh" => Some(three_d_processing_mesh::surface::run_surface_operation(
            request,
        )),
        "three-d-scene-svg" => Some(three_d_scene_svg::surface::run_surface_operation(request)),
        "video-analysis-gaussian-splatting" => {
            Some(video_analysis_gaussian_splatting::surface::run_surface_operation(request))
        }
        "video-analysis-mvs" => Some(video_analysis_mvs::surface::run_surface_operation(request)),
        "video-analysis-radiance-fields" => {
            Some(video_analysis_radiance_fields::surface::run_surface_operation(request))
        }
        "video-analysis-radiance-io" => Some(
            video_analysis_radiance_io::surface::run_surface_operation(request),
        ),
        "video-analysis-radiance-pipeline" => {
            Some(video_analysis_radiance_pipeline::surface::run_surface_operation(request))
        }
        "video-analysis-reconstruction" => {
            Some(video_analysis_reconstruction::surface::run_surface_operation(request))
        }
        "video-analysis-sfm"
            if request.operation.as_str()
                == video_analysis_sfm::surface::RECONSTRUCT_VIDEO_OPERATION =>
        {
            Some(video_analysis_sfm::reconstruct_video_surface_operation(
                request.input,
            ))
        }
        "video-analysis-sfm" => Some(video_analysis_sfm::surface::run_surface_operation(request)),
        _ => None,
    }
}

const MODULES: &[ModuleInfo] = &[
    ModuleInfo {
        package: "animation-core",
        import_path: "video_analysis::animation",
        domain: "animation",
        linked: true,
        required_feature: None,
    },
    ModuleInfo {
        package: "comfyui-data",
        import_path: "video_analysis::comfyui_data",
        domain: "comfyui",
        linked: true,
        required_feature: None,
    },
    ModuleInfo {
        package: "comfyui-latents",
        import_path: "video_analysis::comfyui_latents",
        domain: "comfyui",
        linked: true,
        required_feature: None,
    },
    ModuleInfo {
        package: "comfyui-models",
        import_path: "video_analysis::comfyui_models",
        domain: "comfyui",
        linked: true,
        required_feature: None,
    },
    ModuleInfo {
        package: "image-analysis-comfyui",
        import_path: "video_analysis::image_comfyui",
        domain: "image",
        linked: true,
        required_feature: None,
    },
    ModuleInfo {
        package: "three-d-processing-core",
        import_path: "video_analysis::three_d_core",
        domain: "three-d",
        linked: true,
        required_feature: None,
    },
    ModuleInfo {
        package: "three-d-processing-io",
        import_path: "video_analysis::three_d_io",
        domain: "three-d",
        linked: true,
        required_feature: None,
    },
    ModuleInfo {
        package: "three-d-processing-mesh",
        import_path: "video_analysis::three_d_mesh",
        domain: "three-d",
        linked: true,
        required_feature: None,
    },
    ModuleInfo {
        package: "three-d-scene-svg",
        import_path: "video_analysis::three_d_scene",
        domain: "three-d",
        linked: true,
        required_feature: None,
    },
    ModuleInfo {
        package: "video-analysis-gaussian-splatting",
        import_path: "video_analysis::gaussian_splatting",
        domain: "video",
        linked: true,
        required_feature: None,
    },
    ModuleInfo {
        package: "video-analysis-mvs",
        import_path: "video_analysis::mvs",
        domain: "video",
        linked: true,
        required_feature: None,
    },
    ModuleInfo {
        package: "video-analysis-radiance-fields",
        import_path: "video_analysis::radiance_fields",
        domain: "video",
        linked: true,
        required_feature: None,
    },
    ModuleInfo {
        package: "video-analysis-radiance-io",
        import_path: "video_analysis::radiance_io",
        domain: "video",
        linked: true,
        required_feature: None,
    },
    ModuleInfo {
        package: "video-analysis-radiance-pipeline",
        import_path: "video_analysis::radiance_pipeline",
        domain: "video",
        linked: true,
        required_feature: None,
    },
    ModuleInfo {
        package: "video-analysis-reconstruction",
        import_path: "video_analysis::reconstruction",
        domain: "video",
        linked: true,
        required_feature: None,
    },
    ModuleInfo {
        package: "video-analysis-sfm",
        import_path: "video_analysis::sfm",
        domain: "video",
        linked: true,
        required_feature: None,
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn infers_wrapper_package_from_referer() {
        let module = module_from_referer("http://127.0.0.1:5173/wrappers/comfyui-models/");
        assert_eq!(module.unwrap().package, "comfyui-models");
    }

    #[test]
    fn serves_selected_package_metadata() {
        let request = Request {
            method: "GET".to_string(),
            path: "/api/package".to_string(),
            query: HashMap::new(),
            headers: HashMap::from([(
                "referer".to_string(),
                "http://127.0.0.1:5173/wrappers/three-d-scene-svg/".to_string(),
            )]),
            body: String::new(),
        };
        let response = response_for(&request);
        assert_eq!(response.status_code, 200);
        assert!(response.body.contains("three-d-scene-svg-server"));
    }

    #[test]
    fn linked_modules_expose_package_operations() {
        for module in MODULES.iter().copied().filter(|module| module.linked) {
            let surface = package_surface_for(module)
                .unwrap_or_else(|| panic!("missing package surface for {}", module.package));
            assert!(
                !surface.operations.is_empty(),
                "missing operations for {}",
                module.package
            );
        }
    }

    #[test]
    fn client_disconnect_io_errors_are_ignored() {
        for kind in [
            io::ErrorKind::BrokenPipe,
            io::ErrorKind::ConnectionReset,
            io::ErrorKind::UnexpectedEof,
        ] {
            let error = io::Error::new(kind, "client closed connection");
            assert!(is_client_disconnect(&error), "{kind:?} should be ignored");
        }

        let error = io::Error::new(io::ErrorKind::PermissionDenied, "bind denied");
        assert!(!is_client_disconnect(&error));
    }

    #[test]
    fn serves_colmap_reconstruct_video_from_native_package_route() {
        let request = Request {
            method: "POST".to_string(),
            path: "/api/rust/packages/video-analysis-sfm/api/run".to_string(),
            query: HashMap::new(),
            headers: HashMap::new(),
            body: r#"{"operation":"video.colmap.reconstructVideo","input":{"videoPath":"prototypes/web/video-analysis-web/public/samples/video/missing-test-video.mp4"}}"#.to_string(),
        };
        let response = response_for(&request);
        assert_eq!(response.status_code, 200);
        assert!(response.body.contains("invalid_request"));
        assert!(response.body.contains("not readable"));
    }

    #[test]
    fn serves_package_model_catalog_from_package_route() {
        let request = Request {
            method: "GET".to_string(),
            path: "/api/rust/packages/comfyui-data/api/models".to_string(),
            query: HashMap::new(),
            headers: HashMap::new(),
            body: String::new(),
        };
        let response = response_for(&request);
        assert_eq!(response.status_code, 200);
        assert!(response.body.contains("comfyui-workflow-fixture"));
    }

    #[test]
    fn packages_without_models_return_empty_catalog() {
        let request = Request {
            method: "GET".to_string(),
            path: "/api/rust/packages/three-d-processing-core/api/models".to_string(),
            query: HashMap::new(),
            headers: HashMap::new(),
            body: String::new(),
        };
        let response = response_for(&request);
        assert_eq!(response.status_code, 200);
        assert_eq!(response.body, "[]");
    }
}
