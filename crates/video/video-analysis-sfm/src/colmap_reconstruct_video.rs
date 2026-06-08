use std::path::{Path, PathBuf};

use runtime_core::{Diagnostic, DiagnosticSeverity, OperationId, SurfaceResponse};
use serde::{Deserialize, Serialize};
use video_analysis_radiance_io::read_colmap_text_dir;

use crate::colmap_scene::{
    build_colmap_scene_from_dataset, empty_colmap_scene, empty_colmap_scene_summary,
    scene_summary_from_dataset, ColmapScene, ColmapSceneSummary,
};
use crate::{build_colmap_scene, load_colmap_text_baseline, surface};

/// Workspace-relative path for the optional downloaded COLMAP sample video.
pub const COLMAP_TEST_VIDEO_PATH: &str =
    "prototypes/web/video-analysis-web/public/samples/video/test-video.mp4";
/// Browser URL for the optional downloaded COLMAP sample video.
pub const COLMAP_TEST_VIDEO_URL: &str = "/samples/video/test-video.mp4";
/// Default workspace-relative output directory for native COLMAP sample runs.
pub const DEFAULT_RECONSTRUCT_OUTPUT_DIR: &str = ".external-test-tools/colmap-runs/test-video";

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
/// Request for a native COLMAP reconstruction from a local video.
pub struct ReconstructVideoRequest {
    /// Workspace-relative or absolute local video path.
    pub video_path: PathBuf,
    /// Browser URL for previewing the same video.
    #[serde(default)]
    pub video_url: Option<String>,
    /// Workspace-relative output directory under `.external-test-tools/colmap-runs`.
    #[serde(default = "default_reconstruct_output_dir")]
    pub output_dir: PathBuf,
    /// Frame extraction rate.
    #[serde(default = "default_frame_fps")]
    pub frame_fps: f32,
    /// Maximum number of frames to extract.
    #[serde(default = "default_max_frames")]
    pub max_frames: usize,
    /// Maximum extracted image width.
    #[serde(default = "default_image_width")]
    pub image_width: u32,
    /// Whether to remove the selected output directory before running.
    #[serde(default)]
    pub clean: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
/// Response value for a native COLMAP video reconstruction.
pub struct ReconstructVideoResult {
    /// Video metadata.
    pub video: ReconstructVideoVideo,
    /// Extracted frame metadata.
    pub frames: ReconstructVideoFrames,
    /// COLMAP output locations.
    pub colmap: ReconstructVideoColmap,
    /// Sparse reconstruction summary.
    pub summary: ColmapSceneSummary,
    /// Browser-friendly sparse scene.
    pub scene: ColmapScene,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// Video metadata for the reconstruction response.
pub struct ReconstructVideoVideo {
    /// Local filesystem path.
    pub path: String,
    /// Browser URL when supplied.
    pub url: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// Extracted frame metadata for the reconstruction response.
pub struct ReconstructVideoFrames {
    /// Frame directory path.
    pub dir: String,
    /// Extracted frame count.
    pub count: usize,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// COLMAP output locations for the reconstruction response.
pub struct ReconstructVideoColmap {
    /// COLMAP database path.
    pub database_path: String,
    /// Binary sparse model directory.
    pub sparse_binary_dir: String,
    /// Text sparse model directory.
    pub sparse_text_dir: String,
}

/// Returns the default request used by the COLMAP package UI and surface examples.
pub fn default_reconstruct_video_request() -> ReconstructVideoRequest {
    ReconstructVideoRequest {
        video_path: PathBuf::from(COLMAP_TEST_VIDEO_PATH),
        video_url: Some(COLMAP_TEST_VIDEO_URL.to_string()),
        output_dir: default_reconstruct_output_dir(),
        frame_fps: default_frame_fps(),
        max_frames: default_max_frames(),
        image_width: default_image_width(),
        clean: false,
    }
}

fn default_reconstruct_output_dir() -> PathBuf {
    PathBuf::from(DEFAULT_RECONSTRUCT_OUTPUT_DIR)
}

fn default_frame_fps() -> f32 {
    2.0
}

fn default_max_frames() -> usize {
    80
}

fn default_image_width() -> u32 {
    1280
}

#[cfg(not(target_family = "wasm"))]
#[derive(Debug, Clone)]
struct ValidatedReconstructVideoRequest {
    request: ReconstructVideoRequest,
    video_path: PathBuf,
    output_dir: PathBuf,
}

#[cfg(not(target_family = "wasm"))]
/// Validates a native COLMAP video reconstruction request against the workspace.
pub fn validate_reconstruct_video_request(
    request: ReconstructVideoRequest,
) -> std::result::Result<ReconstructVideoRequest, String> {
    validate_reconstruct_video_request_inner(request).map(|validated| validated.request)
}

#[cfg(not(target_family = "wasm"))]
fn validate_reconstruct_video_request_inner(
    request: ReconstructVideoRequest,
) -> std::result::Result<ValidatedReconstructVideoRequest, String> {
    if request.frame_fps <= 0.0 || !request.frame_fps.is_finite() {
        return Err("frameFps must be a positive finite number".to_string());
    }
    if request.max_frames == 0 {
        return Err("maxFrames must be greater than zero".to_string());
    }
    if request.image_width == 0 {
        return Err("imageWidth must be greater than zero".to_string());
    }

    let workspace = std::env::current_dir()
        .map_err(|error| format!("failed to resolve current workspace directory: {error}"))?
        .canonicalize()
        .map_err(|error| format!("failed to canonicalize workspace directory: {error}"))?;

    let requested_video = absolutize(&workspace, &request.video_path);
    let video_path = requested_video.canonicalize().map_err(|error| {
        format!(
            "videoPath `{}` is not readable: {error}",
            request.video_path.display()
        )
    })?;
    if !video_path.starts_with(&workspace) {
        return Err(format!(
            "videoPath `{}` must be inside workspace `{}`",
            video_path.display(),
            workspace.display()
        ));
    }
    let sample_root = workspace
        .join("prototypes/web/video-analysis-web/public/samples/video")
        .canonicalize()
        .unwrap_or_else(|_| {
            workspace.join("prototypes/web/video-analysis-web/public/samples/video")
        });
    if !video_path.starts_with(&sample_root) {
        return Err(format!(
            "videoPath `{}` must be under `{}` for this server-supported workflow",
            video_path.display(),
            sample_root.display()
        ));
    }

    let output_root = workspace.join(".external-test-tools/colmap-runs");
    let requested_output = absolutize(&workspace, &request.output_dir);
    if !requested_output.starts_with(&output_root) {
        return Err(format!(
            "outputDir `{}` must be under `{}`",
            requested_output.display(),
            output_root.display()
        ));
    }
    let output_dir = canonicalize_existing_or_parent(&requested_output).map_err(|error| {
        format!(
            "failed to validate outputDir `{}`: {error}",
            request.output_dir.display()
        )
    })?;
    let canonical_output_root = canonicalize_existing_or_parent(&output_root).map_err(|error| {
        format!(
            "failed to validate output root `{}`: {error}",
            output_root.display()
        )
    })?;
    if !output_dir.starts_with(&canonical_output_root) {
        return Err(format!(
            "outputDir `{}` must canonicalize under `{}`",
            output_dir.display(),
            canonical_output_root.display()
        ));
    }

    Ok(ValidatedReconstructVideoRequest {
        request,
        video_path,
        output_dir,
    })
}

#[cfg(not(target_family = "wasm"))]
fn absolutize(workspace: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        workspace.join(path)
    }
}

#[cfg(not(target_family = "wasm"))]
fn canonicalize_existing_or_parent(path: &Path) -> std::io::Result<PathBuf> {
    if path.exists() {
        return path.canonicalize();
    }
    let Some(parent) = path.parent() else {
        return path.canonicalize();
    };
    let parent = if parent.exists() {
        parent.canonicalize()?
    } else {
        canonicalize_existing_or_parent(parent)?
    };
    let name = path.file_name().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "path has no final segment",
        )
    })?;
    Ok(parent.join(name))
}

