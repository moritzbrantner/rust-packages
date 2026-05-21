#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(
        video_analysis_transform_cli::LIBRARY_CRATE,
        "video-analysis-transform"
    );
    assert_eq!(video_analysis_transform_cli::SURFACE_KIND, "cli");
}
