#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(video_analysis_sfm_cli::LIBRARY_CRATE, "video-analysis-sfm");
    assert_eq!(video_analysis_sfm_cli::SURFACE_KIND, "cli");
}
