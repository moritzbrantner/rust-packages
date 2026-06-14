use std::path::PathBuf;

use audio_analysis_test_support::{click_track, mixed_sources, stepped_tones, write_pcm16_wav};
use image_analysis_core::OwnedImage;
use image_analysis_io::write_image;
use video_analysis_use_cases::audio_voice_analysis::{
    run_audio_voice_analysis, AudioVoiceAnalysisRequest,
};
use video_analysis_use_cases::image_person_edit::{run_image_person_edit, ImagePersonEditRequest};
use video_analysis_use_cases::video_red_cars::{run_video_red_cars, VideoRedCarsRequest};
use video_analysis_use_cases::{run_youtube_video, YoutubeVideoRequest};

#[test]
#[ignore = "requires real yt-dlp network access"]
fn yt_dlp_can_resolve_default_smoke_test_video() {
    video_analysis_test_support::require_command("yt-dlp");
    let url = std::env::var("YTDLP_TEST_URL")
        .unwrap_or_else(|_| "https://www.youtube.com/watch?v=jNQXAC9IVRw".to_string());
    let output = std::process::Command::new("yt-dlp")
        .args(["--simulate", "--skip-download", "--print", "id"])
        .arg(url)
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(!String::from_utf8(output.stdout).unwrap().trim().is_empty());
}

fn opencv_person_detector_script() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../scripts/opencv_person_detector.py")
}

fn opencv_red_car_detector_script() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../scripts/opencv_red_car_detector.py")
}

#[test]
#[ignore = "downloads Me at the zoo from Wikimedia Commons and requires ffmpeg"]
fn me_at_the_zoo_workflow_reports_one_scene() {
    if !(video_analysis_ffmpeg::is_ffmpeg_available()
        && video_analysis_ffmpeg::is_ffprobe_available())
    {
        eprintln!("skipping Me at the Zoo workflow test because ffmpeg/ffprobe is unavailable");
        return;
    }

    let video = video_analysis_test_support::ensure_me_at_the_zoo_fixture();
    let dir = tempfile::tempdir().unwrap();
    let report = run_youtube_video(YoutubeVideoRequest {
        input: Some(video),
        work_dir: dir.path().join("work"),
        output: Some(dir.path().join("analysis.json")),
        skip_transcription: true,
        visual_sample_every: 30,
        ..YoutubeVideoRequest::default()
    })
    .unwrap();

    assert_eq!(report.video.scenes.len(), 1);
    assert_eq!(report.video.frames_processed, 285);
    assert_eq!(report.video.scenes[0].start_frame, 0);
    assert_eq!(report.video.scenes[0].end_frame, 285);
}

#[test]
#[ignore = "downloads Me at the zoo from Wikimedia Commons and requires ffmpeg + python3 + cv2"]
fn me_at_the_zoo_workflow_counts_one_person_per_sampled_frame() {
    if !(video_analysis_ffmpeg::is_ffmpeg_available()
        && video_analysis_ffmpeg::is_ffprobe_available())
    {
        eprintln!(
            "skipping Me at the Zoo workflow object test because ffmpeg/ffprobe is unavailable"
        );
        return;
    }

    let Some(python) = video_analysis_test_support::find_python_with_modules(&["cv2", "numpy"])
    else {
        eprintln!(
            "skipping Me at the Zoo workflow object test because python cv2/numpy is unavailable"
        );
        return;
    };

    let video = video_analysis_test_support::ensure_me_at_the_zoo_fixture();
    let dir = tempfile::tempdir().unwrap();
    let report = run_youtube_video(YoutubeVideoRequest {
        input: Some(video),
        work_dir: dir.path().join("work"),
        output: Some(dir.path().join("analysis.json")),
        max_frames: Some(121),
        skip_transcription: true,
        object_command: Some(python),
        object_args: vec![opencv_person_detector_script().display().to_string()],
        visual_sample_every: 30,
        ..YoutubeVideoRequest::default()
    })
    .unwrap();

    assert!(report
        .capabilities
        .completed
        .contains(&"object_person_detection".to_string()));

    let mut total_person_observations = 0_usize;
    let mut max_persons_per_frame = 0_usize;
    let mut current_frame = None;
    let mut current_count = 0_usize;
    for observation in report
        .video
        .observations
        .iter()
        .filter(|observation| observation.label.as_deref() == Some("person"))
    {
        total_person_observations += 1;
        if current_frame == observation.frame_index {
            current_count += 1;
        } else {
            max_persons_per_frame = max_persons_per_frame.max(current_count);
            current_frame = observation.frame_index;
            current_count = 1;
        }
    }
    max_persons_per_frame = max_persons_per_frame.max(current_count);

    assert!(total_person_observations >= 4);
    assert_eq!(max_persons_per_frame, 1);
}

