#[test]
fn vanalyze_lists_model_presets_from_binary() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_vanalyze"))
        .args(["models", "presets"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("detr-resnet-50"));
    assert!(stdout.contains("minilm-l6-v2"));
}

#[test]
#[ignore = "requires real ffmpeg and ffprobe"]
fn vanalyze_detect_writes_scene_csv_for_generated_video() {
    video_analysis_test_support::require_command("ffmpeg");
    video_analysis_test_support::require_command("ffprobe");

    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("two-scenes.mp4");
    let output = dir.path().join("scenes.csv");
    video_analysis_ffmpeg::write_two_scene_test_video(&input).unwrap();

    let status = std::process::Command::new(env!("CARGO_BIN_EXE_vanalyze"))
        .arg("detect")
        .arg("--input")
        .arg(&input)
        .arg("--output")
        .arg(&output)
        .arg("--min-scene-len")
        .arg("1")
        .status()
        .unwrap();
    assert!(status.success());
    let csv = std::fs::read_to_string(output).unwrap();
    assert!(csv.contains("Scene Number,Start Frame"));
}
