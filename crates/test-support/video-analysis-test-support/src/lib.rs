//! Public API for the video-analysis-test-support crate.

pub mod surface;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use num_rational::Rational64;
use video_analysis_core::{
    AnalysisEvent, FramePosition, Observation, ObservationKind, OwnedTextSegment, OwnedVideoFrame,
    PixelFormat, Scene, SceneDetector, ScenePipeline, TextSegment, Timebase, Timestamp,
};
use video_analysis_dataset::{
    AnalysisDataset, DatasetRecord, FeatureRecord, FeatureValue, SceneRecord, TextSegmentRecord,
};
use video_analysis_radiance_fields::{CameraModel, ColorRgb, Vec2, Vec3};
use video_analysis_radiance_io::{
    ColmapCamera, ColmapDataset, ColmapImage, ColmapPoint2d, ColmapPoint3d, ColmapTrackElement,
    NerfstudioFrame, NerfstudioTransforms,
};

/// Constant for me at the zoo wikimedia URL.
pub const ME_AT_THE_ZOO_WIKIMEDIA_URL: &str =
    "https://upload.wikimedia.org/wikipedia/commons/e/e0/Me_at_the_zoo.webm";

/// Returns timestamp.
pub fn timestamp(seconds: i64) -> Timestamp {
    Timestamp::new(seconds, Timebase::new(1, 1))
}

/// Returns timestamp at.
pub fn timestamp_at(pts: i64, num: i32, den: i32) -> Timestamp {
    Timestamp::new(pts, Timebase::new(num, den))
}

/// Returns frame position.
pub fn frame_position(frame_index: u64) -> FramePosition {
    FramePosition {
        frame_index,
        timestamp: timestamp(frame_index as i64),
    }
}

/// Returns frame position at rate.
pub fn frame_position_at_rate(frame_index: u64, frame_rate: i64) -> FramePosition {
    FramePosition::from_frame_index(frame_index, Rational64::new(frame_rate, 1))
}

/// Returns scene.
pub fn scene(start: u64, end: u64) -> Scene {
    Scene {
        start: frame_position(start),
        end: frame_position(end),
    }
}

/// Returns RGB frame.
pub fn rgb_frame(width: u32, height: u32, frame_index: u64, rgb: [u8; 3]) -> OwnedVideoFrame {
    let mut data = Vec::with_capacity(width as usize * height as usize * 3);
    for _ in 0..(width as usize * height as usize) {
        data.extend_from_slice(&rgb);
    }
    OwnedVideoFrame {
        position: frame_position(frame_index),
        width,
        height,
        pixel_format: PixelFormat::Rgb24,
        stride: width as usize * 3,
        data,
    }
}

/// Returns checkerboard frame.
pub fn checkerboard_frame(width: u32, height: u32, frame_index: u64) -> OwnedVideoFrame {
    let mut data = Vec::with_capacity(width as usize * height as usize * 3);
    for y in 0..height {
        for x in 0..width {
            let value = if (x + y) % 2 == 0 { 255 } else { 0 };
            data.extend_from_slice(&[value, value, value]);
        }
    }
    OwnedVideoFrame {
        position: frame_position(frame_index),
        width,
        height,
        pixel_format: PixelFormat::Rgb24,
        stride: width as usize * 3,
        data,
    }
}

/// Specification for deterministic synthetic video fixtures.
#[derive(Debug, Clone)]
pub struct SyntheticVideoSpec {
    /// Frames per second.
    pub fps: Rational64,
    /// Frame width in pixels.
    pub width: u32,
    /// Frame height in pixels.
    pub height: u32,
    /// Number of frames to generate.
    pub frame_count: u64,
    /// Ordered scene patterns. Later patterns override earlier pixels.
    pub patterns: Vec<ScenePattern>,
}

impl SyntheticVideoSpec {
    /// Creates a new synthetic video spec with a dark base frame.
    pub fn new(width: u32, height: u32, frame_count: u64, fps: Rational64) -> Self {
        Self {
            fps,
            width,
            height,
            frame_count,
            patterns: vec![ScenePattern::SolidColor {
                start_frame: 0,
                end_frame: frame_count,
                rgb: [32, 32, 32],
            }],
        }
    }

