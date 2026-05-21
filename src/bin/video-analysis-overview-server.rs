use clap::Parser;
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
    image_core, image_detection, image_io, image_models, image_processing, image_segmentation,
    image_synthesis, ingest, inversion, linear, maps_kernels, models, mvs, numbers, opencv_backend,
    output, posture, posture_io, radiance_fields, radiance_io, radiance_pipeline, recognition,
    reconstruction, sfm, sfm_rust_backend, signal, sparse, split, stats, storage, synthesis,
    tensor_data, text_core, text_embeddings, text_generation, text_lexical, text_linguistics,
    text_models, text_retrieval, text_transcripts, text_whisper_cpp, three_d_core, three_d_io,
    three_d_mesh, three_d_scene, tracking, transform, vector_core, vector_index,
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
        "endpoints": [
            "GET /health",
            "GET /api/package",
            "GET /api/schema",
            "POST /api/run"
        ]
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
        package: "image-analysis-models",
        import_path: "video_analysis::image_models",
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
        package: "text-models",
        import_path: "video_analysis::text_models",
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
        package: "text-whisper-cpp",
        import_path: "video_analysis::text_whisper_cpp",
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
        package: "video-analysis-models",
        import_path: "video_analysis::models",
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
}
