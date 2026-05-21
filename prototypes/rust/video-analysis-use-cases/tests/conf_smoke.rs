#[test]
fn video_analysis_use_cases_reads_package_conf_from_current_dir() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("video-analysis-use-cases.conf"),
        "youtube-video --help",
    )
    .unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_video-analysis-use-cases"))
        .current_dir(dir.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    let combined = format!(
        "{}{}",
        String::from_utf8(output.stdout).unwrap(),
        String::from_utf8(output.stderr).unwrap()
    );
    assert!(combined.contains("youtube-video"));
    assert!(combined.contains("--scene-threshold"));
}

#[test]
fn video_analysis_use_cases_reads_explicit_config_file() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("custom.conf"), "youtube-video --help").unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_video-analysis-use-cases"))
        .current_dir(dir.path())
        .args(["--config", "custom.conf"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let combined = format!(
        "{}{}",
        String::from_utf8(output.stdout).unwrap(),
        String::from_utf8(output.stderr).unwrap()
    );
    assert!(combined.contains("youtube-video"));
    assert!(combined.contains("--scene-threshold"));
}