    /// Adds a pattern to this spec.
    pub fn pattern(mut self, pattern: ScenePattern) -> Self {
        self.patterns.push(pattern);
        self
    }
}

/// Deterministic frame patterns for scene-detection tests and benchmarks.
#[derive(Debug, Clone)]
pub enum ScenePattern {
    /// Fills a frame range with one color.
    SolidColor {
        /// Inclusive start frame.
        start_frame: u64,
        /// Exclusive end frame.
        end_frame: u64,
        /// RGB color.
        rgb: [u8; 3],
    },
    /// Applies a two-color hard cut at `at_frame`.
    HardCut {
        /// Cut frame.
        at_frame: u64,
        /// Color before the cut.
        before: [u8; 3],
        /// Color from the cut onward.
        after: [u8; 3],
    },
    /// Applies a temporary flash.
    Flash {
        /// Inclusive start frame.
        start_frame: u64,
        /// Flash length in frames.
        length: u64,
        /// Flash color.
        rgb: [u8; 3],
    },
    /// Creates a fade out/in around the darkest midpoint.
    FadeOutIn {
        /// Inclusive start frame.
        start_frame: u64,
        /// Exclusive end frame.
        end_frame: u64,
        /// Bright scene color.
        high: u8,
        /// Dark fade color.
        low: u8,
    },
    /// Applies a luminance ramp.
    LuminanceRamp {
        /// Inclusive start frame.
        start_frame: u64,
        /// Exclusive end frame.
        end_frame: u64,
        /// Starting luma.
        from: u8,
        /// Ending luma.
        to: u8,
    },
    /// Applies a deterministic stripe pattern with shifted luma distribution.
    HistogramShift {
        /// Inclusive start frame.
        start_frame: u64,
        /// Exclusive end frame.
        end_frame: u64,
    },
    /// Applies a deterministic checker texture offset that changes perceptual hashes.
    TextureShift {
        /// Inclusive start frame.
        start_frame: u64,
        /// Exclusive end frame.
        end_frame: u64,
        /// Checker offset.
        offset: u32,
    },
}

/// Generates deterministic synthetic video frames.
pub fn synthetic_frames(spec: &SyntheticVideoSpec) -> Vec<OwnedVideoFrame> {
    (0..spec.frame_count)
        .map(|frame_index| {
            let mut frame =
                rgb_frame_at_rate(spec.width, spec.height, frame_index, spec.fps, [32, 32, 32]);
            for pattern in &spec.patterns {
                apply_pattern(&mut frame, spec.frame_count, pattern);
            }
            frame
        })
        .collect()
}

/// Runs a scene detector on pre-generated frames using the public scene pipeline.
pub fn run_detector_on_frames<D: SceneDetector + 'static>(
    detector: D,
    frames: &[OwnedVideoFrame],
) -> video_analysis_core::Result<video_analysis_core::DetectionResult> {
    let mut pipeline = ScenePipeline::builder()
        .detector(detector)
        .start_in_scene(true)
        .build()?;
    for frame in frames {
        pipeline.process_frame(frame.clone())?;
    }
    pipeline.finish_detection()
}

/// Asserts scene start boundaries by frame index.
pub fn assert_scene_boundaries(
    result: &video_analysis_core::DetectionResult,
    expected_start_frames: &[u64],
) {
    let actual = result
        .scenes
        .iter()
        .map(|scene| scene.start.frame_index)
        .collect::<Vec<_>>();
    assert_eq!(actual, expected_start_frames);
}

/// Asserts a metric row contains a key.
pub fn assert_metric_present(metrics: &video_analysis_core::MetricsStore, frame: u64, key: &str) {
    let row = metrics
        .rows()
        .get(&frame)
        .unwrap_or_else(|| panic!("missing metric row for frame {frame}"));
    assert!(
        row.contains_key(key),
        "missing metric key `{key}` for frame {frame}; row keys: {:?}",
        row.keys().collect::<Vec<_>>()
    );
}

