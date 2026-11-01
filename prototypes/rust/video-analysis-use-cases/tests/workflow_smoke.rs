#![cfg(unix)]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use image_analysis_core::OwnedImage;
use image_analysis_io::write_image;
use video_analysis_use_cases::image_person_edit::{
    run_image_person_edit, ImagePersonEditRequest, PersonDetectionReport, PersonDetectorConfig,
};
use video_analysis_use_cases::video_red_cars::{run_video_red_cars, VideoRedCarsRequest};
use video_analysis_use_cases::RegionReport;

fn write_script(dir: &Path, name: &str, body: &str) -> PathBuf {
    let path = dir.join(name);
    fs::write(&path, body).unwrap();
    let mut permissions = fs::metadata(&path).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&path, permissions).unwrap();
    path
}

fn media_tools_available() -> bool {
    video_analysis_ffmpeg::is_ffmpeg_available() && video_analysis_ffmpeg::is_ffprobe_available()
}

#[test]
fn video_red_cars_workflow_uses_fake_detector() {
    if !media_tools_available() {
        eprintln!("skipping red-car workflow smoke test because ffmpeg/ffprobe is unavailable");
        return;
    }

    let dir = tempfile::tempdir().unwrap();
    let video = dir.path().join("video.mp4");
    video_analysis_ffmpeg::write_two_scene_test_video(&video).unwrap();
    let detector = write_script(
        dir.path(),
        "fake_detector.sh",
        "#!/bin/sh\ncat >/dev/null\nprintf '{\"predictions\":[{\"kind\":\"object\",\"label\":\"car\",\"score\":0.9,\"region\":{\"x\":2,\"y\":2,\"width\":16,\"height\":16},\"attributes\":{\"color\":\"red\"}}]}'\n",
    );

    let report = run_video_red_cars(VideoRedCarsRequest {
        input: video,
        work_dir: dir.path().join("work"),
        vehicle_detector_command: detector,
        min_scene_len: 1,
        visual_sample_every: 5,
        ..VideoRedCarsRequest::default()
    })
    .unwrap();

    assert_eq!(report.video.scenes.len(), 2);
    assert!(report
        .video
        .scenes
        .iter()
        .all(|scene| scene.red_car_count >= 1));
}

#[test]
fn image_person_edit_workflow_uses_fake_detector_and_editor() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("input.png");
    write_image(
        &input,
        &OwnedImage::new_rgb(32, 32, vec![220; 32 * 32 * 3]).unwrap(),
    )
    .unwrap();

    let detector = write_script(
        dir.path(),
        "fake_detector.sh",
        "#!/bin/sh\nprintf '{\"predictions\":[{\"kind\":\"object\",\"label\":\"person\",\"score\":0.95,\"region\":{\"x\":8,\"y\":8,\"width\":8,\"height\":12},\"attributes\":{}}]}'\n",
    );
    let editor = write_script(
        dir.path(),
        "fake_editor.sh",
        "#!/bin/sh\ncat >/dev/null\nprintf '{\"status\":\"completed\",\"output_image\":\"edited.png\",\"message\":\"ok\",\"metadata\":{\"backend\":\"fake\"}}'\n",
    );

    let report = run_image_person_edit(ImagePersonEditRequest {
        input,
        work_dir: dir.path().join("work"),
        prompt: "replace the person with a statue".to_string(),
        model: "flux1-dev.safetensors".to_string(),
        person_detector_command: detector,
        editor_command: Some(editor),
        ..ImagePersonEditRequest::default()
    })
    .unwrap();

    assert_eq!(report.detections.len(), 1);
    assert_eq!(report.editing.status, "completed");
    assert!(Path::new(&report.assets.person_mask).exists());
    assert!(Path::new(&report.assets.workflow_json).exists());
}

#[test]
fn image_person_edit_workflow_accepts_editor_that_closes_stdin() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("input.png");
    write_image(
        &input,
        &OwnedImage::new_rgb(32, 32, vec![220; 32 * 32 * 3]).unwrap(),
    )
    .unwrap();

    let detector = write_script(
        dir.path(),
        "fake_detector.sh",
        "#!/bin/sh\nprintf '{\"predictions\":[{\"kind\":\"object\",\"label\":\"person\",\"score\":0.95,\"region\":{\"x\":8,\"y\":8,\"width\":8,\"height\":12},\"attributes\":{}}]}'\n",
    );
    let editor = write_script(
        dir.path(),
        "fake_editor.sh",
        "#!/bin/sh\nexec 0<&-\nprintf '{\"status\":\"completed\",\"output_image\":\"edited.png\",\"message\":\"ok\",\"metadata\":{\"backend\":\"fake\"}}'\n",
    );

    let report = run_image_person_edit(ImagePersonEditRequest {
        input,
        work_dir: dir.path().join("work"),
        prompt: "replace the person with a statue ".repeat(20_000),
        model: "flux1-dev.safetensors".to_string(),
        person_detector_command: detector,
        editor_command: Some(editor),
        ..ImagePersonEditRequest::default()
    })
    .unwrap();

    assert_eq!(report.editing.status, "completed");
    assert_eq!(
        report.editing.metadata.get("backend").map(String::as_str),
        Some("fake")
    );
}

#[test]
fn image_person_edit_workflow_uses_stub_detector_and_planned_comfyui() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("input.png");
    write_image(
        &input,
        &OwnedImage::new_rgb(32, 32, vec![220; 32 * 32 * 3]).unwrap(),
    )
    .unwrap();

    let report = run_image_person_edit(ImagePersonEditRequest {
        input,
        work_dir: dir.path().join("work"),
        prompt: "replace the person with a statue".to_string(),
        model: "flux1-dev.safetensors".to_string(),
        person_detector: PersonDetectorConfig::Stub {
            detections: vec![PersonDetectionReport {
                label: "person".to_string(),
                score: Some(0.95),
                region: RegionReport {
                    x: 8,
                    y: 8,
                    width: 8,
                    height: 12,
                },
                attributes: Default::default(),
            }],
        },
        ..ImagePersonEditRequest::default()
    })
    .unwrap();

    assert_eq!(report.detections.len(), 1);
    assert_eq!(report.editing.status, "planned");
    assert!(report
        .capabilities
        .skipped
        .iter()
        .any(|value| value.contains("COMFYUI_URL")));
    assert!(Path::new(&report.assets.workflow_json).exists());
}
