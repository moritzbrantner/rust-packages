#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(
        video_analysis_tracking_cli::LIBRARY_CRATE,
        "video-analysis-tracking"
    );
    assert_eq!(video_analysis_tracking_cli::SURFACE_KIND, "cli");
}
