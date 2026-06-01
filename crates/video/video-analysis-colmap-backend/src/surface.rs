//! Library-owned runtime surface for `video-analysis-colmap-backend`.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use runtime_core::{
    ensure_structured_surface_value, Diagnostic, DiagnosticSeverity, OperationId, PackageSurface,
    RuntimeCapabilities, SurfaceOperation, SurfaceRequest, SurfaceResponse,
};
use serde::Deserialize;

use crate::{
    default_reconstruct_video_request, ReconstructVideoRequest, COLMAP_TEST_VIDEO_PATH,
    COLMAP_TEST_VIDEO_URL, DEFAULT_RECONSTRUCT_OUTPUT_DIR,
};

/// Native server-only COLMAP video reconstruction operation.
pub const RECONSTRUCT_VIDEO_OPERATION: &str = "video.colmap.reconstructVideo";
const COMMAND_PLAN_OPERATION: &str = "video.colmap.commandPlan";
const IMAGE_LIST_OPERATION: &str = "video.colmap.imageList";
const SPARSE_SUMMARY_OPERATION: &str = "video.colmap.sparseSummary";

/// Returns the package surface exposed by every transport wrapper.
pub fn package_surface() -> PackageSurface {
    PackageSurface {
        library: env!("CARGO_PKG_NAME").to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        capabilities: RuntimeCapabilities::pure_rust(),
        operations: vec![
            operation(
                "describe",
                "Describe package",
                "COLMAP command planning and sparse reconstruction adapters for video-analysis.",
                serde_json::json!({
                    "includeOperations": true
                }),
            ),
            operation(
                COMMAND_PLAN_OPERATION,
                "Preview reconstruction command plan",
                "Shows the ffmpeg and COLMAP commands that reconstructVideo would run, without executing external tools.",
                reconstruct_video_example_request(),
            ),
            operation(
                IMAGE_LIST_OPERATION,
                "Inspect image-list JSON",
                "Summarizes inline COLMAP image metadata by frame order and camera group.",
                serde_json::json!({
                    "images": [
                        {"name": "frame_00001.jpg", "cameraId": 1},
                        {"name": "frame_00002.jpg", "cameraId": 1}
                    ]
                }),
            ),
            operation(
                SPARSE_SUMMARY_OPERATION,
                "Inspect sparse-model JSON",
                "Summarizes inline COLMAP sparse-model metadata that has already been parsed into JSON.",
                serde_json::json!({
                    "dataset": {
                        "cameras": [{"id": 1, "model": "PINHOLE"}],
                        "images": [{"id": 1, "name": "frame_00001.jpg", "cameraId": 1}],
                        "points3d": [{"id": 1, "track": [{"imageId": 1, "point2dIndex": 0}]}]
                    }
                }),
            ),
            server_operation(
                RECONSTRUCT_VIDEO_OPERATION,
                "Reconstruct video with COLMAP",
                "Extracts frames from a local video and runs a native COLMAP sparse reconstruction.",
                reconstruct_video_example_request(),
            ),
        ],
    }
}

fn operation(
    id: &str,
    name: &str,
    description: &str,
    example_request: serde_json::Value,
) -> SurfaceOperation {
    SurfaceOperation {
        id: OperationId::new(id),
        name: name.to_string(),
        description: Some(description.to_string()),
        input_schema: serde_json::json!({"type": "object", "additionalProperties": true}),
        output_schema: serde_json::json!({"type": "object"}),
        example_request,
        wasm_supported: true,
        server_supported: true,
    }
}

fn reconstruct_video_example_request() -> serde_json::Value {
    serde_json::json!({
        "videoPath": COLMAP_TEST_VIDEO_PATH,
        "videoUrl": COLMAP_TEST_VIDEO_URL,
        "outputDir": DEFAULT_RECONSTRUCT_OUTPUT_DIR,
        "frameFps": 2,
        "maxFrames": 80,
        "imageWidth": 1280,
        "clean": false
    })
}

fn server_operation(
    id: &str,
    name: &str,
    description: &str,
    example_request: serde_json::Value,
) -> SurfaceOperation {
    SurfaceOperation {
        wasm_supported: false,
        server_supported: true,
        ..operation(id, name, description, example_request)
    }
}