#[cfg(not(target_family = "wasm"))]
/// Runs the native server-only COLMAP video reconstruction surface operation.
pub fn reconstruct_video_surface_operation(
    input: serde_json::Value,
) -> std::result::Result<SurfaceResponse, String> {
    let operation = OperationId::new(surface::RECONSTRUCT_VIDEO_OPERATION);
    let request = match serde_json::from_value::<ReconstructVideoRequest>(input) {
        Ok(request) => request,
        Err(error) => {
            return Ok(error_response(
                operation,
                "invalid_request",
                format!("invalid reconstructVideo request: {error}"),
            ));
        }
    };
    let validated = match validate_reconstruct_video_request_inner(request) {
        Ok(validated) => validated,
        Err(error) => return Ok(error_response(operation, "invalid_request", error)),
    };

    let mut diagnostics = Vec::new();
    let frames_dir = validated.output_dir.join("frames");
    let database_path = validated.output_dir.join("database.db");
    let sparse_root = validated.output_dir.join("sparse");
    let sparse_binary_dir = sparse_root.join("0");
    let sparse_text_dir = validated.output_dir.join("sparse_txt");
    let scene_json_path = validated.output_dir.join("scene.json");

    if let Err(error) = prepare_output_dir(&validated, &frames_dir, &sparse_root, &sparse_text_dir)
    {
        return Ok(error_response(operation, "output_prepare_failed", error));
    }

    let Some(ffmpeg) = find_executable(&["ffmpeg", "/usr/bin/ffmpeg"]) else {
        return Ok(response_with_paths(
            operation,
            &validated,
            0,
            &database_path,
            &sparse_binary_dir,
            &sparse_text_dir,
            vec![diagnostic(
                DiagnosticSeverity::Error,
                "missing_tool",
                "ffmpeg was not found on PATH or at /usr/bin/ffmpeg",
            )],
        ));
    };
    let Some(colmap) = find_executable(&["colmap", "/usr/local/bin/colmap"]) else {
        return Ok(response_with_paths(
            operation,
            &validated,
            0,
            &database_path,
            &sparse_binary_dir,
            &sparse_text_dir,
            vec![diagnostic(
                DiagnosticSeverity::Error,
                "missing_tool",
                "COLMAP was not found on PATH or at /usr/local/bin/colmap",
            )],
        ));
    };

    let frame_pattern = frames_dir.join("frame_%05d.jpg");
    let filter = format!(
        "fps={},scale='min({},iw)':-2",
        validated.request.frame_fps, validated.request.image_width
    );
    if let Err(error) = run_command(
        "ffmpeg",
        &ffmpeg,
        &[
            "-y".into(),
            "-i".into(),
            path_arg(&validated.video_path),
            "-vf".into(),
            filter,
            "-frames:v".into(),
            validated.request.max_frames.to_string(),
            "-q:v".into(),
            "2".into(),
            path_arg(&frame_pattern),
        ],
    ) {
        diagnostics.push(error);
        return Ok(response_with_paths(
            operation,
            &validated,
            count_frames(&frames_dir),
            &database_path,
            &sparse_binary_dir,
            &sparse_text_dir,
            diagnostics,
        ));
    }

    let frame_count = count_frames(&frames_dir);
    if frame_count < 2 {
        diagnostics.push(diagnostic(
            DiagnosticSeverity::Warning,
            "too_few_frames",
            format!("only {frame_count} frame(s) were extracted; COLMAP needs at least two frames"),
        ));
        return Ok(response_with_paths(
            operation,
            &validated,
            frame_count,
            &database_path,
            &sparse_binary_dir,
            &sparse_text_dir,
            diagnostics,
        ));
    }

    let commands = [
        (
            "colmap feature_extractor",
            vec![
                "feature_extractor".into(),
                "--database_path".into(),
                path_arg(&database_path),
                "--image_path".into(),
                path_arg(&frames_dir),
                "--ImageReader.single_camera".into(),
                "1".into(),
                "--SiftExtraction.use_gpu".into(),
                "0".into(),
            ],
        ),
        (
            "colmap exhaustive_matcher",
            vec![
                "exhaustive_matcher".into(),
                "--database_path".into(),
                path_arg(&database_path),
                "--SiftMatching.use_gpu".into(),
                "0".into(),
            ],
        ),
        (
            "colmap mapper",
            vec![
                "mapper".into(),
                "--database_path".into(),
                path_arg(&database_path),
                "--image_path".into(),
                path_arg(&frames_dir),
                "--output_path".into(),
                path_arg(&sparse_root),
            ],
        ),
    ];
    for (label, args) in commands {
        if let Err(error) = run_command(label, &colmap, &args) {
            diagnostics.push(error);
            return Ok(response_with_paths(
                operation,
                &validated,
                frame_count,
                &database_path,
                &sparse_binary_dir,
                &sparse_text_dir,
                diagnostics,
            ));
        }
    }

    if !sparse_binary_dir.exists() {
        diagnostics.push(diagnostic(
            DiagnosticSeverity::Error,
            "no_sparse_model",
            format!(
                "COLMAP mapper completed but no sparse model was found at `{}`",
                sparse_binary_dir.display()
            ),
        ));
        return Ok(response_with_paths(
            operation,
            &validated,
            frame_count,
            &database_path,
            &sparse_binary_dir,
            &sparse_text_dir,
            diagnostics,
        ));
    }

    if let Err(error) = run_command(
        "colmap model_converter",
        &colmap,
        &[
            "model_converter".into(),
            "--input_path".into(),
            path_arg(&sparse_binary_dir),
            "--output_path".into(),
            path_arg(&sparse_text_dir),
            "--output_type".into(),
            "TXT".into(),
        ],
    ) {
        diagnostics.push(error);
        return Ok(response_with_paths(
            operation,
            &validated,
            frame_count,
            &database_path,
            &sparse_binary_dir,
            &sparse_text_dir,
            diagnostics,
        ));
    }

    let (summary, scene) = match load_colmap_text_baseline(&sparse_text_dir) {
        Ok(baseline) => {
            let scene = match build_colmap_scene(&baseline) {
                Ok(scene) => scene,
                Err(error) => {
                    diagnostics.push(diagnostic(
                        DiagnosticSeverity::Error,
                        "scene_build_failed",
                        error.to_string(),
                    ));
                    return Ok(response_with_paths(
                        operation,
                        &validated,
                        frame_count,
                        &database_path,
                        &sparse_binary_dir,
                        &sparse_text_dir,
                        diagnostics,
                    ));
                }
            };
            (
                ColmapSceneSummary {
                    camera_count: baseline.report.camera_count,
                    registered_image_count: baseline.report.registered_image_count,
                    sparse_point_count: baseline.report.sparse_point_count,
                    track_length_histogram: baseline.report.track_length_histogram,
                },
                scene,
            )
        }
        Err(error) => {
            let dataset = match read_colmap_text_dir(&sparse_text_dir) {
                Ok(dataset) => dataset,
                Err(load_error) => {
                    diagnostics.push(diagnostic(
                        DiagnosticSeverity::Error,
                        "sparse_text_load_failed",
                        format!("{error}; fallback text parse also failed: {load_error}"),
                    ));
                    return Ok(response_with_paths(
                        operation,
                        &validated,
                        frame_count,
                        &database_path,
                        &sparse_binary_dir,
                        &sparse_text_dir,
                        diagnostics,
                    ));
                }
            };
            diagnostics.push(diagnostic(
                DiagnosticSeverity::Warning,
                "baseline_conversion_unsupported",
                format!(
                    "full sparse reconstruction conversion failed ({error}); using raw COLMAP text poses and points for the browser scene"
                ),
            ));
            let scene = match build_colmap_scene_from_dataset(&dataset) {
                Ok(scene) => scene,
                Err(scene_error) => {
                    diagnostics.push(diagnostic(
                        DiagnosticSeverity::Error,
                        "scene_build_failed",
                        scene_error.to_string(),
                    ));
                    return Ok(response_with_paths(
                        operation,
                        &validated,
                        frame_count,
                        &database_path,
                        &sparse_binary_dir,
                        &sparse_text_dir,
                        diagnostics,
                    ));
                }
            };
            (scene_summary_from_dataset(&dataset), scene)
        }
    };
    let result = result_with_scene(
        &validated,
        frame_count,
        &database_path,
        &sparse_binary_dir,
        &sparse_text_dir,
        summary,
        scene,
    );
    if let Ok(json) = serde_json::to_string_pretty(&result.scene) {
        if let Err(error) = std::fs::write(&scene_json_path, json) {
            diagnostics.push(diagnostic(
                DiagnosticSeverity::Warning,
                "scene_json_write_failed",
                format!("failed to write scene JSON metadata: {error}"),
            ));
        }
    }

    let mut artifacts = artifacts_for(
        &validated.output_dir.join("frames"),
        &database_path,
        &sparse_binary_dir,
        &sparse_text_dir,
    );
    artifacts.push(serde_json::json!({
        "id": "scene-json",
        "kind": "metadata",
        "path": path_arg(&scene_json_path)
    }));

    Ok(SurfaceResponse {
        operation,
        value: serde_json::to_value(result).map_err(|error| error.to_string())?,
        diagnostics,
        artifacts,
    })
}

