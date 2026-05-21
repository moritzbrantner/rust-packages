#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(
        video_analysis_data_cli::LIBRARY_CRATE,
        "video-analysis-data"
    );
    assert_eq!(video_analysis_data_cli::SURFACE_KIND, "cli");
}
