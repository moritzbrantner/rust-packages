use clap::Parser;
use runtime_core::{
    Diagnostic, DiagnosticSeverity, OperationId, PackageSurface, SurfaceRequest, SurfaceResponse,
};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};

#[allow(unused_imports)]
use video_analysis::{
    animation, audio_core, audio_fourier, audio_io, audio_midi, audio_pitch, audio_processing,
    audio_recognition, audio_rhythm, audio_separation, audio_speakers, audio_synthesis,
    audio_transcription, comfyui_data, comfyui_latents, comfyui_models, data, dataset_records,
    dense, editing, features, ffmpeg, finance, gaussian_splatting, geo_clustering, geo_core,
    geo_io_geojson, geo_viz, geometry2d, graph_core, image_captioning, image_classification,
    image_comfyui, image_core, image_detection, image_embeddings, image_io, image_ocr,
    image_processing, image_segmentation, image_synthesis, ingest, inversion, jobs, linear,
    maps_kernels, model_runtime, mvs, numbers, output, posture, posture_io, radiance_fields,
    radiance_io, radiance_pipeline, recognition, reconstruction, sfm, signal, sparse, split, stats,
    storage, synthesis, tensor_data, text_analysis, text_classification, text_core,
    text_embeddings, text_generation, text_generation_linguistics, text_lexical, text_linguistics,
    text_model_runtime, text_question_answering, text_retrieval, text_transcripts, three_d_core,
    three_d_io, three_d_mesh, three_d_scene, tracking, transform, vector_core, vector_index,
    video_segmentation, Timebase, Timestamp,
};

#[cfg(feature = "onnx-backend")]
#[allow(unused_imports)]
use video_analysis::{image_onnx, onnx};

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
        match stream {
            Ok(stream) => {
                if let Err(error) = handle_stream(stream) {
                    if is_client_disconnect(&error) {
                        eprintln!("ignored client disconnect: {error}");
                    } else {
                        return Err(error);
                    }
                }
            }
            Err(error) => return Err(error),
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
        ("POST", "/api/entities") if module.package == "text-linguistics" => {
            text_linguistics_run_response(body)
        }
        ("POST", path) if module.package == "text-linguistics" && is_text_nlp_task_path(path) => {
            text_nlp_task_response(path, body)
        }
        ("POST", path)
            if module.package == "audio-analysis-recognition" && is_audio_model_task_path(path) =>
        {
            audio_model_task_response(path, body)
        }
        ("POST", "/api/transcribe") if module.package == "audio-analysis-transcription" => {
            audio_transcription_response(body)
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
    let normalized = geo_package_alias(normalized).unwrap_or(normalized);
    MODULES
        .iter()
        .copied()
        .find(|module| module.package == normalized || slugify(module.package) == normalized)
}

fn geo_package_alias(value: &str) -> Option<&'static str> {
    match value {
        "geo-core" => Some("moritzbrantner-geo-core"),
        "geo-io-geojson" => Some("moritzbrantner-geo-io-geojson"),
        "geo-clustering" => Some("moritzbrantner-geo-clustering"),
        "geo-viz" => Some("moritzbrantner-geo-viz"),
        _ => None,
    }
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
    let endpoints = if module.package == "text-linguistics" {
        vec![
            "GET /health",
            "GET /api/package",
            "GET /api/schema",
            "GET /api/operations",
            "GET /api/benchmarks",
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
            "POST /api/run",
        ]
    } else {
        vec![
            "GET /health",
            "GET /api/package",
            "GET /api/schema",
            "GET /api/operations",
            "GET /api/benchmarks",
            "POST /api/run",
            "POST /api/<operation-id>",
        ]
    };
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
    if module.package == "text-linguistics" {
        return json_response(
            200,
            "OK",
            json!({
                "openapi": "3.1.0",
                "info": {
                    "title": "text-linguistics API",
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
                    "/api/question-answer": { "post": { "summary": "Question answering" } },
                    "/api/run": { "post": { "summary": "Legacy generic operation entrypoint" } }
                },
                "components": {
                    "schemas": {
                        "textNlp": text_schema_summary()
                    }
                }
            }),
        );
    }
    if module.package == "audio-analysis-recognition" {
        return json_response(
            200,
            "OK",
            json!({
                "openapi": "3.1.0",
                "info": {
                    "title": "audio-analysis-recognition API",
                    "version": env!("CARGO_PKG_VERSION")
                },
                "paths": {
                    "/health": { "get": { "summary": "Health check" } },
                    "/api/package": { "get": { "summary": "Package metadata" } },
                    "/api/schema": { "get": { "summary": "API schema" } },
                    "/api/models": { "get": { "summary": "List audio task runtimes" } },
                    "/api/models/{task}": { "get": { "summary": "List audio task runtimes for a task" } },
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
                        "audioRuntimes": audio_schema_summary()
                    }
                }
            }),
        );
    }
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

fn is_text_nlp_task_path(path: &str) -> bool {
    matches!(
        path,
        "/api/classify"
            | "/api/sentiment"
            | "/api/embed"
            | "/api/zero-shot"
            | "/api/summarize"
            | "/api/rerank"
            | "/api/question-answer"
    )
}