#[test]
#[ignore = "requires ffmpeg + python3 + cv2/numpy"]
fn generated_video_red_cars_workflow_counts_red_cars() {
    if !(video_analysis_ffmpeg::is_ffmpeg_available()
        && video_analysis_ffmpeg::is_ffprobe_available())
    {
        eprintln!("skipping red-car workflow external test because ffmpeg/ffprobe is unavailable");
        return;
    }
    let Some(python) = video_analysis_test_support::find_python_with_modules(&["cv2", "numpy"])
    else {
        eprintln!(
            "skipping red-car workflow external test because python cv2/numpy is unavailable"
        );
        return;
    };

    let dir = tempfile::tempdir().unwrap();
    let video = dir.path().join("red-cars.mp4");
    let status = std::process::Command::new("ffmpeg")
        .args([
            "-y",
            "-v",
            "error",
            "-f",
            "lavfi",
            "-i",
            "color=c=black:s=96x96:d=0.5:r=10",
            "-f",
            "lavfi",
            "-i",
            "color=c=white:s=96x96:d=0.5:r=10",
            "-filter_complex",
            "[0:v]drawbox=x=10:y=20:w=30:h=18:color=red:t=fill[a];[1:v]drawbox=x=40:y=30:w=24:h=16:color=red:t=fill[b];[a][b]concat=n=2:v=1:a=0",
            "-pix_fmt",
            "yuv420p",
        ])
        .arg(&video)
        .status()
        .unwrap();
    assert!(status.success());

    let report = run_video_red_cars(VideoRedCarsRequest {
        input: video,
        work_dir: dir.path().join("work"),
        min_scene_len: 1,
        visual_sample_every: 5,
        vehicle_detector_command: python,
        vehicle_detector_args: vec![opencv_red_car_detector_script().display().to_string()],
        ..VideoRedCarsRequest::default()
    })
    .unwrap();

    assert!(report.video.scenes.len() >= 2);
    assert!(
        report
            .video
            .scenes
            .iter()
            .map(|scene| scene.red_car_count)
            .sum::<u64>()
            >= 2
    );
}

#[test]
#[ignore = "requires ffmpeg + demucs; optional transcription via env"]
fn audio_voice_analysis_workflow_runs_with_real_tools_when_configured() {
    if !(video_analysis_ffmpeg::is_ffmpeg_available()
        && video_analysis_ffmpeg::is_ffprobe_available())
    {
        eprintln!(
            "skipping audio voice analysis external test because ffmpeg/ffprobe is unavailable"
        );
        return;
    }
    if !audio_analysis_separation::is_demucs_available() {
        eprintln!("skipping audio voice analysis external test because demucs is unavailable");
        return;
    }

    let dir = tempfile::tempdir().unwrap();
    let audio = dir.path().join("voice.wav");
    let samples = mixed_sources(&[
        click_track(16_000, 120.0, 4.0),
        stepped_tones(&[(440.0, 2.0), (493.88, 2.0)], 16_000),
    ]);
    write_pcm16_wav(&audio, 16_000, 1, &samples).unwrap();

    let transcriber_command = std::env::var("AUDIO_VOICE_ANALYSIS_TRANSCRIBER")
        .ok()
        .map(PathBuf::from);
    let report = run_audio_voice_analysis(AudioVoiceAnalysisRequest {
        input: audio,
        work_dir: dir.path().join("work"),
        transcription: if let Some(command) = transcriber_command {
            video_analysis_use_cases::TranscriptionConfig {
                enabled: true,
                engine: video_analysis_use_cases::TranscriptionEngine::Whisper,
                command: Some(video_analysis_use_cases::ExternalCommandConfig {
                    command,
                    args: Vec::new(),
                }),
                whisper_cpp: video_analysis_use_cases::WhisperCppConfig::default(),
            }
        } else {
            video_analysis_use_cases::TranscriptionConfig {
                enabled: false,
                ..video_analysis_use_cases::TranscriptionConfig::default()
            }
        },
        ..AudioVoiceAnalysisRequest::default()
    })
    .unwrap();

    assert!(report.separation.is_some());
    assert!(!report.sung_notes.is_empty());
}

#[test]
#[ignore = "requires configured detector/editor commands"]
fn image_person_edit_workflow_runs_with_real_tools_when_configured() {
    let Some(input) = std::env::var_os("IMAGE_PERSON_EDIT_INPUT") else {
        eprintln!(
            "skipping image person edit external test because IMAGE_PERSON_EDIT_INPUT is unset"
        );
        return;
    };
    let Some(detector_command) = std::env::var_os("IMAGE_PERSON_EDIT_DETECTOR_COMMAND") else {
        eprintln!("skipping image person edit external test because IMAGE_PERSON_EDIT_DETECTOR_COMMAND is unset");
        return;
    };

    let dir = tempfile::tempdir().unwrap();
    let local_input = dir.path().join("input.png");
    let _ = std::fs::copy(&input, &local_input).unwrap_or_else(|_| {
        write_image(
            &local_input,
            &OwnedImage::new_rgb(32, 32, vec![220; 32 * 32 * 3]).unwrap(),
        )
        .unwrap();
        0
    });

    let editor_command = std::env::var_os("IMAGE_PERSON_EDIT_EDITOR_COMMAND").map(PathBuf::from);
    let report = run_image_person_edit(ImagePersonEditRequest {
        input: local_input,
        work_dir: dir.path().join("work"),
        prompt: "replace the person with a statue".to_string(),
        model: "flux1-dev.safetensors".to_string(),
        person_detector_command: detector_command.into(),
        editor_command,
        ..ImagePersonEditRequest::default()
    });

    match report {
        Ok(report) => {
            assert!(!report.detections.is_empty());
            assert!(std::path::Path::new(&report.assets.workflow_json).exists());
        }
        Err(error) => {
            if std::env::var_os("STRICT_EXTERNAL_RUNTIME_CHECKS").is_some() {
                panic!(
                    "image person edit detector/editor failed under STRICT_EXTERNAL_RUNTIME_CHECKS: {error}"
                );
            }
            eprintln!(
                "skipping image person edit external test because detector/editor failed: {error}"
            );
        }
    }
}