#[cfg(target_family = "wasm")]
/// Reports native-only unsupported execution for WASM builds.
pub fn reconstruct_video_surface_operation(
    _input: serde_json::Value,
) -> std::result::Result<SurfaceResponse, String> {
    Err("video.colmap.reconstructVideo is only available from native server dispatch".to_string())
}

#[cfg(not(target_family = "wasm"))]
fn prepare_output_dir(
    validated: &ValidatedReconstructVideoRequest,
    frames_dir: &Path,
    sparse_root: &Path,
    sparse_text_dir: &Path,
) -> std::result::Result<(), String> {
    if validated.request.clean && validated.output_dir.exists() {
        std::fs::remove_dir_all(&validated.output_dir).map_err(|error| {
            format!(
                "failed to clean outputDir `{}`: {error}",
                validated.output_dir.display()
            )
        })?;
    }
    std::fs::create_dir_all(frames_dir).map_err(|error| {
        format!(
            "failed to create frame directory `{}`: {error}",
            frames_dir.display()
        )
    })?;
    std::fs::create_dir_all(sparse_root).map_err(|error| {
        format!(
            "failed to create sparse directory `{}`: {error}",
            sparse_root.display()
        )
    })?;
    if sparse_text_dir.exists() {
        std::fs::remove_dir_all(sparse_text_dir).map_err(|error| {
            format!(
                "failed to clear sparse text directory `{}`: {error}",
                sparse_text_dir.display()
            )
        })?;
    }
    std::fs::create_dir_all(sparse_text_dir).map_err(|error| {
        format!(
            "failed to create sparse text directory `{}`: {error}",
            sparse_text_dir.display()
        )
    })?;
    Ok(())
}