fn is_audio_model_task_path(path: &str) -> bool {
    matches!(
        path,
        "/api/classify"
            | "/api/events"
            | "/api/embed"
            | "/api/diarize"
            | "/api/separate"
            | "/api/generate"
    )
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

fn text_linguistics_run_response(body: &str) -> HttpResponse {
    let payload = match serde_json::from_str::<Value>(body) {
        Ok(value) => value,
        Err(error) => {
            return json_response(
                400,
                "Bad Request",
                json!({
                    "package": "text-linguistics-server",
                    "library": "text-linguistics",
                    "accepted": false,
                    "error": format!("invalid JSON: {error}")
                }),
            );
        }
    };
    let text = payload
        .get("text")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if text.trim().is_empty() {
        return json_response(
            400,
            "Bad Request",
            json!({
                "package": "text-linguistics-server",
                "library": "text-linguistics",
                "accepted": false,
                "error": "request body must include a non-empty `text` string"
            }),
        );
    }

    match analyze_text_linguistics_for_payload(text, &payload) {
        Ok(analysis) => json_response(
            200,
            "OK",
            text_linguistics_payload(text, &analysis.analysis, analysis.model_metadata),
        ),
        Err(error) => json_response(
            500,
            "Internal Server Error",
            json!({
                "package": "text-linguistics-server",
                "library": "text-linguistics",
                "accepted": false,
                "error": error.to_string()
            }),
        ),
    }
}

fn text_nlp_task_response(path: &str, body: &str) -> HttpResponse {
    match path {
        "/api/classify" => {
            match serde_json::from_str::<text_classification::TextClassificationRequest>(body) {
                Ok(request) => nlp_result_response(text_classification::classify_text(request)),
                Err(error) => text_nlp_error_response(
                    400,
                    "Bad Request",
                    "invalid_request",
                    &error.to_string(),
                ),
            }
        }
        "/api/sentiment" => {
            match serde_json::from_str::<text_classification::SentimentRequest>(body) {
                Ok(request) => nlp_result_response(text_classification::analyze_sentiment(request)),
                Err(error) => text_nlp_error_response(
                    400,
                    "Bad Request",
                    "invalid_request",
                    &error.to_string(),
                ),
            }
        }
        "/api/embed" => text_surface_operation(
            text_embeddings::surface::run_surface_operation,
            "embeddings.embed",
            body,
        ),
        "/api/zero-shot" => match serde_json::from_str::<
            text_classification::ZeroShotClassificationRequest,
        >(body)
        {
            Ok(request) => nlp_result_response(text_classification::zero_shot_classify(request)),
            Err(error) => {
                text_nlp_error_response(400, "Bad Request", "invalid_request", &error.to_string())
            }
        },
        "/api/summarize" => text_surface_operation(
            text_lexical::surface::run_surface_operation,
            "lexical.analyze",
            body,
        ),
        "/api/rerank" => match serde_json::from_str::<text_retrieval::RerankRequest>(body) {
            Ok(request) => nlp_result_response(text_retrieval::rerank_documents(request)),
            Err(error) => {
                text_nlp_error_response(400, "Bad Request", "invalid_request", &error.to_string())
            }
        },
        "/api/question-answer" => match serde_json::from_str::<
            text_question_answering::QuestionAnsweringRequest,
        >(body)
        {
            Ok(request) => nlp_result_response(text_question_answering::answer_question(request)),
            Err(error) => {
                text_nlp_error_response(400, "Bad Request", "invalid_request", &error.to_string())
            }
        },
        _ => text_nlp_error_response(404, "Not Found", "not_found", "unknown NLP task endpoint"),
    }
}

fn text_surface_operation(
    runner: fn(SurfaceRequest) -> Result<SurfaceResponse, String>,
    operation: &str,
    body: &str,
) -> HttpResponse {
    let input = match serde_json::from_str::<Value>(body) {
        Ok(input) => input,
        Err(error) => {
            return text_nlp_error_response(
                400,
                "Bad Request",
                "invalid_request",
                &error.to_string(),
            )
        }
    };
    match runner(SurfaceRequest {
        operation: OperationId::new(operation),
        input,
    }) {
        Ok(response) => json_response(200, "OK", json!(response.value)),
        Err(error) => {
            text_nlp_error_response(422, "Unprocessable Entity", "operation_error", &error)
        }
    }
}

fn nlp_result_response<T: serde::Serialize>(
    result: video_analysis_core::Result<T>,
) -> HttpResponse {
    match result {
        Ok(value) => json_response(200, "OK", json!(value)),
        Err(error) => {
            let message = error.to_string();
            if message.contains("unsupported_runtime") {
                text_nlp_error_response(
                    422,
                    "Unprocessable Entity",
                    "unsupported_runtime",
                    &message,
                )
            } else if message.contains("non-empty") || message.contains("must include") {
                text_nlp_error_response(400, "Bad Request", "empty_input", &message)
            } else {
                text_nlp_error_response(
                    500,
                    "Internal Server Error",
                    "model_output_mismatch",
                    &message,
                )
            }
        }
    }
}

fn text_nlp_error_response(
    status_code: u16,
    reason: &'static str,
    code: &str,
    message: &str,
) -> HttpResponse {
    json_response(
        status_code,
        reason,
        json!({
            "package": "text-linguistics-server",
            "library": "text-linguistics",
            "accepted": false,
            "error": {
                "code": code,
                "message": message
            }
        }),
    )
}

fn text_model_catalog(task: Option<&str>) -> Vec<Value> {
    let mut entries = Vec::new();
    if task.is_none() || matches!(task, Some("classify" | "sentiment" | "zero-shot")) {
        let classification_task = task.and_then(text_classification::parse_task);
        entries.extend(
            text_classification::model_catalog(classification_task)
                .into_iter()
                .filter_map(|model| serde_json::to_value(model).ok()),
        );
    }
    if task.is_none() || task == Some("question-answer") {
        entries.extend(
            text_question_answering::model_catalog()
                .into_iter()
                .filter_map(|model| serde_json::to_value(model).ok()),
        );
    }
    entries
}

fn text_schema_summary() -> Value {
    json!({
        "tasks": [
            "classify",
            "sentiment",
            "embed",
            "zero-shot",
            "summarize",
            "rerank",
            "question-answer"
        ],
        "classification": text_classification::schema_summary(),
        "questionAnswering": text_question_answering::schema_summary(),
        "models": text_model_catalog(None)
    })
}

fn audio_model_catalog(task: Option<&str>) -> Vec<Value> {
    let entries = vec![
        audio_model_entry(
            "ast-audioset",
            "MIT/ast-finetuned-audioset-10-10-0.4593",
            "classify",
            "onnx",
            false,
            Some("spectral-audio-classifier"),
            Some("Hugging Face preset metadata is registered; native AST execution is explicit/fallback-gated."),
        ),
        audio_model_entry(
            "spectral-audio-classifier",
            "spectral_heuristic",
            "classify",
            "spectral",
            true,
            None,
            Some("Uses feature summaries for deterministic local classification."),
        ),
        audio_model_entry(
            "audioset-event-detector",
            "MIT/ast-finetuned-audioset-10-10-0.4593",
            "events",
            "onnx",
            false,
            Some("energy-event-detector"),
            Some("Windowed event schema is available; native model execution is not bundled."),
        ),
        audio_model_entry(
            "energy-event-detector",
            "energy_threshold",
            "events",
            "heuristic",
            true,
            None,
            Some("Detects high-energy windows from supplied frame summaries."),
        ),
        audio_model_entry(
            "clap-htsat-unfused",
            "laion/clap-htsat-unfused",
            "embed",
            "onnx",
            false,
            Some("spectral-audio-embedding"),
            Some("Use imported embeddings or explicit fallback until CLAP execution is wired."),
        ),
        audio_model_entry(
            "spectral-audio-embedding",
            "spectral_embedding",
            "embed",
            "spectral",
            true,
            None,
            Some("Builds deterministic vectors from feature summaries."),
        ),
        audio_model_entry(
            "imported-transcript",
            "imported-transcript",
            "transcribe",
            "imported",
            true,
            None,
            Some("Normalizes caller-supplied transcript segments or transcript contracts without native ASR."),
        ),
        audio_model_entry(
            "whisper-cpp",
            "openai/whisper-tiny.en",
            "transcribe",
            "whisper_cpp",
            false,
            Some("imported-transcript"),
            Some("Planned native provider through text-transcripts; default audio surfaces do not download models or execute Whisper."),
        ),
        audio_model_entry(
            "external-transcriber",
            "external-command",
            "transcribe",
            "external",
            false,
            Some("imported-transcript"),
            Some("Reference provider family for explicit external transcription tools; not executed by default routes."),
        ),
        audio_model_entry(
            "wav2vec2-base-960h",
            "facebook/wav2vec2-base-960h",
            "transcribe",
            "onnx",
            false,
            Some("imported-transcript"),
            Some("Generic transcription schema is available; native ONNX decoding is not wired."),
        ),
        audio_model_entry(
            "pyannote-speaker-diarization-3.1",
            "pyannote/speaker-diarization-3.1",
            "diarize",
            "external",
            false,
            Some("single-speaker-heuristic"),
            Some("Gated external model; use imported segments or heuristic fallback."),
        ),
        audio_model_entry(
            "single-speaker-heuristic",
            "single_speaker",
            "diarize",
            "heuristic",
            true,
            None,
            Some("Creates one speaker segment over the provided duration."),
        ),
        audio_model_entry(
            "demucs-music-separation",
            "facebook/demucs",
            "separate",
            "demucs",
            false,
            Some("demucs-stem-plan"),
            Some("Use audio-analysis-separation for command-backed Demucs execution."),
        ),
        audio_model_entry(
            "demucs-stem-plan",
            "stem_plan",
            "separate",
            "heuristic",
            true,
            None,
            Some("Returns requested stem descriptors without writing audio files."),
        ),
        audio_model_entry(
            "musicgen-small",
            "facebook/musicgen-small",
            "generate",
            "external",
            false,
            None,
            Some("Prompt schema is available; waveform generation is outside the default package app."),
        ),
    ];

    entries
        .into_iter()
        .filter(|entry| {
            task.map(|task| entry.get("task").and_then(Value::as_str) == Some(task))
                .unwrap_or(true)
        })
        .collect()
}

fn audio_model_entry(
    id: &str,
    model_id: &str,
    task: &str,
    runtime: &str,
    supported: bool,
    fallback: Option<&str>,
    note: Option<&str>,
) -> Value {
    json!({
        "id": id,
        "modelId": model_id,
        "task": task,
        "runtime": runtime,
        "supported": supported,
        "loadable": supported,
        "fallback": fallback,
        "requiredFeature": null,
        "requiredSetup": null,
        "smokeOperation": null,
        "note": note,
    })
}

fn benchmark_catalog_for(module: ModuleInfo) -> Vec<Value> {
    match module.package {
        "text-core" => text_benchmark_entries(&[
            ("tokenize", "Tokenize", "text.tokenize"),
            ("boundaries", "Boundaries", "text.boundaries"),
            ("statistics", "Statistics", "text.statistics"),
        ]),
        "text-lexical" => text_benchmark_entries(&[
            ("keywords", "Keywords", "lexical.keywords"),
            ("corpus-search", "Corpus Search", "lexical.corpusSearch"),
        ]),
        "text-embeddings" => text_benchmark_entries(&[
            ("hashed-embed", "Hashed Embed", "embeddings.embed"),
            (
                "semantic-search",
                "Semantic Search",
                "embeddings.semanticSearch",
            ),
        ]),
        "text-retrieval" => text_benchmark_entries(&[
            ("chunk", "Chunk", "retrieval.chunk"),
            ("hybrid-search", "Hybrid Search", "retrieval.search"),
        ]),
        "text-analysis" => text_benchmark_entries(&[
            ("document-report", "Document Report", "analysis.document"),
            ("corpus-report", "Corpus Report", "analysis.corpus"),
        ]),
        "text-linguistics" => text_benchmark_entries(&[
            ("fast-analysis", "Fast Analysis", "linguistics.analyze"),
            (
                "balanced-analysis",
                "Balanced Analysis",
                "linguistics.analyze",
            ),
            ("rich-analysis", "Rich Analysis", "linguistics.analyze"),
        ]),
        "text-model-runtime" => text_benchmark_entries(&[
            (
                "tokenizer-summary",
                "Tokenizer Summary",
                "runtime.tokenizeSummary",
            ),
            ("softmax", "Softmax", "runtime.softmax"),
        ]),
        "text-classification" => text_benchmark_entries(&[
            (
                "lexical-classify",
                "Lexical Classify",
                "classification.classify",
            ),
            ("sentiment", "Sentiment", "classification.sentiment"),
            ("zero-shot", "Zero-shot", "classification.zeroShot"),
        ]),
        "text-question-answering" => {
            text_benchmark_entries(&[("imported-span", "Imported Span", "qa.answer")])
        }
        "text-generation" => text_benchmark_entries(&[
            (
                "markov-predict",
                "Markov Predict",
                "generation.markovPredict",
            ),
            (
                "markov-generate",
                "Markov Generate",
                "generation.markovGenerate",
            ),
        ]),
        "text-generation-linguistics" => text_benchmark_entries(&[
            (
                "synthesize-from-analysis",
                "Synthesize",
                "generationLinguistics.synthesizeFromAnalysis",
            ),
            (
                "analysis-terms",
                "Terms",
                "generationLinguistics.analysisTerms",
            ),
        ]),
        "text-transcripts" => text_benchmark_entries(&[
            ("parse-srt", "Parse SRT", "transcripts.parse"),
            ("normalize", "Normalize", "transcripts.normalize"),
            ("format-srt", "Format SRT", "transcripts.formatSrt"),
        ]),
        _ => Vec::new(),
    }
}

fn text_benchmark_entries(entries: &[(&str, &str, &str)]) -> Vec<Value> {
    entries
        .iter()
        .map(|(id, label, operation)| {
            json!({
                "id": id,
                "label": label,
                "operation": operation,
                "iterations": 50,
                "runtimeModes": ["client-wasm", "overview-server", "standalone-server"]
            })
        })
        .collect()
}

fn audio_schema_summary() -> Value {
    json!({
        "tasks": ["classify", "events", "embed", "transcribe", "diarize", "separate", "generate"],
        "runtimes": audio_model_catalog(None),
        "models": audio_model_catalog(None),
        "registeredPresets": model_runtime::ModelPreset::ALL
            .iter()
            .map(|preset| preset.as_str().to_string())
            .filter(|preset| {
                preset.contains("audio")
                    || preset.contains("ast")
                    || preset.contains("clap")
                    || preset.contains("whisper")
                    || preset.contains("wav2vec")
                    || preset.contains("pyannote")
                    || preset.contains("demucs")
                    || preset.contains("musicgen")
            })
            .collect::<Vec<_>>()
    })
}

fn model_catalog_for(module: ModuleInfo, task: Option<&str>) -> Vec<Value> {
    let entries = match module.package {
        "text-linguistics" => text_model_catalog(task),
        "text-analysis" => vec![
            model_catalog_entry(
                "deterministic-text-analysis",
                "Deterministic text analysis",
                "analyze",
                "deterministic",
                true,
                None,
                Some("Default lexical, linguistic, and hashed embedding profile."),
            ),
            model_catalog_entry(
                "model-backed-text-analysis",
                "Model-backed text analysis",
                "analyze",
                "candle",
                true,
                Some("deterministic-text-analysis"),
                Some("Uses local model bundles when available through the overview server."),
            ),
        ],
        "text-classification" => text_model_catalog(task),
        "text-embeddings" => vec![
            model_catalog_entry(
                "hashed-text-embedding",
                "Hashed text embedding",
                "embed",
                "deterministic",
                true,
                None,
                Some("Pure Rust deterministic embedding fallback."),
            ),
            model_catalog_entry(
                "sentence-transformer-local",
                "Sentence transformer local",
                "embed",
                "candle",
                false,
                Some("hashed-text-embedding"),
                Some("Registered as a local model preset; execution is opt-in."),
            ),
        ],
        "text-question-answering" => text_model_catalog(task),
        "text-transcripts" => vec![
            model_catalog_entry(
                "whisper-tiny-en",
                "Whisper tiny English",
                "transcribe",
                "whisper_cpp",
                false,
                None,
                Some("Requires local whisper.cpp model files."),
            ),
            model_catalog_entry(
                "fixture-transcript",
                "Fixture transcript",
                "transcribe",
                "deterministic",
                true,
                None,
                Some("Uses checked-in transcript fixture style data."),
            ),
        ],
        "audio-analysis-recognition" => audio_model_catalog(task),
        "audio-analysis-separation" => vec![
            model_catalog_entry(
                "demucs-stem-plan",
                "Demucs stem plan",
                "separate",
                "heuristic",
                true,
                None,
                Some("Plans stems without invoking external Demucs."),
            ),
            model_catalog_entry(
                "demucs-local",
                "Demucs local",
                "separate",
                "external",
                false,
                Some("demucs-stem-plan"),
                Some("Requires external audio tooling."),
            ),
        ],
        "audio-analysis-speakers" => vec![
            model_catalog_entry(
                "single-speaker-heuristic",
                "Single speaker heuristic",
                "diarize",
                "heuristic",
                true,
                None,
                Some("Creates one speaker segment for quick testing."),
            ),
            model_catalog_entry(
                "pyannote-local",
                "Pyannote local",
                "diarize",
                "external",
                false,
                Some("single-speaker-heuristic"),
                Some("Requires gated external model access."),
            ),
        ],
        package if package.starts_with("audio-") => vec![model_catalog_entry(
            "deterministic-audio",
            "Deterministic audio runtime",
            "analyze",
            "deterministic",
            true,
            None,
            Some("Uses frame summaries and local feature calculations."),
        )],
        "image-analysis-classification" => vec![
            model_catalog_entry(
                "color-histogram-classifier",
                "Color histogram classifier",
                "classify",
                "heuristic",
                true,
                None,
                Some("Deterministic image classification fallback."),
            ),
            model_catalog_entry(
                "mobilenet-onnx",
                "MobileNet ONNX",
                "classify",
                "onnx",
                false,
                Some("color-histogram-classifier"),
                Some("Requires ONNX backend wiring."),
            ),
        ],
        "image-analysis-detection" => vec![
            model_catalog_entry(
                "mask-proposal-demo",
                "Mask proposal demo",
                "detect",
                "heuristic",
                true,
                None,
                Some("Pure Rust detection-style fixture output."),
            ),
            model_catalog_entry(
                "yolo-onnx",
                "YOLO ONNX",
                "detect",
                "onnx",
                false,
                Some("mask-proposal-demo"),
                Some("Requires ONNX backend wiring."),
            ),
        ],
        "image-analysis-segmentation" => vec![
            model_catalog_entry(
                "threshold-segmentation",
                "Threshold segmentation",
                "segment",
                "heuristic",
                true,
                None,
                Some("Local deterministic segmentation fallback."),
            ),
            model_catalog_entry(
                "sam-onnx",
                "Segment Anything ONNX",
                "segment",
                "onnx",
                false,
                Some("threshold-segmentation"),
                Some("Requires ONNX backend wiring."),
            ),
        ],
        "image-analysis-ocr" => vec![
            model_catalog_entry(
                "fixture-ocr",
                "Fixture OCR",
                "ocr",
                "heuristic",
                true,
                None,
                Some("Uses OCR-compatible text output fixtures."),
            ),
            model_catalog_entry(
                "tesseract-local",
                "Tesseract local",
                "ocr",
                "external",
                false,
                Some("fixture-ocr"),
                Some("Requires external OCR tooling."),
            ),
        ],
        "image-analysis-captioning" => vec![
            model_catalog_entry(
                "metadata-captioner",
                "Metadata captioner",
                "caption",
                "heuristic",
                true,
                None,
                Some("Builds captions from supplied image metadata."),
            ),
            model_catalog_entry(
                "blip-onnx",
                "BLIP ONNX",
                "caption",
                "onnx",
                false,
                Some("metadata-captioner"),
                Some("Requires ONNX backend wiring."),
            ),
        ],
        package if package.starts_with("image-") => vec![model_catalog_entry(
            "deterministic-image",
            "Deterministic image runtime",
            "analyze",
            "deterministic",
            true,
            None,
            Some("Uses local image metadata and pixel summaries."),
        )],
        "video-analysis-recognition" => vec![
            model_catalog_entry(
                "scene-label-heuristic",
                "Scene label heuristic",
                "recognize",
                "heuristic",
                true,
                None,
                Some("Deterministic recognition fallback."),
            ),
            model_catalog_entry(
                "action-recognition-onnx",
                "Action recognition ONNX",
                "recognize",
                "onnx",
                false,
                Some("scene-label-heuristic"),
                Some("Requires ONNX backend wiring."),
            ),
        ],
        "video-analysis-posture" => vec![
            model_catalog_entry(
                "stick-figure-posture",
                "Stick figure posture",
                "pose",
                "heuristic",
                true,
                None,
                Some("Keypoint-compatible deterministic fallback."),
            ),
            model_catalog_entry(
                "openpose-local",
                "OpenPose local",
                "pose",
                "external",
                false,
                Some("stick-figure-posture"),
                Some("Requires external posture tooling."),
            ),
        ],
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

fn audio_model_task_response(path: &str, body: &str) -> HttpResponse {
    match path {
        "/api/classify" => {
            match serde_json::from_str::<audio_analysis_recognition::AudioClassificationRequest>(
                body,
            ) {
                Ok(request) => {
                    audio_model_result_response(audio_analysis_recognition::classify_audio(request))
                }
                Err(error) => audio_model_error_response(
                    400,
                    "Bad Request",
                    "invalid_request",
                    &error.to_string(),
                ),
            }
        }
        "/api/events" => {
            match serde_json::from_str::<audio_analysis_recognition::AudioEventDetectionRequest>(
                body,
            ) {
                Ok(request) => audio_model_result_response(
                    audio_analysis_recognition::detect_audio_events(request),
                ),
                Err(error) => audio_model_error_response(
                    400,
                    "Bad Request",
                    "invalid_request",
                    &error.to_string(),
                ),
            }
        }
        "/api/embed" => {
            match serde_json::from_str::<audio_analysis_recognition::AudioEmbeddingRequest>(body) {
                Ok(request) => {
                    audio_model_result_response(audio_analysis_recognition::embed_audio(request))
                }
                Err(error) => audio_model_error_response(
                    400,
                    "Bad Request",
                    "invalid_request",
                    &error.to_string(),
                ),
            }
        }
        "/api/transcribe" => audio_transcription_response(body),
        "/api/diarize" => {
            match serde_json::from_str::<audio_analysis_speakers::SpeakerDiarizationRequest>(body) {
                Ok(request) => {
                    audio_model_result_response(audio_analysis_speakers::diarize_speakers(request))
                }
                Err(error) => audio_model_error_response(
                    400,
                    "Bad Request",
                    "invalid_request",
                    &error.to_string(),
                ),
            }
        }
        "/api/separate" => {
            match serde_json::from_str::<audio_analysis_separation::SourceSeparationRequest>(body) {
                Ok(request) => audio_model_result_response(
                    audio_analysis_separation::separate_sources(request),
                ),
                Err(error) => audio_model_error_response(
                    400,
                    "Bad Request",
                    "invalid_request",
                    &error.to_string(),
                ),
            }
        }
        "/api/generate" => {
            match serde_json::from_str::<audio_analysis_synthesis::AudioGenerationRequest>(body) {
                Ok(request) => {
                    audio_model_result_response(audio_analysis_synthesis::generate_audio(request))
                }
                Err(error) => audio_model_error_response(
                    400,
                    "Bad Request",
                    "invalid_request",
                    &error.to_string(),
                ),
            }
        }
        _ => audio_model_error_response(
            404,
            "Not Found",
            "not_found",
            "unknown audio model task endpoint",
        ),
    }
}

fn audio_transcription_response(body: &str) -> HttpResponse {
    match serde_json::from_str::<audio_transcription::TranscriptionPipelineRequest>(body) {
        Ok(request) => audio_transcription_result_response(audio_transcription::transcribe(request)),
        Err(error) => audio_transcription_error_response(
            400,
            "Bad Request",
            "invalid_request",
            &format!(
                "invalid audio transcription request: {error}; /api/transcribe now requires a real source path and provider config. Use /api/rust/packages/text-transcripts/api/run with operation `transcripts.normalize` for imported transcript normalization."
            ),
        ),
    }
}

fn audio_transcription_result_response(
    result: video_analysis_core::Result<audio_transcription::TranscriptionPipelineResponse>,
) -> HttpResponse {
    match result {
        Ok(value) => json_response(200, "OK", json!(value)),
        Err(error) => {
            let message = error.to_string();
            if message.contains("setup_error") {
                audio_transcription_error_response(400, "Bad Request", "setup_error", &message)
            } else if message.contains("timeout") {
                audio_transcription_error_response(504, "Gateway Timeout", "timeout", &message)
            } else {
                audio_transcription_error_response(
                    500,
                    "Internal Server Error",
                    "transcription_failed",
                    &message,
                )
            }
        }
    }
}

fn audio_transcription_error_response(
    status_code: u16,
    reason: &'static str,
    code: &str,
    message: &str,
) -> HttpResponse {
    json_response(
        status_code,
        reason,
        json!({
            "package": "audio-analysis-transcription-server",
            "library": "audio-analysis-transcription",
            "accepted": false,
            "error": {
                "code": code,
                "message": message
            }
        }),
    )
}

fn audio_model_result_response<T: serde::Serialize>(
    result: video_analysis_core::Result<T>,
) -> HttpResponse {
    match result {
        Ok(value) => json_response(200, "OK", json!(value)),
        Err(error) => {
            let message = error.to_string();
            if message.contains("unsupported_runtime") {
                audio_model_error_response(
                    422,
                    "Unprocessable Entity",
                    "unsupported_runtime",
                    &message,
                )
            } else if message.contains("non-empty") || message.contains("must include") {
                audio_model_error_response(400, "Bad Request", "empty_input", &message)
            } else {
                audio_model_error_response(
                    500,
                    "Internal Server Error",
                    "model_output_mismatch",
                    &message,
                )
            }
        }
    }
}

fn audio_model_error_response(
    status_code: u16,
    reason: &'static str,
    code: &str,
    message: &str,
) -> HttpResponse {
    json_response(
        status_code,
        reason,
        json!({
            "package": "audio-analysis-recognition-server",
            "library": "audio-analysis-recognition",
            "accepted": false,
            "error": {
                "code": code,
                "message": message
            }
        }),
    )
}

#[derive(Debug, Clone, Copy)]
struct TextLinguisticsModelMetadata {
    entity_recognition: &'static str,
    entity_model: Option<&'static str>,
}

#[derive(Debug)]
struct TextLinguisticsRunAnalysis {
    analysis: text_linguistics::LinguisticAnalysis,
    model_metadata: TextLinguisticsModelMetadata,
}

fn analyze_text_linguistics_for_payload(
    text: &str,
    payload: &Value,
) -> Result<TextLinguisticsRunAnalysis, String> {
    match payload
        .get("entityRecognition")
        .or_else(|| payload.get("modelMode"))
        .or_else(|| payload.get("mode"))
        .and_then(Value::as_str)
    {
        Some("heuristic") | Some("rules") => {
            let analysis = text_linguistics::analyze_text(
                text,
                &text_linguistics::LinguisticAnalysisOptions::heuristic(),
            )
            .map_err(|error| error.to_string())?;
            Ok(TextLinguisticsRunAnalysis {
                analysis,
                model_metadata: TextLinguisticsModelMetadata {
                    entity_recognition: "heuristic",
                    entity_model: None,
                },
            })
        }
        _ => {
            let config = text_linguistics_config_from_payload(payload);
            let model_metadata = text_linguistics_model_metadata_for_config(&config);
            let analysis = text_linguistics::TextNlpPipeline::new(config)
                .analyze_text(text)
                .map_err(|error| error.to_string())?;
            Ok(TextLinguisticsRunAnalysis {
                analysis,
                model_metadata,
            })
        }
    }
}

fn text_linguistics_config_from_payload(payload: &Value) -> text_linguistics::TextNlpConfig {
    match payload.get("profile").and_then(Value::as_str) {
        Some("fast") => text_linguistics::TextNlpConfig::fast(),
        Some("balanced") => text_linguistics::TextNlpConfig::balanced(),
        _ => text_linguistics::TextNlpConfig::rich(),
    }
}

fn text_linguistics_model_metadata_for_config(
    config: &text_linguistics::TextNlpConfig,
) -> TextLinguisticsModelMetadata {
    if matches!(config.profile, text_linguistics::AnalysisProfile::Fast) {
        TextLinguisticsModelMetadata {
            entity_recognition: "heuristic",
            entity_model: None,
        }
    } else {
        TextLinguisticsModelMetadata {
            entity_recognition: "local-model",
            entity_model: Some("bert-base-ner"),
        }
    }
}

fn text_linguistics_payload(
    text: &str,
    analysis: &text_linguistics::LinguisticAnalysis,
    model_metadata: TextLinguisticsModelMetadata,
) -> Value {
    json!({
        "package": "text-linguistics-server",
        "library": "text-linguistics",
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
            "primary": analysis.language.primary.as_ref().map(|prediction| json!({
                "language": prediction.language,
                "confidence": prediction.confidence,
                "script": prediction.script,
                "reason": prediction.reason
            })),
            "dominantScript": analysis.language.dominant_script,
            "isMixed": analysis.language.is_mixed,
            "tokenCount": analysis.language.token_count
        },
        "tokens": analysis.tokens.iter().enumerate().map(|(index, token)| json!({
            "index": index,
            "text": token.text,
            "normalized": token.normalized,
            "kind": format!("{:?}", token.kind),
            "start": token.span.char_start,
            "end": token.span.char_end
        })).collect::<Vec<_>>(),
        "sentences": analysis.sentences.iter().enumerate().map(|(index, sentence)| json!({
            "index": index,
            "text": sentence.text,
            "start": sentence.span.char_start,
            "end": sentence.span.char_end,
            "tokenCount": sentence.token_count
        })).collect::<Vec<_>>(),
        "lemmas": analysis.lemmas.iter().map(|lemma| json!({
            "tokenIndex": lemma.token_index,
            "token": analysis.tokens.get(lemma.token_index).map(|token| token.text.as_str()),
            "lemma": lemma.value,
            "language": lemma.language,
            "confidence": lemma.confidence
        })).collect::<Vec<_>>(),
        "pos": analysis.pos.iter().map(|pos| json!({
            "tokenIndex": pos.token_index,
            "token": analysis.tokens.get(pos.token_index).map(|token| token.text.as_str()),
            "tag": format!("{:?}", pos.tag),
            "confidence": pos.confidence,
            "reason": pos.reason
        })).collect::<Vec<_>>(),
        "entities": analysis.entities.iter().map(|entity| json!({
            "id": entity.id,
            "text": entity.mention.text,
            "normalized": entity.normalized,
            "kind": format!("{:?}", entity.entity_type),
            "sentenceIndex": entity.sentence_index,
            "tokenStart": entity.token_start,
            "tokenEnd": entity.token_end,
            "confidence": entity.confidence
        })).collect::<Vec<_>>(),
        "events": analysis.events.iter().map(|event| json!({
            "sentenceIndex": event.sentence_index,
            "predicate": event.predicate,
            "lemma": event.lemma,
            "relationType": format!("{:?}", event.relation_type),
            "confidence": event.confidence,
            "arguments": event.arguments.iter().map(|argument| json!({
                "role": argument.role,
                "text": argument.text,
                "confidence": argument.confidence
            })).collect::<Vec<_>>()
        })).collect::<Vec<_>>(),
        "relations": analysis.relations.iter().map(|relation| json!({
            "subject": relation.subject,
            "relation": relation.relation,
            "object": relation.object,
            "relationType": format!("{:?}", relation.relation_type),
            "confidence": relation.confidence
        })).collect::<Vec<_>>(),
        "topics": analysis.topics.descriptors.iter().map(|topic| json!({
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

fn smoke_value() -> Value {
    let timestamp = Timestamp::new(3, Timebase::new(1, 2));
    let vector = vector_core::DenseVector::new([3.0, 4.0]).expect("valid vector");
    let graph = graph_core::Graph::directed();
    let image = image_synthesis::solid_image(
        image_synthesis::RgbColor::new(1, 2, 3),
        image_synthesis::ImageSynthesisConfig {
            width: 2,
            height: 2,
            pixel_format: image_core::ImagePixelFormat::Rgb24,
        },
    )
    .expect("valid image");

    json!({
        "ok": true,
        "checks": {
            "timestampSeconds": timestamp.seconds(),
            "vectorNorm": vector_core::l2_norm(vector.as_slice()).expect("valid norm"),
            "graphNodes": graph.node_count(),
            "imageWidth": image.value.width,
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
        "audio-analysis-core" => Some(audio_analysis_core::surface::package_surface()),
        "audio-analysis-fourier" => Some(audio_analysis_fourier::surface::package_surface()),
        "audio-analysis-io" => Some(audio_analysis_io::surface::package_surface()),
        "audio-analysis-pitch" => Some(audio_analysis_pitch::surface::package_surface()),
        "audio-analysis-processing" => Some(audio_analysis_processing::surface::package_surface()),
        "audio-analysis-recognition" => {
            Some(audio_analysis_recognition::surface::package_surface())
        }
        "audio-analysis-rhythm" => Some(audio_analysis_rhythm::surface::package_surface()),
        "audio-analysis-separation" => Some(audio_analysis_separation::surface::package_surface()),
        "audio-analysis-speakers" => Some(audio_analysis_speakers::surface::package_surface()),
        "audio-analysis-synthesis" => Some(audio_analysis_synthesis::surface::package_surface()),
        "audio-generation-midi" => Some(audio_generation_midi::surface::package_surface()),
        "comfyui-data" => Some(comfyui_data::surface::package_surface()),
        "comfyui-latents" => Some(comfyui_latents::surface::package_surface()),
        "comfyui-models" => Some(comfyui_models::surface::package_surface()),
        "data-inversion-core" => Some(data_inversion_core::surface::package_surface()),
        "dense-data" => Some(dense_data::surface::package_surface()),
        "finance-statistics" => Some(finance_statistics::surface::package_surface()),
        "geo-core" | "moritzbrantner-geo-core" => Some(geo_core::surface::package_surface()),
        "geo-io-geojson" | "moritzbrantner-geo-io-geojson" => {
            Some(geo_io_geojson::surface::package_surface())
        }
        "geo-clustering" | "moritzbrantner-geo-clustering" => {
            Some(geo_clustering::surface::package_surface())
        }
        "geo-viz" | "moritzbrantner-geo-viz" => Some(geo_viz::surface::package_surface()),
        "graph-analysis-core" => Some(graph_analysis_core::surface::package_surface()),
        "image-analysis-captioning" => Some(image_analysis_captioning::surface::package_surface()),
        "image-analysis-classification" => {
            Some(image_analysis_classification::surface::package_surface())
        }
        "image-analysis-comfyui" => Some(image_analysis_comfyui::surface::package_surface()),
        "image-analysis-core" => Some(image_analysis_core::surface::package_surface()),
        "image-analysis-detection" => Some(image_analysis_detection::surface::package_surface()),
        "image-analysis-embeddings" => Some(image_analysis_embeddings::surface::package_surface()),
        "image-analysis-io" => Some(image_analysis_io::surface::package_surface()),
        "image-analysis-ocr" => Some(image_analysis_ocr::surface::package_surface()),
        "image-analysis-processing" => Some(image_analysis_processing::surface::package_surface()),
        "image-analysis-segmentation" => {
            Some(image_analysis_segmentation::surface::package_surface())
        }
        "image-analysis-synthesis" => Some(image_analysis_synthesis::surface::package_surface()),
        "jobs-core" => Some(jobs_core::surface::package_surface()),
        "maps-kernels-core" => Some(maps_kernels_core::surface::package_surface()),
        "math-geometry-2d" => Some(math_geometry_2d::surface::package_surface()),
        "math-linear" => Some(math_linear::surface::package_surface()),
        "math-signal-core" => Some(math_signal_core::surface::package_surface()),
        "math-sparse-data" => Some(math_sparse_data::surface::package_surface()),
        "math-statistics" => Some(math_statistics::surface::package_surface()),
        "model-runtime" => Some(model_runtime::surface::package_surface()),
        "numbers-core" => Some(numbers_core::surface::package_surface()),
        "tensor-data" => Some(tensor_data::surface::package_surface()),
        "text-analysis" => Some(text_analysis::surface::package_surface()),
        "text-classification" => Some(text_classification::surface::package_surface()),
        "text-core" => Some(text_core::surface::package_surface()),
        "text-embeddings" => Some(text_embeddings::surface::package_surface()),
        "text-generation" => Some(text_generation::surface::package_surface()),
        "text-generation-linguistics" => {
            Some(text_generation_linguistics::surface::package_surface())
        }
        "text-lexical" => Some(text_lexical::surface::package_surface()),
        "text-linguistics" => Some(text_linguistics::surface::package_surface()),
        "text-model-runtime" => Some(text_model_runtime::surface::package_surface()),
        "text-question-answering" => Some(text_question_answering::surface::package_surface()),
        "text-retrieval" => Some(text_retrieval::surface::package_surface()),
        "text-transcripts" => Some(text_transcripts::surface::package_surface()),
        "three-d-processing-core" => Some(three_d_processing_core::surface::package_surface()),
        "three-d-processing-io" => Some(three_d_processing_io::surface::package_surface()),
        "three-d-processing-mesh" => Some(three_d_processing_mesh::surface::package_surface()),
        "three-d-scene-svg" => Some(three_d_scene_svg::surface::package_surface()),
        "vector-analysis-core" => Some(vector_analysis_core::surface::package_surface()),
        "vector-analysis-index" => Some(vector_analysis_index::surface::package_surface()),
        "video-analysis-core" => Some(video_analysis_core::surface::package_surface()),
        "video-analysis-data" => Some(video_analysis_data::surface::package_surface()),
        "video-analysis-dataset" => Some(video_analysis_dataset::surface::package_surface()),
        "video-analysis-detectors" => Some(video_analysis_detectors::surface::package_surface()),
        "video-analysis-editing" => Some(video_analysis_editing::surface::package_surface()),
        "video-analysis-features" => Some(video_analysis_features::surface::package_surface()),
        "video-analysis-ffmpeg" => Some(video_analysis_ffmpeg::surface::package_surface()),
        "video-analysis-gaussian-splatting" => {
            Some(video_analysis_gaussian_splatting::surface::package_surface())
        }
        "video-analysis-ingest" => Some(video_analysis_ingest::surface::package_surface()),
        "video-analysis-mvs" => Some(video_analysis_mvs::surface::package_surface()),
        "video-analysis-output" => Some(video_analysis_output::surface::package_surface()),
        "video-analysis-posture" => Some(video_analysis_posture::surface::package_surface()),
        "video-analysis-posture-io" => Some(video_analysis_posture_io::surface::package_surface()),
        "video-analysis-radiance-fields" => {
            Some(video_analysis_radiance_fields::surface::package_surface())
        }
        "video-analysis-radiance-io" => {
            Some(video_analysis_radiance_io::surface::package_surface())
        }
        "video-analysis-radiance-pipeline" => {
            Some(video_analysis_radiance_pipeline::surface::package_surface())
        }
        "video-analysis-recognition" => {
            Some(video_analysis_recognition::surface::package_surface())
        }
        "video-analysis-reconstruction" => {
            Some(video_analysis_reconstruction::surface::package_surface())
        }
        "video-analysis-segmentation" => {
            Some(video_analysis_segmentation::surface::package_surface())
        }
        "video-analysis-sfm" => Some(video_analysis_sfm::surface::package_surface()),
        "video-analysis-split" => Some(video_analysis_split::surface::package_surface()),
        "video-analysis-storage" => Some(video_analysis_storage::surface::package_surface()),
        "video-analysis-synthesis" => Some(video_analysis_synthesis::surface::package_surface()),
        "video-analysis-tracking" => Some(video_analysis_tracking::surface::package_surface()),
        "video-analysis-transform" => Some(video_analysis_transform::surface::package_surface()),
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
        "audio-analysis-core" => Some(audio_analysis_core::surface::run_surface_operation(request)),
        "audio-analysis-fourier" => Some(audio_analysis_fourier::surface::run_surface_operation(
            request,
        )),
        "audio-analysis-io" => Some(audio_analysis_io::surface::run_surface_operation(request)),
        "audio-analysis-pitch" => Some(audio_analysis_pitch::surface::run_surface_operation(
            request,
        )),
        "audio-analysis-processing" => Some(
            audio_analysis_processing::surface::run_surface_operation(request),
        ),
        "audio-analysis-recognition" => Some(
            audio_analysis_recognition::surface::run_surface_operation(request),
        ),
        "audio-analysis-rhythm" => Some(audio_analysis_rhythm::surface::run_surface_operation(
            request,
        )),
        "audio-analysis-separation" => Some(
            audio_analysis_separation::surface::run_surface_operation(request),
        ),
        "audio-analysis-speakers" => Some(audio_analysis_speakers::surface::run_surface_operation(
            request,
        )),
        "audio-analysis-synthesis" => Some(
            audio_analysis_synthesis::surface::run_surface_operation(request),
        ),
        "audio-generation-midi" => Some(audio_generation_midi::surface::run_surface_operation(
            request,
        )),
        "comfyui-data" => Some(comfyui_data::surface::run_surface_operation(request)),
        "comfyui-latents" => Some(comfyui_latents::surface::run_surface_operation(request)),
        "comfyui-models" => Some(comfyui_models::surface::run_surface_operation(request)),
        "data-inversion-core" => Some(data_inversion_core::surface::run_surface_operation(request)),
        "dense-data" => Some(dense_data::surface::run_surface_operation(request)),
        "finance-statistics" => Some(finance_statistics::surface::run_surface_operation(request)),
        "geo-core" | "moritzbrantner-geo-core" => {
            Some(geo_core::surface::run_surface_operation(request))
        }
        "geo-io-geojson" | "moritzbrantner-geo-io-geojson" => {
            Some(geo_io_geojson::surface::run_surface_operation(request))
        }
        "geo-clustering" | "moritzbrantner-geo-clustering" => {
            Some(geo_clustering::surface::run_surface_operation(request))
        }
        "geo-viz" | "moritzbrantner-geo-viz" => {
            Some(geo_viz::surface::run_surface_operation(request))
        }
        "graph-analysis-core" => Some(graph_analysis_core::surface::run_surface_operation(request)),
        "image-analysis-captioning" => Some(
            image_analysis_captioning::surface::run_surface_operation(request),
        ),
        "image-analysis-classification" => {
            Some(image_analysis_classification::surface::run_surface_operation(request))
        }
        "image-analysis-comfyui" => Some(image_analysis_comfyui::surface::run_surface_operation(
            request,
        )),
        "image-analysis-core" => Some(image_analysis_core::surface::run_surface_operation(request)),
        "image-analysis-detection" => Some(
            image_analysis_detection::surface::run_surface_operation(request),
        ),
        "image-analysis-embeddings" => Some(
            image_analysis_embeddings::surface::run_surface_operation(request),
        ),
        "image-analysis-io" => Some(image_analysis_io::surface::run_surface_operation(request)),
        "image-analysis-ocr" => Some(image_analysis_ocr::surface::run_surface_operation(request)),
        "image-analysis-processing" => Some(
            image_analysis_processing::surface::run_surface_operation(request),
        ),
        "image-analysis-segmentation" => Some(
            image_analysis_segmentation::surface::run_surface_operation(request),
        ),
        "image-analysis-synthesis" => Some(
            image_analysis_synthesis::surface::run_surface_operation(request),
        ),
        "jobs-core" => Some(jobs_core::surface::run_surface_operation(request)),
        "maps-kernels-core" => Some(maps_kernels_core::surface::run_surface_operation(request)),
        "math-geometry-2d" => Some(math_geometry_2d::surface::run_surface_operation(request)),
        "math-linear" => Some(math_linear::surface::run_surface_operation(request)),
        "math-signal-core" => Some(math_signal_core::surface::run_surface_operation(request)),
        "math-sparse-data" => Some(math_sparse_data::surface::run_surface_operation(request)),
        "math-statistics" => Some(math_statistics::surface::run_surface_operation(request)),
        "model-runtime" => Some(model_runtime::surface::run_surface_operation(request)),
        "numbers-core" => Some(numbers_core::surface::run_surface_operation(request)),
        "tensor-data" => Some(tensor_data::surface::run_surface_operation(request)),
        "text-analysis" => Some(text_analysis::surface::run_surface_operation(request)),
        "text-classification" => Some(text_classification::surface::run_surface_operation(request)),
        "text-core" => Some(text_core::surface::run_surface_operation(request)),
        "text-embeddings" => Some(text_embeddings::surface::run_surface_operation(request)),
        "text-generation" => Some(text_generation::surface::run_surface_operation(request)),
        "text-generation-linguistics" => Some(
            text_generation_linguistics::surface::run_surface_operation(request),
        ),
        "text-lexical" => Some(text_lexical::surface::run_surface_operation(request)),
        "text-linguistics" => Some(text_linguistics::surface::run_surface_operation(request)),
        "text-model-runtime" => Some(text_model_runtime::surface::run_surface_operation(request)),
        "text-question-answering" => Some(text_question_answering::surface::run_surface_operation(
            request,
        )),
        "text-retrieval" => Some(text_retrieval::surface::run_surface_operation(request)),
        "text-transcripts" => Some(text_transcripts::surface::run_surface_operation(request)),
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
        "vector-analysis-core" => Some(vector_analysis_core::surface::run_surface_operation(
            request,
        )),
        "vector-analysis-index" => Some(vector_analysis_index::surface::run_surface_operation(
            request,
        )),
        "video-analysis-sfm"
            if request.operation.as_str()
                == video_analysis_sfm::surface::RECONSTRUCT_VIDEO_OPERATION =>
        {
            Some(video_analysis_sfm::reconstruct_video_surface_operation(
                request.input,
            ))
        }
        "video-analysis-core" => Some(video_analysis_core::surface::run_surface_operation(request)),
        "video-analysis-data" => Some(video_analysis_data::surface::run_surface_operation(request)),
        "video-analysis-dataset" => Some(video_analysis_dataset::surface::run_surface_operation(
            request,
        )),
        "video-analysis-detectors" => Some(
            video_analysis_detectors::surface::run_surface_operation(request),
        ),
        "video-analysis-editing" => Some(video_analysis_editing::surface::run_surface_operation(
            request,
        )),
        "video-analysis-features" => Some(video_analysis_features::surface::run_surface_operation(
            request,
        )),
        "video-analysis-ffmpeg" => Some(video_analysis_ffmpeg::surface::run_surface_operation(
            request,
        )),
        "video-analysis-gaussian-splatting" => {
            Some(video_analysis_gaussian_splatting::surface::run_surface_operation(request))
        }
        "video-analysis-ingest" => Some(video_analysis_ingest::surface::run_surface_operation(
            request,
        )),
        "video-analysis-mvs" => Some(video_analysis_mvs::surface::run_surface_operation(request)),
        "video-analysis-output" => Some(video_analysis_output::surface::run_surface_operation(
            request,
        )),
        "video-analysis-posture" => Some(video_analysis_posture::surface::run_surface_operation(
            request,
        )),
        "video-analysis-posture-io" => Some(
            video_analysis_posture_io::surface::run_surface_operation(request),
        ),
        "video-analysis-radiance-fields" => {
            Some(video_analysis_radiance_fields::surface::run_surface_operation(request))
        }
        "video-analysis-radiance-io" => Some(
            video_analysis_radiance_io::surface::run_surface_operation(request),
        ),
        "video-analysis-radiance-pipeline" => {
            Some(video_analysis_radiance_pipeline::surface::run_surface_operation(request))
        }
        "video-analysis-recognition" => Some(
            video_analysis_recognition::surface::run_surface_operation(request),
        ),
        "video-analysis-reconstruction" => {
            Some(video_analysis_reconstruction::surface::run_surface_operation(request))
        }
        "video-analysis-segmentation" => Some(
            video_analysis_segmentation::surface::run_surface_operation(request),
        ),
        "video-analysis-sfm" => Some(video_analysis_sfm::surface::run_surface_operation(request)),
        "video-analysis-split" => Some(video_analysis_split::surface::run_surface_operation(
            request,
        )),
        "video-analysis-storage" => Some(video_analysis_storage::surface::run_surface_operation(
            request,
        )),
        "video-analysis-synthesis" => Some(
            video_analysis_synthesis::surface::run_surface_operation(request),
        ),
        "video-analysis-tracking" => Some(video_analysis_tracking::surface::run_surface_operation(
            request,
        )),
        "video-analysis-transform" => Some(
            video_analysis_transform::surface::run_surface_operation(request),
        ),
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
        package: "audio-analysis-core",
        import_path: "video_analysis::audio_core",
        domain: "audio",
        linked: true,
        required_feature: None,
    },
    ModuleInfo {
        package: "audio-analysis-fourier",
        import_path: "video_analysis::audio_fourier",
        domain: "audio",
        linked: true,
        required_feature: None,
    },
    ModuleInfo {
        package: "audio-analysis-io",
        import_path: "video_analysis::audio_io",
        domain: "audio",
        linked: true,
        required_feature: None,
    },
    ModuleInfo {
        package: "audio-analysis-pitch",
        import_path: "video_analysis::audio_pitch",
        domain: "audio",
        linked: true,
        required_feature: None,
    },
    ModuleInfo {
        package: "audio-analysis-processing",
        import_path: "video_analysis::audio_processing",
        domain: "audio",
        linked: true,
        required_feature: None,
    },
    ModuleInfo {
        package: "audio-analysis-recognition",
        import_path: "video_analysis::audio_recognition",
        domain: "audio",
        linked: true,
        required_feature: None,
    },
    ModuleInfo {
        package: "audio-analysis-rhythm",
        import_path: "video_analysis::audio_rhythm",
        domain: "audio",
        linked: true,
        required_feature: None,
    },
    ModuleInfo {
        package: "audio-analysis-separation",
        import_path: "video_analysis::audio_separation",
        domain: "audio",
        linked: true,
        required_feature: None,
    },
    ModuleInfo {
        package: "audio-analysis-speakers",
        import_path: "video_analysis::audio_speakers",
        domain: "audio",
        linked: true,
        required_feature: None,
    },
    ModuleInfo {
        package: "audio-analysis-synthesis",
        import_path: "video_analysis::audio_synthesis",
        domain: "audio",
        linked: true,
        required_feature: None,
    },
    ModuleInfo {
        package: "audio-generation-midi",
        import_path: "video_analysis::audio_midi",
        domain: "audio",
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
        package: "data-inversion-core",
        import_path: "video_analysis::inversion",
        domain: "data",
        linked: true,
        required_feature: None,
    },
    ModuleInfo {
        package: "dense-data",
        import_path: "video_analysis::dense",
        domain: "data",
        linked: true,
        required_feature: None,
    },
    ModuleInfo {
        package: "finance-statistics",
        import_path: "video_analysis::finance",
        domain: "math",
        linked: true,
        required_feature: None,
    },
    ModuleInfo {
        package: "moritzbrantner-geo-core",
        import_path: "video_analysis::geo_core",
        domain: "data",
        linked: true,
        required_feature: None,
    },
    ModuleInfo {
        package: "moritzbrantner-geo-io-geojson",
        import_path: "video_analysis::geo_io_geojson",
        domain: "data",
        linked: true,
        required_feature: None,
    },
    ModuleInfo {
        package: "moritzbrantner-geo-clustering",
        import_path: "video_analysis::geo_clustering",
        domain: "data",
        linked: true,
        required_feature: None,
    },
    ModuleInfo {
        package: "moritzbrantner-geo-viz",
        import_path: "video_analysis::geo_viz",
        domain: "data",
        linked: true,
        required_feature: None,
    },
    ModuleInfo {
        package: "graph-analysis-core",
        import_path: "video_analysis::graph_core",
        domain: "data",
        linked: true,
        required_feature: None,
    },
    ModuleInfo {
        package: "image-analysis-captioning",
        import_path: "video_analysis::image_captioning",
        domain: "image",
        linked: true,
        required_feature: None,
    },
    ModuleInfo {
        package: "image-analysis-classification",
        import_path: "video_analysis::image_classification",
        domain: "image",
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
        package: "image-analysis-core",
        import_path: "video_analysis::image_core",
        domain: "image",
        linked: true,
        required_feature: None,
    },
    ModuleInfo {
        package: "image-analysis-detection",
        import_path: "video_analysis::image_detection",
        domain: "image",
        linked: true,
        required_feature: None,
    },
    ModuleInfo {
        package: "image-analysis-embeddings",
        import_path: "video_analysis::image_embeddings",
        domain: "image",
        linked: true,
        required_feature: None,
    },
    ModuleInfo {
        package: "image-analysis-io",
        import_path: "video_analysis::image_io",
        domain: "image",
        linked: true,
        required_feature: None,
    },
    ModuleInfo {
        package: "image-analysis-ocr",
        import_path: "video_analysis::image_ocr",
        domain: "image",
        linked: true,
        required_feature: None,
    },
    ModuleInfo {
        package: "image-analysis-processing",
        import_path: "video_analysis::image_processing",
        domain: "image",
        linked: true,
        required_feature: None,
    },
    ModuleInfo {
        package: "image-analysis-segmentation",
        import_path: "video_analysis::image_segmentation",
        domain: "image",
        linked: true,
        required_feature: None,
    },
    ModuleInfo {
        package: "image-analysis-synthesis",
        import_path: "video_analysis::image_synthesis",
        domain: "image",
        linked: true,
        required_feature: None,
    },
    ModuleInfo {
        package: "jobs-core",
        import_path: "video_analysis::jobs",
        domain: "jobs",
        linked: true,
        required_feature: None,
    },
    ModuleInfo {
        package: "maps-kernels-core",
        import_path: "video_analysis::maps_kernels",
        domain: "math",
        linked: true,
        required_feature: None,
    },
    ModuleInfo {
        package: "model-runtime",
        import_path: "video_analysis::model_runtime",
        domain: "runtime",
        linked: true,
        required_feature: None,
    },
    ModuleInfo {
        package: "math-geometry-2d",
        import_path: "video_analysis::geometry2d",
        domain: "math",
        linked: true,
        required_feature: None,
    },
    ModuleInfo {
        package: "math-linear",
        import_path: "video_analysis::linear",
        domain: "math",
        linked: true,
        required_feature: None,
    },
    ModuleInfo {
        package: "math-signal-core",
        import_path: "video_analysis::signal",
        domain: "math",
        linked: true,
        required_feature: None,
    },
    ModuleInfo {
        package: "math-sparse-data",
        import_path: "video_analysis::sparse",
        domain: "math",
        linked: true,
        required_feature: None,
    },
    ModuleInfo {
        package: "math-statistics",
        import_path: "video_analysis::stats",
        domain: "math",
        linked: true,
        required_feature: None,
    },
    ModuleInfo {
        package: "numbers-core",
        import_path: "video_analysis::numbers",
        domain: "data",
        linked: true,
        required_feature: None,
    },
    ModuleInfo {
        package: "tensor-data",
        import_path: "video_analysis::tensor_data",
        domain: "data",
        linked: true,
        required_feature: None,
    },
    ModuleInfo {
        package: "text-analysis",
        import_path: "video_analysis::text_analysis",
        domain: "text",
        linked: true,
        required_feature: None,
    },
    ModuleInfo {
        package: "text-classification",
        import_path: "video_analysis::text_classification",
        domain: "text",
        linked: true,
        required_feature: None,
    },
    ModuleInfo {
        package: "text-core",
        import_path: "video_analysis::text_core",
        domain: "text",
        linked: true,
        required_feature: None,
    },
    ModuleInfo {
        package: "text-lexical",
        import_path: "video_analysis::text_lexical",
        domain: "text",
        linked: true,
        required_feature: None,
    },
    ModuleInfo {
        package: "text-linguistics",
        import_path: "video_analysis::text_linguistics",
        domain: "text",
        linked: true,
        required_feature: None,
    },
    ModuleInfo {
        package: "text-model-runtime",
        import_path: "video_analysis::text_model_runtime",
        domain: "text",
        linked: true,
        required_feature: None,
    },
    ModuleInfo {
        package: "text-question-answering",
        import_path: "video_analysis::text_question_answering",
        domain: "text",
        linked: true,
        required_feature: None,
    },
    ModuleInfo {
        package: "text-generation",
        import_path: "video_analysis::text_generation",
        domain: "text",
        linked: true,
        required_feature: None,
    },
    ModuleInfo {
        package: "text-generation-linguistics",
        import_path: "video_analysis::text_generation_linguistics",
        domain: "text",
        linked: true,
        required_feature: None,
    },
    ModuleInfo {
        package: "text-retrieval",
        import_path: "video_analysis::text_retrieval",
        domain: "text",
        linked: true,
        required_feature: None,
    },
    ModuleInfo {
        package: "text-embeddings",
        import_path: "video_analysis::text_embeddings",
        domain: "text",
        linked: true,
        required_feature: None,
    },
    ModuleInfo {
        package: "text-transcripts",
        import_path: "video_analysis::text_transcripts",
        domain: "text",
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
        package: "vector-analysis-core",
        import_path: "video_analysis::vector_core",
        domain: "vector",
        linked: true,
        required_feature: None,
    },
    ModuleInfo {
        package: "vector-analysis-index",
        import_path: "video_analysis::vector_index",
        domain: "vector",
        linked: true,
        required_feature: None,
    },
    ModuleInfo {
        package: "video-analysis-core",
        import_path: "video_analysis",
        domain: "video",
        linked: true,
        required_feature: None,
    },
    ModuleInfo {
        package: "video-analysis-data",
        import_path: "video_analysis::data",
        domain: "video",
        linked: true,
        required_feature: None,
    },
    ModuleInfo {
        package: "video-analysis-dataset",
        import_path: "video_analysis::dataset_records",
        domain: "video",
        linked: true,
        required_feature: None,
    },
    ModuleInfo {
        package: "video-analysis-detectors",
        import_path: "video_analysis",
        domain: "video",
        linked: true,
        required_feature: None,
    },
    ModuleInfo {
        package: "video-analysis-editing",
        import_path: "video_analysis::editing",
        domain: "video",
        linked: true,
        required_feature: None,
    },
    ModuleInfo {
        package: "video-analysis-features",
        import_path: "video_analysis::features",
        domain: "video",
        linked: true,
        required_feature: None,
    },
    ModuleInfo {
        package: "video-analysis-ffmpeg",
        import_path: "video_analysis::ffmpeg",
        domain: "video",
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
        package: "video-analysis-ingest",
        import_path: "video_analysis::ingest",
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
        package: "video-analysis-output",
        import_path: "video_analysis::output",
        domain: "video",
        linked: true,
        required_feature: None,
    },
    ModuleInfo {
        package: "video-analysis-posture",
        import_path: "video_analysis::posture",
        domain: "video",
        linked: true,
        required_feature: None,
    },
    ModuleInfo {
        package: "video-analysis-posture-io",
        import_path: "video_analysis::posture_io",
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
        package: "video-analysis-recognition",
        import_path: "video_analysis::recognition",
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
        package: "video-analysis-segmentation",
        import_path: "video_analysis::video_segmentation",
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
    ModuleInfo {
        package: "video-analysis-split",
        import_path: "video_analysis::split",
        domain: "video",
        linked: true,
        required_feature: None,
    },
    ModuleInfo {
        package: "video-analysis-storage",
        import_path: "video_analysis::storage",
        domain: "video",
        linked: true,
        required_feature: None,
    },
    ModuleInfo {
        package: "video-analysis-synthesis",
        import_path: "video_analysis::synthesis",
        domain: "video",
        linked: true,
        required_feature: None,
    },
    ModuleInfo {
        package: "video-analysis-tracking",
        import_path: "video_analysis::tracking",
        domain: "video",
        linked: true,
        required_feature: None,
    },
    ModuleInfo {
        package: "video-analysis-transform",
        import_path: "video_analysis::transform",
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
        let module = module_from_referer("http://127.0.0.1:5173/wrappers/video-analysis-core/");
        assert_eq!(module.unwrap().package, "video-analysis-core");
    }

    #[test]
    fn serves_selected_package_metadata() {
        let request = Request {
            method: "GET".to_string(),
            path: "/api/package".to_string(),
            query: HashMap::new(),
            headers: HashMap::from([(
                "referer".to_string(),
                "http://127.0.0.1:5173/wrappers/text-core/".to_string(),
            )]),
            body: String::new(),
        };
        let response = response_for(&request);
        assert_eq!(response.status_code, 200);
        assert!(response.body.contains("text-core-server"));
        assert!(response.body.contains("text.statistics"));
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
    fn serves_text_linguistics_analysis_from_package_route() {
        let request = Request {
            method: "POST".to_string(),
            path: "/api/rust/packages/text-linguistics/api/run".to_string(),
            query: HashMap::new(),
            headers: HashMap::new(),
            body: r#"{"operation":"linguistics.analyze","input":{"profile":"fast","text":"Alice presented the roadmap in Berlin."}}"#.to_string(),
        };
        let response = response_for(&request);
        assert_eq!(response.status_code, 200);
        assert!(response
            .body
            .contains("\"operation\":\"linguistics.analyze\""));
        assert!(response.body.contains("\"tokens\""));
    }

    #[test]
    fn serves_text_analysis_metadata_from_package_route() {
        let request = Request {
            method: "GET".to_string(),
            path: "/api/rust/packages/text-analysis/api/package".to_string(),
            query: HashMap::new(),
            headers: HashMap::new(),
            body: String::new(),
        };
        let response = response_for(&request);
        assert_eq!(response.status_code, 200);
        assert!(response.body.contains("text-analysis-server"));
        assert!(response.body.contains("analysis.document"));
    }

    #[test]
    fn serves_text_analysis_document_operation_from_package_route() {
        let request = Request {
            method: "POST".to_string(),
            path: "/api/rust/packages/text-analysis/api/run".to_string(),
            query: HashMap::new(),
            headers: HashMap::new(),
            body: r#"{"operation":"analysis.document","input":{"id":"doc-1","text":"Alice presented the tokenizer roadmap in Berlin."}}"#.to_string(),
        };
        let response = response_for(&request);
        assert_eq!(response.status_code, 200);
        assert!(response
            .body
            .contains("\"operation\":\"analysis.document\""));
        assert!(response.body.contains("\"enrichedStats\""));
        assert!(response.body.contains("\"lexical\""));
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
            path: "/api/rust/packages/image-analysis-detection/api/models".to_string(),
            query: HashMap::new(),
            headers: HashMap::new(),
            body: String::new(),
        };
        let response = response_for(&request);
        assert_eq!(response.status_code, 200);
        assert!(response.body.contains("mask-proposal-demo"));
        assert!(response.body.contains("yolo-onnx"));
    }

    #[test]
    fn packages_without_models_return_empty_catalog() {
        let request = Request {
            method: "GET".to_string(),
            path: "/api/rust/packages/math-linear/api/models".to_string(),
            query: HashMap::new(),
            headers: HashMap::new(),
            body: String::new(),
        };
        let response = response_for(&request);
        assert_eq!(response.status_code, 200);
        assert_eq!(response.body, "[]");
    }

    #[test]
    fn onnx_model_catalog_flows_through_task_crate() {
        let request = Request {
            method: "GET".to_string(),
            path: "/api/rust/packages/image-analysis-detection/api/models".to_string(),
            query: HashMap::new(),
            headers: HashMap::new(),
            body: String::new(),
        };
        let response = response_for(&request);
        assert_eq!(response.status_code, 200);
        assert!(response.body.contains("mask-proposal-demo"));
    }

    #[test]
    fn expected_text_library_crates_have_overview_surfaces() {
        let expected = [
            "text-analysis",
            "text-core",
            "text-classification",
            "text-lexical",
            "text-linguistics",
            "text-model-runtime",
            "text-question-answering",
            "text-generation",
            "text-generation-linguistics",
            "text-retrieval",
            "text-embeddings",
            "text-transcripts",
        ];

        for package in expected {
            let module = MODULES
                .iter()
                .copied()
                .find(|module| module.package == package)
                .unwrap_or_else(|| panic!("missing text module registration for {package}"));
            let surface = package_surface_for(module)
                .unwrap_or_else(|| panic!("missing package surface for {package}"));
            assert!(
                !surface.operations.is_empty(),
                "missing operations for {package}"
            );
        }
    }

    #[test]
    fn serves_text_nlp_task_endpoint_from_package_route() {
        let request = Request {
            method: "POST".to_string(),
            path: "/api/rust/packages/text-linguistics/api/sentiment".to_string(),
            query: HashMap::new(),
            headers: HashMap::new(),
            body: r#"{"text":"excellent reliable work","model":{"fallbackPolicy":"lexical_fallback"}}"#
                .to_string(),
        };
        let response = response_for(&request);
        assert_eq!(response.status_code, 200);
        assert!(response.body.contains("\"operation\":\"sentiment\""));
        assert!(response.body.contains("\"runtime\":\"lexical\""));
    }

    #[test]
    fn serves_audio_model_task_endpoint_from_package_route() {
        let request = Request {
            method: "POST".to_string(),
            path: "/api/rust/packages/audio-analysis-recognition/api/classify".to_string(),
            query: HashMap::new(),
            headers: HashMap::new(),
            body: r#"{"features":{"rms":0.1,"peak":0.2,"spectralCentroidHz":1800},"labels":["speech","music"],"model":{"fallbackPolicy":"heuristic_fallback"}}"#
                .to_string(),
        };
        let response = response_for(&request);
        assert_eq!(response.status_code, 200);
        assert!(response.body.contains("\"operation\":\"classify\""));
        assert!(response.body.contains("\"runtime\":\"spectral\""));
    }

    #[test]
    fn serves_imported_transcript_normalization_from_text_transcripts_package_route() {
        let request = Request {
            method: "POST".to_string(),
            path: "/api/rust/packages/text-transcripts/api/run".to_string(),
            query: HashMap::new(),
            headers: HashMap::new(),
            body: r#"{"operation":"transcripts.normalize","input":{"source":"fixture.wav","language":"en","segments":[{"index":0,"startSeconds":0.0,"endSeconds":1.0,"text":" hello ","isFinal":true}]}}"#
                .to_string(),
        };
        let response = response_for(&request);
        assert_eq!(response.status_code, 200);
        assert!(response
            .body
            .contains("\"operation\":\"transcripts.normalize\""));
        assert!(response.body.contains("\"text\":\"hello\""));
    }

    #[test]
    fn text_linguistics_default_config_uses_rich_local_model() {
        let config = text_linguistics_config_from_payload(&json!({}));
        assert_eq!(config.profile, text_linguistics::AnalysisProfile::Rich);
        let metadata = text_linguistics_model_metadata_for_config(&config);
        assert_eq!(metadata.entity_recognition, "local-model");
        assert_eq!(metadata.entity_model, Some("bert-base-ner"));
    }
}
