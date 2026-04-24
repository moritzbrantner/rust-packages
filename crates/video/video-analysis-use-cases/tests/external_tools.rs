use std::path::PathBuf;

use video_analysis_use_cases::{run_youtube_video, YoutubeVideoRequest};

#[test]
#[ignore = "requires real yt-dlp network access"]
fn yt_dlp_can_resolve_default_smoke_test_video() {
    video_analysis_test_support::require_command("yt-dlp");
    let url = std::env::var("YTDLP_TEST_URL")
        .unwrap_or_else(|_| "https://www.youtube.com/watch?v=BaW_jenozKc".to_string());
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