#[cfg(not(target_family = "wasm"))]
pub(crate) fn find_executable(candidates: &[&str]) -> Option<PathBuf> {
    for candidate in candidates {
        let path = PathBuf::from(candidate);
        if path.components().count() > 1 && path.is_file() {
            return Some(path);
        }
        if path.components().count() == 1 {
            if let Some(found) = find_on_path(candidate) {
                return Some(found);
            }
        }
    }
    None
}

#[cfg(not(target_family = "wasm"))]
fn find_on_path(name: &str) -> Option<PathBuf> {
    let paths = std::env::var_os("PATH")?;
    std::env::split_paths(&paths)
        .map(|dir| dir.join(name))
        .find(|candidate| candidate.is_file())
}

#[cfg(not(target_family = "wasm"))]
fn run_command(
    label: &str,
    executable: &Path,
    args: &[String],
) -> std::result::Result<(), Diagnostic> {
    let output = std::process::Command::new(executable)
        .args(args)
        .output()
        .map_err(|error| {
            diagnostic(
                DiagnosticSeverity::Error,
                "command_spawn_failed",
                format!("{label} failed to start: {error}"),
            )
        })?;
    if output.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let details = if stderr.trim().is_empty() {
        stdout.trim()
    } else {
        stderr.trim()
    };
    Err(diagnostic(
        DiagnosticSeverity::Error,
        "command_failed",
        format!(
            "{label} exited with status {}: {}",
            output.status,
            truncate(details, 1200)
        ),
    ))
}

