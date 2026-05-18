//! Public API for the video-analysis-test-support crate.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use num_rational::Rational64;
use video_analysis_core::{
    AnalysisEvent, FramePosition, Observation, ObservationKind, OwnedTextSegment, OwnedVideoFrame,
    PixelFormat, Scene, TextSegment, Timebase, Timestamp,
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
