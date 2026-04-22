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
