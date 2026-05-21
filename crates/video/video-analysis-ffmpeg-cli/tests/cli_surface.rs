#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(
        video_analysis_ffmpeg_cli::LIBRARY_CRATE,
        "video-analysis-ffmpeg"
    );
    assert_eq!(video_analysis_ffmpeg_cli::SURFACE_KIND, "cli");
}