/// Runs one library-owned operation.
pub fn run_surface_operation(request: SurfaceRequest) -> Result<SurfaceResponse, String> {
    let surface = package_surface();
    let operation = request.operation.clone();
    let Some(surface_operation) = surface
        .operations
        .iter()
        .find(|candidate| candidate.id.as_str() == operation.as_str())
    else {
        return Err(format!(
            "unsupported operation `{}` for {}",
            operation.as_str(),
            env!("CARGO_PKG_NAME")
        ));
    };

    let value = match operation.as_str() {
        "describe" => describe_value(&surface, request.input),
        COMMAND_PLAN_OPERATION => command_plan_value(request.input)?,
        IMAGE_LIST_OPERATION => image_list_value(request.input)?,
        SPARSE_SUMMARY_OPERATION => sparse_summary_value(request.input),
        RECONSTRUCT_VIDEO_OPERATION => {
            return Ok(native_operation_diagnostic(operation));
        }
        _ => deterministic_operation_value(&surface, surface_operation, request.input),
    };
    let value = ensure_structured_surface_value(
        &operation,
        surface_operation.name.clone(),
        surface_operation
            .description
            .clone()
            .unwrap_or_else(|| format!("Ran package-surface operation `{}`.", operation.as_str())),
        value,
    );

    Ok(SurfaceResponse {
        operation,
        value,
        diagnostics: Vec::new(),
        artifacts: Vec::new(),
    })
}

fn describe_value(surface: &PackageSurface, input: serde_json::Value) -> serde_json::Value {
    serde_json::json!({
        "library": &surface.library,
        "version": &surface.version,
        "operationCount": surface.operations.len(),
        "operations": surface
            .operations
            .iter()
            .map(|operation| operation.id.as_str())
            .collect::<Vec<_>>(),
        "input": input
    })
}

fn deterministic_operation_value(
    surface: &PackageSurface,
    operation: &SurfaceOperation,
    input: serde_json::Value,
) -> serde_json::Value {
    let operation_id = operation.id.as_str();
    let result = serde_json::json!({
        "library": &surface.library,
        "version": &surface.version,
        "operation": operation_id,
        "name": &operation.name,
        "description": &operation.description,
        "deterministic": true,
        "externalToolsRequired": false,
        "request": input,
        "output": {
            "operationFamily": operation_family(operation_id),
            "sideEffects": false,
            "artifactMode": "inline-json"
        }
    });
    ensure_structured_surface_value(
        &operation.id,
        operation.name.clone(),
        operation
            .description
            .clone()
            .unwrap_or_else(|| format!("Ran package-surface operation `{operation_id}`.")),
        result,
    )
}

fn operation_family(operation_id: &str) -> &str {
    operation_id
        .split_once('.')
        .map(|(family, _)| family)
        .unwrap_or(operation_id)
}

