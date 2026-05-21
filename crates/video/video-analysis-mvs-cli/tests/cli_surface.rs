#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(video_analysis_mvs_cli::LIBRARY_CRATE, "video-analysis-mvs");
    assert_eq!(video_analysis_mvs_cli::SURFACE_KIND, "cli");
}
