#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(
        video_analysis_features_cli::LIBRARY_CRATE,
        "video-analysis-features"
    );
    assert_eq!(video_analysis_features_cli::SURFACE_KIND, "cli");
}