#[cfg(not(target_family = "wasm"))]
fn count_frames(frames_dir: &Path) -> usize {
    std::fs::read_dir(frames_dir)
        .map(|entries| {
            entries
                .filter_map(std::result::Result::ok)
                .filter(|entry| {
                    entry
                        .path()
                        .extension()
                        .and_then(|extension| extension.to_str())
                        .map(|extension| extension.eq_ignore_ascii_case("jpg"))
                        .unwrap_or(false)
                })
                .count()
        })
        .unwrap_or(0)
}

#[cfg(not(target_family = "wasm"))]
fn path_arg(path: &Path) -> String {
    path.display().to_string()
}

#[cfg(not(target_family = "wasm"))]
fn truncate(value: &str, max_chars: usize) -> String {
    let mut output = value.chars().take(max_chars).collect::<String>();
    if value.chars().count() > max_chars {
        output.push_str("...");
    }
    output
}

#[cfg(not(target_family = "wasm"))]
fn response_with_paths(
    operation: OperationId,
    validated: &ValidatedReconstructVideoRequest,
    frame_count: usize,
    database_path: &Path,
    sparse_binary_dir: &Path,
    sparse_text_dir: &Path,
    diagnostics: Vec<Diagnostic>,
) -> SurfaceResponse {
    let result = result_with_scene(
        validated,
        frame_count,
        database_path,
        sparse_binary_dir,
        sparse_text_dir,
        empty_colmap_scene_summary(),
        empty_colmap_scene(),
    );
    SurfaceResponse {
        operation,
        value: serde_json::to_value(result).unwrap_or_else(|_| serde_json::json!({})),
        diagnostics,
        artifacts: artifacts_for(
            &validated.output_dir.join("frames"),
            database_path,
            sparse_binary_dir,
            sparse_text_dir,
        ),
    }
}