fn rgb_frame_at_rate(
    width: u32,
    height: u32,
    frame_index: u64,
    fps: Rational64,
    rgb: [u8; 3],
) -> OwnedVideoFrame {
    let mut frame = rgb_frame(width, height, frame_index, rgb);
    frame.position = FramePosition::from_frame_index(frame_index, fps);
    frame
}

fn apply_pattern(frame: &mut OwnedVideoFrame, frame_count: u64, pattern: &ScenePattern) {
    let frame_index = frame.position.frame_index;
    match *pattern {
        ScenePattern::SolidColor {
            start_frame,
            end_frame,
            rgb,
        } if in_range(frame_index, start_frame, end_frame) => fill_frame(frame, rgb),
        ScenePattern::HardCut {
            at_frame,
            before,
            after,
        } => {
            if frame_index < at_frame {
                fill_frame(frame, before);
            } else if frame_index < frame_count {
                fill_frame(frame, after);
            }
        }
        ScenePattern::Flash {
            start_frame,
            length,
            rgb,
        } if in_range(frame_index, start_frame, start_frame.saturating_add(length)) => {
            fill_frame(frame, rgb);
        }
        ScenePattern::FadeOutIn {
            start_frame,
            end_frame,
            high,
            low,
        } if in_range(frame_index, start_frame, end_frame) => {
            let len = end_frame.saturating_sub(start_frame).max(1);
            let local = frame_index.saturating_sub(start_frame);
            let midpoint = len / 2;
            let edge_weight = if local <= midpoint {
                1.0 - (local as f32 / midpoint.max(1) as f32)
            } else {
                (local.saturating_sub(midpoint)) as f32 / (len - midpoint).max(1) as f32
            };
            let value = low as f32 + (high as f32 - low as f32) * edge_weight.clamp(0.0, 1.0);
            fill_frame(frame, [value as u8, value as u8, value as u8]);
        }
        ScenePattern::LuminanceRamp {
            start_frame,
            end_frame,
            from,
            to,
        } if in_range(frame_index, start_frame, end_frame) => {
            let len = end_frame.saturating_sub(start_frame).max(1);
            let local = frame_index.saturating_sub(start_frame);
            let t = local as f32 / len as f32;
            let value = from as f32 + (to as f32 - from as f32) * t;
            fill_frame(frame, [value as u8, value as u8, value as u8]);
        }
        ScenePattern::HistogramShift {
            start_frame,
            end_frame,
        } if in_range(frame_index, start_frame, end_frame) => {
            for y in 0..frame.height {
                for x in 0..frame.width {
                    let value = if (x / 4 + y / 4) % 3 == 0 { 16 } else { 236 };
                    set_pixel(frame, x, y, [value, value, value]);
                }
            }
        }
        ScenePattern::TextureShift {
            start_frame,
            end_frame,
            offset,
        } if in_range(frame_index, start_frame, end_frame) => {
            for y in 0..frame.height {
                for x in 0..frame.width {
                    let value = ((x * 5 + y * 3 + offset * 17) % 256) as u8;
                    set_pixel(frame, x, y, [value, value, value]);
                }
            }
        }
        _ => {}
    }
}

fn in_range(frame_index: u64, start_frame: u64, end_frame: u64) -> bool {
    frame_index >= start_frame && frame_index < end_frame
}

fn fill_frame(frame: &mut OwnedVideoFrame, rgb: [u8; 3]) {
    for pixel in frame.data.chunks_exact_mut(3) {
        pixel.copy_from_slice(&rgb);
    }
}

fn set_pixel(frame: &mut OwnedVideoFrame, x: u32, y: u32, rgb: [u8; 3]) {
    let offset = y as usize * frame.stride + x as usize * 3;
    frame.data[offset..offset + 3].copy_from_slice(&rgb);
}

/// Returns owned text segment.
pub fn owned_text_segment(index: u64, text: impl Into<String>) -> OwnedTextSegment {
    OwnedTextSegment::new(index, text).timestamp(timestamp(index as i64))
}

/// Returns text segment.
pub fn text_segment(index: u64, text: &str) -> TextSegment<'_> {
    TextSegment {
        segment_index: index,
        timestamp: Some(timestamp(index as i64)),
        text,
        language: Some("en"),
        is_final: true,
    }
}

