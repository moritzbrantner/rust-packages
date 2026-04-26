#![cfg(unix)]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use audio_analysis_test_support::{click_track, mixed_sources, stepped_tones, write_pcm16_wav};
use image_analysis_core::OwnedImage;
use image_analysis_io::write_image;
use video_analysis_use_cases::audio_voice_analysis::{
    run_audio_voice_analysis, AudioVoiceAnalysisRequest,
};
use video_analysis_use_cases::image_person_edit::{run_image_person_edit, ImagePersonEditRequest};
use video_analysis_use_cases::video_red_cars::{run_video_red_cars, VideoRedCarsRequest};

fn write_script(dir: &Path, name: &str, body: &str) -> PathBuf {
    let path = dir.join(name);
    fs::write(&path, body).unwrap();
    let mut permissions = fs::metadata(&path).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&path, permissions).unwrap();
    path
}

#[test]
fn video_red_cars_workflow_uses_fake_detector() {
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
fn audio_voice_analysis_workflow_uses_fake_separator_and_transcriber() {
    let dir = tempfile::tempdir().unwrap();
    let audio = dir.path().join("input.wav");
    let samples = mixed_sources(&[
        click_track(16_000, 120.0, 4.0),
        stepped_tones(&[(440.0, 2.0), (493.88, 2.0)], 16_000),
    ]);
    write_pcm16_wav(&audio, 16_000, 1, &samples).unwrap();

    let transcriber = write_script(
        dir.path(),
        "fake_transcriber.sh",
        "#!/bin/sh\ninput=\"$1\"\nshift\noutput_dir=\"\"\nwhile [ \"$#\" -gt 0 ]; do\n  if [ \"$1\" = \"--output_dir\" ]; then\n    shift\n    output_dir=\"$1\"\n  fi\n  shift\n done\nmkdir -p \"$output_dir\"\ncat > \"$output_dir/$(basename \"$input\").json\" <<'JSON'\n{\"text\":\"la la\",\"segments\":[{\"id\":0,\"start\":0.0,\"end\":0.5,\"text\":\"la la\"}]}\nJSON\n",
    );
    let demucs = write_script(
        dir.path(),
        "fake_demucs.sh",
        "#!/bin/sh\noutput_dir=\"\"\nmodel=\"htdemucs\"\ninput=\"\"\nwhile [ \"$#\" -gt 0 ]; do\n  case \"$1\" in\n    -o) shift; output_dir=\"$1\" ;;\n    -n) shift; model=\"$1\" ;;\n    --two-stems) shift ;;\n    --*) ;;\n    *) input=\"$1\" ;;\n  esac\n  shift\n done\nstem=$(basename \"$input\" .wav)\nmkdir -p \"$output_dir/$model/$stem\"\ncp \"$input\" \"$output_dir/$model/$stem/vocals.wav\"\ncp \"$input\" \"$output_dir/$model/$stem/no_vocals.wav\"\n",
    );

    let report = run_audio_voice_analysis(AudioVoiceAnalysisRequest {
        input: audio,
        work_dir: dir.path().join("work"),
        transcription: video_analysis_use_cases::TranscriptionConfig {
            enabled: true,
            engine: video_analysis_use_cases::TranscriptionEngine::Whisper,
            command: Some(video_analysis_use_cases::ExternalCommandConfig {
                command: transcriber,
                args: Vec::new(),
            }),
            whisper_cpp: video_analysis_use_cases::WhisperCppConfig::default(),
        },
        audio_separation: video_analysis_use_cases::AudioSeparationConfig {
            enabled: true,
            command: Some(video_analysis_use_cases::ExternalCommandConfig {
                command: demucs,
                args: Vec::new(),
            }),
            ..video_analysis_use_cases::AudioSeparationConfig::default()
        },
        ..AudioVoiceAnalysisRequest::default()
    })
    .unwrap();

    assert_eq!(report.transcription.status, "completed");
    assert!(report.tempo_confidence >= 0.0);
    assert!(report.sung_notes.iter().any(|note| note.note_name == "A4"));
    assert!(report
        .assets
        .voice_stem
        .as_deref()
        .unwrap()
        .contains("vocals.wav"));
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