fn native_operation_diagnostic(operation: OperationId) -> SurfaceResponse {
    let message = format!(
        "`{RECONSTRUCT_VIDEO_OPERATION}` is a native server-only operation; call the video-analysis-colmap-backend server or overview server dispatch path"
    );
    let result = serde_json::json!({
        "operation": operation.as_str(),
        "supported": false,
        "runtime": "native-server",
        "requiredTools": ["ffmpeg", "colmap"],
        "nextStep": "Run the package server or overview server dispatch path to execute native reconstruction.",
    });
    SurfaceResponse {
        value: runtime_core::structured_surface_value(
            &operation,
            "COLMAP native reconstruction unavailable in library runner",
            &message,
            serde_json::json!({
                "status": "unsupported",
                "nativeServerOnly": true,
                "requiredTools": ["ffmpeg", "colmap"],
            }),
            result,
        ),
        operation,
        diagnostics: vec![Diagnostic {
            severity: DiagnosticSeverity::Warning,
            code: "native_server_only".into(),
            message,
            source: Some(env!("CARGO_PKG_NAME").to_string()),
            help: Some(
                "Use video.colmap.commandPlan for a deterministic dry-run preview.".to_string(),
            ),
        }],
        artifacts: Vec::new(),
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PartialReconstructVideoRequest {
    video_path: Option<PathBuf>,
    video_url: Option<String>,
    output_dir: Option<PathBuf>,
    frame_fps: Option<f32>,
    max_frames: Option<usize>,
    image_width: Option<u32>,
    clean: Option<bool>,
}

fn command_plan_value(input: serde_json::Value) -> Result<serde_json::Value, String> {
    let request = reconstruct_request_from_input(input)?;
    let frames_dir = request.output_dir.join("frames");
    let database_path = request.output_dir.join("database.db");
    let sparse_root = request.output_dir.join("sparse");
    let sparse_binary_dir = sparse_root.join("0");
    let sparse_text_dir = request.output_dir.join("sparse_txt");
    let frame_pattern = frames_dir.join("frame_%05d.jpg");
    let filter = format!(
        "fps={},scale='min({},iw)':-2",
        request.frame_fps, request.image_width
    );
    let stages = vec![
        command_stage(
            "extractFrames",
            "ffmpeg",
            "Extract bounded JPEG frames from the local video.",
            vec![
                "-y".to_string(),
                "-i".to_string(),
                path_string(&request.video_path),
                "-vf".to_string(),
                filter,
                "-frames:v".to_string(),
                request.max_frames.to_string(),
                "-q:v".to_string(),
                "2".to_string(),
                path_string(&frame_pattern),
            ],
        ),
        command_stage(
            "featureExtraction",
            "colmap",
            "Detect local features for extracted frames with one shared camera.",
            vec![
                "feature_extractor".to_string(),
                "--database_path".to_string(),
                path_string(&database_path),
                "--image_path".to_string(),
                path_string(&frames_dir),
                "--ImageReader.single_camera".to_string(),
                "1".to_string(),
                "--SiftExtraction.use_gpu".to_string(),
                "0".to_string(),
            ],
        ),
        command_stage(
            "matching",
            "colmap",
            "Exhaustively match extracted frame features.",
            vec![
                "exhaustive_matcher".to_string(),
                "--database_path".to_string(),
                path_string(&database_path),
                "--SiftMatching.use_gpu".to_string(),
                "0".to_string(),
            ],
        ),
        command_stage(
            "mapping",
            "colmap",
            "Register frames and build a sparse reconstruction.",
            vec![
                "mapper".to_string(),
                "--database_path".to_string(),
                path_string(&database_path),
                "--image_path".to_string(),
                path_string(&frames_dir),
                "--output_path".to_string(),
                path_string(&sparse_root),
            ],
        ),
        command_stage(
            "textExport",
            "colmap",
            "Convert COLMAP binary sparse model into text files for Rust parsing.",
            vec![
                "model_converter".to_string(),
                "--input_path".to_string(),
                path_string(&sparse_binary_dir),
                "--output_path".to_string(),
                path_string(&sparse_text_dir),
                "--output_type".to_string(),
                "TXT".to_string(),
            ],
        ),
    ];

    Ok(serde_json::json!({
        "title": "COLMAP reconstruction command preview",
        "operation": COMMAND_PLAN_OPERATION,
        "summary": {
            "willExecute": false,
            "stageCount": stages.len(),
            "videoPath": path_string(&request.video_path),
            "outputDir": path_string(&request.output_dir),
            "frameFps": request.frame_fps,
            "maxFrames": request.max_frames,
            "imageWidth": request.image_width,
        },
        "message": "This is a dry-run preview. Use video.colmap.reconstructVideo to execute the native reconstruction pipeline.",
        "deterministic": true,
        "executes": false,
        "externalToolsRequired": ["ffmpeg", "colmap"],
        "nextOperation": RECONSTRUCT_VIDEO_OPERATION,
        "request": request,
        "outputs": {
            "framesDir": path_string(&frames_dir),
            "databasePath": path_string(&database_path),
            "sparseBinaryDir": path_string(&sparse_binary_dir),
            "sparseTextDir": path_string(&sparse_text_dir)
        },
        "stages": stages,
    }))
}

fn reconstruct_request_from_input(
    input: serde_json::Value,
) -> Result<ReconstructVideoRequest, String> {
    if input.as_object().is_none_or(serde_json::Map::is_empty) {
        return Ok(default_reconstruct_video_request());
    }

    let partial = serde_json::from_value::<PartialReconstructVideoRequest>(input)
        .map_err(|error| format!("invalid command plan request: {error}"))?;
    let mut request = default_reconstruct_video_request();
    if let Some(video_path) = partial.video_path {
        request.video_path = video_path;
    }
    if partial.video_url.is_some() {
        request.video_url = partial.video_url;
    }
    if let Some(output_dir) = partial.output_dir {
        request.output_dir = output_dir;
    }
    if let Some(frame_fps) = partial.frame_fps {
        request.frame_fps = frame_fps;
    }
    if let Some(max_frames) = partial.max_frames {
        request.max_frames = max_frames;
    }
    if let Some(image_width) = partial.image_width {
        request.image_width = image_width;
    }
    if let Some(clean) = partial.clean {
        request.clean = clean;
    }
    Ok(request)
}

fn image_list_value(input: serde_json::Value) -> Result<serde_json::Value, String> {
    let images = image_values_from_input(&input)?;
    let mut records = images
        .into_iter()
        .enumerate()
        .map(|(index, image)| {
            let name = image_name(&image).unwrap_or_else(|| format!("image_{:05}.jpg", index + 1));
            let camera_id = image
                .get("cameraId")
                .or_else(|| image.get("camera_id"))
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(1);
            serde_json::json!({
                "inputIndex": index,
                "name": name,
                "cameraId": camera_id,
                "frameNumber": infer_frame_number(&name),
            })
        })
        .collect::<Vec<_>>();
    records.sort_by(|left, right| {
        let left_frame = left.get("frameNumber").and_then(serde_json::Value::as_u64);
        let right_frame = right.get("frameNumber").and_then(serde_json::Value::as_u64);
        left_frame
            .cmp(&right_frame)
            .then_with(|| image_name(left).cmp(&image_name(right)))
            .then_with(|| {
                left.get("inputIndex")
                    .and_then(serde_json::Value::as_u64)
                    .cmp(&right.get("inputIndex").and_then(serde_json::Value::as_u64))
            })
    });

    let mut camera_groups = BTreeMap::<u64, usize>::new();
    for record in &records {
        if let Some(camera_id) = record.get("cameraId").and_then(serde_json::Value::as_u64) {
            *camera_groups.entry(camera_id).or_default() += 1;
        }
    }
    let camera_group_list = camera_groups
        .iter()
        .map(|(camera_id, image_count)| {
            serde_json::json!({
                "cameraId": camera_id,
                "imageCount": image_count,
            })
        })
        .collect::<Vec<_>>();
    let frame_numbers = records
        .iter()
        .filter_map(|record| {
            record
                .get("frameNumber")
                .and_then(serde_json::Value::as_u64)
        })
        .collect::<Vec<_>>();
    let frame_range = match (frame_numbers.first(), frame_numbers.last()) {
        (Some(first), Some(last)) => serde_json::json!({"first": first, "last": last}),
        _ => serde_json::Value::Null,
    };

    Ok(serde_json::json!({
        "title": "COLMAP image-list summary",
        "operation": IMAGE_LIST_OPERATION,
        "summary": {
            "imageCount": records.len(),
            "cameraCount": camera_groups.len(),
            "frameNumbersDetected": frame_numbers.len(),
            "frameRange": frame_range,
        },
        "message": "This report only inspects inline JSON. It does not scan an image directory on disk.",
        "imageCount": records.len(),
        "cameraGroups": camera_groups,
        "cameraGroupList": camera_group_list,
        "images": records,
    }))
}

fn image_values_from_input(input: &serde_json::Value) -> Result<Vec<serde_json::Value>, String> {
    let root = input
        .get("dataset")
        .or_else(|| input.get("input"))
        .unwrap_or(input);
    if let Some(images) = root.get("images").and_then(serde_json::Value::as_array) {
        return Ok(images.clone());
    }
    if let Some(paths) = root.get("imagePaths").and_then(serde_json::Value::as_array) {
        return Ok(paths
            .iter()
            .filter_map(|path| path.as_str())
            .map(|path| serde_json::json!({ "name": path, "cameraId": 1 }))
            .collect());
    }
    Err("imageList input must include `images` or `imagePaths`".to_string())
}

fn image_name(value: &serde_json::Value) -> Option<String> {
    if let Some(name) = value.as_str() {
        return Some(name.to_string());
    }
    value
        .get("name")
        .or_else(|| value.get("path"))
        .or_else(|| value.get("filePath"))
        .and_then(serde_json::Value::as_str)
        .map(ToString::to_string)
}

fn infer_frame_number(name: &str) -> Option<u64> {
    let mut digits = String::new();
    let mut last_digits = String::new();
    for character in name.chars() {
        if character.is_ascii_digit() {
            digits.push(character);
        } else if !digits.is_empty() {
            last_digits = std::mem::take(&mut digits);
        }
    }
    if !digits.is_empty() {
        last_digits = digits;
    }
    if last_digits.is_empty() {
        None
    } else {
        last_digits.parse().ok()
    }
}

fn sparse_summary_value(input: serde_json::Value) -> serde_json::Value {
    let root = input
        .get("dataset")
        .or_else(|| input.get("input"))
        .unwrap_or(&input);
    let cameras = array_len(root, &["cameras"]);
    let images = array_len(root, &["images"]);
    let points = points_array(root);
    let mut track_length_histogram = BTreeMap::<usize, usize>::new();
    for point in points {
        let track_length = point
            .get("track")
            .and_then(serde_json::Value::as_array)
            .map(Vec::len)
            .unwrap_or(0);
        *track_length_histogram.entry(track_length).or_default() += 1;
    }
    let track_lengths = points
        .iter()
        .map(|point| {
            point
                .get("track")
                .and_then(serde_json::Value::as_array)
                .map(Vec::len)
                .unwrap_or(0)
        })
        .collect::<Vec<_>>();
    let min_track_length = track_lengths.iter().min().copied().unwrap_or(0);
    let max_track_length = track_lengths.iter().max().copied().unwrap_or(0);
    let mean_track_length = if track_lengths.is_empty() {
        0.0
    } else {
        track_lengths.iter().sum::<usize>() as f64 / track_lengths.len() as f64
    };
    let histogram = track_length_histogram
        .iter()
        .map(|(track_length, point_count)| {
            serde_json::json!({
                "trackLength": track_length,
                "pointCount": point_count,
            })
        })
        .collect::<Vec<_>>();
    let model_status = if cameras == 0 && images == 0 && points.is_empty() {
        "empty"
    } else if points.is_empty() {
        "camera-only"
    } else {
        "sparse-points"
    };

    serde_json::json!({
        "title": "COLMAP sparse-model summary",
        "operation": SPARSE_SUMMARY_OPERATION,
        "summary": {
            "cameraCount": cameras,
            "registeredImageCount": images,
            "sparsePointCount": points.len(),
            "modelStatus": model_status,
        },
        "message": "This report summarizes already-parsed sparse model JSON. It does not read cameras.txt, images.txt, or points3D.txt from disk.",
        "cameraCount": cameras,
        "registeredImageCount": images,
        "sparsePointCount": points.len(),
        "trackLengthHistogram": track_length_histogram,
        "trackLength": {
            "min": min_track_length,
            "max": max_track_length,
            "mean": mean_track_length,
            "histogram": histogram,
        },
    })
}

fn array_len(root: &serde_json::Value, keys: &[&str]) -> usize {
    keys.iter()
        .find_map(|key| root.get(*key).and_then(serde_json::Value::as_array))
        .map(Vec::len)
        .unwrap_or(0)
}

fn points_array(root: &serde_json::Value) -> &[serde_json::Value] {
    root.get("points3d")
        .or_else(|| root.get("points3D"))
        .or_else(|| root.get("points"))
        .and_then(serde_json::Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[])
}

fn command_stage(id: &str, tool: &str, description: &str, args: Vec<String>) -> serde_json::Value {
    serde_json::json!({
        "id": id,
        "tool": tool,
        "description": description,
        "command": shell_command(tool, &args),
        "args": args,
    })
}

fn shell_command(tool: &str, args: &[String]) -> String {
    std::iter::once(shell_quote(tool))
        .chain(args.iter().map(|arg| shell_quote(arg)))
        .collect::<Vec<_>>()
        .join(" ")
}

fn shell_quote(value: &str) -> String {
    if value.chars().all(|character| {
        character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.' | '/' | ':' | '=')
    }) {
        return value.to_string();
    }
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}

fn path_string(path: &Path) -> String {
    path.display().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn package_surface_has_multiple_operations() {
        let surface = package_surface();
        assert_eq!(surface.library, env!("CARGO_PKG_NAME"));
        assert!(surface
            .operations
            .iter()
            .any(|operation| operation.id.as_str() == "describe"));
        assert!(surface.operations.len() >= 3);
    }

    #[test]
    fn reconstruct_video_operation_is_server_only() {
        let surface = package_surface();
        let operation = surface
            .operations
            .iter()
            .find(|operation| operation.id.as_str() == RECONSTRUCT_VIDEO_OPERATION)
            .expect("reconstruct video operation");

        assert!(!operation.wasm_supported);
        assert!(operation.server_supported);
    }

    #[test]
    fn describe_operation_returns_surface_summary() {
        let response = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("describe"),
            input: serde_json::json!({"includeOperations": true}),
        })
        .expect("describe operation");

        assert_eq!(response.operation.as_str(), "describe");
        assert_eq!(response.value["library"], env!("CARGO_PKG_NAME"));
        assert!(response.value["operationCount"].as_u64().unwrap() >= 3);
    }

    #[test]
    fn command_plan_returns_explicit_stages() {
        let response = run_surface_operation(SurfaceRequest {
            operation: OperationId::new(COMMAND_PLAN_OPERATION),
            input: serde_json::json!({}),
        })
        .expect("command plan operation");

        assert_eq!(response.value["deterministic"], true);
        assert_eq!(response.value["executes"], false);
        assert_eq!(response.value["summary"]["willExecute"], false);
        assert_eq!(response.value["summary"]["stageCount"], 5);
        assert_eq!(
            response.value["message"],
            "This is a dry-run preview. Use video.colmap.reconstructVideo to execute the native reconstruction pipeline."
        );
        assert_eq!(response.value["stages"].as_array().unwrap().len(), 5);
        assert_eq!(response.value["stages"][0]["tool"], "ffmpeg");
        assert!(response.value["stages"][0]["command"]
            .as_str()
            .unwrap()
            .contains("ffmpeg -y -i"));
        assert_eq!(response.value["stages"][4]["id"], "textExport");
    }

    #[test]
    fn image_list_summarizes_frame_order_and_camera_groups() {
        let response = run_surface_operation(SurfaceRequest {
            operation: OperationId::new(IMAGE_LIST_OPERATION),
            input: serde_json::json!({
                "images": [
                    {"name": "frame_00001.jpg", "cameraId": 7},
                    {"name": "frame_00002.jpg", "cameraId": 7}
                ]
            }),
        })
        .expect("image list operation");

        assert_eq!(response.value["imageCount"], 2);
        assert_eq!(response.value["summary"]["cameraCount"], 1);
        assert_eq!(response.value["summary"]["frameRange"]["first"], 1);
        assert_eq!(response.value["summary"]["frameRange"]["last"], 2);
        assert_eq!(response.value["images"][0]["frameNumber"], 1);
        assert_eq!(response.value["cameraGroups"]["7"], 2);
        assert_eq!(response.value["cameraGroupList"][0]["cameraId"], 7);
        assert_eq!(response.value["cameraGroupList"][0]["imageCount"], 2);
    }

    #[test]
    fn sparse_summary_returns_scene_counts() {
        let response = run_surface_operation(SurfaceRequest {
            operation: OperationId::new(SPARSE_SUMMARY_OPERATION),
            input: serde_json::json!({
                "dataset": {
                    "cameras": [{"id": 1}],
                    "images": [{"id": 1}, {"id": 2}],
                    "points3d": [
                        {"id": 1, "track": [{"imageId": 1}, {"imageId": 2}]},
                        {"id": 2, "track": [{"imageId": 2}]}
                    ]
                }
            }),
        })
        .expect("sparse summary operation");

        assert_eq!(response.value["cameraCount"], 1);
        assert_eq!(response.value["registeredImageCount"], 2);
        assert_eq!(response.value["sparsePointCount"], 2);
        assert_eq!(response.value["summary"]["modelStatus"], "sparse-points");
        assert_eq!(response.value["trackLength"]["min"], 1);
        assert_eq!(response.value["trackLength"]["max"], 2);
        assert_eq!(response.value["trackLengthHistogram"]["1"], 1);
        assert_eq!(response.value["trackLengthHistogram"]["2"], 1);
        assert_eq!(
            response.value["trackLength"]["histogram"][0]["trackLength"],
            1
        );
    }

    #[test]
    fn generic_runner_reports_native_reconstruct_video_diagnostic() {
        let response = run_surface_operation(SurfaceRequest {
            operation: OperationId::new(RECONSTRUCT_VIDEO_OPERATION),
            input: serde_json::json!({}),
        })
        .expect("native diagnostic");

        assert_eq!(response.value["summary"]["status"], "unsupported");
        assert_eq!(response.value["result"]["supported"], false);
        assert_eq!(response.diagnostics[0].code.as_str(), "native_server_only");
    }
}