/// Returns dataset with scene text and feature.
pub fn dataset_with_scene_text_and_feature() -> AnalysisDataset {
    let mut dataset = AnalysisDataset::empty();
    let scene = scene(0, 2);
    let text = text_segment(0, "Rust video analysis test fixture.");
    dataset.push(DatasetRecord::Scene(SceneRecord::from_scene(0, &scene)));
    dataset.push(DatasetRecord::TextSegment(TextSegmentRecord::from_segment(
        "transcript",
        &text,
    )));
    dataset.extend_observations([Observation::new("fixture", ObservationKind::Text)
        .at_timestamp(timestamp(0))
        .in_scene(0)
        .label("fixture")
        .text(text.text)]);
    dataset
        .extend_events([AnalysisEvent::new("fixture", "text:fixture").at_timestamp(timestamp(0))]);
    dataset.push(DatasetRecord::Feature(
        FeatureRecord::new("fixture.vector", FeatureValue::Vector(vec![1.0, 2.0, 3.0]))
            .scope("global")
            .timestamp(timestamp(0)),
    ));
    dataset
}

/// Returns comfy workflow JSON.
pub fn comfy_workflow_json() -> &'static str {
    r#"{
  "version": 0.4,
  "nodes": [
    {"id": 1, "type": "LoadImage", "outputs": [{"name": "IMAGE", "type": "IMAGE", "links": [1]}]},
    {"id": 2, "type": "SaveImage", "inputs": [{"name": "images", "type": "IMAGE", "link": 1}]}
  ],
  "links": [[1, 1, 0, 2, 0, "IMAGE"]],
  "last_node_id": 2,
  "last_link_id": 1
}"#
}

/// Returns comfy prompt JSON.
pub fn comfy_prompt_json() -> &'static str {
    r#"{
  "1": {"class_type": "LoadImage", "inputs": {"image": "input.png"}},
  "2": {"class_type": "SaveImage", "inputs": {"images": ["1", 0]}}
}"#
}

/// Returns minimal COLMAP dataset.
pub fn minimal_colmap_dataset() -> ColmapDataset {
    ColmapDataset {
        cameras: vec![ColmapCamera {
            id: 1,
            model: CameraModel::Pinhole,
            raw_model: "PINHOLE".to_string(),
            width: 64,
            height: 48,
            params: vec![50.0, 50.0, 32.0, 24.0],
        }],
        images: vec![ColmapImage {
            id: 1,
            qw: 1.0,
            qx: 0.0,
            qy: 0.0,
            qz: 0.0,
            tx: 0.0,
            ty: 0.0,
            tz: 0.0,
            camera_id: 1,
            name: "frame_0001.png".to_string(),
            points2d: vec![ColmapPoint2d {
                xy: Vec2::new(32.0, 24.0),
                point3d_id: Some(1),
            }],
        }],
        points: vec![ColmapPoint3d {
            id: 1,
            xyz: Vec3::new(0.0, 0.0, 1.0),
            color: ColorRgb::new(1.0, 0.0, 0.0),
            error: 0.1,
            track: vec![ColmapTrackElement {
                image_id: 1,
                point2d_index: 0,
            }],
        }],
    }
}

/// Returns minimal Nerfstudio transforms.
pub fn minimal_nerfstudio_transforms() -> NerfstudioTransforms {
    NerfstudioTransforms {
        camera_model: Some("PINHOLE".to_string()),
        fl_x: Some(50.0),
        fl_y: Some(50.0),
        cx: Some(32.0),
        cy: Some(24.0),
        w: Some(64),
        h: Some(48),
        frames: vec![NerfstudioFrame {
            file_path: "images/frame_0001.png".to_string(),
            transform_matrix: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
            fl_x: None,
            fl_y: None,
            cx: None,
            cy: None,
            w: None,
            h: None,
        }],
    }
}

/// Returns require command.
pub fn require_command(command: &str) -> PathBuf {
    find_command(command).unwrap_or_else(|| panic!("required command `{command}` is unavailable"))
}