#[cfg(not(target_family = "wasm"))]
fn result_with_scene(
    validated: &ValidatedReconstructVideoRequest,
    frame_count: usize,
    database_path: &Path,
    sparse_binary_dir: &Path,
    sparse_text_dir: &Path,
    summary: ColmapSceneSummary,
    scene: ColmapScene,
) -> ReconstructVideoResult {
    ReconstructVideoResult {
        video: ReconstructVideoVideo {
            path: path_arg(&validated.video_path),
            url: validated.request.video_url.clone(),
        },
        frames: ReconstructVideoFrames {
            dir: path_arg(&validated.output_dir.join("frames")),
            count: frame_count,
        },
        colmap: ReconstructVideoColmap {
            database_path: path_arg(database_path),
            sparse_binary_dir: path_arg(sparse_binary_dir),
            sparse_text_dir: path_arg(sparse_text_dir),
        },
        summary,
        scene,
    }
}

#[cfg(not(target_family = "wasm"))]
fn artifacts_for(
    frames_dir: &Path,
    database_path: &Path,
    sparse_binary_dir: &Path,
    sparse_text_dir: &Path,
) -> Vec<serde_json::Value> {
    vec![
        serde_json::json!({"id": "frames", "kind": "directory", "path": path_arg(frames_dir)}),
        serde_json::json!({"id": "colmap-database", "kind": "sqlite", "path": path_arg(database_path)}),
        serde_json::json!({"id": "sparse-binary", "kind": "colmap-binary-model", "path": path_arg(sparse_binary_dir)}),
        serde_json::json!({"id": "sparse-text", "kind": "colmap-text-model", "path": path_arg(sparse_text_dir)}),
    ]
}

#[cfg(not(target_family = "wasm"))]
fn error_response(
    operation: OperationId,
    code: &str,
    message: impl Into<String>,
) -> SurfaceResponse {
    SurfaceResponse {
        operation,
        value: serde_json::json!({}),
        diagnostics: vec![diagnostic(DiagnosticSeverity::Error, code, message)],
        artifacts: Vec::new(),
    }
}

#[cfg(not(target_family = "wasm"))]
fn diagnostic(
    severity: DiagnosticSeverity,
    code: impl Into<String>,
    message: impl Into<String>,
) -> Diagnostic {
    Diagnostic {
        severity,
        code: code.into().into(),
        message: message.into(),
        source: Some(env!("CARGO_PKG_NAME").to_string()),
        help: None,
    }
}
