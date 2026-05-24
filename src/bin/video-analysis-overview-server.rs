use clap::Parser;
use runtime_contracts::{OperationId, SurfaceRequest, SurfaceResponse};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};

#[allow(unused_imports)]
use video_analysis::{
    animation, audio_core, audio_fourier, audio_io, audio_midi, audio_pitch, audio_processing,
    audio_recognition, audio_rhythm, audio_separation, audio_speakers, audio_synthesis,
    colmap_backend, comfyui_data, comfyui_latents, comfyui_models, data, dataset_records, dense,
    editing, features, ffmpeg, finance, gaussian_splatting, geometry2d, graph_core, image_comfyui,
    image_core, image_detection, image_io, image_processing, image_segmentation, image_synthesis,
    ingest, inversion, linear, maps_kernels, model_runtime, mvs, numbers, opencv_backend, output,
    posture, posture_io, radiance_fields, radiance_io, radiance_pipeline, recognition,
    reconstruction, sfm, sfm_rust_backend, signal, sparse, split, stats, storage, synthesis,
    tensor_data, text_classification, text_core, text_embeddings, text_generation, text_lexical,
    text_linguistics, text_retrieval, text_transcripts, three_d_core, three_d_io, three_d_mesh,
    three_d_scene, tracking, transform, vector_core, vector_index, video_segmentation, Timebase,
    Timestamp,
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
        handle_stream(stream?)?;
    }
    Ok(())
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
        ("GET", "/api/models") if module.package == "text-linguistics" => {
            json_response(200, "OK", json!(text_model_catalog(None)))
        }
        ("GET", "/api/models") if module.package == "audio-analysis-recognition" => {
            json_response(200, "OK", json!(audio_model_catalog(None)))
        }
        ("GET", path)
            if module.package == "text-linguistics" && path.starts_with("/api/models/") =>
        {
            let task = path.trim_start_matches("/api/models/");
            match parse_text_model_task(task) {
                Some(task) => json_response(200, "OK", json!(text_model_catalog(Some(task)))),
                None => json_response(
                    400,
                    "Bad Request",
                    json!({
                        "package": "text-linguistics-server",
                        "library": "text-linguistics",
                        "accepted": false,
                        "error": {
                            "code": "invalid_request",
                            "message": format!("unknown NLP task `{task}`")
                        }
                    }),
                ),
            }
        }
        ("GET", path)
            if module.package == "audio-analysis-recognition"
                && path.starts_with("/api/models/") =>
        {
            let task = path.trim_start_matches("/api/models/");
            match parse_audio_model_task(task) {
                Some(task) => json_response(200, "OK", json!(audio_model_catalog(Some(task)))),
                None => json_response(
                    400,
                    "Bad Request",
                    json!({
                        "package": "audio-analysis-recognition-server",
                        "library": "audio-analysis-recognition",
                        "accepted": false,
                        "error": {
                            "code": "invalid_request",
                            "message": format!("unknown audio task `{task}`")
                        }
                    }),
                ),
            }
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
        ("POST", "/api/run") => package_run_response(module, body),
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
    json_response(
        if module.linked { 200 } else { 503 },
        if module.linked {
            "OK"
        } else {
            "Service Unavailable"
        },
        json!({
            "ok": module.linked,
            "package": format!("{}-server", module.package),
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

fn package_metadata_value(module: &ModuleInfo) -> Value {
    let endpoints = if module.package == "text-linguistics" {
        vec![
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
            "POST /api/run",
        ]
    } else {
        vec![
            "GET /health",
            "GET /api/package",
            "GET /api/schema",
            "POST /api/run",
        ]
    };
    json!({
        "package": format!("{}-server", module.package),
        "surface": "api",
        "library": module.package,
        "libraryImport": format!("use {}", module.import_path),
        "cliPackage": format!("{}-cli", module.package),
        "appPackage": format!("{}-app", module.package),
        "domain": module.domain,
        "linked": module.linked,
        "requiredFeature": module.required_feature,
        "endpoints": endpoints
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
    json_response(
        200,
        "OK",
        json!({
            "openapi": "3.1.0",
            "info": {
                "title": format!("{} API", module.package),
                "version": env!("CARGO_PKG_VERSION")
            },
            "paths": {
                "/health": { "get": { "summary": "Health check" } },
                "/api/package": { "get": { "summary": "Package metadata" } },
                "/api/schema": { "get": { "summary": "API schema" } },
                "/api/run": { "post": { "summary": "Generic operation entrypoint" } }
            }
        }),
    )
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
            | "/api/transcribe"
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

    if module.package == "text-linguistics" {
        return text_linguistics_run_response(body);
    }

    let operation = serde_json::from_str::<Value>(body).ok().and_then(|value| {
        value
            .get("operation")
            .and_then(Value::as_str)
            .map(str::to_owned)
    });

    json_response(
        200,
        "OK",
        json!({
            "package": format!("{}-server", module.package),
            "library": module.package,
            "accepted": true,
            "operation": operation.unwrap_or_else(|| "raw".to_string()),
            "input": body,
            "module": package_metadata_value(&module),
            "note": "The overview server resolved this request to the selected Rust package."
        }),
    )
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

fn parse_text_model_task(input: &str) -> Option<&'static str> {
    match input {
        "classify" | "text_classification" | "classification" | "TextClassification" => {
            Some("classify")
        }
        "sentiment" | "Sentiment" => Some("sentiment"),
        "zero-shot" | "zero_shot" | "zero_shot_classification" | "ZeroShotClassification" => {
            Some("zero-shot")
        }
        "embed" | "embedding" | "text_embedding" | "TextEmbedding" => Some("embed"),
        "summarize" | "summary" | "summarization" | "Summarization" => Some("summarize"),
        "rerank" | "reranking" | "Reranking" => Some("rerank"),
        "question-answer" | "question_answer" | "question_answering" | "QuestionAnswering" => {
            Some("question-answer")
        }
        _ => None,
    }
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

fn parse_audio_model_task(input: &str) -> Option<&'static str> {
    match input {
        "classify" | "audio_classification" | "classification" | "AudioClassification" => {
            Some("classify")
        }
        "events" | "audio_event_detection" | "event_detection" | "AudioEventDetection" => {
            Some("events")
        }
        "embed" | "audio_embedding" | "embedding" | "AudioEmbedding" => Some("embed"),
        "transcribe" | "speech_recognition" | "asr" | "SpeechRecognition" => Some("transcribe"),
        "diarize" | "speaker_diarization" | "speakers" | "SpeakerDiarization" => Some("diarize"),
        "separate" | "source_separation" | "separation" | "SourceSeparation" => Some("separate"),
        "generate" | "audio_generation" | "synthesis" | "AudioGeneration" => Some("generate"),
        _ => None,
    }
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
            "whisper-tiny-en",
            "openai/whisper-tiny.en",
            "transcribe",
            "whisper_cpp",
            false,
            None,
            Some("Use text-transcripts native whisper.cpp support or imported segments for execution."),
        ),
        audio_model_entry(
            "wav2vec2-base-960h",
            "facebook/wav2vec2-base-960h",
            "transcribe",
            "onnx",
            false,
            Some("whisper-tiny-en"),
            Some("ASR schema and imported transcript support are available; native ONNX decoding is not wired."),
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
        "fallback": fallback,
        "note": note,
    })
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
        "/api/transcribe" => {
            match serde_json::from_str::<audio_analysis_recognition::SpeechRecognitionRequest>(body)
            {
                Ok(request) => audio_model_result_response(
                    audio_analysis_recognition::transcribe_audio(request),
                ),
                Err(error) => audio_model_error_response(
                    400,
                    "Bad Request",
                    "invalid_request",
                    &error.to_string(),
                ),
            }
        }
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
        package: "graph-analysis-core",
        import_path: "video_analysis::graph_core",
        domain: "data",
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
        package: "image-analysis-io",
        import_path: "video_analysis::image_io",
        domain: "image",
        linked: true,
        required_feature: None,
    },
    ModuleInfo {
        package: "image-analysis-onnx",
        import_path: "video_analysis::image_onnx",
        domain: "image",
        linked: cfg!(feature = "onnx-backend"),
        required_feature: Some("onnx-backend"),
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
        package: "text-generation",
        import_path: "video_analysis::text_generation",
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
        package: "video-analysis-colmap-backend",
        import_path: "video_analysis::colmap_backend",
        domain: "video",
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
        package: "video-analysis-onnx",
        import_path: "video_analysis::onnx",
        domain: "video",
        linked: cfg!(feature = "onnx-backend"),
        required_feature: Some("onnx-backend"),
    },
    ModuleInfo {
        package: "video-analysis-opencv-backend",
        import_path: "video_analysis::opencv_backend",
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
        package: "video-analysis-sfm-rust-backend",
        import_path: "video_analysis::sfm_rust_backend",
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
    }

    #[test]
    fn serves_text_linguistics_analysis_from_package_route() {
        let request = Request {
            method: "POST".to_string(),
            path: "/api/rust/packages/text-linguistics/api/run".to_string(),
            query: HashMap::new(),
            headers: HashMap::new(),
            body: r#"{"operation":"analyze","modelMode":"heuristic","text":"Alice presented the roadmap in Berlin."}"#.to_string(),
        };
        let response = response_for(&request);
        assert_eq!(response.status_code, 200);
        assert!(response.body.contains("\"operation\":\"analyze\""));
        assert!(response.body.contains("\"entityCount\""));
        assert!(response
            .body
            .contains("\"entityRecognition\":\"heuristic\""));
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
    fn text_linguistics_default_config_uses_rich_local_model() {
        let config = text_linguistics_config_from_payload(&json!({}));
        assert_eq!(config.profile, text_linguistics::AnalysisProfile::Rich);
        let metadata = text_linguistics_model_metadata_for_config(&config);
        assert_eq!(metadata.entity_recognition, "local-model");
        assert_eq!(metadata.entity_model, Some("bert-base-ner"));
    }
}