/// Returns find command.
pub fn find_command(command: &str) -> Option<PathBuf> {
    let path = Path::new(command);
    if path.components().count() > 1 && is_executable_command(path) {
        return Some(path.to_path_buf());
    }
    std::env::var_os("PATH").and_then(|paths| {
        std::env::split_paths(&paths)
            .map(|dir| dir.join(command))
            .find(|candidate| is_executable_command(candidate))
    })
}

/// Returns command succeeds.
pub fn command_succeeds(command: &str, args: &[&str]) -> bool {
    Command::new(command)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

/// Returns find python with modules.
pub fn find_python_with_modules(modules: &[&str]) -> Option<PathBuf> {
    let import = format!("import {}", modules.join(", "));
    [
        PathBuf::from("python3"),
        PathBuf::from("python"),
        PathBuf::from(".external-test-tools/model-python-venv/bin/python"),
    ]
    .into_iter()
    .find(|candidate| {
        Command::new(candidate)
            .args(["-c", &import])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|status| status.success())
            .unwrap_or(false)
    })
}

/// Returns ensure me at the zoo fixture.
pub fn ensure_me_at_the_zoo_fixture() -> PathBuf {
    if let Some(path) = std::env::var_os("ME_AT_THE_ZOO_VIDEO_PATH") {
        let path = PathBuf::from(path);
        assert_nonempty_file(&path);
        return path;
    }

    let url = std::env::var("ME_AT_THE_ZOO_VIDEO_URL")
        .unwrap_or_else(|_| ME_AT_THE_ZOO_WIKIMEDIA_URL.to_string());
    let cache_dir = std::env::temp_dir().join("video-analysis-fixtures");
    fs::create_dir_all(&cache_dir).expect("expected fixture cache directory to be writable");
    let path = cache_dir.join("me-at-the-zoo.webm");
    if path.exists() {
        assert_nonempty_file(&path);
        return path;
    }

    download_to_path(&url, &path);
    assert_nonempty_file(&path);
    path
}

/// Asserts that a file exists and is nonempty.
pub fn assert_nonempty_file(path: impl AsRef<Path>) {
    let path = path.as_ref();
    let metadata = fs::metadata(path)
        .unwrap_or_else(|err| panic!("expected `{}` metadata: {err}", path.display()));
    assert!(
        metadata.is_file() && metadata.len() > 0,
        "expected `{}` to be a non-empty file",
        path.display()
    );
}

fn is_executable_command(path: &Path) -> bool {
    path.is_file()
}

fn download_to_path(url: &str, path: &Path) {
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("expected system time after unix epoch")
        .as_nanos();
    let tmp = path.with_extension(format!("part-{unique}-{}", std::process::id()));
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .unwrap_or_else(|err| panic!("expected `{}` directory: {err}", parent.display()));
    }

    if path.exists() {
        assert_nonempty_file(path);
        return;
    }

    let curl = find_command("curl");
    let wget = find_command("wget");
    let status = if let Some(curl) = curl {
        Command::new(curl)
            .args(["-L", "--fail", "--silent", "--show-error", "-o"])
            .arg(&tmp)
            .arg(url)
            .status()
    } else if let Some(wget) = wget {
        Command::new(wget).args(["-O"]).arg(&tmp).arg(url).status()
    } else {
        panic!("required command `curl` or `wget` is unavailable");
    }
    .unwrap_or_else(|err| panic!("expected fixture download command to start: {err}"));

    assert!(
        status.success(),
        "expected fixture download from `{url}` to succeed"
    );
    if path.exists() {
        let _ = fs::remove_file(&tmp);
        assert_nonempty_file(path);
        return;
    }
    fs::rename(&tmp, path).unwrap_or_else(|err| {
        panic!(
            "expected downloaded fixture `{}` to move into place: {err}",
            path.display()
        )
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builders_create_valid_core_fixtures() {
        let frame = checkerboard_frame(4, 2, 3);
        assert_eq!(frame.data.len(), 4 * 2 * 3);
        assert_eq!(frame.as_frame().width, 4);
        let dataset = dataset_with_scene_text_and_feature();
        assert_eq!(dataset.scenes().count(), 1);
        assert_eq!(dataset.features().count(), 1);
    }
}
