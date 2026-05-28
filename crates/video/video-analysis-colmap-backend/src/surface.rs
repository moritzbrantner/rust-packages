//! Library-owned runtime surface for `video-analysis-colmap-backend`.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use video_analysis_core::runtime::{
    OperationId, PackageSurface, RuntimeCapabilities, SurfaceOperation, SurfaceRequest,
    SurfaceResponse,
};

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
                "Plan COLMAP commands",
                "Builds a deterministic COLMAP command plan without executing external tools.",
                reconstruct_video_example_request(),
            ),
            operation(
                IMAGE_LIST_OPERATION,
                "List COLMAP images",
                "Summarizes image paths, camera groups, and frame ordering for COLMAP ingestion.",
                serde_json::json!({
                    "images": [
                        {"name": "frame_00001.jpg", "cameraId": 1},
                        {"name": "frame_00002.jpg", "cameraId": 1}
                    ]
                }),
            ),
            operation(
                SPARSE_SUMMARY_OPERATION,
                "Summarize sparse model",
                "Summarizes sparse reconstruction cameras, images, and point statistics.",
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

    if operation.as_str() == RECONSTRUCT_VIDEO_OPERATION {
        return Err(format!(
            "`{}` is a native server-only operation; call the video-analysis-colmap-backend server or overview server dispatch path",
            RECONSTRUCT_VIDEO_OPERATION
        ));
    }

    let value = match operation.as_str() {
        "describe" => describe_value(&surface, request.input),
        COMMAND_PLAN_OPERATION => command_plan_value(request.input)?,
        IMAGE_LIST_OPERATION => image_list_value(request.input)?,
        SPARSE_SUMMARY_OPERATION => sparse_summary_value(request.input),
        _ => deterministic_operation_value(&surface, surface_operation, request.input),
    };

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
    serde_json::json!({
        "library": &surface.library,
        "version": &surface.version,
        "operation": operation_id,
        "name": &operation.name,
        "description": &operation.description,
        "deterministic": true,
        "externalToolsRequired": false,
        "request": input,
        "plan": {
            "accepts": "JSON request metadata for the operation-specific package surface",
            "produces": "A deterministic summary or execution plan owned by the Rust library",
            "operationFamily": operation_family(operation_id),
        }
    })
}

fn operation_family(operation_id: &str) -> &str {
    operation_id
        .split_once('.')
        .map(|(family, _)| family)
        .unwrap_or(operation_id)
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

    Ok(serde_json::json!({
        "operation": COMMAND_PLAN_OPERATION,
        "deterministic": true,
        "executes": false,
        "externalToolsRequired": ["ffmpeg", "colmap"],
        "request": request,
        "outputs": {
            "framesDir": path_string(&frames_dir),
            "databasePath": path_string(&database_path),
            "sparseBinaryDir": path_string(&sparse_binary_dir),
            "sparseTextDir": path_string(&sparse_text_dir)
        },
        "stages": [
            {
                "id": "extractFrames",
                "tool": "ffmpeg",
                "description": "Extract bounded JPEG frames from the local video.",
                "args": [
                    "-y", "-i", path_string(&request.video_path), "-vf", filter,
                    "-frames:v", request.max_frames.to_string(), "-q:v", "2",
                    path_string(&frame_pattern)
                ]
            },
            {
                "id": "featureExtraction",
                "tool": "colmap",
                "description": "Detect local features for extracted frames with one shared camera.",
                "args": [
                    "feature_extractor", "--database_path", path_string(&database_path),
                    "--image_path", path_string(&frames_dir), "--ImageReader.single_camera", "1",
                    "--SiftExtraction.use_gpu", "0"
                ]
            },
            {
                "id": "matching",
                "tool": "colmap",
                "description": "Exhaustively match extracted frame features.",
                "args": [
                    "exhaustive_matcher", "--database_path", path_string(&database_path),
                    "--SiftMatching.use_gpu", "0"
                ]
            },
            {
                "id": "mapping",
                "tool": "colmap",
                "description": "Register frames and build a sparse reconstruction.",
                "args": [
                    "mapper", "--database_path", path_string(&database_path),
                    "--image_path", path_string(&frames_dir), "--output_path", path_string(&sparse_root)
                ]
            },
            {
                "id": "textExport",
                "tool": "colmap",
                "description": "Convert COLMAP binary sparse model into text files for Rust parsing.",
                "args": [
                    "model_converter", "--input_path", path_string(&sparse_binary_dir),
                    "--output_path", path_string(&sparse_text_dir), "--output_type", "TXT"
                ]
            }
        ]
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
    let records = images
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
                "index": index,
                "name": name,
                "cameraId": camera_id,
                "frameNumber": infer_frame_number(&name),
            })
        })
        .collect::<Vec<_>>();

    let mut camera_groups = BTreeMap::<u64, usize>::new();
    for record in &records {
        if let Some(camera_id) = record.get("cameraId").and_then(serde_json::Value::as_u64) {
            *camera_groups.entry(camera_id).or_default() += 1;
        }
    }

    Ok(serde_json::json!({
        "operation": IMAGE_LIST_OPERATION,
        "imageCount": records.len(),
        "cameraGroups": camera_groups,
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

    serde_json::json!({
        "operation": SPARSE_SUMMARY_OPERATION,
        "cameraCount": cameras,
        "registeredImageCount": images,
        "sparsePointCount": points.len(),
        "trackLengthHistogram": track_length_histogram,
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
        assert_eq!(response.value["stages"].as_array().unwrap().len(), 5);
        assert_eq!(response.value["stages"][0]["tool"], "ffmpeg");
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
        assert_eq!(response.value["images"][0]["frameNumber"], 1);
        assert_eq!(response.value["cameraGroups"]["7"], 2);
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
        assert_eq!(response.value["trackLengthHistogram"]["1"], 1);
        assert_eq!(response.value["trackLengthHistogram"]["2"], 1);
    }

    #[test]
    fn generic_runner_rejects_native_reconstruct_video_operation() {
        let error = run_surface_operation(SurfaceRequest {
            operation: OperationId::new(RECONSTRUCT_VIDEO_OPERATION),
            input: serde_json::json!({}),
        })
        .unwrap_err();

        assert!(error.contains("server-only"));
    }
}
